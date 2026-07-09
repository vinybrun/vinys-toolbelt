use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use tokio::fs;

use super::config::BrowserConfig;

pub struct ArtifactStore {
    root: PathBuf,
}

impl ArtifactStore {
    pub fn new(cfg: &BrowserConfig) -> Self {
        Self {
            root: cfg.artifacts_dir.clone(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub async fn ensure(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.root).await
    }

    pub fn run_dir(&self, run_id: &str) -> PathBuf {
        self.root.join("runs").join(run_id)
    }

    pub fn new_run_id() -> String {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format!("run_{ts}_{}", &uuid::Uuid::new_v4().to_string()[..8])
    }

    pub async fn write_json(&self, rel: &str, value: &Value) -> std::io::Result<PathBuf> {
        let path = self.root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&path, serde_json::to_vec_pretty(value)?).await?;
        Ok(path)
    }

    pub async fn write_bytes(&self, rel: &str, bytes: &[u8]) -> std::io::Result<PathBuf> {
        let path = self.root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&path, bytes).await?;
        Ok(path)
    }

    pub async fn write_text(&self, rel: &str, text: &str) -> std::io::Result<PathBuf> {
        self.write_bytes(rel, text.as_bytes()).await
    }

    pub async fn list(&self) -> std::io::Result<Vec<Value>> {
        self.ensure().await?;
        let mut out = Vec::new();
        let mut stack = vec![self.root.clone()];
        while let Some(dir) = stack.pop() {
            let mut rd = fs::read_dir(&dir).await?;
            while let Some(ent) = rd.next_entry().await? {
                let p = ent.path();
                if p.is_dir() {
                    stack.push(p);
                } else if let Ok(rel) = p.strip_prefix(&self.root) {
                    let meta = ent.metadata().await?;
                    out.push(json!({
                        "path": rel.to_string_lossy(),
                        "bytes": meta.len(),
                        "uri": format!("artifact://{}", rel.to_string_lossy()),
                    }));
                }
            }
        }
        out.sort_by(|a, b| {
            a["path"]
                .as_str()
                .unwrap_or("")
                .cmp(b["path"].as_str().unwrap_or(""))
        });
        Ok(out)
    }

    pub async fn read_rel(&self, rel: &str) -> std::io::Result<Vec<u8>> {
        let path = self.root.join(rel);
        fs::read(path).await
    }

    pub fn junit_xml(name: &str, tests: &[(String, bool, String)]) -> String {
        let total = tests.len();
        let failures = tests.iter().filter(|t| !t.1).count();
        let mut cases = String::new();
        for (n, ok, msg) in tests {
            if *ok {
                cases.push_str(&format!(
                    "  <testcase name=\"{}\" classname=\"{name}\"/>\n",
                    xml_escape(n)
                ));
            } else {
                cases.push_str(&format!(
                    "  <testcase name=\"{}\" classname=\"{name}\"><failure message=\"{}\"/></testcase>\n",
                    xml_escape(n),
                    xml_escape(msg)
                ));
            }
        }
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuite name="{name}" tests="{total}" failures="{failures}">
{cases}</testsuite>
"#
        )
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
