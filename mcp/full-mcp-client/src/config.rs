//! Optional TOML configuration for presets and defaults.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub default_preset: Option<String>,
    #[serde(default)]
    pub json: bool,
    #[serde(default)]
    pub presets: HashMap<String, PresetConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PresetConfig {
    /// e.g. ["node", "server.js"]
    pub stdio: Option<Vec<String>>,
    pub http: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub cwd: Option<String>,
}

impl Config {
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let path = match path {
            Some(p) => p.to_path_buf(),
            None => default_config_path().ok_or_else(|| anyhow::anyhow!("no config path"))?,
        };
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("read config {}", path.display()))?;
        let cfg: Config = toml::from_str(&text).context("parse config TOML")?;
        Ok(cfg)
    }

    pub fn try_load_default() -> Self {
        Self::load(None).unwrap_or_default()
    }
}

pub fn default_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("mcp-client").join("config.toml"))
}

/// Built-in presets for servers in this monorepo.
pub fn builtin_preset(name: &str) -> Option<PresetConfig> {
    match name {
        "browser" | "browser-testing" => Some(PresetConfig {
            stdio: Some(vec![
                "cargo".into(),
                "run".into(),
                "-q".into(),
                "-p".into(),
                "mcp-server-examples".into(),
                "--example".into(),
                "servers_browser_testing_stdio".into(),
            ]),
            http: None,
            env: HashMap::from([
                (
                    "MCP_BROWSER_WORKSPACE".into(),
                    std::env::var("MCP_BROWSER_WORKSPACE").unwrap_or_else(|_| {
                        // default to servers example dir if present
                        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                            .join("../servers")
                            .canonicalize()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|_| ".".into());
                        p
                    }),
                ),
                (
                    "MCP_BROWSER_ALLOWED_HOSTS".into(),
                    std::env::var("MCP_BROWSER_ALLOWED_HOSTS")
                        .unwrap_or_else(|_| "127.0.0.1,localhost".into()),
                ),
            ]),
            headers: HashMap::new(),
            cwd: Some(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .canonicalize()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| ".".into()),
            ),
        }),
        "counter" => Some(PresetConfig {
            stdio: Some(vec![
                "cargo".into(),
                "run".into(),
                "-q".into(),
                "-p".into(),
                "mcp-server-examples".into(),
                "--example".into(),
                "servers_counter_stdio".into(),
            ]),
            http: None,
            env: HashMap::new(),
            headers: HashMap::new(),
            cwd: Some(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .canonicalize()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| ".".into()),
            ),
        }),
        "dev-tools" | "devtools" => Some(PresetConfig {
            stdio: Some(vec![
                "cargo".into(),
                "run".into(),
                "-q".into(),
                "-p".into(),
                "mcp-server-examples".into(),
                "--example".into(),
                "servers_dev_tools_stdio".into(),
            ]),
            http: None,
            env: HashMap::new(),
            headers: HashMap::new(),
            cwd: Some(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .canonicalize()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| ".".into()),
            ),
        }),
        "task" | "task-demo" => Some(PresetConfig {
            stdio: Some(vec![
                "cargo".into(),
                "run".into(),
                "-q".into(),
                "-p".into(),
                "mcp-server-examples".into(),
                "--example".into(),
                "servers_task_stdio".into(),
            ]),
            http: None,
            env: HashMap::new(),
            headers: HashMap::new(),
            cwd: Some(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .canonicalize()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| ".".into()),
            ),
        }),
        _ => None,
    }
}
