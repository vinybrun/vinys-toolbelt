//! Browser testing MCP server — full suite surface.
#![allow(dead_code, clippy::too_many_arguments)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{
        router::{prompt::PromptRouter, tool::ToolRouter},
        wrapper::Parameters,
    },
    model::*,
    prompt, prompt_handler, prompt_router, schemars, task_handler,
    task_manager::OperationProcessor,
    tool, tool_handler, tool_router,
    service::RequestContext,
};
use serde_json::{Value, json};
use tokio::process::Command;
use tokio::sync::Mutex;

use super::artifacts::ArtifactStore;
use super::bridge::Bridge;
use super::config::BrowserConfig;
use super::safety::{check_url_allowed, redact_secrets, resolve_workspace_path};
use super::suite::{discover_suites, load_suite_file};
use super::visual::pixel_diff;

// ── arg types ──────────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EmptyArgs {}

#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct InstallDepsArgs {
    /// Also install Playwright browsers (default true)
    #[serde(default)]
    pub install_browsers: Option<bool>,
}

#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct BrowserCloseArgs {
    #[serde(default)]
    pub browser_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PageSwitchArgs {
    pub page_id: String,
}

#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct ExpectVisibleArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub test_id: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub hidden: Option<bool>,
    #[serde(default)]
    pub timeout: Option<u64>,
}

#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct ExpectTitleArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub exact: Option<bool>,
}

#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct PdfSaveArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct ClockInstallArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub time: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LaunchArgs {
    /// Browser engine: chromium | firefox | webkit
    #[serde(default)]
    pub browser: Option<String>,
    #[serde(default)]
    pub headless: Option<bool>,
    #[serde(default)]
    pub slow_mo: Option<u64>,
    #[serde(default)]
    pub devtools: Option<bool>,
    #[serde(default)]
    pub channel: Option<String>,
    /// Create a default context + page after launch (default true)
    #[serde(default = "default_true")]
    pub create_page: Option<bool>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub viewport_width: Option<i64>,
    #[serde(default)]
    pub viewport_height: Option<i64>,
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub color_scheme: Option<String>,
    #[serde(default)]
    pub record_trace: Option<bool>,
    #[serde(default)]
    pub record_video: Option<bool>,
    #[serde(default)]
    pub storage_state_path: Option<String>,
}

fn default_true() -> Option<bool> {
    Some(true)
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BrowserIdArgs {
    pub browser_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ContextNewArgs {
    pub browser_id: String,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub viewport_width: Option<i64>,
    #[serde(default)]
    pub viewport_height: Option<i64>,
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub color_scheme: Option<String>,
    #[serde(default)]
    pub offline: Option<bool>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub storage_state_path: Option<String>,
    #[serde(default)]
    pub record_trace: Option<bool>,
    #[serde(default)]
    pub record_video: Option<bool>,
    #[serde(default)]
    pub permissions: Option<Vec<String>>,
    #[serde(default)]
    pub geolocation: Option<Value>,
    #[serde(default)]
    pub extra_http_headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub ignore_https_errors: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ContextIdArgs {
    pub context_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PageNewArgs {
    pub context_id: String,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PageIdArgs {
    /// Page id. Omit to use the active page.
    #[serde(default)]
    pub page_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GotoArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    pub url: String,
    #[serde(default)]
    pub wait_until: Option<String>,
    #[serde(default)]
    pub timeout: Option<u64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LocatorArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub xpath: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub test_id: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub exact: Option<bool>,
    #[serde(default)]
    pub nth: Option<i64>,
    #[serde(default)]
    pub has_text: Option<String>,
    #[serde(default)]
    pub timeout: Option<u64>,
    #[serde(default)]
    pub force: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FillArgs {
    #[serde(flatten)]
    pub locator: LocatorArgs,
    pub value: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TypeArgs {
    #[serde(flatten)]
    pub locator: LocatorArgs,
    /// Characters to type (key-by-key). Prefer this over `text` which is a locator filter when flattened.
    pub content: String,
    #[serde(default)]
    pub delay: Option<u64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PressArgs {
    #[serde(flatten)]
    pub locator: LocatorArgs,
    pub key: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SelectArgs {
    #[serde(flatten)]
    pub locator: LocatorArgs,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub index: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UploadArgs {
    #[serde(flatten)]
    pub locator: LocatorArgs,
    /// Workspace-relative file path(s)
    pub files: Vec<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DragArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    pub source_selector: String,
    pub target_selector: String,
    #[serde(default)]
    pub timeout: Option<u64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ScrollArgs {
    #[serde(flatten)]
    pub locator: LocatorArgs,
    #[serde(default)]
    pub x: Option<i64>,
    #[serde(default)]
    pub y: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExpectTextArgs {
    #[serde(flatten)]
    pub locator: LocatorArgs,
    pub text: String,
    #[serde(default)]
    pub exact: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExpectUrlArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    pub url: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExpectCountArgs {
    #[serde(flatten)]
    pub locator: LocatorArgs,
    #[serde(default)]
    pub count: Option<i64>,
    #[serde(default)]
    pub min: Option<i64>,
    #[serde(default)]
    pub max: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ScreenshotArgs {
    #[serde(flatten)]
    pub locator: LocatorArgs,
    #[serde(default)]
    pub full_page: Option<bool>,
    /// Workspace-relative output path
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub include_base64: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RouteMockArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub context_id: Option<String>,
    pub url: String,
    #[serde(default)]
    pub abort: Option<bool>,
    #[serde(default)]
    pub status: Option<i64>,
    #[serde(default)]
    pub body: Option<Value>,
    #[serde(default)]
    pub content_type: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WaitResponseArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub url_includes: Option<String>,
    #[serde(default)]
    pub status: Option<i64>,
    #[serde(default)]
    pub timeout: Option<u64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StorageStateArgs {
    pub context_id: String,
    /// Workspace-relative path to save/load
    pub path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LoginAsArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    pub url: String,
    pub user_selector: String,
    pub password_selector: String,
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub submit_selector: Option<String>,
    #[serde(default)]
    pub success_url: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RunSuiteArgs {
    /// Workspace-relative suite path (.yaml/.yml/.json) or absolute name from discover
    pub path: String,
    #[serde(default)]
    pub browser: Option<String>,
    #[serde(default)]
    pub headless: Option<bool>,
    #[serde(default)]
    pub base_url: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RunPlaywrightArgs {
    /// Extra args to `npx playwright test`
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub grep: Option<String>,
    #[serde(default)]
    pub headed: Option<bool>,
    #[serde(default)]
    pub workers: Option<i64>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RunStepsArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    pub steps: Vec<Value>,
    #[serde(default)]
    pub stop_on_failure: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct VisualDiffArgs {
    /// Workspace-relative baseline image
    pub baseline_path: String,
    /// Workspace-relative current image (or omit to screenshot first)
    #[serde(default)]
    pub current_path: Option<String>,
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub threshold: Option<f64>,
    #[serde(default)]
    pub update_baseline: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct A11yArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    /// minimum impact: minor|moderate|serious|critical
    #[serde(default)]
    pub impact: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AppStartArgs {
    pub program: String,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub cwd: Option<String>,
    /// Health URL to wait for (must pass allowlist)
    #[serde(default)]
    pub health_url: Option<String>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PathArgs {
    pub path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ConnectCdpArgs {
    pub endpoint: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PersistentArgs {
    /// Workspace-relative user data dir
    pub user_data_dir: String,
    #[serde(default)]
    pub browser: Option<String>,
    #[serde(default)]
    pub headless: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EmulateMediaArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub media: Option<String>,
    #[serde(default)]
    pub color_scheme: Option<String>,
    #[serde(default)]
    pub reduced_motion: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DialogPolicyArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    /// accept | dismiss
    pub policy: String,
    #[serde(default)]
    pub prompt_text: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EvaluateArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    pub expression: String,
    #[serde(default)]
    pub arg: Option<Value>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct NetworkLogArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub url_includes: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExportCodeArgs {
    pub steps: Vec<Value>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SuggestLocatorArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ViewportArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    pub width: i64,
    pub height: i64,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct OfflineArgs {
    pub context_id: String,
    pub offline: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PermissionsArgs {
    pub context_id: String,
    pub permissions: Vec<String>,
    #[serde(default)]
    pub origin: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CookiesArgs {
    pub context_id: String,
    #[serde(default)]
    pub cookies: Option<Value>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct HarArgs {
    pub context_id: String,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TraceArgs {
    pub context_id: String,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WaitUrlArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    pub url: String,
    #[serde(default)]
    pub timeout: Option<u64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LogLimitArgs {
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub types: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct NoteArgs {
    pub note: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FixtureServerArgs {
    #[serde(default)]
    pub port: Option<u16>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct FlowPromptArgs {
    pub feature: String,
    #[serde(default)]
    pub base_url: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct DebugFailPromptArgs {
    pub error: String,
    #[serde(default)]
    pub context: Option<String>,
}

// ── server state ───────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct BrowserTesting {
    cfg: Arc<BrowserConfig>,
    bridge: Arc<Bridge>,
    artifacts: Arc<ArtifactStore>,
    tool_router: ToolRouter<Self>,
    prompt_router: PromptRouter<Self>,
    processor: Arc<Mutex<OperationProcessor>>,
    /// active session handles
    session: Arc<Mutex<SessionState>>,
    notes: Arc<Mutex<Vec<String>>>,
    app_child: Arc<Mutex<Option<u32>>>, // pid
    fixture_child: Arc<Mutex<Option<tokio::process::Child>>>,
}

#[derive(Default)]
struct SessionState {
    browser_id: Option<String>,
    context_id: Option<String>,
    page_id: Option<String>,
    base_url: Option<String>,
    last_screenshot: Option<PathBuf>,
    last_run_id: Option<String>,
    recording_steps: Vec<Value>,
    recording: bool,
}

impl BrowserTesting {
    pub fn new(workspace: impl Into<PathBuf>) -> Self {
        let cfg = BrowserConfig::from_env(workspace.into());
        let artifacts = ArtifactStore::new(&cfg);
        let bridge = Bridge::new(cfg.clone());
        Self {
            bridge: Arc::new(bridge),
            artifacts: Arc::new(artifacts),
            cfg: Arc::new(cfg),
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
            processor: Arc::new(Mutex::new(OperationProcessor::new())),
            session: Arc::new(Mutex::new(SessionState::default())),
            notes: Arc::new(Mutex::new(Vec::new())),
            app_child: Arc::new(Mutex::new(None)),
            fixture_child: Arc::new(Mutex::new(None)),
        }
    }

    async fn page_id(&self, explicit: Option<String>) -> Result<String, McpError> {
        if let Some(p) = explicit {
            return Ok(p);
        }
        let s = self.session.lock().await;
        s.page_id.clone().ok_or_else(|| {
            McpError::invalid_params(
                "no active page; call browser_launch or page_new first",
                None,
            )
        })
    }

    async fn context_id(&self, explicit: Option<String>) -> Result<String, McpError> {
        if let Some(c) = explicit {
            return Ok(c);
        }
        let s = self.session.lock().await;
        s.context_id.clone().ok_or_else(|| {
            McpError::invalid_params("no active context; call browser_launch first", None)
        })
    }

    fn locator_params(loc: &LocatorArgs, page_id: &str) -> Value {
        let mut m = json!({ "pageId": page_id });
        let o = m.as_object_mut().unwrap();
        if let Some(v) = &loc.selector {
            o.insert("selector".into(), json!(v));
        }
        if let Some(v) = &loc.xpath {
            o.insert("xpath".into(), json!(v));
        }
        if let Some(v) = &loc.role {
            o.insert("role".into(), json!(v));
        }
        if let Some(v) = &loc.name {
            o.insert("name".into(), json!(v));
        }
        if let Some(v) = &loc.test_id {
            o.insert("testId".into(), json!(v));
        }
        if let Some(v) = &loc.label {
            o.insert("label".into(), json!(v));
        }
        if let Some(v) = &loc.placeholder {
            o.insert("placeholder".into(), json!(v));
        }
        if let Some(v) = &loc.text {
            o.insert("text".into(), json!(v));
        }
        if let Some(v) = loc.exact {
            o.insert("exact".into(), json!(v));
        }
        if let Some(v) = loc.nth {
            o.insert("nth".into(), json!(v));
        }
        if let Some(v) = &loc.has_text {
            o.insert("hasText".into(), json!(v));
        }
        if let Some(v) = loc.timeout {
            o.insert("timeout".into(), json!(v));
        }
        if let Some(v) = loc.force {
            o.insert("force".into(), json!(v));
        }
        m
    }

    async fn call(&self, method: &str, params: Value) -> Result<Value, McpError> {
        self.bridge.call(method, params).await
    }

    async fn record_step(&self, step: Value) {
        let mut s = self.session.lock().await;
        if s.recording {
            s.recording_steps.push(step);
        }
    }

    fn ok_json(v: Value) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![ContentBlock::text(
            redact_secrets(&serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string())),
        )]))
    }

    fn ok_text(s: impl Into<String>) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![ContentBlock::text(redact_secrets(&s.into()))]))
    }

    async fn resolve_url(&self, url: &str) -> Result<String, McpError> {
        let s = self.session.lock().await;
        let full = if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else if let Some(base) = s.base_url.as_ref().or(self.cfg.base_url.as_ref()) {
            let base = base.trim_end_matches('/');
            if url.starts_with('/') {
                format!("{base}{url}")
            } else {
                format!("{base}/{url}")
            }
        } else {
            url.to_string()
        };
        check_url_allowed(&self.cfg, &full)?;
        Ok(full)
    }
}

// ── tools ──────────────────────────────────────────────────────────────────

#[tool_router]
impl BrowserTesting {
    #[tool(
        name = "browser_info",
        description = "Show browser MCP server config, active session, artifact root, allowlist, and capability overview. Call first."
    )]
    async fn browser_info(&self) -> Result<CallToolResult, McpError> {
        let s = self.session.lock().await;
        Self::ok_json(json!({
            "workspace": self.cfg.workspace,
            "artifacts": self.artifacts.root(),
            "bridge_script": self.cfg.bridge_script,
            "allowed_hosts": self.cfg.allowed_hosts,
            "allow_all_hosts": self.cfg.allow_all_hosts,
            "headless_only": self.cfg.headless_only,
            "base_url": s.base_url.as_ref().or(self.cfg.base_url.as_ref()),
            "session": {
                "browser_id": s.browser_id,
                "context_id": s.context_id,
                "page_id": s.page_id,
                "recording": s.recording,
                "last_run_id": s.last_run_id,
                "last_screenshot": s.last_screenshot,
            },
            "capabilities": [
                "launch/close multi-browser (chromium/firefox/webkit)",
                "contexts, pages, device emulation, permissions, geolocation",
                "locators: css, xpath, role, testid, label, text",
                "actions: click/fill/type/press/select/check/upload/drag/scroll",
                "observe: a11y snapshot, screenshot, console, network",
                "assert: text/url/title/count/attribute/console",
                "network mock, HAR, tracing, video",
                "auth storageState, login helper",
                "declarative YAML suites + Playwright CLI",
                "a11y axe scan, visual pixel diff",
                "codegen export, recording, fixtures server",
                "app lifecycle, performance/web vitals, cookie/CSP reports",
                "MCP resources + prompts + long-running tasks"
            ],
            "safety": "URL allowlist, workspace-scoped paths, secret redaction, no file://",
            "env": {
                "MCP_BROWSER_ALLOWED_HOSTS": "comma hosts",
                "MCP_BROWSER_ALLOW_ALL_HOSTS": "1 to disable allowlist",
                "MCP_BROWSER_BASE_URL": "default base",
                "MCP_BROWSER_ARTIFACTS": "artifact dir",
                "MCP_BROWSER_HEADLESS_ONLY": "1 for CI",
            }
        }))
    }

    #[tool(
        name = "browser_install_deps",
        description = "Install Node bridge dependencies (playwright, axe-core) and optionally Playwright browsers. Run once per machine."
    )]
    async fn browser_install_deps(
        &self,
        Parameters(args): Parameters<InstallDepsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let install_browsers = args.install_browsers.unwrap_or(true);
        let bridge_dir = self
            .cfg
            .bridge_script
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.cfg.workspace.clone());
        let npm = Command::new("npm")
            .args(["install"])
            .current_dir(&bridge_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| McpError::internal_error(format!("npm install failed: {e}"), None))?;
        let mut out = format!(
            "npm install status={}\n{}\n{}\n",
            npm.status,
            String::from_utf8_lossy(&npm.stdout),
            String::from_utf8_lossy(&npm.stderr)
        );
        if install_browsers {
            let pw = Command::new("npx")
                .args(["playwright", "install", "chromium"])
                .current_dir(&bridge_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|e| {
                    McpError::internal_error(format!("playwright install failed: {e}"), None)
                })?;
            out.push_str(&format!(
                "playwright install chromium status={}\n{}\n{}\n",
                pw.status,
                String::from_utf8_lossy(&pw.stdout),
                String::from_utf8_lossy(&pw.stderr)
            ));
        }
        Self::ok_text(out)
    }

    #[tool(
        name = "browser_launch",
        description = "Launch a browser (chromium/firefox/webkit), create context+page by default, return session ids."
    )]
    async fn browser_launch(
        &self,
        Parameters(args): Parameters<LaunchArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mut headless = args.headless.unwrap_or(self.cfg.headless_default);
        if self.cfg.headless_only {
            headless = true;
        }
        let mut launch_params = json!({
            "browser": args.browser.unwrap_or_else(|| "chromium".into()),
            "headless": headless,
            "slowMo": args.slow_mo.unwrap_or(0),
            "devtools": args.devtools.unwrap_or(false),
        });
        if let Some(ch) = args.channel {
            launch_params["channel"] = json!(ch);
        }
        let launch = self.call("launch", launch_params).await?;
        let browser_id = launch["browserId"].as_str().unwrap_or("").to_string();
        let mut result = json!({ "launch": launch });

        if args.create_page.unwrap_or(true) {
            let mut ctx_params = json!({
                "browserId": browser_id,
                "locale": args.locale,
                "timezone": args.timezone,
                "colorScheme": args.color_scheme,
                "baseURL": args.base_url.as_ref().or(self.cfg.base_url.as_ref()),
                "device": args.device,
                "trace": args.record_trace.unwrap_or(false),
                "ignoreHTTPSErrors": true,
            });
            if let (Some(w), Some(h)) = (args.viewport_width, args.viewport_height) {
                ctx_params["viewport"] = json!({ "width": w, "height": h });
            }
            if args.record_video.unwrap_or(false) {
                let video_dir = self.artifacts.root().join("video");
                let _ = tokio::fs::create_dir_all(&video_dir).await;
                ctx_params["recordVideo"] = json!({ "dir": video_dir });
            }
            if let Some(ss) = &args.storage_state_path {
                let p = resolve_workspace_path(&self.cfg, ss)?;
                ctx_params["storageState"] = json!(p);
            }
            let ctx = self.call("context_new", ctx_params).await?;
            let context_id = ctx["contextId"].as_str().unwrap_or("").to_string();
            let page = self
                .call("page_new", json!({ "contextId": context_id }))
                .await?;
            let page_id = page["pageId"].as_str().unwrap_or("").to_string();
            let mut s = self.session.lock().await;
            s.browser_id = Some(browser_id.clone());
            s.context_id = Some(context_id.clone());
            s.page_id = Some(page_id.clone());
            if let Some(b) = args.base_url.or(self.cfg.base_url.clone()) {
                s.base_url = Some(b);
            }
            result["context"] = ctx;
            result["page"] = page;
            result["active"] = json!({
                "browser_id": browser_id,
                "context_id": context_id,
                "page_id": page_id,
            });
        } else {
            let mut s = self.session.lock().await;
            s.browser_id = Some(browser_id);
        }
        Self::ok_json(result)
    }

    #[tool(name = "browser_close", description = "Close browser and clear active session.")]
    async fn browser_close(
        &self,
        Parameters(args): Parameters<BrowserCloseArgs>,
    ) -> Result<CallToolResult, McpError> {
        let browser_id = if let Some(id) = args.browser_id {
            id
        } else {
            self.session
                .lock()
                .await
                .browser_id
                .clone()
                .ok_or_else(|| McpError::invalid_params("no browser_id", None))?
        };
        let r = self
            .call("close_browser", json!({ "browserId": browser_id }))
            .await?;
        *self.session.lock().await = SessionState::default();
        Self::ok_json(r)
    }

    #[tool(name = "browser_connect_cdp", description = "Connect over CDP to an existing Chrome (remote debugging endpoint).")]
    async fn browser_connect_cdp(
        &self,
        Parameters(args): Parameters<ConnectCdpArgs>,
    ) -> Result<CallToolResult, McpError> {
        let r = self
            .call("connect_cdp", json!({ "endpoint": args.endpoint }))
            .await?;
        let mut s = self.session.lock().await;
        s.browser_id = r["browserId"].as_str().map(|x| x.to_string());
        s.context_id = r["contextId"].as_str().map(|x| x.to_string());
        Self::ok_json(r)
    }

    #[tool(name = "browser_persistent", description = "Launch persistent context with user data dir (keeps logins).")]
    async fn browser_persistent(
        &self,
        Parameters(args): Parameters<PersistentArgs>,
    ) -> Result<CallToolResult, McpError> {
        let dir = resolve_workspace_path(&self.cfg, &args.user_data_dir)?;
        tokio::fs::create_dir_all(&dir).await.map_err(|e| {
            McpError::internal_error(format!("create user_data_dir: {e}"), None)
        })?;
        let mut headless = args.headless.unwrap_or(true);
        if self.cfg.headless_only {
            headless = true;
        }
        let r = self
            .call(
                "context_persistent",
                json!({
                    "userDataDir": dir,
                    "browser": args.browser.unwrap_or_else(|| "chromium".into()),
                    "headless": headless,
                }),
            )
            .await?;
        let mut s = self.session.lock().await;
        s.browser_id = r["browserId"].as_str().map(|x| x.to_string());
        s.context_id = r["contextId"].as_str().map(|x| x.to_string());
        if let Some(cid) = &s.context_id {
            let page = self
                .call("page_new", json!({ "contextId": cid }))
                .await?;
            s.page_id = page["pageId"].as_str().map(|x| x.to_string());
        }
        Self::ok_json(r)
    }

    #[tool(name = "context_new", description = "Create a new browser context (isolated cookies/storage).")]
    async fn context_new(
        &self,
        Parameters(args): Parameters<ContextNewArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mut params = json!({
            "browserId": args.browser_id,
            "device": args.device,
            "userAgent": args.user_agent,
            "locale": args.locale,
            "timezone": args.timezone,
            "colorScheme": args.color_scheme,
            "offline": args.offline,
            "baseURL": args.base_url,
            "permissions": args.permissions,
            "geolocation": args.geolocation,
            "extraHTTPHeaders": args.extra_http_headers,
            "ignoreHTTPSErrors": args.ignore_https_errors.unwrap_or(true),
            "trace": args.record_trace.unwrap_or(false),
        });
        if let (Some(w), Some(h)) = (args.viewport_width, args.viewport_height) {
            params["viewport"] = json!({ "width": w, "height": h });
        }
        if let Some(ss) = args.storage_state_path {
            params["storageState"] = json!(resolve_workspace_path(&self.cfg, &ss)?);
        }
        if args.record_video.unwrap_or(false) {
            let video_dir = self.artifacts.root().join("video");
            let _ = tokio::fs::create_dir_all(&video_dir).await;
            params["recordVideo"] = json!({ "dir": video_dir });
        }
        let r = self.call("context_new", params).await?;
        let mut s = self.session.lock().await;
        s.context_id = r["contextId"].as_str().map(|x| x.to_string());
        Self::ok_json(r)
    }

    #[tool(name = "context_close", description = "Close a browser context.")]
    async fn context_close(
        &self,
        Parameters(args): Parameters<ContextIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let r = self
            .call("context_close", json!({ "contextId": args.context_id }))
            .await?;
        Self::ok_json(r)
    }

    #[tool(name = "page_new", description = "Open a new page in a context.")]
    async fn page_new(
        &self,
        Parameters(args): Parameters<PageNewArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mut params = json!({ "contextId": args.context_id });
        if let Some(url) = args.url {
            params["url"] = json!(self.resolve_url(&url).await?);
        }
        let r = self.call("page_new", params).await?;
        let mut s = self.session.lock().await;
        s.page_id = r["pageId"].as_str().map(|x| x.to_string());
        s.context_id = Some(args.context_id);
        Self::ok_json(r)
    }

    #[tool(name = "page_close", description = "Close a page.")]
    async fn page_close(
        &self,
        Parameters(args): Parameters<PageIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        let r = self.call("page_close", json!({ "pageId": page_id })).await?;
        Self::ok_json(r)
    }

    #[tool(name = "page_list", description = "List open pages.")]
    async fn page_list(&self) -> Result<CallToolResult, McpError> {
        Self::ok_json(self.call("page_list", json!({})).await?)
    }

    #[tool(name = "page_switch", description = "Set the active page id for subsequent tools.")]
    async fn page_switch(
        &self,
        Parameters(args): Parameters<PageSwitchArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = args.page_id;
        self.session.lock().await.page_id = Some(page_id.clone());
        let _ = self
            .call("page_bring_to_front", json!({ "pageId": page_id }))
            .await;
        Self::ok_json(json!({ "active_page_id": page_id }))
    }

    #[tool(name = "goto", description = "Navigate to URL (allowlisted). Supports relative paths when base_url is set.")]
    async fn goto(
        &self,
        Parameters(args): Parameters<GotoArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        let url = self.resolve_url(&args.url).await?;
        let r = self
            .call(
                "goto",
                json!({
                    "pageId": page_id,
                    "url": url,
                    "waitUntil": args.wait_until.unwrap_or_else(|| "load".into()),
                    "timeout": args.timeout,
                }),
            )
            .await?;
        self.record_step(json!({"action":"goto","url": url})).await;
        Self::ok_json(r)
    }

    #[tool(name = "go_back", description = "History back.")]
    async fn go_back(
        &self,
        Parameters(args): Parameters<PageIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(self.call("go_back", json!({ "pageId": page_id })).await?)
    }

    #[tool(name = "go_forward", description = "History forward.")]
    async fn go_forward(
        &self,
        Parameters(args): Parameters<PageIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(self.call("go_forward", json!({ "pageId": page_id })).await?)
    }

    #[tool(name = "reload", description = "Reload page.")]
    async fn reload(
        &self,
        Parameters(args): Parameters<PageIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(self.call("reload", json!({ "pageId": page_id })).await?)
    }

    #[tool(name = "wait_for_url", description = "Wait until URL matches glob/string.")]
    async fn wait_for_url(
        &self,
        Parameters(args): Parameters<WaitUrlArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(
            self.call(
                "wait_for_url",
                json!({ "pageId": page_id, "url": args.url, "timeout": args.timeout }),
            )
            .await?,
        )
    }

    #[tool(name = "click", description = "Click element by selector/role/testid/label/text.")]
    async fn click(
        &self,
        Parameters(args): Parameters<LocatorArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id.clone()).await?;
        let params = Self::locator_params(&args, &page_id);
        let r = self.call("click", params.clone()).await?;
        self.record_step(json!({"action":"click","locator": params})).await;
        Self::ok_json(r)
    }

    #[tool(name = "dblclick", description = "Double-click element.")]
    async fn dblclick(
        &self,
        Parameters(args): Parameters<LocatorArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id.clone()).await?;
        Self::ok_json(self.call("dblclick", Self::locator_params(&args, &page_id)).await?)
    }

    #[tool(name = "hover", description = "Hover element.")]
    async fn hover(
        &self,
        Parameters(args): Parameters<LocatorArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id.clone()).await?;
        Self::ok_json(self.call("hover", Self::locator_params(&args, &page_id)).await?)
    }

    #[tool(name = "fill", description = "Clear and fill an input.")]
    async fn fill(
        &self,
        Parameters(args): Parameters<FillArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.locator.page_id.clone()).await?;
        let mut params = Self::locator_params(&args.locator, &page_id);
        params["value"] = json!(args.value);
        let r = self.call("fill", params.clone()).await?;
        self.record_step(json!({"action":"fill","locator": params, "value": "***"})).await;
        Self::ok_json(r)
    }

    #[tool(name = "type_text", description = "Type text key-by-key into element.")]
    async fn type_text(
        &self,
        Parameters(args): Parameters<TypeArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.locator.page_id.clone()).await?;
        let mut params = Self::locator_params(&args.locator, &page_id);
        // Bridge treats `content` as keystrokes; `text` is reserved for getByText locators.
        if let Some(obj) = params.as_object_mut() {
            obj.remove("text");
            obj.insert("content".into(), json!(args.content));
            obj.insert("delay".into(), json!(args.delay));
        }
        Self::ok_json(self.call("type", params).await?)
    }

    #[tool(name = "press", description = "Press a keyboard key (optionally on a locator).")]
    async fn press(
        &self,
        Parameters(args): Parameters<PressArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.locator.page_id.clone()).await?;
        let mut params = Self::locator_params(&args.locator, &page_id);
        params["key"] = json!(args.key);
        Self::ok_json(self.call("press", params).await?)
    }

    #[tool(name = "select_option", description = "Select option in <select> by value/label/index.")]
    async fn select_option(
        &self,
        Parameters(args): Parameters<SelectArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.locator.page_id.clone()).await?;
        let mut params = Self::locator_params(&args.locator, &page_id);
        params["value"] = json!(args.value);
        params["label"] = json!(args.label);
        params["index"] = json!(args.index);
        Self::ok_json(self.call("select_option", params).await?)
    }

    #[tool(name = "check", description = "Check checkbox/radio.")]
    async fn check(
        &self,
        Parameters(args): Parameters<LocatorArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id.clone()).await?;
        Self::ok_json(self.call("check", Self::locator_params(&args, &page_id)).await?)
    }

    #[tool(name = "uncheck", description = "Uncheck checkbox.")]
    async fn uncheck(
        &self,
        Parameters(args): Parameters<LocatorArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id.clone()).await?;
        Self::ok_json(self.call("uncheck", Self::locator_params(&args, &page_id)).await?)
    }

    #[tool(name = "upload_file", description = "Set input[type=file] to workspace-relative paths.")]
    async fn upload_file(
        &self,
        Parameters(args): Parameters<UploadArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.locator.page_id.clone()).await?;
        let mut files = Vec::new();
        for f in &args.files {
            files.push(resolve_workspace_path(&self.cfg, f)?);
        }
        let mut params = Self::locator_params(&args.locator, &page_id);
        params["files"] = json!(files);
        Self::ok_json(self.call("set_input_files", params).await?)
    }

    #[tool(name = "drag_drop", description = "Drag source selector onto target selector.")]
    async fn drag_drop(
        &self,
        Parameters(args): Parameters<DragArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(
            self.call(
                "drag_to",
                json!({
                    "pageId": page_id,
                    "source": { "selector": args.source_selector },
                    "target": { "selector": args.target_selector },
                    "timeout": args.timeout,
                }),
            )
            .await?,
        )
    }

    #[tool(name = "scroll", description = "Scroll element into view or by x/y pixels.")]
    async fn scroll(
        &self,
        Parameters(args): Parameters<ScrollArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.locator.page_id.clone()).await?;
        let mut params = Self::locator_params(&args.locator, &page_id);
        params["x"] = json!(args.x);
        params["y"] = json!(args.y);
        Self::ok_json(self.call("scroll", params).await?)
    }

    #[tool(name = "set_viewport", description = "Set viewport size.")]
    async fn set_viewport(
        &self,
        Parameters(args): Parameters<ViewportArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(
            self.call(
                "set_viewport",
                json!({ "pageId": page_id, "width": args.width, "height": args.height }),
            )
            .await?,
        )
    }

    #[tool(name = "emulate_media", description = "Emulate media type / color scheme / reduced motion.")]
    async fn emulate_media(
        &self,
        Parameters(args): Parameters<EmulateMediaArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(
            self.call(
                "emulated_media",
                json!({
                    "pageId": page_id,
                    "media": args.media,
                    "colorScheme": args.color_scheme,
                    "reducedMotion": args.reduced_motion,
                }),
            )
            .await?,
        )
    }

    #[tool(name = "set_dialog_policy", description = "Set how JS dialogs are handled: accept or dismiss.")]
    async fn set_dialog_policy(
        &self,
        Parameters(args): Parameters<DialogPolicyArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(
            self.call(
                "set_dialog_policy",
                json!({
                    "pageId": page_id,
                    "policy": args.policy,
                    "promptText": args.prompt_text,
                }),
            )
            .await?,
        )
    }

    #[tool(name = "evaluate", description = "Evaluate JavaScript in the page.")]
    async fn evaluate(
        &self,
        Parameters(args): Parameters<EvaluateArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(
            self.call(
                "evaluate",
                json!({ "pageId": page_id, "expression": args.expression, "arg": args.arg }),
            )
            .await?,
        )
    }

    #[tool(name = "snapshot", description = "Accessibility-oriented page snapshot for agent observation (preferred over raw HTML).")]
    async fn snapshot(
        &self,
        Parameters(args): Parameters<PageIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(self.call("snapshot_a11y", json!({ "pageId": page_id })).await?)
    }

    #[tool(name = "screenshot", description = "Capture screenshot (viewport, full page, or element). Saves under artifacts/.")]
    async fn screenshot(
        &self,
        Parameters(args): Parameters<ScreenshotArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.locator.page_id.clone()).await?;
        let rel = args.path.unwrap_or_else(|| {
            format!(
                "screenshots/shot_{}.png",
                chrono::Utc::now().format("%Y%m%d_%H%M%S")
            )
        });
        let path = self.artifacts.root().join(&rel);
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let mut params = Self::locator_params(&args.locator, &page_id);
        params["path"] = json!(path);
        params["fullPage"] = json!(args.full_page.unwrap_or(false));
        params["includeBase64"] = json!(args.include_base64.unwrap_or(false));
        let r = self.call("screenshot", params).await?;
        self.session.lock().await.last_screenshot = Some(path.clone());
        Self::ok_json(json!({
            "result": r,
            "path": path,
            "uri": format!("artifact://{}", rel),
        }))
    }

    #[tool(name = "page_title", description = "Get page title and URL.")]
    async fn page_title(
        &self,
        Parameters(args): Parameters<PageIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(self.call("title", json!({ "pageId": page_id })).await?)
    }

    #[tool(name = "console_messages", description = "Get buffered console messages.")]
    async fn console_messages(
        &self,
        Parameters(args): Parameters<LogLimitArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(
            self.call(
                "console_messages",
                json!({ "pageId": page_id, "limit": args.limit, "types": args.types }),
            )
            .await?,
        )
    }

    #[tool(name = "page_errors", description = "Get page error / uncaught exception buffer.")]
    async fn page_errors(
        &self,
        Parameters(args): Parameters<LogLimitArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(
            self.call(
                "page_errors",
                json!({ "pageId": page_id, "limit": args.limit }),
            )
            .await?,
        )
    }

    #[tool(name = "network_log", description = "Get buffered network request/response log.")]
    async fn network_log(
        &self,
        Parameters(args): Parameters<NetworkLogArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(
            self.call(
                "network_log",
                json!({
                    "pageId": page_id,
                    "urlIncludes": args.url_includes,
                    "kind": args.kind,
                    "limit": args.limit,
                }),
            )
            .await?,
        )
    }

    #[tool(name = "clear_logs", description = "Clear console/network/error buffers for a page.")]
    async fn clear_logs(
        &self,
        Parameters(args): Parameters<PageIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(self.call("clear_logs", json!({ "pageId": page_id })).await?)
    }

    #[tool(name = "expect_visible", description = "Assert element is visible (or hidden if hidden=true).")]
    async fn expect_visible(
        &self,
        Parameters(args): Parameters<ExpectVisibleArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id.clone()).await?;
        let mut params = json!({
            "pageId": page_id,
            "selector": args.selector,
            "role": args.role,
            "name": args.name,
            "testId": args.test_id,
            "text": args.text,
            "hidden": args.hidden,
            "timeout": args.timeout,
        });
        Self::ok_json(self.call("expect_visible", params).await?)
    }

    #[tool(name = "expect_text", description = "Assert text is present on page or in locator.")]
    async fn expect_text(
        &self,
        Parameters(args): Parameters<ExpectTextArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.locator.page_id.clone()).await?;
        let mut params = Self::locator_params(&args.locator, &page_id);
        params["text"] = json!(args.text);
        params["exact"] = json!(args.exact);
        let r = self.call("expect_text", params).await?;
        self.record_step(json!({"action":"expect_text","text": args.text})).await;
        Self::ok_json(r)
    }

    #[tool(name = "expect_url", description = "Assert current URL matches pattern.")]
    async fn expect_url(
        &self,
        Parameters(args): Parameters<ExpectUrlArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(
            self.call(
                "expect_url",
                json!({ "pageId": page_id, "url": args.url }),
            )
            .await?,
        )
    }

    #[tool(name = "expect_title", description = "Assert page title contains/equals text.")]
    async fn expect_title(
        &self,
        Parameters(args): Parameters<ExpectTitleArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id.clone()).await?;
        Self::ok_json(self.call("expect_title", json!({
            "pageId": page_id,
            "title": args.title,
            "text": args.text,
            "exact": args.exact,
        })).await?)
    }

    #[tool(name = "expect_count", description = "Assert locator match count.")]
    async fn expect_count(
        &self,
        Parameters(args): Parameters<ExpectCountArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.locator.page_id.clone()).await?;
        let mut params = Self::locator_params(&args.locator, &page_id);
        params["count"] = json!(args.count);
        params["min"] = json!(args.min);
        params["max"] = json!(args.max);
        Self::ok_json(self.call("expect_count", params).await?)
    }

    #[tool(name = "expect_no_console_errors", description = "Fail if console errors were logged.")]
    async fn expect_no_console_errors(
        &self,
        Parameters(args): Parameters<PageIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(
            self.call("expect_no_console_errors", json!({ "pageId": page_id }))
                .await?,
        )
    }

    #[tool(name = "locator_info", description = "Inspect locator: count, visible, enabled, text, box.")]
    async fn locator_info(
        &self,
        Parameters(args): Parameters<LocatorArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id.clone()).await?;
        Self::ok_json(self.call("locator_info", Self::locator_params(&args, &page_id)).await?)
    }

    #[tool(name = "why_not_actionable", description = "Diagnose why a locator is not clickable.")]
    async fn why_not_actionable(
        &self,
        Parameters(args): Parameters<LocatorArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id.clone()).await?;
        Self::ok_json(
            self.call("why_not_actionable", Self::locator_params(&args, &page_id))
                .await?,
        )
    }

    #[tool(name = "suggest_locator", description = "Suggest resilient locators from coordinates or text.")]
    async fn suggest_locator(
        &self,
        Parameters(args): Parameters<SuggestLocatorArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(
            self.call(
                "suggest_locator",
                json!({ "pageId": page_id, "x": args.x, "y": args.y, "text": args.text }),
            )
            .await?,
        )
    }

    #[tool(name = "route_mock", description = "Mock or abort network routes (URL glob).")]
    async fn route_mock(
        &self,
        Parameters(args): Parameters<RouteMockArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mut params = json!({ "url": args.url, "abort": args.abort });
        if let Some(pid) = args.page_id {
            params["pageId"] = json!(pid);
        } else if let Some(cid) = args.context_id {
            params["contextId"] = json!(cid);
        } else {
            params["pageId"] = json!(self.page_id(None).await?);
        }
        if args.body.is_some() || args.status.is_some() {
            params["fulfill"] = json!({
                "status": args.status.unwrap_or(200),
                "body": args.body,
                "contentType": args.content_type.unwrap_or_else(|| "application/json".into()),
            });
        }
        Self::ok_json(self.call("route_mock", params).await?)
    }

    #[tool(name = "wait_for_response", description = "Wait for a network response matching URL/status.")]
    async fn wait_for_response(
        &self,
        Parameters(args): Parameters<WaitResponseArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(
            self.call(
                "wait_for_response",
                json!({
                    "pageId": page_id,
                    "urlIncludes": args.url_includes,
                    "status": args.status,
                    "timeout": args.timeout,
                }),
            )
            .await?,
        )
    }

    #[tool(name = "har_start", description = "Start capturing HAR-like network log on a context.")]
    async fn har_start(
        &self,
        Parameters(args): Parameters<HarArgs>,
    ) -> Result<CallToolResult, McpError> {
        Self::ok_json(
            self.call("har_start", json!({ "contextId": args.context_id }))
                .await?,
        )
    }

    #[tool(name = "har_stop", description = "Stop HAR capture and save to artifacts.")]
    async fn har_stop(
        &self,
        Parameters(args): Parameters<HarArgs>,
    ) -> Result<CallToolResult, McpError> {
        let path = match args.path {
            Some(p) => resolve_workspace_path(&self.cfg, &p)?,
            None => self.artifacts.root().join(format!(
                "har/{}.har",
                chrono::Utc::now().format("%Y%m%d_%H%M%S")
            )),
        };
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        Self::ok_json(
            self.call(
                "har_stop",
                json!({ "contextId": args.context_id, "path": path }),
            )
            .await?,
        )
    }

    #[tool(name = "tracing_start", description = "Start Playwright tracing on a context.")]
    async fn tracing_start(
        &self,
        Parameters(args): Parameters<TraceArgs>,
    ) -> Result<CallToolResult, McpError> {
        Self::ok_json(
            self.call("tracing_start", json!({ "contextId": args.context_id }))
                .await?,
        )
    }

    #[tool(name = "tracing_stop", description = "Stop tracing and write a .zip under artifacts.")]
    async fn tracing_stop(
        &self,
        Parameters(args): Parameters<TraceArgs>,
    ) -> Result<CallToolResult, McpError> {
        let path = match args.path {
            Some(p) => resolve_workspace_path(&self.cfg, &p)?,
            None => self.artifacts.root().join(format!(
                "traces/trace_{}.zip",
                chrono::Utc::now().format("%Y%m%d_%H%M%S")
            )),
        };
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        Self::ok_json(
            self.call(
                "tracing_stop",
                json!({ "contextId": args.context_id, "path": path }),
            )
            .await?,
        )
    }

    #[tool(name = "storage_state_save", description = "Save cookies/localStorage for auth reuse.")]
    async fn storage_state_save(
        &self,
        Parameters(args): Parameters<StorageStateArgs>,
    ) -> Result<CallToolResult, McpError> {
        let path = resolve_workspace_path(&self.cfg, &args.path)?;
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        Self::ok_json(
            self.call(
                "context_storage_state",
                json!({ "contextId": args.context_id, "path": path }),
            )
            .await?,
        )
    }

    #[tool(name = "login_as", description = "Helper: goto login URL, fill user/pass, submit, optional success URL wait.")]
    async fn login_as(
        &self,
        Parameters(args): Parameters<LoginAsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        let url = self.resolve_url(&args.url).await?;
        self.call("goto", json!({ "pageId": page_id, "url": url }))
            .await?;
        self.call(
            "fill",
            json!({ "pageId": page_id, "selector": args.user_selector, "value": args.username }),
        )
        .await?;
        self.call(
            "fill",
            json!({ "pageId": page_id, "selector": args.password_selector, "value": args.password }),
        )
        .await?;
        if let Some(sel) = args.submit_selector {
            self.call("click", json!({ "pageId": page_id, "selector": sel }))
                .await?;
        } else {
            self.call("press", json!({ "pageId": page_id, "selector": args.password_selector, "key": "Enter" }))
                .await?;
        }
        if let Some(success) = args.success_url {
            self.call(
                "wait_for_url",
                json!({ "pageId": page_id, "url": success }),
            )
            .await?;
        }
        Self::ok_json(json!({ "ok": true, "url": self.call("title", json!({ "pageId": page_id })).await? }))
    }

    #[tool(name = "context_set_offline", description = "Toggle offline mode for a context.")]
    async fn context_set_offline(
        &self,
        Parameters(args): Parameters<OfflineArgs>,
    ) -> Result<CallToolResult, McpError> {
        Self::ok_json(
            self.call(
                "context_set_offline",
                json!({ "contextId": args.context_id, "offline": args.offline }),
            )
            .await?,
        )
    }

    #[tool(name = "context_grant_permissions", description = "Grant browser permissions (geolocation, clipboard-read, notifications, ...).")]
    async fn context_grant_permissions(
        &self,
        Parameters(args): Parameters<PermissionsArgs>,
    ) -> Result<CallToolResult, McpError> {
        Self::ok_json(
            self.call(
                "context_grant_permissions",
                json!({
                    "contextId": args.context_id,
                    "permissions": args.permissions,
                    "origin": args.origin,
                }),
            )
            .await?,
        )
    }

    #[tool(name = "cookies_get", description = "List cookies for a context.")]
    async fn cookies_get(
        &self,
        Parameters(args): Parameters<ContextIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        Self::ok_json(
            self.call("cookies", json!({ "contextId": args.context_id }))
                .await?,
        )
    }

    #[tool(name = "cookies_set", description = "Add cookies to a context.")]
    async fn cookies_set(
        &self,
        Parameters(args): Parameters<CookiesArgs>,
    ) -> Result<CallToolResult, McpError> {
        Self::ok_json(
            self.call(
                "context_add_cookies",
                json!({ "contextId": args.context_id, "cookies": args.cookies }),
            )
            .await?,
        )
    }

    #[tool(name = "cookie_security_report", description = "Report Secure/HttpOnly/SameSite flags on cookies.")]
    async fn cookie_security_report(
        &self,
        Parameters(args): Parameters<ContextIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        Self::ok_json(
            self.call(
                "cookie_security_report",
                json!({ "contextId": args.context_id }),
            )
            .await?,
        )
    }

    #[tool(name = "csp_report", description = "Report Content-Security-Policy headers/meta for current page.")]
    async fn csp_report(
        &self,
        Parameters(args): Parameters<PageIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(self.call("csp_report", json!({ "pageId": page_id })).await?)
    }

    #[tool(name = "a11y_scan", description = "Run axe-core accessibility scan; returns violations.")]
    async fn a11y_scan(
        &self,
        Parameters(args): Parameters<A11yArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        let r = self
            .call(
                "a11y_scan",
                json!({ "pageId": page_id, "impact": args.impact }),
            )
            .await?;
        let run_id = ArtifactStore::new_run_id();
        let _ = self
            .artifacts
            .write_json(&format!("a11y/{run_id}.json"), &r)
            .await;
        Self::ok_json(r)
    }

    #[tool(name = "visual_diff", description = "Pixel-diff current screenshot vs baseline image. Optionally update baseline.")]
    async fn visual_diff(
        &self,
        Parameters(args): Parameters<VisualDiffArgs>,
    ) -> Result<CallToolResult, McpError> {
        let baseline_path = resolve_workspace_path(&self.cfg, &args.baseline_path)?;
        let current_path = if let Some(p) = args.current_path {
            let wp = resolve_workspace_path(&self.cfg, &p)?;
            if wp.exists() {
                wp
            } else {
                let art = self.artifacts.root().join(&p);
                if art.exists() { art } else { wp }
            }
        } else {
            let page_id = self.page_id(args.page_id).await?;
            let path = self.artifacts.root().join("screenshots/visual_current.png");
            if let Some(parent) = path.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            self.call(
                "screenshot",
                json!({ "pageId": page_id, "path": path, "includeBase64": false }),
            )
            .await?;
            path
        };
        if args.update_baseline.unwrap_or(false) {
            if let Some(parent) = baseline_path.parent() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    McpError::internal_error(format!("mkdir baseline: {e}"), None)
                })?;
            }
            if !current_path.exists() {
                return Err(McpError::internal_error(
                    format!("current image missing: {}", current_path.display()),
                    None,
                ));
            }
            tokio::fs::copy(&current_path, &baseline_path)
                .await
                .map_err(|e| McpError::internal_error(format!("copy baseline: {e}"), None))?;
            return Self::ok_json(json!({ "updated_baseline": baseline_path }));
        }
        let baseline = tokio::fs::read(&baseline_path)
            .await
            .map_err(|e| McpError::internal_error(format!("read baseline: {e}"), None))?;
        let current = tokio::fs::read(&current_path)
            .await
            .map_err(|e| McpError::internal_error(format!("read current: {e}"), None))?;
        let threshold = args.threshold.unwrap_or(0.01);
        let diff = pixel_diff(&baseline, &current, threshold)
            .map_err(|e| McpError::internal_error(e, None))?;
        Self::ok_json(json!({
            "diff": diff,
            "baseline": baseline_path,
            "current": current_path,
        }))
    }

    #[tool(name = "performance_metrics", description = "Navigation timing / paint metrics from the page.")]
    async fn performance_metrics(
        &self,
        Parameters(args): Parameters<PageIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(
            self.call("performance_metrics", json!({ "pageId": page_id }))
                .await?,
        )
    }

    #[tool(name = "web_vitals", description = "Approximate LCP/CLS web vitals.")]
    async fn web_vitals(
        &self,
        Parameters(args): Parameters<PageIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(self.call("web_vitals", json!({ "pageId": page_id })).await?)
    }

    #[tool(name = "list_devices", description = "List Playwright device descriptors for emulation.")]
    async fn list_devices(&self) -> Result<CallToolResult, McpError> {
        Self::ok_json(self.call("list_devices", json!({})).await?)
    }

    #[tool(name = "list_suites", description = "Discover declarative YAML/JSON suites under browser-tests/, e2e/, etc.")]
    async fn list_suites(&self) -> Result<CallToolResult, McpError> {
        let suites = discover_suites(&self.cfg.workspace);
        let items: Vec<Value> = suites
            .into_iter()
            .map(|p| {
                let rel = p
                    .strip_prefix(&self.cfg.workspace)
                    .map(|x| x.to_string_lossy().to_string())
                    .unwrap_or_else(|_| p.display().to_string());
                let meta = load_suite_file(&p).ok();
                json!({
                    "path": rel,
                    "name": meta.as_ref().and_then(|m| m.name.clone()),
                    "tags": meta.as_ref().and_then(|m| m.tags.clone()),
                    "steps": meta.as_ref().map(|m| m.steps.len()),
                })
            })
            .collect();
        Self::ok_json(json!({ "suites": items }))
    }

    #[tool(
        name = "run_suite",
        description = "Run a declarative YAML/JSON suite (launches browser if needed). Long-running — prefer as task.",
        execution(task_support = "optional")
    )]
    async fn run_suite(
        &self,
        Parameters(args): Parameters<RunSuiteArgs>,
    ) -> Result<CallToolResult, McpError> {
        let path = resolve_workspace_path(&self.cfg, &args.path)?;
        let suite = load_suite_file(&path).map_err(|e| McpError::invalid_params(e, None))?;
        let run_id = ArtifactStore::new_run_id();
        let _ = self.artifacts.ensure().await;

        // ensure session
        {
            let s = self.session.lock().await;
            if s.page_id.is_none() {
                drop(s);
                let _ = self
                    .browser_launch(Parameters(LaunchArgs {
                        browser: args.browser.or(suite.browser.clone()),
                        headless: Some(args.headless.or(suite.headless).unwrap_or(true)),
                        create_page: Some(true),
                        base_url: args
                            .base_url
                            .clone()
                            .or(suite.base_url.clone())
                            .or(self.cfg.base_url.clone()),
                        slow_mo: None,
                        devtools: None,
                        channel: None,
                        device: None,
                        viewport_width: None,
                        viewport_height: None,
                        locale: None,
                        timezone: None,
                        color_scheme: None,
                        record_trace: Some(true),
                        record_video: None,
                        storage_state_path: None,
                    }))
                    .await?;
            }
        }
        if let Some(base) = args.base_url.or(suite.base_url) {
            self.session.lock().await.base_url = Some(base);
        }

        let page_id = self.page_id(None).await?;
        let mut steps = suite.steps;
        // rewrite select->select_option etc
        for step in &mut steps {
            if let Some(a) = step.get("action").and_then(|v| v.as_str()) {
                if a == "select" {
                    step.as_object_mut()
                        .unwrap()
                        .insert("action".into(), json!("select_option"));
                }
            }
        }
        let r = self
            .call(
                "run_steps",
                json!({
                    "pageId": page_id,
                    "steps": steps,
                    "stopOnFailure": true,
                }),
            )
            .await?;
        let passed = r["passed"].as_bool().unwrap_or(false);
        let report = json!({
            "run_id": run_id,
            "suite": suite.name,
            "path": path,
            "passed": passed,
            "result": r,
        });
        let report_path = self
            .artifacts
            .write_json(&format!("runs/{run_id}/report.json"), &report)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let tests = vec![(
            suite.name.unwrap_or_else(|| args.path.clone()),
            passed,
            if passed {
                "ok".into()
            } else {
                "failed".into()
            },
        )];
        let junit = ArtifactStore::junit_xml("browser-suite", &tests);
        let _ = self
            .artifacts
            .write_text(&format!("runs/{run_id}/junit.xml"), &junit)
            .await;
        self.session.lock().await.last_run_id = Some(run_id.clone());

        // screenshot on failure
        if !passed {
            let shot = self.artifacts.root().join(format!("runs/{run_id}/failure.png"));
            let _ = self
                .call(
                    "screenshot",
                    json!({ "pageId": page_id, "path": shot, "fullPage": true, "includeBase64": false }),
                )
                .await;
        }

        Self::ok_json(json!({ "report": report, "report_path": report_path, "passed": passed }))
    }

    #[tool(
        name = "run_playwright_tests",
        description = "Run checked-in Playwright tests via `npx playwright test` in the workspace. Long-running.",
        execution(task_support = "optional")
    )]
    async fn run_playwright_tests(
        &self,
        Parameters(args): Parameters<RunPlaywrightArgs>,
    ) -> Result<CallToolResult, McpError> {
        let mut cmd_args = vec!["playwright".into(), "test".into()];
        if let Some(p) = args.project {
            cmd_args.push(format!("--project={p}"));
        }
        if let Some(g) = args.grep {
            cmd_args.push(format!("--grep={g}"));
        }
        if args.headed.unwrap_or(false) && !self.cfg.headless_only {
            cmd_args.push("--headed".into());
        }
        if let Some(w) = args.workers {
            cmd_args.push(format!("--workers={w}"));
        }
        if let Some(extra) = args.args {
            cmd_args.extend(extra);
        }
        let timeout = args.timeout_secs.unwrap_or(self.cfg.max_suite_timeout_secs);
        let child = Command::new("npx")
            .args(&cmd_args)
            .current_dir(&self.cfg.workspace)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| McpError::internal_error(format!("spawn playwright: {e}"), None))?;
        let fut = child.wait_with_output();
        let output = tokio::time::timeout(std::time::Duration::from_secs(timeout), fut)
            .await
            .map_err(|_| McpError::internal_error("playwright timed out", None))?
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let run_id = ArtifactStore::new_run_id();
        let report = json!({
            "run_id": run_id,
            "status": output.status.code(),
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "args": cmd_args,
        });
        let _ = self
            .artifacts
            .write_json(&format!("runs/{run_id}/playwright.json"), &report)
            .await;
        Self::ok_json(report)
    }

    #[tool(name = "run_steps", description = "Execute a batch of step objects against the active page.")]
    async fn run_steps(
        &self,
        Parameters(args): Parameters<RunStepsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id).await?;
        Self::ok_json(
            self.call(
                "run_steps",
                json!({
                    "pageId": page_id,
                    "steps": args.steps,
                    "stopOnFailure": args.stop_on_failure.unwrap_or(true),
                }),
            )
            .await?,
        )
    }

    #[tool(name = "recording_start", description = "Start recording tool steps for later codegen export.")]
    async fn recording_start(&self) -> Result<CallToolResult, McpError> {
        let mut s = self.session.lock().await;
        s.recording = true;
        s.recording_steps.clear();
        Self::ok_json(json!({ "recording": true }))
    }

    #[tool(name = "recording_stop", description = "Stop recording and return captured steps.")]
    async fn recording_stop(&self) -> Result<CallToolResult, McpError> {
        let mut s = self.session.lock().await;
        s.recording = false;
        Self::ok_json(json!({ "steps": s.recording_steps }))
    }

    #[tool(name = "export_playwright_code", description = "Export steps to a Playwright test file (workspace-relative path optional).")]
    async fn export_playwright_code(
        &self,
        Parameters(args): Parameters<ExportCodeArgs>,
    ) -> Result<CallToolResult, McpError> {
        let path = if let Some(p) = args.path {
            Some(resolve_workspace_path(&self.cfg, &p)?)
        } else {
            None
        };
        Self::ok_json(
            self.call(
                "export_playwright_code",
                json!({ "steps": args.steps, "name": args.name, "path": path }),
            )
            .await?,
        )
    }

    #[tool(name = "list_artifacts", description = "List files under the artifacts directory with artifact:// URIs.")]
    async fn list_artifacts(&self) -> Result<CallToolResult, McpError> {
        let items = self
            .artifacts
            .list()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Self::ok_json(json!({ "artifacts": items }))
    }

    #[tool(name = "read_artifact", description = "Read a text artifact by relative path under artifacts/.")]
    async fn read_artifact(
        &self,
        Parameters(args): Parameters<PathArgs>,
    ) -> Result<CallToolResult, McpError> {
        let bytes = self
            .artifacts
            .read_rel(&args.path)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        // text-ish
        match String::from_utf8(bytes.clone()) {
            Ok(s) => Self::ok_text(s),
            Err(_) => Self::ok_json(json!({
                "path": args.path,
                "bytes": bytes.len(),
                "base64": base64_encode(&bytes),
            })),
        }
    }

    #[tool(name = "app_start", description = "Start an app process (allowlisted programs) and optional health wait.")]
    async fn app_start(
        &self,
        Parameters(args): Parameters<AppStartArgs>,
    ) -> Result<CallToolResult, McpError> {
        let allowed = [
            "npm", "npx", "node", "pnpm", "yarn", "bun", "cargo", "python", "python3", "uv",
            "docker", "docker-compose", "make", "just",
        ];
        if !allowed.contains(&args.program.as_str()) {
            return Err(McpError::invalid_params(
                format!("program '{}' not allowlisted for app_start", args.program),
                None,
            ));
        }
        let cwd = if let Some(c) = args.cwd {
            resolve_workspace_path(&self.cfg, &c)?
        } else {
            self.cfg.workspace.clone()
        };
        let child = Command::new(&args.program)
            .args(args.args.clone().unwrap_or_default())
            .current_dir(cwd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let pid = child.id().unwrap_or(0);
        // leak child into background by detaching — store pid only; use fixture_child style
        // For simplicity keep child alive via std::mem::forget is bad; use a task
        let child = Arc::new(Mutex::new(Some(child)));
        let child_bg = child.clone();
        tokio::spawn(async move {
            // keep until process exits
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                let mut g = child_bg.lock().await;
                if let Some(c) = g.as_mut() {
                    match c.try_wait() {
                        Ok(Some(_)) => {
                            *g = None;
                            break;
                        }
                        Ok(None) => {}
                        Err(_) => break,
                    }
                } else {
                    break;
                }
            }
        });
        *self.app_child.lock().await = Some(pid);
        if let Some(health) = args.health_url {
            let url = self.resolve_url(&health).await?;
            let timeout = args.timeout_secs.unwrap_or(60);
            let start = std::time::Instant::now();
            loop {
                if start.elapsed().as_secs() > timeout {
                    return Err(McpError::internal_error(
                        format!("health check timed out for {url}"),
                        None,
                    ));
                }
                if reqwest::get(&url).await.map(|r| r.status().is_success()).unwrap_or(false) {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            }
            return Self::ok_json(json!({ "pid": pid, "health": url, "ready": true }));
        }
        Self::ok_json(json!({ "pid": pid, "started": true }))
    }

    #[tool(name = "app_stop", description = "Stop the last app started by app_start (best-effort by pid).")]
    async fn app_stop(&self) -> Result<CallToolResult, McpError> {
        let pid = self.app_child.lock().await.take();
        if let Some(pid) = pid {
            let _ = Command::new("kill").args(["-TERM", &pid.to_string()]).status().await;
            Self::ok_json(json!({ "killed": pid }))
        } else {
            Self::ok_json(json!({ "killed": null }))
        }
    }

    #[tool(name = "fixture_server_start", description = "Serve browser-fixtures/ over HTTP for local smoke tests.")]
    async fn fixture_server_start(
        &self,
        Parameters(args): Parameters<FixtureServerArgs>,
    ) -> Result<CallToolResult, McpError> {
        let port = args.port.unwrap_or(8765);
        let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("browser-fixtures");
        // Prefer python http.server
        let child = Command::new("python3")
            .args(["-m", "http.server", &port.to_string(), "--bind", "127.0.0.1"])
            .current_dir(&fixtures)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        // wait briefly
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let url = format!("http://127.0.0.1:{port}/");
        self.session.lock().await.base_url = Some(url.trim_end_matches('/').to_string());
        *self.fixture_child.lock().await = Some(child);
        Self::ok_json(json!({ "url": url, "root": fixtures, "port": port }))
    }

    #[tool(name = "fixture_server_stop", description = "Stop the fixture static server.")]
    async fn fixture_server_stop(&self) -> Result<CallToolResult, McpError> {
        if let Some(mut c) = self.fixture_child.lock().await.take() {
            let _ = c.kill().await;
            Self::ok_json(json!({ "stopped": true }))
        } else {
            Self::ok_json(json!({ "stopped": false }))
        }
    }

    #[tool(name = "pdf_save", description = "Save page as PDF (Chromium).")]
    async fn pdf_save(
        &self,
        Parameters(args): Parameters<PdfSaveArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id.clone()).await?;
        let rel = args.path.unwrap_or_else(|| {
            format!("pdf/page_{}.pdf", chrono::Utc::now().format("%Y%m%d_%H%M%S"))
        });
        let path = self.artifacts.root().join(&rel);
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        Self::ok_json(
            self.call("pdf", json!({ "pageId": page_id, "path": path }))
                .await?,
        )
    }

    #[tool(name = "clock_install", description = "Install fixed clock / freeze time for determinism.")]
    async fn clock_install(
        &self,
        Parameters(args): Parameters<ClockInstallArgs>,
    ) -> Result<CallToolResult, McpError> {
        let page_id = self.page_id(args.page_id.clone()).await?;
        Self::ok_json(
            self.call(
                "clock_install",
                json!({ "pageId": page_id, "time": args.time }),
            )
            .await?,
        )
    }

    #[tool(name = "agent_note", description = "Append a note to the session scratchpad.")]
    async fn agent_note(
        &self,
        Parameters(args): Parameters<NoteArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.notes.lock().await.push(args.note);
        Self::ok_json(json!({ "notes": self.notes.lock().await.len() }))
    }

    #[tool(name = "list_agent_notes", description = "List session scratchpad notes.")]
    async fn list_agent_notes(&self) -> Result<CallToolResult, McpError> {
        Self::ok_json(json!({ "notes": *self.notes.lock().await }))
    }

    #[tool(name = "emergency_nuke", description = "Kill bridge browsers and reset session (kill switch).")]
    async fn emergency_nuke(&self) -> Result<CallToolResult, McpError> {
        let _ = self.bridge.call("shutdown", json!({})).await;
        let _ = self.bridge.shutdown().await;
        *self.session.lock().await = SessionState::default();
        if let Some(mut c) = self.fixture_child.lock().await.take() {
            let _ = c.kill().await;
        }
        Self::ok_json(json!({ "nuked": true }))
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.encode(bytes)
}

// ── prompts ────────────────────────────────────────────────────────────────

#[prompt_router]
impl BrowserTesting {
    #[prompt(
        name = "explore_and_map_app",
        description = "Crawl primary navigation and summarize app structure using browser tools"
    )]
    async fn explore_and_map_app(
        &self,
        Parameters(args): Parameters<FlowPromptArgs>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        let base = args
            .base_url
            .or_else(|| self.cfg.base_url.clone())
            .unwrap_or_else(|| "http://127.0.0.1:8765".into());
        Ok(GetPromptResult::new(vec![PromptMessage::new_text(
            Role::User,
            format!(
                "Explore the web app for feature/context: {}.\n\
                 Base URL: {base}\n\
                 Use browser_info → browser_launch → goto → snapshot repeatedly.\n\
                 Map: main nav, primary user journeys, forms, auth gates, testids.\n\
                 Prefer role/name and data-testid locators. Do not leave allowlisted hosts.\n\
                 End with a structured map and suggested smoke tests.",
                args.feature
            ),
        )])
        .with_description("Explore app and produce a structure map"))
    }

    #[prompt(
        name = "write_e2e_for_flow",
        description = "Turn a user story into suite steps + Playwright codegen export"
    )]
    async fn write_e2e_for_flow(
        &self,
        Parameters(args): Parameters<FlowPromptArgs>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        Ok(GetPromptResult::new(vec![PromptMessage::new_text(
            Role::User,
            format!(
                "Write an E2E test for: {}.\n\
                 1. browser_launch + recording_start\n\
                 2. Drive the flow with click/fill/expect_* using resilient locators\n\
                 3. recording_stop + export_playwright_code\n\
                 4. Optionally save YAML under browser-tests/\n\
                 5. run_suite or run_playwright_tests to verify\n\
                 Prefer expect_text/expect_url over hard sleeps.",
                args.feature
            ),
        )])
        .with_description("Generate E2E coverage for a flow"))
    }

    #[prompt(
        name = "debug_failing_test",
        description = "Debug a failing browser test using artifacts, snapshots, and diagnostics"
    )]
    async fn debug_failing_test(
        &self,
        Parameters(args): Parameters<DebugFailPromptArgs>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        Ok(GetPromptResult::new(vec![PromptMessage::new_text(
            Role::User,
            format!(
                "Debug this browser test failure:\n```\n{}\n```\n{}\n\
                 Steps: list_artifacts, read failure screenshot, re-run with snapshot + why_not_actionable,\n\
                 check console_messages/network_log, consider flake (retry) vs real bug.\n\
                 Propose a minimal fix to the test or app.",
                args.error,
                args.context.unwrap_or_default()
            ),
        )])
        .with_description("Debug failing browser test"))
    }

    #[prompt(name = "a11y_review", description = "Run accessibility review workflow on the current page")]
    async fn a11y_review(
        &self,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        Ok(GetPromptResult::new(vec![PromptMessage::new_text(
            Role::User,
            "Run a11y_scan with impact=serious, review violations, snapshot the page, \
             and propose concrete fixes ordered by impact. Re-scan after fixes if possible.",
        )])
        .with_description("Accessibility review"))
    }

    #[prompt(name = "smoke_critical_paths", description = "Run smoke coverage of critical paths")]
    async fn smoke_critical_paths(
        &self,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        Ok(GetPromptResult::new(vec![PromptMessage::new_text(
            Role::User,
            "list_suites, run all suites tagged smoke (or run_suite on browser-tests/smoke.yaml).\n\
             Start fixture_server if testing the bundled fixture. Report pass/fail with artifact URIs.",
        )])
        .with_description("Smoke critical paths"))
    }

    #[prompt(name = "flaky_triage", description = "Triage a flaky browser test")]
    async fn flaky_triage(
        &self,
        Parameters(args): Parameters<DebugFailPromptArgs>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        Ok(GetPromptResult::new(vec![PromptMessage::new_text(
            Role::User,
            format!(
                "Flaky triage for:\n{}\n\
                 Re-run 3x with tracing_start/stop, compare snapshots, check network races,\n\
                 replace timing assumptions with expect_*/wait_for_response, consider quarantine tag.",
                args.error
            ),
        )])
        .with_description("Flaky test triage"))
    }
}

// ── handler ────────────────────────────────────────────────────────────────

#[tool_handler]
#[prompt_handler]
#[task_handler]
impl ServerHandler for BrowserTesting {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_prompts()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::new(
            "rmcp-browser-testing",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_instructions(
            "Full browser automated testing MCP (Rust + Playwright bridge).\n\n\
             Quick start:\n\
             1. browser_install_deps (once)\n\
             2. browser_info\n\
             3. fixture_server_start OR set MCP_BROWSER_BASE_URL\n\
             4. browser_launch → goto → snapshot → click/fill → expect_*\n\
             5. run_suite / run_playwright_tests for suites\n\
             6. a11y_scan / visual_diff / tracing_* / list_artifacts\n\n\
             Safety: host allowlist (default localhost only), workspace-scoped paths, secret redaction.\n\
             Set MCP_BROWSER_ALLOW_ALL_HOSTS=1 for broader browsing.\n\
             Resources: browser://session, browser://guide, artifact://...\n\
             Prompts: explore_and_map_app, write_e2e_for_flow, debug_failing_test, a11y_review, smoke_critical_paths, flaky_triage."
                .to_string(),
        )
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let mut resources = vec![
            Resource::new("browser://session", "session")
                .with_description("Active browser session status")
                .with_mime_type("application/json"),
            Resource::new("browser://guide", "guide")
                .with_description("How to use the browser testing MCP")
                .with_mime_type("text/markdown"),
            Resource::new("browser://config", "config")
                .with_description("Server configuration")
                .with_mime_type("application/json"),
        ];
        if let Ok(items) = self.artifacts.list().await {
            for it in items.into_iter().take(50) {
                if let Some(uri) = it.get("uri").and_then(|u| u.as_str()) {
                    resources.push(
                        Resource::new(uri, it.get("path").and_then(|p| p.as_str()).unwrap_or("artifact"))
                            .with_description("Test artifact")
                            .with_mime_type("application/octet-stream"),
                    );
                }
            }
        }
        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let uri = request.uri.as_str();
        if uri == "browser://session" {
            let s = self.session.lock().await;
            let text = serde_json::to_string_pretty(&json!({
                "browser_id": s.browser_id,
                "context_id": s.context_id,
                "page_id": s.page_id,
                "base_url": s.base_url,
                "recording": s.recording,
                "last_run_id": s.last_run_id,
                "last_screenshot": s.last_screenshot,
            }))
            .unwrap();
            return Ok(ReadResourceResult::new(vec![ResourceContents::text(
                text, request.uri,
            )]));
        }
        if uri == "browser://config" {
            let text = serde_json::to_string_pretty(&json!({
                "workspace": self.cfg.workspace,
                "artifacts": self.artifacts.root(),
                "allowed_hosts": self.cfg.allowed_hosts,
                "allow_all_hosts": self.cfg.allow_all_hosts,
                "headless_only": self.cfg.headless_only,
            }))
            .unwrap();
            return Ok(ReadResourceResult::new(vec![ResourceContents::text(
                text, request.uri,
            )]));
        }
        if uri == "browser://guide" {
            let text = r#"# Browser Testing MCP

1. browser_install_deps (once)
2. browser_info
3. fixture_server_start or set base URL
4. browser_launch → goto → snapshot → interact → expect_*
5. run_suite / run_playwright_tests
6. a11y_scan, visual_diff, list_artifacts

Safety: host allowlist, workspace paths, no file://.
Env: MCP_BROWSER_ALLOWED_HOSTS, MCP_BROWSER_ALLOW_ALL_HOSTS, MCP_BROWSER_BASE_URL, MCP_BROWSER_ARTIFACTS.
"#;
            return Ok(ReadResourceResult::new(vec![ResourceContents::text(
                text, request.uri,
            )]));
        }
        if let Some(rel) = uri.strip_prefix("artifact://") {
            let bytes = self
                .artifacts
                .read_rel(rel)
                .await
                .map_err(|_| McpError::resource_not_found("not found", Some(json!({ "uri": uri }))))?;
            if let Ok(s) = String::from_utf8(bytes.clone()) {
                return Ok(ReadResourceResult::new(vec![ResourceContents::text(
                    s, request.uri,
                )]));
            }
            return Ok(ReadResourceResult::new(vec![ResourceContents::text(
                format!("base64:{}", base64_encode(&bytes)),
                request.uri,
            )]));
        }
        Err(McpError::resource_not_found(
            "resource_not_found",
            Some(json!({ "uri": uri })),
        ))
    }
}
