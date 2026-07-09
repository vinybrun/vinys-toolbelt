//! Systematic coverage of browser MCP tools (+ resources/prompts).
//!
//! Run: `mcp-client --preset browser coverage`

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use anyhow::Result;
use colored::Colorize;
use serde_json::{Map, Value, json};

use crate::session::Session;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Pass,
    Fail,
    Skip,
}

struct Row {
    name: String,
    kind: String,
    outcome: Outcome,
    detail: String,
    ms: u128,
}

pub async fn run_browser_coverage(session: &Session, base_url: &str) -> Result<i32> {
    let started = Instant::now();
    let mut rows: Vec<Row> = Vec::new();
    let mut state = State {
        browser_id: None,
        context_id: None,
        page_id: None,
        second_page_id: None,
        second_context_id: None,
        base_url: base_url.trim_end_matches('/').to_string(),
    };

    // Discover tools advertised by the server
    let advertised: HashSet<String> = session
        .list_tools()
        .await?
        .into_iter()
        .map(|t| t.name.to_string())
        .collect();

    println!(
        "{} {} tools advertised by server",
        "Coverage".bold().cyan(),
        advertised.len()
    );
    println!("Base URL: {}\n", state.base_url);

    // Ordered plan: (name, kind, runner)
    // Runners close over state via sequential execution below.

    // --- helpers ---
    async fn call(
        session: &Session,
        rows: &mut Vec<Row>,
        name: &str,
        args: Value,
    ) -> Result<Option<Value>> {
        let t0 = Instant::now();
        let map: Option<Map<String, Value>> = match args {
            Value::Object(m) => Some(m),
            Value::Null => None,
            other => Some(Map::from_iter([("value".into(), other)])),
        };
        match session.call_tool(name, map).await {
            Ok(result) => {
                let is_err = result.is_error.unwrap_or(false);
                let text = result
                    .content
                    .iter()
                    .filter_map(|b| b.as_text().map(|t| t.text.clone()))
                    .collect::<Vec<_>>()
                    .join("\n");
                let parsed: Value = serde_json::from_str(&text).unwrap_or(Value::String(text.clone()));
                if is_err {
                    rows.push(Row {
                        name: name.into(),
                        kind: "tool".into(),
                        outcome: Outcome::Fail,
                        detail: truncate(&text, 180),
                        ms: t0.elapsed().as_millis(),
                    });
                    Ok(None)
                } else {
                    rows.push(Row {
                        name: name.into(),
                        kind: "tool".into(),
                        outcome: Outcome::Pass,
                        detail: truncate(&summary(&parsed), 120),
                        ms: t0.elapsed().as_millis(),
                    });
                    Ok(Some(parsed))
                }
            }
            Err(e) => {
                rows.push(Row {
                    name: name.into(),
                    kind: "tool".into(),
                    outcome: Outcome::Fail,
                    detail: truncate(&e.to_string(), 180),
                    ms: t0.elapsed().as_millis(),
                });
                Ok(None)
            }
        }
    }

    fn skip(rows: &mut Vec<Row>, name: &str, kind: &str, reason: &str) {
        rows.push(Row {
            name: name.into(),
            kind: kind.into(),
            outcome: Outcome::Skip,
            detail: reason.into(),
            ms: 0,
        });
    }

    // ========== TOOLS ==========
    // Meta / no browser
    let _ = call(session, &mut rows, "browser_info", json!({})).await?;
    let _ = call(
        session,
        &mut rows,
        "browser_install_deps",
        json!({ "install_browsers": false }),
    )
    .await?;
    let _ = call(session, &mut rows, "list_devices", json!({})).await?;
    let _ = call(session, &mut rows, "list_suites", json!({})).await?;
    let _ = call(session, &mut rows, "list_artifacts", json!({})).await?;
    let _ = call(
        session,
        &mut rows,
        "agent_note",
        json!({ "note": "coverage run" }),
    )
    .await?;
    let _ = call(session, &mut rows, "list_agent_notes", json!({})).await?;

    // CDP requires external chrome — skip unless MCP_COVERAGE_CDP set
    if let Ok(endpoint) = std::env::var("MCP_COVERAGE_CDP") {
        let _ = call(
            session,
            &mut rows,
            "browser_connect_cdp",
            json!({ "endpoint": endpoint }),
        )
        .await?;
    } else {
        skip(
            &mut rows,
            "browser_connect_cdp",
            "tool",
            "set MCP_COVERAGE_CDP=http://host:9222 to exercise",
        );
    }

    // fixture server (optional path — may conflict with external site; still call start/stop)
    // We primarily use external base_url; fixture is still invoked for coverage.
    let _ = call(
        session,
        &mut rows,
        "fixture_server_start",
        json!({ "port": 8766 }),
    )
    .await?;
    let _ = call(session, &mut rows, "fixture_server_stop", json!({})).await?;

    // Launch main browser
    if let Some(v) = call(
        session,
        &mut rows,
        "browser_launch",
        json!({
            "headless": true,
            "browser": "chromium",
            "base_url": state.base_url,
            "create_page": true
        }),
    )
    .await?
    {
        extract_ids(&v, &mut state);
    } else {
        // cannot continue page tools
        mark_remaining_skip(&mut rows, &advertised, "browser_launch failed");
        return finish(session, &rows, &advertised, started).await;
    }

    // Persistent profile (separate quick launch path — use then leave main session)
    // We test after main flow with a note: call creates new browser. Do it mid-run carefully.
    // Better: test persistent near end before nuke. For now call and if success close later.
    let _ = call(
        session,
        &mut rows,
        "browser_persistent",
        json!({
            "user_data_dir": "browser-artifacts/coverage-profile",
            "headless": true,
            "browser": "chromium"
        }),
    )
    .await?;
    // Re-launch clean session after persistent (it overwrites session ids)
    if let Some(v) = call(
        session,
        &mut rows,
        "browser_launch",
        json!({
            "headless": true,
            "browser": "chromium",
            "base_url": state.base_url,
            "create_page": true
        }),
    )
    .await?
    {
        // Don't double-count browser_launch name — use synthetic pass detail only if we need
        // Actually this creates a second browser_launch row. Use a different approach:
        extract_ids(&v, &mut state);
    }

    // Wait - we now have two browser_launch rows which is OK for coverage of the tool once...
    // Actually we called browser_launch twice. That's fine for "function works".
    // But first launch row already Pass. Second also Pass. advertised coverage still counts as tested.

    // Context / pages
    let browser_id = state.browser_id.clone().unwrap_or_default();
    if let Some(v) = call(
        session,
        &mut rows,
        "context_new",
        json!({ "browser_id": browser_id, "viewport_width": 1280, "viewport_height": 720 }),
    )
    .await?
    {
        state.second_context_id = v.get("contextId").and_then(|x| x.as_str()).map(str::to_string);
    }

    if let Some(cid) = state.context_id.clone().or(state.second_context_id.clone()) {
        if let Some(v) = call(
            session,
            &mut rows,
            "page_new",
            json!({ "context_id": cid, "url": format!("{}/coverage.html", state.base_url) }),
        )
        .await?
        {
            state.second_page_id = v.get("pageId").and_then(|x| x.as_str()).map(str::to_string);
            if state.page_id.is_none() {
                state.page_id = state.second_page_id.clone();
            }
        }
    }

    let _ = call(session, &mut rows, "page_list", json!({})).await?;
    if let Some(pid) = state.page_id.clone().or(state.second_page_id.clone()) {
        let _ = call(
            session,
            &mut rows,
            "page_switch",
            json!({ "page_id": pid }),
        )
        .await?;
        state.page_id = Some(pid);
    }

    // Navigation on coverage page
    let cov = format!("{}/coverage.html", state.base_url);
    let _ = call(session, &mut rows, "goto", json!({ "url": cov })).await?;
    let _ = call(session, &mut rows, "page_title", json!({})).await?;
    let _ = call(session, &mut rows, "reload", json!({})).await?;
    let _ = call(
        session,
        &mut rows,
        "wait_for_url",
        json!({ "url": "**/coverage.html" }),
    )
    .await?;

    // go_back / go_forward
    let _ = call(
        session,
        &mut rows,
        "goto",
        json!({ "url": format!("{}/index.html", state.base_url) }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "goto",
        json!({ "url": format!("{}/coverage.html", state.base_url) }),
    )
    .await?;
    let _ = call(session, &mut rows, "go_back", json!({})).await?;
    let _ = call(session, &mut rows, "go_forward", json!({})).await?;
    let _ = call(
        session,
        &mut rows,
        "goto",
        json!({ "url": format!("{}/coverage.html", state.base_url) }),
    )
    .await?;

    // Interactions
    let _ = call(
        session,
        &mut rows,
        "click",
        json!({ "selector": "#btn-click" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "dblclick",
        json!({ "selector": "#btn-dbl" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "hover",
        json!({ "selector": "#btn-click" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "fill",
        json!({ "selector": "#text-input", "value": "hello coverage" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "type_text",
        json!({ "selector": "#text-input", "content": "!", "delay": 5 }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "press",
        json!({ "selector": "#text-input", "key": "End" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "select_option",
        json!({ "selector": "#role-select", "value": "admin" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "check",
        json!({ "selector": "#agree" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "uncheck",
        json!({ "selector": "#agree" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "check",
        json!({ "selector": "#agree" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "upload_file",
        json!({ "selector": "#file-input", "files": ["demo-site/assets/upload-sample.txt"] }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "drag_drop",
        json!({ "source_selector": "#drag-src", "target_selector": "#drag-dst" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "scroll",
        json!({ "selector": "#bottom" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "set_viewport",
        json!({ "width": 1100, "height": 800 }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "emulate_media",
        json!({ "color_scheme": "dark" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "set_dialog_policy",
        json!({ "policy": "accept" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "click",
        json!({ "selector": "#btn-dialog" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "evaluate",
        json!({ "expression": "() => document.title" }),
    )
    .await?;

    // Observe / assert
    let _ = call(session, &mut rows, "snapshot", json!({})).await?;
    let _ = call(
        session,
        &mut rows,
        "screenshot",
        json!({ "path": "screenshots/coverage.png", "full_page": true }),
    )
    .await?;
    let _ = call(session, &mut rows, "console_messages", json!({})).await?;
    let _ = call(session, &mut rows, "page_errors", json!({})).await?;
    let _ = call(session, &mut rows, "network_log", json!({})).await?;
    let _ = call(session, &mut rows, "clear_logs", json!({})).await?;
    let _ = call(
        session,
        &mut rows,
        "expect_visible",
        json!({ "selector": "#btn-click" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "expect_text",
        json!({ "text": "MCP Coverage Harness" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "expect_url",
        json!({ "url": "**/coverage.html" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "expect_title",
        json!({ "text": "Coverage" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "expect_count",
        json!({ "selector": "button", "min": 1 }),
    )
    .await?;
    let _ = call(session, &mut rows, "expect_no_console_errors", json!({})).await?;
    let _ = call(
        session,
        &mut rows,
        "locator_info",
        json!({ "selector": "#btn-click" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "why_not_actionable",
        json!({ "selector": "#btn-click" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "suggest_locator",
        json!({ "text": "Click me" }),
    )
    .await?;

    // Network
    let _ = call(
        session,
        &mut rows,
        "route_mock",
        json!({
            "url": "**/mock-coverage.json",
            "status": 200,
            "body": { "ok": true },
            "content_type": "application/json"
        }),
    )
    .await?;
    // wait_for_response: fire fetch then wait — use evaluate + wait concurrent is hard in series
    // Start wait via run_steps or evaluate fetch after scheduling — simple approach: skip with note if flaky
    // Try evaluate to fetch a real asset we already loaded
    {
        let t0 = Instant::now();
        let wait_fut = session.call_tool(
            "wait_for_response",
            Some(Map::from_iter([
                ("url_includes".into(), json!("styles.css")),
                ("timeout".into(), json!(8000)),
            ])),
        );
        let reload_fut = async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            session.call_tool("reload", None).await
        };
        let (w, _r) = tokio::join!(wait_fut, reload_fut);
        match w {
            Ok(result) if !result.is_error.unwrap_or(false) => {
                rows.push(Row {
                    name: "wait_for_response".into(),
                    kind: "tool".into(),
                    outcome: Outcome::Pass,
                    detail: "matched styles.css".into(),
                    ms: t0.elapsed().as_millis(),
                });
            }
            Ok(result) => {
                let text = result
                    .content
                    .iter()
                    .filter_map(|b| b.as_text().map(|t| t.text.clone()))
                    .collect::<Vec<_>>()
                    .join(" ");
                rows.push(Row {
                    name: "wait_for_response".into(),
                    kind: "tool".into(),
                    outcome: Outcome::Fail,
                    detail: truncate(&text, 160),
                    ms: t0.elapsed().as_millis(),
                });
            }
            Err(e) => {
                rows.push(Row {
                    name: "wait_for_response".into(),
                    kind: "tool".into(),
                    outcome: Outcome::Fail,
                    detail: truncate(&e.to_string(), 160),
                    ms: t0.elapsed().as_millis(),
                });
            }
        }
    }

    let ctx = state
        .context_id
        .clone()
        .or(state.second_context_id.clone())
        .unwrap_or_default();
    let _ = call(
        session,
        &mut rows,
        "har_start",
        json!({ "context_id": ctx }),
    )
    .await?;
    let _ = call(session, &mut rows, "reload", json!({})).await?;
    let _ = call(
        session,
        &mut rows,
        "har_stop",
        json!({ "context_id": ctx, "path": "browser-artifacts/coverage.har" }),
    )
    .await?;

    let _ = call(
        session,
        &mut rows,
        "tracing_start",
        json!({ "context_id": ctx }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "click",
        json!({ "selector": "#btn-click" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "tracing_stop",
        json!({ "context_id": ctx, "path": "browser-artifacts/coverage-trace.zip" }),
    )
    .await?;

    // Auth / cookies / offline / perms
    let _ = call(
        session,
        &mut rows,
        "storage_state_save",
        json!({ "context_id": ctx, "path": "browser-artifacts/coverage-storage.json" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "login_as",
        json!({
            "url": format!("{}/login.html", state.base_url),
            "user_selector": "#email",
            "password_selector": "#password",
            "username": "demo@nebula.test",
            "password": "nebula1",
            "submit_selector": "#login-submit",
            "success_url": "**/login.html"
        }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "context_set_offline",
        json!({ "context_id": ctx, "offline": false }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "context_grant_permissions",
        json!({
            "context_id": ctx,
            "permissions": ["geolocation"],
            "origin": state.base_url
        }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "cookies_get",
        json!({ "context_id": ctx }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "cookies_set",
        json!({
            "context_id": ctx,
            "cookies": [{
                "name": "coverage",
                "value": "1",
                "domain": "127.0.0.1",
                "path": "/"
            }]
        }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "cookie_security_report",
        json!({ "context_id": ctx }),
    )
    .await?;

    let _ = call(
        session,
        &mut rows,
        "goto",
        json!({ "url": format!("{}/coverage.html", state.base_url) }),
    )
    .await?;
    let _ = call(session, &mut rows, "csp_report", json!({})).await?;
    let _ = call(
        session,
        &mut rows,
        "a11y_scan",
        json!({ "impact": "serious" }),
    )
    .await?;

    // visual_diff: update baseline then compare
    let _ = call(
        session,
        &mut rows,
        "screenshot",
        json!({ "path": "screenshots/visual-current.png" }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "visual_diff",
        json!({
            "baseline_path": "browser-artifacts/baselines/coverage.png",
            "current_path": "screenshots/visual-current.png",
            "update_baseline": true
        }),
    )
    .await?;
    let _ = call(
        session,
        &mut rows,
        "visual_diff",
        json!({
            "baseline_path": "browser-artifacts/baselines/coverage.png",
            "current_path": "screenshots/visual-current.png",
            "threshold": 0.05
        }),
    )
    .await?;

    let _ = call(session, &mut rows, "performance_metrics", json!({})).await?;
    let _ = call(session, &mut rows, "web_vitals", json!({})).await?;
    let _ = call(session, &mut rows, "pdf_save", json!({ "path": "pdf/coverage.pdf" })).await?;
    let _ = call(
        session,
        &mut rows,
        "clock_install",
        json!({ "time": "2026-01-15T12:00:00.000Z" }),
    )
    .await?;

    // Suites / steps / recording
    let _ = call(
        session,
        &mut rows,
        "run_steps",
        json!({
            "steps": [
                { "action": "goto", "url": format!("{}/coverage.html", state.base_url) },
                { "action": "expect_text", "text": "Coverage Harness" }
            ]
        }),
    )
    .await?;
    let _ = call(session, &mut rows, "recording_start", json!({})).await?;
    let _ = call(
        session,
        &mut rows,
        "click",
        json!({ "selector": "#btn-click" }),
    )
    .await?;
    let _ = call(session, &mut rows, "recording_stop", json!({})).await?;
    let _ = call(
        session,
        &mut rows,
        "export_playwright_code",
        json!({
            "name": "coverage-export",
            "path": "browser-artifacts/coverage-export.spec.ts",
            "steps": [
                { "action": "goto", "url": "/" },
                { "action": "click", "selector": "#btn-click" }
            ]
        }),
    )
    .await?;
    // Fresh navigation suite (login suite can fail if session already authenticated)
    let _ = call(
        session,
        &mut rows,
        "run_suite",
        json!({ "path": "browser-tests/nebula-smoke.yaml", "headless": true }),
    )
    .await?;

    // Playwright project may not exist
    if std::env::var("MCP_COVERAGE_PLAYWRIGHT").ok().as_deref() == Some("1") {
        let _ = call(
            session,
            &mut rows,
            "run_playwright_tests",
            json!({ "args": ["--list"], "timeout_secs": 60 }),
        )
        .await?;
    } else {
        skip(
            &mut rows,
            "run_playwright_tests",
            "tool",
            "set MCP_COVERAGE_PLAYWRIGHT=1 (needs playwright test project)",
        );
    }

    // app_start / app_stop — start a short-lived python http on random high port
    let _ = call(
        session,
        &mut rows,
        "app_start",
        json!({
            "program": "python3",
            "args": ["-m", "http.server", "8899", "--bind", "127.0.0.1"],
            "cwd": "demo-site",
            "health_url": "http://127.0.0.1:8899/",
            "timeout_secs": 15
        }),
    )
    .await?;
    let _ = call(session, &mut rows, "app_stop", json!({})).await?;

    // read_artifact if any text report exists
    let artifacts = call(session, &mut rows, "list_artifacts", json!({})).await?;
    if let Some(Value::Object(map)) = artifacts {
        // already recorded list_artifacts twice — that's ok
        let _ = map;
    }
    // Try read a known json if present
    let _ = call(
        session,
        &mut rows,
        "read_artifact",
        json!({ "path": "screenshots/coverage.png" }),
    )
    .await?; // may return base64 — still exercises tool

    // context_close / page_close on secondary if any
    if let Some(pid) = state.second_page_id.clone() {
        if Some(&pid) != state.page_id.as_ref() {
            let _ = call(
                session,
                &mut rows,
                "page_close",
                json!({ "page_id": pid }),
            )
            .await?;
        }
    }
    if let Some(cid) = state.second_context_id.clone() {
        if Some(&cid) != state.context_id.as_ref() {
            let _ = call(
                session,
                &mut rows,
                "context_close",
                json!({ "context_id": cid }),
            )
            .await?;
        }
    }

    // browser_close then we need browser again for emergency_nuke? nuke kills all.
    let _ = call(session, &mut rows, "browser_close", json!({})).await?;

    // emergency_nuke after close (still valid — resets bridge)
    let _ = call(session, &mut rows, "emergency_nuke", json!({})).await?;

    // ========== RESOURCES ==========
    {
        let t0 = Instant::now();
        match session.list_resources().await {
            Ok(list) => {
                rows.push(Row {
                    name: "resources/list".into(),
                    kind: "resource".into(),
                    outcome: Outcome::Pass,
                    detail: format!("{} resources", list.len()),
                    ms: t0.elapsed().as_millis(),
                });
                for uri in ["browser://session", "browser://guide", "browser://config"] {
                    let t1 = Instant::now();
                    match session.read_resource(uri).await {
                        Ok(_) => rows.push(Row {
                            name: format!("resources/read:{uri}"),
                            kind: "resource".into(),
                            outcome: Outcome::Pass,
                            detail: "ok".into(),
                            ms: t1.elapsed().as_millis(),
                        }),
                        Err(e) => rows.push(Row {
                            name: format!("resources/read:{uri}"),
                            kind: "resource".into(),
                            outcome: Outcome::Fail,
                            detail: truncate(&e.to_string(), 120),
                            ms: t1.elapsed().as_millis(),
                        }),
                    }
                }
            }
            Err(e) => rows.push(Row {
                name: "resources/list".into(),
                kind: "resource".into(),
                outcome: Outcome::Fail,
                detail: truncate(&e.to_string(), 120),
                ms: t0.elapsed().as_millis(),
            }),
        }
        let t0 = Instant::now();
        match session.list_resource_templates().await {
            Ok(t) => rows.push(Row {
                name: "resources/templates".into(),
                kind: "resource".into(),
                outcome: Outcome::Pass,
                detail: format!("{} templates", t.len()),
                ms: t0.elapsed().as_millis(),
            }),
            Err(e) => rows.push(Row {
                name: "resources/templates".into(),
                kind: "resource".into(),
                outcome: Outcome::Fail,
                detail: truncate(&e.to_string(), 120),
                ms: t0.elapsed().as_millis(),
            }),
        }
        // subscribe may fail if server doesn't support — record outcome
        let t0 = Instant::now();
        match session.subscribe("browser://session").await {
            Ok(()) => {
                rows.push(Row {
                    name: "resources/subscribe".into(),
                    kind: "resource".into(),
                    outcome: Outcome::Pass,
                    detail: "browser://session".into(),
                    ms: t0.elapsed().as_millis(),
                });
                let _ = session.unsubscribe("browser://session").await;
                rows.push(Row {
                    name: "resources/unsubscribe".into(),
                    kind: "resource".into(),
                    outcome: Outcome::Pass,
                    detail: "browser://session".into(),
                    ms: 0,
                });
            }
            Err(e) => {
                rows.push(Row {
                    name: "resources/subscribe".into(),
                    kind: "resource".into(),
                    outcome: Outcome::Skip,
                    detail: truncate(&format!("not supported: {e}"), 120),
                    ms: t0.elapsed().as_millis(),
                });
                skip(
                    &mut rows,
                    "resources/unsubscribe",
                    "resource",
                    "subscribe unavailable",
                );
            }
        }
    }

    // ========== PROMPTS ==========
    {
        let t0 = Instant::now();
        match session.list_prompts().await {
            Ok(list) => {
                rows.push(Row {
                    name: "prompts/list".into(),
                    kind: "prompt".into(),
                    outcome: Outcome::Pass,
                    detail: format!("{} prompts", list.len()),
                    ms: t0.elapsed().as_millis(),
                });
                for p in &list {
                    let t1 = Instant::now();
                    let args = match p.name.as_str() {
                        "explore_and_map_app" | "write_e2e_for_flow" => Some(Map::from_iter([(
                            "feature".into(),
                            json!("coverage"),
                        )])),
                        "debug_failing_test" | "flaky_triage" => Some(Map::from_iter([(
                            "error".into(),
                            json!("timeout waiting for selector"),
                        )])),
                        _ => None,
                    };
                    match session.get_prompt(&p.name, args).await {
                        Ok(_) => rows.push(Row {
                            name: format!("prompts/get:{}", p.name),
                            kind: "prompt".into(),
                            outcome: Outcome::Pass,
                            detail: "ok".into(),
                            ms: t1.elapsed().as_millis(),
                        }),
                        Err(e) => rows.push(Row {
                            name: format!("prompts/get:{}", p.name),
                            kind: "prompt".into(),
                            outcome: Outcome::Fail,
                            detail: truncate(&e.to_string(), 120),
                            ms: t1.elapsed().as_millis(),
                        }),
                    }
                }
            }
            Err(e) => rows.push(Row {
                name: "prompts/list".into(),
                kind: "prompt".into(),
                outcome: Outcome::Fail,
                detail: truncate(&e.to_string(), 120),
                ms: t0.elapsed().as_millis(),
            }),
        }
    }

    finish(session, &rows, &advertised, started).await
}

struct State {
    browser_id: Option<String>,
    context_id: Option<String>,
    page_id: Option<String>,
    second_page_id: Option<String>,
    second_context_id: Option<String>,
    base_url: String,
}

fn extract_ids(v: &Value, state: &mut State) {
    if let Some(id) = v
        .pointer("/active/browser_id")
        .or_else(|| v.pointer("/launch/browserId"))
        .or_else(|| v.get("browserId"))
        .and_then(|x| x.as_str())
    {
        state.browser_id = Some(id.to_string());
    }
    if let Some(id) = v
        .pointer("/active/context_id")
        .or_else(|| v.pointer("/context/contextId"))
        .or_else(|| v.get("contextId"))
        .and_then(|x| x.as_str())
    {
        state.context_id = Some(id.to_string());
    }
    if let Some(id) = v
        .pointer("/active/page_id")
        .or_else(|| v.pointer("/page/pageId"))
        .or_else(|| v.get("pageId"))
        .and_then(|x| x.as_str())
    {
        state.page_id = Some(id.to_string());
    }
    // nested launch result shape from browser_launch
    if let Some(a) = v.get("active") {
        extract_ids(a, state);
    }
    if let Some(l) = v.get("launch") {
        if let Some(id) = l.get("browserId").and_then(|x| x.as_str()) {
            state.browser_id = Some(id.to_string());
        }
    }
    if let Some(c) = v.get("context") {
        if let Some(id) = c.get("contextId").and_then(|x| x.as_str()) {
            state.context_id = Some(id.to_string());
        }
    }
    if let Some(p) = v.get("page") {
        if let Some(id) = p.get("pageId").and_then(|x| x.as_str()) {
            state.page_id = Some(id.to_string());
        }
    }
}

fn mark_remaining_skip(rows: &mut Vec<Row>, advertised: &HashSet<String>, reason: &str) {
    let tested: HashSet<_> = rows.iter().map(|r| r.name.clone()).collect();
    for name in advertised {
        if !tested.contains(name) {
            rows.push(Row {
                name: name.clone(),
                kind: "tool".into(),
                outcome: Outcome::Skip,
                detail: reason.into(),
                ms: 0,
            });
        }
    }
}

async fn finish(
    session: &Session,
    rows: &[Row],
    advertised: &HashSet<String>,
    started: Instant,
) -> Result<i32> {
    // Coverage of advertised tools
    let mut tool_outcomes: HashMap<String, Outcome> = HashMap::new();
    for r in rows {
        if r.kind == "tool" {
            // keep worst outcome if multiple
            tool_outcomes
                .entry(r.name.clone())
                .and_modify(|o| {
                    *o = worst(*o, r.outcome);
                })
                .or_insert(r.outcome);
        }
    }

    let mut missing = Vec::new();
    for name in advertised {
        if !tool_outcomes.contains_key(name) {
            missing.push(name.clone());
        }
    }

    // Print table
    println!("{}", "Results".bold().cyan());
    println!(
        "{:<42} {:<8} {:>6}  {}",
        "NAME".dimmed(),
        "STATUS".dimmed(),
        "ms".dimmed(),
        "DETAIL".dimmed()
    );
    for r in rows {
        let status = match r.outcome {
            Outcome::Pass => "PASS".green().bold(),
            Outcome::Fail => "FAIL".red().bold(),
            Outcome::Skip => "SKIP".yellow().bold(),
        };
        println!(
            "{:<42} {:<8} {:>6}  {}",
            r.name,
            status,
            r.ms,
            r.detail.replace('\n', " ")
        );
    }

    if !missing.is_empty() {
        println!(
            "\n{} {} advertised tools never invoked:",
            "MISSING".red().bold(),
            missing.len()
        );
        for m in &missing {
            println!("  - {m}");
        }
    }

    let pass = rows.iter().filter(|r| r.outcome == Outcome::Pass).count();
    let fail = rows.iter().filter(|r| r.outcome == Outcome::Fail).count();
    let skip = rows.iter().filter(|r| r.outcome == Outcome::Skip).count();
    let tool_pass = tool_outcomes
        .values()
        .filter(|o| **o == Outcome::Pass)
        .count();
    let tool_fail = tool_outcomes
        .values()
        .filter(|o| **o == Outcome::Fail)
        .count();
    let tool_skip = tool_outcomes
        .values()
        .filter(|o| **o == Outcome::Skip)
        .count();

    println!();
    println!("{}", "Summary".bold().cyan());
    println!(
        "  rows:  {} pass · {} fail · {} skip  ({} total) in {:.1}s",
        pass,
        fail,
        skip,
        rows.len(),
        started.elapsed().as_secs_f64()
    );
    println!(
        "  tools: {}/{} advertised  ({} pass · {} fail · {} skip · {} missing)",
        tool_outcomes.len(),
        advertised.len(),
        tool_pass,
        tool_fail,
        tool_skip,
        missing.len()
    );

    // Write JSON report via agent path — use list is enough; write with tool if possible
    let report = json!({
        "pass": pass,
        "fail": fail,
        "skip": skip,
        "advertised_tools": advertised.len(),
        "tools_exercised": tool_outcomes.len(),
        "missing_tools": missing,
        "rows": rows.iter().map(|r| json!({
            "name": r.name,
            "kind": r.kind,
            "outcome": match r.outcome {
                Outcome::Pass => "pass",
                Outcome::Fail => "fail",
                Outcome::Skip => "skip",
            },
            "detail": r.detail,
            "ms": r.ms,
        })).collect::<Vec<_>>(),
    });
    let report_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../servers/browser-artifacts/coverage-report.json");
    if let Some(parent) = report_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).unwrap_or_default(),
    );
    println!("  report: {}", report_path.display());

    let _ = session; // keep session alive until caller shutdown

    if fail > 0 || !missing.is_empty() {
        Ok(3)
    } else {
        Ok(0)
    }
}

fn worst(a: Outcome, b: Outcome) -> Outcome {
    use Outcome::*;
    match (a, b) {
        (Fail, _) | (_, Fail) => Fail,
        (Pass, Pass) => Pass,
        (Skip, Pass) | (Pass, Skip) => Pass, // if any pass, count tool as pass
        (Skip, Skip) => Skip,
    }
}

fn truncate(s: &str, n: usize) -> String {
    let s = s.replace('\n', " ");
    if s.chars().count() <= n {
        s
    } else {
        format!("{}…", s.chars().take(n.saturating_sub(1)).collect::<String>())
    }
}

fn summary(v: &Value) -> String {
    match v {
        Value::Object(m) => {
            if let Some(b) = m.get("ok").and_then(|x| x.as_bool()) {
                return format!("ok={b}");
            }
            if let Some(p) = m.get("passed").and_then(|x| x.as_bool()) {
                return format!("passed={p}");
            }
            if let Some(n) = m.get("browserId")
                .or_else(|| m.get("active").and_then(|a| a.get("browser_id")))
            {
                return format!("browserId={n}");
            }
            format!("{} keys", m.len())
        }
        Value::String(s) => truncate(s, 100),
        other => truncate(&other.to_string(), 100),
    }
}
