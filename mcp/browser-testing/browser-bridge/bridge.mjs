#!/usr/bin/env node
/**
 * Playwright JSON-RPC bridge for rmcp browser testing MCP server.
 * Protocol: one JSON object per line on stdin; one JSON response per line on stdout.
 * { "id": number|string, "method": string, "params": object }
 * { "id": ..., "result": ... } | { "id": ..., "error": { "message": string, "data"?: any } }
 */
import { chromium, firefox, webkit, devices, request as pwRequest } from 'playwright';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import readline from 'node:readline';

const require = createRequire(import.meta.url);
const __dirname = path.dirname(fileURLToPath(import.meta.url));

/** @type {Map<string, any>} */
const browsers = new Map();
/** @type {Map<string, any>} */
const contexts = new Map();
/** @type {Map<string, any>} */
const pages = new Map();
/** @type {Map<string, any>} */
const apiRequests = new Map();
/** @type {Map<string, object>} */
const meta = new Map(); // pageId -> { console: [], network: [], errors: [], dialogs: [] }
/** @type {Map<string, any>} */
const recordings = new Map();

let idSeq = 1;
const nid = (p = 'x') => `${p}_${idSeq++}`;

function ok(id, result) {
  process.stdout.write(JSON.stringify({ id, result }) + '\n');
}
function fail(id, message, data) {
  process.stdout.write(JSON.stringify({ id, error: { message, data } }) + '\n');
}

function pageMeta(pageId) {
  if (!meta.has(pageId)) {
    meta.set(pageId, { console: [], network: [], errors: [], dialogs: [], downloads: [] });
  }
  return meta.get(pageId);
}

function attachPageListeners(page, pageId) {
  const m = pageMeta(pageId);
  page.on('console', (msg) => {
    m.console.push({ type: msg.type(), text: msg.text(), ts: Date.now() });
    if (m.console.length > 2000) m.console.shift();
  });
  page.on('pageerror', (err) => {
    m.errors.push({ message: String(err), ts: Date.now() });
    if (m.errors.length > 500) m.errors.shift();
  });
  page.on('request', (req) => {
    m.network.push({
      kind: 'request',
      method: req.method(),
      url: req.url(),
      resourceType: req.resourceType(),
      ts: Date.now(),
    });
    if (m.network.length > 4000) m.network.shift();
  });
  page.on('response', async (res) => {
    m.network.push({
      kind: 'response',
      method: res.request().method(),
      url: res.url(),
      status: res.status(),
      ts: Date.now(),
    });
    if (m.network.length > 4000) m.network.shift();
  });
  page.on('download', async (download) => {
    m.downloads.push({
      url: download.url(),
      suggestedFilename: download.suggestedFilename(),
      ts: Date.now(),
    });
  });
  page.on('dialog', async (dialog) => {
    m.dialogs.push({ type: dialog.type(), message: dialog.message(), ts: Date.now() });
    // default: accept — can be overridden via dialog_action tool before trigger
    const policy = page.__dialogPolicy || 'accept';
    try {
      if (policy === 'dismiss') await dialog.dismiss();
      else if (policy === 'accept') await dialog.accept(page.__dialogPromptText || undefined);
      else await dialog.dismiss();
    } catch {
      /* already handled */
    }
  });
}

function getPage(pageId) {
  const p = pages.get(pageId);
  if (!p) throw new Error(`unknown pageId: ${pageId}`);
  return p;
}
function getContext(contextId) {
  const c = contexts.get(contextId);
  if (!c) throw new Error(`unknown contextId: ${contextId}`);
  return c;
}
function getBrowser(browserId) {
  const b = browsers.get(browserId);
  if (!b) throw new Error(`unknown browserId: ${browserId}`);
  return b;
}

async function resolveLocator(page, params) {
  if (params.locator && typeof params.locator === 'object') {
    return buildLocator(page, params.locator);
  }
  if (params.role) {
    let loc = page.getByRole(params.role, {
      name: params.name,
      exact: params.exact ?? false,
    });
    if (params.nth != null) loc = loc.nth(params.nth);
    return loc;
  }
  if (params.testId) return page.getByTestId(params.testId);
  if (params.label) return page.getByLabel(params.label, { exact: params.exact ?? false });
  if (params.placeholder) return page.getByPlaceholder(params.placeholder);
  if (params.altText) return page.getByAltText(params.altText);
  if (params.text) return page.getByText(params.text, { exact: params.exact ?? false });
  if (params.selector) {
    let loc = page.locator(params.selector);
    if (params.nth != null) loc = loc.nth(params.nth);
    if (params.hasText) loc = loc.filter({ hasText: params.hasText });
    return loc;
  }
  if (params.xpath) return page.locator(`xpath=${params.xpath}`);
  throw new Error('locator required: provide selector, role, testId, label, text, xpath, or locator object');
}

function buildLocator(page, spec) {
  let loc;
  if (spec.role) loc = page.getByRole(spec.role, { name: spec.name, exact: spec.exact ?? false });
  else if (spec.testId) loc = page.getByTestId(spec.testId);
  else if (spec.label) loc = page.getByLabel(spec.label, { exact: spec.exact ?? false });
  else if (spec.placeholder) loc = page.getByPlaceholder(spec.placeholder);
  else if (spec.altText) loc = page.getByAltText(spec.altText);
  else if (spec.text) loc = page.getByText(spec.text, { exact: spec.exact ?? false });
  else if (spec.selector) loc = page.locator(spec.selector);
  else if (spec.xpath) loc = page.locator(`xpath=${spec.xpath}`);
  else throw new Error('invalid locator object');

  if (spec.hasText) loc = loc.filter({ hasText: spec.hasText });
  if (spec.has) loc = loc.filter({ has: buildLocator(page, spec.has) });
  if (spec.nth != null) loc = loc.nth(spec.nth);
  return loc;
}

const handlers = {
  async ping() {
    return { pong: true, playwright: true };
  },

  async launch(params = {}) {
    const engine = (params.browser || params.engine || 'chromium').toLowerCase();
    const launcher = { chromium, firefox, webkit }[engine];
    if (!launcher) throw new Error(`unsupported browser engine: ${engine}`);
    const headless = params.headless !== false;
    const launchOpts = {
      headless,
      slowMo: params.slowMo || 0,
      devtools: !!params.devtools,
      args: params.args || [],
    };
    if (params.channel) launchOpts.channel = params.channel;
    if (params.proxy) launchOpts.proxy = params.proxy;
    if (params.downloadsPath) launchOpts.downloadsPath = params.downloadsPath;
    if (params.tracesDir) launchOpts.tracesDir = params.tracesDir;
    if (params.executablePath) launchOpts.executablePath = params.executablePath;
    if (params.ignoreHTTPSErrors != null) {
      // applied on context
    }
    const browser = await launcher.launch(launchOpts);
    const browserId = nid('browser');
    browsers.set(browserId, browser);
    return {
      browserId,
      engine,
      headless,
      version: browser.version(),
    };
  },

  async connect_cdp(params = {}) {
    if (!params.endpoint) throw new Error('endpoint required');
    const browser = await chromium.connectOverCDP(params.endpoint);
    const browserId = nid('browser');
    browsers.set(browserId, browser);
    const ctx = browser.contexts()[0];
    let contextId;
    if (ctx) {
      contextId = nid('ctx');
      contexts.set(contextId, ctx);
    }
    return { browserId, contextId, engine: 'chromium', connected: true };
  },

  async close_browser(params) {
    const b = getBrowser(params.browserId);
    await b.close();
    browsers.delete(params.browserId);
    for (const [cid, c] of [...contexts.entries()]) {
      if (c.browser && c.browser() === b) contexts.delete(cid);
    }
    return { closed: true };
  },

  async context_new(params) {
    const browser = getBrowser(params.browserId);
    const opts = {
      viewport: params.viewport || { width: 1280, height: 720 },
      userAgent: params.userAgent,
      locale: params.locale,
      timezoneId: params.timezone,
      geolocation: params.geolocation,
      permissions: params.permissions,
      colorScheme: params.colorScheme,
      reducedMotion: params.reducedMotion,
      forcedColors: params.forcedColors,
      offline: params.offline,
      ignoreHTTPSErrors: params.ignoreHTTPSErrors ?? true,
      acceptDownloads: params.acceptDownloads !== false,
      recordVideo: params.recordVideo
        ? { dir: params.recordVideo.dir || params.videoDir, size: params.recordVideo.size }
        : undefined,
      storageState: params.storageState,
      extraHTTPHeaders: params.extraHTTPHeaders,
      httpCredentials: params.httpCredentials,
      proxy: params.proxy,
      baseURL: params.baseURL,
      deviceScaleFactor: params.deviceScaleFactor,
      isMobile: params.isMobile,
      hasTouch: params.hasTouch,
      javaScriptEnabled: params.javaScriptEnabled,
    };
    if (params.device) {
      const d = devices[params.device];
      if (!d) throw new Error(`unknown device: ${params.device}. Use list_devices.`);
      Object.assign(opts, d);
    }
    if (params.userDataDir) {
      // persistent context path is launchPersistentContext — handle separately
    }
    const context = await browser.newContext(opts);
    if (params.grantPermissions?.length) {
      await context.grantPermissions(params.grantPermissions, {
        origin: params.permissionOrigin,
      });
    }
    const contextId = nid('ctx');
    contexts.set(contextId, context);
    if (params.trace) {
      await context.tracing.start({
        screenshots: true,
        snapshots: true,
        sources: true,
      });
      context.__traceEnabled = true;
    }
    return { contextId };
  },

  async context_persistent(params) {
    const engine = (params.browser || 'chromium').toLowerCase();
    const launcher = { chromium, firefox, webkit }[engine];
    if (!launcher) throw new Error(`unsupported browser: ${engine}`);
    if (!params.userDataDir) throw new Error('userDataDir required');
    const context = await launcher.launchPersistentContext(params.userDataDir, {
      headless: params.headless !== false,
      viewport: params.viewport || { width: 1280, height: 720 },
      locale: params.locale,
      timezoneId: params.timezone,
      ignoreHTTPSErrors: params.ignoreHTTPSErrors ?? true,
      args: params.args || [],
    });
    const contextId = nid('ctx');
    contexts.set(contextId, context);
    // also store browser-like handle
    const browserId = nid('browser');
    browsers.set(browserId, {
      close: async () => context.close(),
      version: () => 'persistent',
      contexts: () => [context],
      __persistent: true,
    });
    return { browserId, contextId, persistent: true };
  },

  async context_close(params) {
    const c = getContext(params.contextId);
    await c.close();
    contexts.delete(params.contextId);
    return { closed: true };
  },

  async context_storage_state(params) {
    const c = getContext(params.contextId);
    const state = await c.storageState({ path: params.path });
    return { storageState: state, path: params.path || null };
  },

  async context_add_cookies(params) {
    const c = getContext(params.contextId);
    await c.addCookies(params.cookies || []);
    return { ok: true };
  },

  async context_clear_cookies(params) {
    const c = getContext(params.contextId);
    await c.clearCookies();
    return { ok: true };
  },

  async context_set_offline(params) {
    const c = getContext(params.contextId);
    await c.setOffline(!!params.offline);
    return { offline: !!params.offline };
  },

  async context_set_geolocation(params) {
    const c = getContext(params.contextId);
    await c.setGeolocation(params.geolocation);
    return { ok: true };
  },

  async context_grant_permissions(params) {
    const c = getContext(params.contextId);
    await c.grantPermissions(params.permissions || [], { origin: params.origin });
    return { ok: true };
  },

  async context_set_extra_headers(params) {
    const c = getContext(params.contextId);
    await c.setExtraHTTPHeaders(params.headers || {});
    return { ok: true };
  },

  async tracing_start(params) {
    const c = getContext(params.contextId);
    await c.tracing.start({
      screenshots: params.screenshots !== false,
      snapshots: params.snapshots !== false,
      sources: params.sources !== false,
      name: params.name,
    });
    c.__traceEnabled = true;
    return { started: true };
  },

  async tracing_stop(params) {
    const c = getContext(params.contextId);
    const out = params.path;
    if (!out) throw new Error('path required for tracing_stop');
    fs.mkdirSync(path.dirname(out), { recursive: true });
    await c.tracing.stop({ path: out });
    c.__traceEnabled = false;
    return { path: out };
  },

  async page_new(params) {
    const c = getContext(params.contextId);
    const page = await c.newPage();
    const pageId = nid('page');
    pages.set(pageId, page);
    page.__contextId = params.contextId;
    attachPageListeners(page, pageId);
    if (params.url) await page.goto(params.url, { waitUntil: params.waitUntil || 'load' });
    return { pageId, url: page.url() };
  },

  async page_close(params) {
    const p = getPage(params.pageId);
    await p.close();
    pages.delete(params.pageId);
    meta.delete(params.pageId);
    return { closed: true };
  },

  async page_list(params = {}) {
    const out = [];
    for (const [pageId, page] of pages) {
      if (params.contextId && page.__contextId !== params.contextId) continue;
      out.push({
        pageId,
        contextId: page.__contextId,
        url: page.url(),
        title: await page.title().catch(() => ''),
      });
    }
    return { pages: out };
  },

  async page_bring_to_front(params) {
    const p = getPage(params.pageId);
    await p.bringToFront();
    return { ok: true };
  },

  async goto(params) {
    const p = getPage(params.pageId);
    const resp = await p.goto(params.url, {
      waitUntil: params.waitUntil || 'load',
      timeout: params.timeout,
      referer: params.referer,
    });
    return {
      url: p.url(),
      status: resp ? resp.status() : null,
      ok: resp ? resp.ok() : null,
    };
  },

  async go_back(params) {
    const p = getPage(params.pageId);
    await p.goBack({ waitUntil: params.waitUntil || 'load' });
    return { url: p.url() };
  },

  async go_forward(params) {
    const p = getPage(params.pageId);
    await p.goForward({ waitUntil: params.waitUntil || 'load' });
    return { url: p.url() };
  },

  async reload(params) {
    const p = getPage(params.pageId);
    await p.reload({ waitUntil: params.waitUntil || 'load' });
    return { url: p.url() };
  },

  async wait_for_url(params) {
    const p = getPage(params.pageId);
    await p.waitForURL(params.url, { timeout: params.timeout });
    return { url: p.url() };
  },

  async wait_for_load_state(params) {
    const p = getPage(params.pageId);
    await p.waitForLoadState(params.state || 'load', { timeout: params.timeout });
    return { state: params.state || 'load', url: p.url() };
  },

  async wait_for_selector(params) {
    const p = getPage(params.pageId);
    const loc = await resolveLocator(p, params);
    await loc.waitFor({ state: params.state || 'visible', timeout: params.timeout });
    return { ok: true };
  },

  async click(params) {
    const p = getPage(params.pageId);
    const loc = await resolveLocator(p, params);
    await loc.click({
      button: params.button,
      clickCount: params.clickCount,
      delay: params.delay,
      force: params.force,
      modifiers: params.modifiers,
      position: params.position,
      timeout: params.timeout,
      trial: params.trial,
    });
    return { ok: true, url: p.url() };
  },

  async dblclick(params) {
    const p = getPage(params.pageId);
    const loc = await resolveLocator(p, params);
    await loc.dblclick({ timeout: params.timeout, force: params.force });
    return { ok: true };
  },

  async hover(params) {
    const p = getPage(params.pageId);
    const loc = await resolveLocator(p, params);
    await loc.hover({ timeout: params.timeout, force: params.force });
    return { ok: true };
  },

  async focus(params) {
    const p = getPage(params.pageId);
    const loc = await resolveLocator(p, params);
    await loc.focus({ timeout: params.timeout });
    return { ok: true };
  },

  async fill(params) {
    const p = getPage(params.pageId);
    const loc = await resolveLocator(p, params);
    await loc.fill(String(params.value ?? ''), { timeout: params.timeout, force: params.force });
    return { ok: true };
  },

  async type(params) {
    const p = getPage(params.pageId);
    // `text`/`content` is the string to type — do not use it as a text locator.
    const typeContent = params.content ?? params.text ?? '';
    const locParams = { ...params };
    delete locParams.text;
    delete locParams.content;
    const loc = await resolveLocator(p, locParams);
    await loc.pressSequentially(String(typeContent), {
      delay: params.delay,
      timeout: params.timeout,
    });
    return { ok: true };
  },

  async press(params) {
    const p = getPage(params.pageId);
    if (params.selector || params.role || params.testId || params.locator) {
      const loc = await resolveLocator(p, params);
      await loc.press(params.key, { delay: params.delay, timeout: params.timeout });
    } else {
      await p.keyboard.press(params.key, { delay: params.delay });
    }
    return { ok: true };
  },

  async select_option(params) {
    const p = getPage(params.pageId);
    const loc = await resolveLocator(p, params);
    const values = await loc.selectOption(
      params.value != null
        ? params.value
        : params.label != null
          ? { label: params.label }
          : params.index != null
            ? { index: params.index }
            : params.values || [],
      { timeout: params.timeout },
    );
    return { values };
  },

  async check(params) {
    const p = getPage(params.pageId);
    const loc = await resolveLocator(p, params);
    await loc.check({ timeout: params.timeout, force: params.force });
    return { ok: true };
  },

  async uncheck(params) {
    const p = getPage(params.pageId);
    const loc = await resolveLocator(p, params);
    await loc.uncheck({ timeout: params.timeout, force: params.force });
    return { ok: true };
  },

  async set_input_files(params) {
    const p = getPage(params.pageId);
    const loc = await resolveLocator(p, params);
    await loc.setInputFiles(params.files || params.path, { timeout: params.timeout });
    return { ok: true };
  },

  async drag_to(params) {
    const p = getPage(params.pageId);
    const source = await resolveLocator(p, params.source || params);
    const target = await resolveLocator(p, params.target);
    await source.dragTo(target, { timeout: params.timeout, force: params.force });
    return { ok: true };
  },

  async scroll(params) {
    const p = getPage(params.pageId);
    if (params.selector || params.role || params.testId || params.locator) {
      const loc = await resolveLocator(p, params);
      await loc.scrollIntoViewIfNeeded({ timeout: params.timeout });
    } else {
      await p.evaluate(
        ({ x, y }) => window.scrollBy(x || 0, y || 0),
        { x: params.x, y: params.y },
      );
    }
    return { ok: true };
  },

  async mouse_click(params) {
    const p = getPage(params.pageId);
    await p.mouse.click(params.x, params.y, {
      button: params.button,
      clickCount: params.clickCount,
    });
    return { ok: true };
  },

  async mouse_wheel(params) {
    const p = getPage(params.pageId);
    await p.mouse.wheel(params.deltaX || 0, params.deltaY || 0);
    return { ok: true };
  },

  async touchscreen_tap(params) {
    const p = getPage(params.pageId);
    await p.touchscreen.tap(params.x, params.y);
    return { ok: true };
  },

  async set_viewport(params) {
    const p = getPage(params.pageId);
    await p.setViewportSize({ width: params.width, height: params.height });
    return { width: params.width, height: params.height };
  },

  async set_dialog_policy(params) {
    const p = getPage(params.pageId);
    p.__dialogPolicy = params.policy || 'accept';
    p.__dialogPromptText = params.promptText;
    return { policy: p.__dialogPolicy };
  },

  async evaluate(params) {
    const p = getPage(params.pageId);
    const result = await p.evaluate(params.expression, params.arg);
    return { result };
  },

  async title(params) {
    const p = getPage(params.pageId);
    return { title: await p.title(), url: p.url() };
  },

  async content(params) {
    const p = getPage(params.pageId);
    const html = await p.content();
    const max = params.maxChars || 200_000;
    return {
      html: html.length > max ? html.slice(0, max) + '\n<!-- truncated -->' : html,
      truncated: html.length > max,
      length: html.length,
    };
  },

  async snapshot_a11y(params) {
    const p = getPage(params.pageId);
    // Prefer Playwright aria snapshot when available
    try {
      const body = p.locator('body');
      if (typeof body.ariaSnapshot === 'function') {
        const snap = await body.ariaSnapshot();
        return { format: 'aria', snapshot: snap, url: p.url(), title: await p.title() };
      }
    } catch {
      /* fall through */
    }
    const tree = await p.evaluate(() => {
      const interesting = new Set([
        'a', 'button', 'input', 'select', 'textarea', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
        'nav', 'main', 'header', 'footer', 'form', 'label', 'img', 'summary', 'dialog',
      ]);
      function roleOf(el) {
        return el.getAttribute('role') || el.tagName.toLowerCase();
      }
      function nameOf(el) {
        return (
          el.getAttribute('aria-label') ||
          el.getAttribute('alt') ||
          el.getAttribute('placeholder') ||
          el.getAttribute('title') ||
          (el.innerText || '').trim().slice(0, 120)
        );
      }
      const out = [];
      const walk = (node, depth) => {
        if (depth > 12 || out.length > 800) return;
        if (node.nodeType !== 1) return;
        const el = node;
        const tag = el.tagName.toLowerCase();
        if (interesting.has(tag) || el.getAttribute('role') || el.tabIndex >= 0) {
          out.push({
            role: roleOf(el),
            name: nameOf(el),
            tag,
            testId: el.getAttribute('data-testid') || el.getAttribute('data-test'),
            href: el.getAttribute('href'),
            type: el.getAttribute('type'),
            value: el.tagName === 'INPUT' ? el.value : undefined,
            checked: el.checked,
            disabled: el.disabled,
            depth,
          });
        }
        for (const c of el.children) walk(c, depth + 1);
      };
      walk(document.body, 0);
      return out;
    });
    return {
      format: 'tree',
      snapshot: tree,
      url: p.url(),
      title: await p.title(),
    };
  },

  async screenshot(params) {
    const p = getPage(params.pageId);
    const opts = {
      fullPage: !!params.fullPage,
      type: params.type || 'png',
      omitBackground: !!params.omitBackground,
      timeout: params.timeout,
    };
    if (params.path) {
      fs.mkdirSync(path.dirname(params.path), { recursive: true });
      opts.path = params.path;
    }
    let buffer;
    if (params.selector || params.role || params.testId || params.locator) {
      const loc = await resolveLocator(p, params);
      buffer = await loc.screenshot(opts);
    } else {
      buffer = await p.screenshot(opts);
    }
    const b64 = Buffer.from(buffer).toString('base64');
    return {
      path: params.path || null,
      base64: params.includeBase64 === false ? null : b64,
      bytes: buffer.length,
      mime: opts.type === 'jpeg' ? 'image/jpeg' : 'image/png',
    };
  },

  async pdf(params) {
    const p = getPage(params.pageId);
    if (!params.path) throw new Error('path required');
    fs.mkdirSync(path.dirname(params.path), { recursive: true });
    await p.pdf({
      path: params.path,
      format: params.format || 'A4',
      printBackground: params.printBackground !== false,
    });
    return { path: params.path };
  },

  async console_messages(params) {
    const m = pageMeta(params.pageId);
    const types = params.types;
    let items = m.console;
    if (types?.length) items = items.filter((c) => types.includes(c.type));
    const limit = params.limit || 100;
    return { messages: items.slice(-limit) };
  },

  async page_errors(params) {
    const m = pageMeta(params.pageId);
    return { errors: m.errors.slice(-(params.limit || 50)) };
  },

  async network_log(params) {
    const m = pageMeta(params.pageId);
    let items = m.network;
    if (params.urlIncludes) {
      items = items.filter((n) => n.url && n.url.includes(params.urlIncludes));
    }
    if (params.kind) items = items.filter((n) => n.kind === params.kind);
    return { entries: items.slice(-(params.limit || 100)) };
  },

  async clear_logs(params) {
    const m = pageMeta(params.pageId);
    m.console = [];
    m.network = [];
    m.errors = [];
    m.dialogs = [];
    return { cleared: true };
  },

  async expect_visible(params) {
    const p = getPage(params.pageId);
    const loc = await resolveLocator(p, params);
    await loc.waitFor({ state: params.hidden ? 'hidden' : 'visible', timeout: params.timeout ?? 5000 });
    return { ok: true };
  },

  async expect_text(params) {
    const p = getPage(params.pageId);
    if (params.selector || params.role || params.testId || params.locator) {
      const loc = await resolveLocator(p, params);
      await loc.waitFor({ state: 'visible', timeout: params.timeout ?? 5000 });
      const text = await loc.innerText();
      const expected = String(params.text ?? '');
      const pass = params.exact ? text === expected : text.includes(expected);
      if (!pass) throw new Error(`text mismatch: expected ${JSON.stringify(expected)}, got ${JSON.stringify(text.slice(0, 200))}`);
      return { ok: true, text };
    }
    const body = await p.locator('body').innerText();
    const expected = String(params.text ?? '');
    const pass = params.exact ? body === expected : body.includes(expected);
    if (!pass) throw new Error(`page text does not include ${JSON.stringify(expected)}`);
    return { ok: true };
  },

  async expect_url(params) {
    const p = getPage(params.pageId);
    const url = p.url();
    const pattern = params.url || params.pattern;
    let pass = false;
    if (pattern.startsWith('/') && pattern.lastIndexOf('/') > 0) {
      const re = new RegExp(pattern.slice(1, pattern.lastIndexOf('/')));
      pass = re.test(url);
    } else if (pattern.includes('*')) {
      const re = new RegExp('^' + pattern.split('*').map(escapeRe).join('.*') + '$');
      pass = re.test(url);
    } else {
      pass = url.includes(pattern);
    }
    if (!pass) throw new Error(`url mismatch: ${url} vs ${pattern}`);
    return { ok: true, url };
  },

  async expect_title(params) {
    const p = getPage(params.pageId);
    const title = await p.title();
    const expected = String(params.title ?? params.text ?? '');
    const pass = params.exact ? title === expected : title.includes(expected);
    if (!pass) throw new Error(`title mismatch: ${title} vs ${expected}`);
    return { ok: true, title };
  },

  async expect_count(params) {
    const p = getPage(params.pageId);
    const loc = await resolveLocator(p, params);
    const count = await loc.count();
    if (params.count != null && count !== params.count) {
      throw new Error(`count mismatch: expected ${params.count}, got ${count}`);
    }
    if (params.min != null && count < params.min) throw new Error(`count ${count} < min ${params.min}`);
    if (params.max != null && count > params.max) throw new Error(`count ${count} > max ${params.max}`);
    return { ok: true, count };
  },

  async expect_attribute(params) {
    const p = getPage(params.pageId);
    const loc = await resolveLocator(p, params);
    const val = await loc.getAttribute(params.attribute);
    if (params.value != null && val !== String(params.value)) {
      throw new Error(`attribute ${params.attribute}: expected ${params.value}, got ${val}`);
    }
    return { ok: true, value: val };
  },

  async expect_no_console_errors(params) {
    const m = pageMeta(params.pageId);
    const errs = m.console.filter((c) => c.type === 'error');
    if (errs.length) throw new Error(`console errors: ${JSON.stringify(errs.slice(0, 5))}`);
    return { ok: true };
  },

  async locator_info(params) {
    const p = getPage(params.pageId);
    const loc = await resolveLocator(p, params);
    const count = await loc.count();
    const first = loc.first();
    const info = {
      count,
      visible: count ? await first.isVisible().catch(() => false) : false,
      enabled: count ? await first.isEnabled().catch(() => false) : false,
      editable: count ? await first.isEditable().catch(() => false) : false,
      checked: count ? await first.isChecked().catch(() => null) : null,
      text: count ? await first.innerText().catch(() => '') : '',
      box: count ? await first.boundingBox().catch(() => null) : null,
    };
    return info;
  },

  async why_not_actionable(params) {
    const p = getPage(params.pageId);
    const loc = await resolveLocator(p, params);
    const count = await loc.count();
    if (count === 0) return { actionable: false, reasons: ['no elements matched locator'] };
    if (count > 1 && params.strict !== false) {
      return { actionable: false, reasons: [`strict mode: ${count} elements matched`], count };
    }
    const el = loc.first();
    const reasons = [];
    if (!(await el.isVisible())) reasons.push('not visible');
    if (!(await el.isEnabled())) reasons.push('not enabled');
    const box = await el.boundingBox();
    if (!box) reasons.push('no bounding box');
    try {
      await el.click({ trial: true, timeout: params.timeout ?? 2000 });
      return { actionable: true, reasons: [], count, box };
    } catch (e) {
      reasons.push(String(e.message || e));
      return { actionable: false, reasons, count, box };
    }
  },

  async route_mock(params) {
    const target = params.pageId ? getPage(params.pageId) : getContext(params.contextId);
    const url = params.url;
    await target.route(url, async (route) => {
      if (params.abort) return route.abort();
      if (params.fulfill) {
        return route.fulfill({
          status: params.fulfill.status ?? 200,
          contentType: params.fulfill.contentType || 'application/json',
          body:
            typeof params.fulfill.body === 'string'
              ? params.fulfill.body
              : JSON.stringify(params.fulfill.body ?? {}),
          headers: params.fulfill.headers,
        });
      }
      if (params.headers || params.status) {
        const response = await route.fetch();
        const headers = { ...response.headers(), ...(params.headers || {}) };
        return route.fulfill({
          response,
          status: params.status || response.status(),
          headers,
        });
      }
      return route.continue();
    });
    return { ok: true, url };
  },

  async route_unroute(params) {
    const target = params.pageId ? getPage(params.pageId) : getContext(params.contextId);
    await target.unroute(params.url);
    return { ok: true };
  },

  async wait_for_response(params) {
    const p = getPage(params.pageId);
    const resp = await p.waitForResponse(
      (r) => {
        if (params.urlIncludes && !r.url().includes(params.urlIncludes)) return false;
        if (params.url && r.url() !== params.url) {
          if (typeof params.url === 'string' && params.url.includes('*')) {
            const re = new RegExp('^' + params.url.split('*').map(escapeRe).join('.*') + '$');
            if (!re.test(r.url())) return false;
          } else if (r.url() !== params.url) return false;
        }
        if (params.status && r.status() !== params.status) return false;
        return true;
      },
      { timeout: params.timeout },
    );
    let body = null;
    try {
      body = await resp.text();
      if (body.length > 50_000) body = body.slice(0, 50_000) + '...';
    } catch {
      body = null;
    }
    return { url: resp.url(), status: resp.status(), body };
  },

  async wait_for_request(params) {
    const p = getPage(params.pageId);
    const req = await p.waitForRequest(
      (r) => {
        if (params.urlIncludes && !r.url().includes(params.urlIncludes)) return false;
        if (params.method && r.method() !== params.method) return false;
        return true;
      },
      { timeout: params.timeout },
    );
    return { url: req.url(), method: req.method() };
  },

  async har_start(params) {
    // HAR is typically at context level via recordHar in newContext — emulate via route logging
    const c = getContext(params.contextId);
    c.__har = [];
    await c.route('**/*', async (route) => {
      const req = route.request();
      const start = Date.now();
      const response = await route.fetch();
      c.__har.push({
        startedDateTime: new Date(start).toISOString(),
        request: { method: req.method(), url: req.url(), headers: req.headers() },
        response: { status: response.status(), headers: response.headers() },
        time: Date.now() - start,
      });
      await route.fulfill({ response });
    });
    return { started: true };
  },

  async har_stop(params) {
    const c = getContext(params.contextId);
    const entries = c.__har || [];
    const har = {
      log: {
        version: '1.2',
        creator: { name: 'rmcp-browser-bridge', version: '1.0' },
        entries,
      },
    };
    if (params.path) {
      fs.mkdirSync(path.dirname(params.path), { recursive: true });
      fs.writeFileSync(params.path, JSON.stringify(har, null, 2));
    }
    try {
      await c.unroute('**/*');
    } catch {
      /* */
    }
    c.__har = [];
    return { path: params.path || null, entries: entries.length };
  },

  async a11y_scan(params) {
    const p = getPage(params.pageId);
    // inject axe
    let axePath;
    try {
      axePath = require.resolve('axe-core/axe.min.js');
    } catch {
      throw new Error('axe-core not installed in browser-bridge');
    }
    await p.addScriptTag({ path: axePath });
    const results = await p.evaluate(async (opts) => {
      // eslint-disable-next-line no-undef
      return await axe.run(document, opts || {});
    }, params.options || {});
    const impact = params.impact || null;
    let violations = results.violations || [];
    if (impact) {
      const order = ['minor', 'moderate', 'serious', 'critical'];
      const min = order.indexOf(impact);
      violations = violations.filter((v) => order.indexOf(v.impact) >= min);
    }
    return {
      url: p.url(),
      violations: violations.map((v) => ({
        id: v.id,
        impact: v.impact,
        description: v.description,
        help: v.help,
        helpUrl: v.helpUrl,
        nodes: (v.nodes || []).slice(0, 10).map((n) => ({
          html: n.html,
          target: n.target,
          failureSummary: n.failureSummary,
        })),
      })),
      passes: (results.passes || []).length,
      incomplete: (results.incomplete || []).length,
    };
  },

  async list_devices() {
    return { devices: Object.keys(devices) };
  },

  async cookies(params) {
    const c = getContext(params.contextId);
    return { cookies: await c.cookies(params.urls) };
  },

  async add_init_script(params) {
    const c = params.contextId ? getContext(params.contextId) : null;
    const p = params.pageId ? getPage(params.pageId) : null;
    if (c) await c.addInitScript(params.script);
    else if (p) await p.addInitScript(params.script);
    else throw new Error('contextId or pageId required');
    return { ok: true };
  },

  async expose_binding(params) {
    const p = getPage(params.pageId);
    // not generally useful cross-process — skip or no-op
    return { ok: false, message: 'expose_binding not supported across bridge' };
  },

  async frame_locator_click(params) {
    const p = getPage(params.pageId);
    let frame = p.frameLocator(params.frameSelector);
    if (params.selector) {
      await frame.locator(params.selector).click({ timeout: params.timeout });
    } else if (params.role) {
      await frame.getByRole(params.role, { name: params.name }).click({ timeout: params.timeout });
    } else {
      throw new Error('selector or role required inside frame');
    }
    return { ok: true };
  },

  async download_save(params) {
    const p = getPage(params.pageId);
    const [download] = await Promise.all([
      p.waitForEvent('download', { timeout: params.timeout }),
      params.triggerClick
        ? (async () => {
            const loc = await resolveLocator(p, params);
            await loc.click();
          })()
        : Promise.resolve(),
    ]);
    const out = params.path || path.join(params.downloadsPath || '/tmp', download.suggestedFilename());
    fs.mkdirSync(path.dirname(out), { recursive: true });
    await download.saveAs(out);
    return { path: out, suggestedFilename: download.suggestedFilename(), url: download.url() };
  },

  async emulated_media(params) {
    const p = getPage(params.pageId);
    await p.emulateMedia({
      media: params.media,
      colorScheme: params.colorScheme,
      reducedMotion: params.reducedMotion,
      forcedColors: params.forcedColors,
    });
    return { ok: true };
  },

  async clock_install(params) {
    const p = getPage(params.pageId);
    if (p.clock && typeof p.clock.install === 'function') {
      await p.clock.install({ time: params.time });
      return { ok: true, mode: 'playwright-clock' };
    }
    // fallback: freeze Date
    await p.addInitScript(
      ({ t }) => {
        const Fixed = t ? new Date(t).getTime() : Date.now();
        const RealDate = Date;
        // eslint-disable-next-line no-global-assign
        Date = class extends RealDate {
          constructor(...args) {
            if (args.length === 0) super(Fixed);
            else super(...args);
          }
          static now() {
            return Fixed;
          }
        };
      },
      { t: params.time },
    );
    return { ok: true, mode: 'init-script' };
  },

  async performance_metrics(params) {
    const p = getPage(params.pageId);
    const metrics = await p.evaluate(() => {
      const nav = performance.getEntriesByType('navigation')[0];
      const paint = performance.getEntriesByType('paint');
      return {
        navigation: nav
          ? {
              domContentLoaded: nav.domContentLoadedEventEnd,
              loadEventEnd: nav.loadEventEnd,
              responseEnd: nav.responseEnd,
              transferSize: nav.transferSize,
            }
          : null,
        paint: paint.map((x) => ({ name: x.name, startTime: x.startTime })),
        memory: performance.memory
          ? {
              usedJSHeapSize: performance.memory.usedJSHeapSize,
              totalJSHeapSize: performance.memory.totalJSHeapSize,
            }
          : null,
      };
    });
    return metrics;
  },

  async web_vitals(params) {
    const p = getPage(params.pageId);
    // approximate LCP/CLS via PerformanceObserver buffer
    const vitals = await p.evaluate(() => {
      return new Promise((resolve) => {
        const out = { lcp: null, cls: 0, entries: [] };
        try {
          new PerformanceObserver((list) => {
            for (const e of list.getEntries()) {
              if (e.entryType === 'largest-contentful-paint') out.lcp = e.startTime;
              if (e.entryType === 'layout-shift' && !e.hadRecentInput) out.cls += e.value;
            }
          }).observe({ type: 'largest-contentful-paint', buffered: true });
          new PerformanceObserver((list) => {
            for (const e of list.getEntries()) {
              if (!e.hadRecentInput) out.cls += e.value;
            }
          }).observe({ type: 'layout-shift', buffered: true });
        } catch {
          /* */
        }
        setTimeout(() => resolve(out), 100);
      });
    });
    return vitals;
  },

  async cookie_security_report(params) {
    const c = getContext(params.contextId);
    const cookies = await c.cookies();
    return {
      cookies: cookies.map((ck) => ({
        name: ck.name,
        domain: ck.domain,
        path: ck.path,
        secure: ck.secure,
        httpOnly: ck.httpOnly,
        sameSite: ck.sameSite,
        expires: ck.expires,
      })),
    };
  },

  async csp_report(params) {
    const p = getPage(params.pageId);
    const headers = await p.evaluate(async () => {
      try {
        const res = await fetch(location.href, { method: 'HEAD' });
        return {
          csp: res.headers.get('content-security-policy'),
          cspReportOnly: res.headers.get('content-security-policy-report-only'),
        };
      } catch (e) {
        return { error: String(e) };
      }
    });
    // also meta CSP
    const metaCsp = await p.locator('meta[http-equiv="Content-Security-Policy"]').count();
    return { ...headers, metaCspTags: metaCsp };
  },

  async suggest_locator(params) {
    const p = getPage(params.pageId);
    const suggestions = await p.evaluate(({ x, y, text }) => {
      let el = null;
      if (x != null && y != null) el = document.elementFromPoint(x, y);
      else if (text) {
        const all = [...document.querySelectorAll('a,button,input,label,[role]')];
        el = all.find((e) => (e.innerText || e.getAttribute('aria-label') || '').includes(text));
      }
      if (!el) return [];
      const out = [];
      const testId = el.getAttribute('data-testid') || el.getAttribute('data-test');
      if (testId) out.push({ type: 'testId', value: testId });
      const role = el.getAttribute('role') || el.tagName.toLowerCase();
      const name =
        el.getAttribute('aria-label') ||
        el.getAttribute('placeholder') ||
        (el.innerText || '').trim().slice(0, 80);
      if (name) out.push({ type: 'role', role, name });
      if (el.id) out.push({ type: 'selector', value: `#${el.id}` });
      return out;
    }, { x: params.x, y: params.y, text: params.text });
    return { suggestions };
  },

  async run_steps(params) {
    const results = [];
    for (const step of params.steps || []) {
      const method = step.action || step.method;
      const stepParams = { ...step, pageId: step.pageId || params.pageId, contextId: step.contextId || params.contextId };
      delete stepParams.action;
      delete stepParams.method;
      try {
        if (!handlers[method]) throw new Error(`unknown step action: ${method}`);
        const result = await handlers[method](stepParams);
        results.push({ action: method, ok: true, result });
      } catch (e) {
        results.push({ action: method, ok: false, error: String(e.message || e) });
        if (params.stopOnFailure !== false) break;
      }
    }
    return { results, passed: results.every((r) => r.ok) };
  },

  async export_playwright_code(params) {
    const steps = params.steps || [];
    const lines = [
      "import { test, expect } from '@playwright/test';",
      '',
      `test(${JSON.stringify(params.name || 'generated')}, async ({ page }) => {`,
    ];
    for (const s of steps) {
      const a = s.action || s.method;
      if (a === 'goto') lines.push(`  await page.goto(${JSON.stringify(s.url)});`);
      else if (a === 'click' && s.selector) lines.push(`  await page.locator(${JSON.stringify(s.selector)}).click();`);
      else if (a === 'click' && s.role) lines.push(`  await page.getByRole(${JSON.stringify(s.role)}, { name: ${JSON.stringify(s.name)} }).click();`);
      else if (a === 'fill' && s.selector) lines.push(`  await page.locator(${JSON.stringify(s.selector)}).fill(${JSON.stringify(s.value)});`);
      else if (a === 'expect_text') lines.push(`  await expect(page.getByText(${JSON.stringify(s.text)})).toBeVisible();`);
      else lines.push(`  // TODO: ${a} ${JSON.stringify(s)}`);
    }
    lines.push('});', '');
    const code = lines.join('\n');
    if (params.path) {
      fs.mkdirSync(path.dirname(params.path), { recursive: true });
      fs.writeFileSync(params.path, code);
    }
    return { code, path: params.path || null };
  },

  async shutdown() {
    for (const [, p] of pages) {
      try {
        await p.close();
      } catch {
        /* */
      }
    }
    for (const [, c] of contexts) {
      try {
        await c.close();
      } catch {
        /* */
      }
    }
    for (const [, b] of browsers) {
      try {
        await b.close();
      } catch {
        /* */
      }
    }
    pages.clear();
    contexts.clear();
    browsers.clear();
    return { shutdown: true };
  },
};


function stripNulls(value) {
  if (Array.isArray(value)) return value.map(stripNulls);
  if (value && typeof value === "object") {
    const out = {};
    for (const [k, v] of Object.entries(value)) {
      if (v === null || v === undefined) continue;
      out[k] = stripNulls(v);
    }
    return out;
  }
  return value;
}

function escapeRe(s) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });

rl.on('line', async (line) => {
  line = line.trim();
  if (!line) return;
  let msg;
  try {
    msg = JSON.parse(line);
  } catch (e) {
    fail(null, `invalid json: ${e.message}`);
    return;
  }
  const { id, method, params } = msg;
  if (!method || !handlers[method]) {
    fail(id, `unknown method: ${method}`);
    return;
  }
  try {
    const result = await handlers[method](stripNulls(params || {}));
    ok(id, result);
  } catch (e) {
    fail(id, e.message || String(e), { stack: e.stack });
  }
});

process.stdin.on('end', async () => {
  try {
    await handlers.shutdown();
  } catch {
    /* */
  }
  process.exit(0);
});

// ready signal
process.stdout.write(JSON.stringify({ id: null, result: { ready: true } }) + '\n');
