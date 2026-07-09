use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct BrowserConfig {
    pub workspace: PathBuf,
    pub artifacts_dir: PathBuf,
    pub bridge_script: PathBuf,
    pub node_bin: String,
    pub allowed_hosts: Vec<String>,
    pub allow_all_hosts: bool,
    pub headless_default: bool,
    pub max_action_timeout_ms: u64,
    pub max_suite_timeout_secs: u64,
    pub base_url: Option<String>,
    pub headless_only: bool,
}

impl BrowserConfig {
    pub fn from_env(workspace: PathBuf) -> Self {
        let artifacts_dir = std::env::var("MCP_BROWSER_ARTIFACTS")
            .map(PathBuf::from)
            .unwrap_or_else(|_| workspace.join("browser-artifacts"));
        let bridge_script = std::env::var("MCP_BROWSER_BRIDGE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                // examples/servers/browser-bridge/bridge.mjs relative to CARGO_MANIFEST_DIR
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("browser-bridge/bridge.mjs")
            });
        let allowed_hosts = std::env::var("MCP_BROWSER_ALLOWED_HOSTS")
            .map(|s| {
                s.split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect()
            })
            .unwrap_or_else(|_| {
                vec![
                    "127.0.0.1".into(),
                    "localhost".into(),
                    "[::1]".into(),
                ]
            });
        let allow_all_hosts = std::env::var("MCP_BROWSER_ALLOW_ALL_HOSTS")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let headless_only = std::env::var("MCP_BROWSER_HEADLESS_ONLY")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let base_url = std::env::var("MCP_BROWSER_BASE_URL").ok();
        Self {
            workspace,
            artifacts_dir,
            bridge_script,
            node_bin: std::env::var("MCP_BROWSER_NODE").unwrap_or_else(|_| "node".into()),
            allowed_hosts,
            allow_all_hosts,
            headless_default: true,
            max_action_timeout_ms: 30_000,
            max_suite_timeout_secs: 600,
            base_url,
            headless_only,
        }
    }
}
