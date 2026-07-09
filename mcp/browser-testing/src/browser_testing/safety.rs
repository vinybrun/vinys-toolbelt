use std::path::{Component, Path, PathBuf};

use rmcp::ErrorData as McpError;
use serde_json::json;
use url::Url;

use super::config::BrowserConfig;

pub fn resolve_workspace_path(cfg: &BrowserConfig, relative: &str) -> Result<PathBuf, McpError> {
    let relative = relative.trim();
    if relative.is_empty() || relative == "." {
        return Ok(cfg.workspace.clone());
    }
    let p = Path::new(relative);
    if p.is_absolute() {
        return Err(McpError::invalid_params(
            "path must be relative to workspace root",
            Some(json!({ "path": relative })),
        ));
    }
    for c in p.components() {
        if matches!(c, Component::ParentDir | Component::RootDir | Component::Prefix(_)) {
            return Err(McpError::invalid_params(
                "path must not contain '..' or absolute components",
                Some(json!({ "path": relative })),
            ));
        }
    }
    let full = cfg.workspace.join(relative);
    if let (Ok(ws), Ok(canon)) = (cfg.workspace.canonicalize(), full.canonicalize()) {
        if !canon.starts_with(&ws) {
            return Err(McpError::invalid_params(
                "path escapes workspace",
                Some(json!({ "path": relative })),
            ));
        }
        return Ok(canon);
    }
    Ok(full)
}

pub fn check_url_allowed(cfg: &BrowserConfig, url: &str) -> Result<(), McpError> {
    if cfg.allow_all_hosts {
        return Ok(());
    }
    // Allow relative paths when base_url is set
    if url.starts_with('/') || !url.contains("://") {
        return Ok(());
    }
    let parsed = Url::parse(url).map_err(|e| {
        McpError::invalid_params(format!("invalid url: {e}"), Some(json!({ "url": url })))
    })?;
    let scheme = parsed.scheme();
    if scheme == "file" {
        return Err(McpError::invalid_params(
            "file:// URLs are blocked; serve fixtures over http or use workspace paths with the fixture server",
            Some(json!({ "url": url })),
        ));
    }
    if scheme != "http" && scheme != "https" {
        return Err(McpError::invalid_params(
            format!("scheme not allowed: {scheme}"),
            Some(json!({ "url": url })),
        ));
    }
    let host = parsed.host_str().unwrap_or("");
    // Block obvious SSRF targets unless explicitly allowed
    let blocked_prefixes = ["169.254.", "metadata.google"];
    for b in blocked_prefixes {
        if host.contains(b) {
            return Err(McpError::invalid_params(
                "host blocked by SSRF guard",
                Some(json!({ "host": host })),
            ));
        }
    }
    let allowed = cfg.allowed_hosts.iter().any(|h| {
        host.eq_ignore_ascii_case(h)
            || host.ends_with(&format!(".{h}"))
            || h == "*"
    });
    if !allowed {
        return Err(McpError::invalid_params(
            format!(
                "host '{host}' not in allowlist {:?}. Set MCP_BROWSER_ALLOW_ALL_HOSTS=1 or MCP_BROWSER_ALLOWED_HOSTS",
                cfg.allowed_hosts
            ),
            Some(json!({ "host": host, "allowed": cfg.allowed_hosts })),
        ));
    }
    Ok(())
}

pub fn redact_secrets(text: &str) -> String {
    // simple redaction for common secret patterns in logs
    let mut out = text.to_string();
    for (pat, rep) in [
        (
            regex_lite_password(),
            "$1=***",
        ),
    ] {
        out = pat.replace_all(&out, rep).into_owned();
    }
    out
}

fn regex_lite_password() -> regex::Regex {
    regex::Regex::new(r"(?i)(password|passwd|secret|token|api[_-]?key)\s*[=:]\s*\S+").unwrap()
}
