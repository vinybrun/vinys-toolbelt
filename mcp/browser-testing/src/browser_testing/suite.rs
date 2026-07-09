use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct SuiteFile {
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub tags: Option<Vec<String>>,
    pub steps: Vec<Value>,
    #[serde(default)]
    pub browser: Option<String>,
    #[serde(default)]
    pub headless: Option<bool>,
}

pub fn load_suite_file(path: &Path) -> Result<SuiteFile, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e == "yaml" || e == "yml")
    {
        serde_yml::from_str(&text).map_err(|e| e.to_string())
    } else {
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }
}

pub fn discover_suites(root: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let candidates = [
        root.join("browser-tests"),
        root.join("e2e"),
        root.join("tests/e2e"),
        root.join("playwright"),
    ];
    for dir in candidates {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for ent in rd.flatten() {
                let p = ent.path();
                if p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| matches!(e, "yaml" | "yml" | "json"))
                {
                    out.push(p);
                }
            }
        }
    }
    out.sort();
    out
}
