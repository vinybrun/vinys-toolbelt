use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use rmcp::ErrorData as McpError;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{Mutex, oneshot};
use tokio::time::{Duration, timeout};

use super::config::BrowserConfig;

pub struct Bridge {
    cfg: BrowserConfig,
    inner: Mutex<Option<BridgeInner>>,
    next_id: AtomicU64,
}

struct BridgeInner {
    child: Child,
    stdin: ChildStdin,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>,
    _reader_task: tokio::task::JoinHandle<()>,
}

impl Bridge {
    pub fn new(cfg: BrowserConfig) -> Self {
        Self {
            cfg,
            inner: Mutex::new(None),
            next_id: AtomicU64::new(1),
        }
    }

    pub async fn ensure_started(&self) -> Result<(), McpError> {
        let mut guard = self.inner.lock().await;
        if guard.is_some() {
            return Ok(());
        }
        if !self.cfg.bridge_script.exists() {
            return Err(McpError::internal_error(
                format!(
                    "bridge script not found at {}. Set MCP_BROWSER_BRIDGE.",
                    self.cfg.bridge_script.display()
                ),
                None,
            ));
        }
        let mut child = Command::new(&self.cfg.node_bin)
            .arg(&self.cfg.bridge_script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .current_dir(self.cfg.bridge_script.parent().unwrap_or(self.cfg.workspace.as_path()))
            .spawn()
            .map_err(|e| {
                McpError::internal_error(format!("failed to spawn node bridge: {e}"), None)
            })?;

        let stdin = child.stdin.take().expect("stdin");
        let stdout = child.stdout.take().expect("stdout");
        let stderr = child.stderr.take().expect("stderr");

        // drain stderr
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::info!(target: "browser_bridge", "{line}");
            }
        });

        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_r = pending.clone();

        let reader_task = tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let Ok(v) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                // ready signal has id null
                if v.get("id").map(|i| i.is_null()).unwrap_or(false) {
                    continue;
                }
                let id = match v.get("id").and_then(|i| i.as_u64()) {
                    Some(id) => id,
                    None => continue,
                };
                let result = if let Some(err) = v.get("error") {
                    Err(err
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("bridge error")
                        .to_string())
                } else {
                    Ok(v.get("result").cloned().unwrap_or(Value::Null))
                };
                if let Some(tx) = pending_r.lock().await.remove(&id) {
                    let _ = tx.send(result);
                }
            }
        });

        // Wait for ready line is handled by reader skipping null ids; small delay
        tokio::time::sleep(Duration::from_millis(100)).await;

        *guard = Some(BridgeInner {
            child,
            stdin,
            pending,
            _reader_task: reader_task,
        });
        Ok(())
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value, McpError> {
        self.ensure_started().await?;
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        {
            let mut guard = self.inner.lock().await;
            let inner = guard.as_mut().ok_or_else(|| {
                McpError::internal_error("bridge not running", None)
            })?;
            inner.pending.lock().await.insert(id, tx);
            let msg = json!({ "id": id, "method": method, "params": params });
            let line = serde_json::to_string(&msg).unwrap() + "\n";
            inner.stdin.write_all(line.as_bytes()).await.map_err(|e| {
                McpError::internal_error(format!("bridge write failed: {e}"), None)
            })?;
            inner.stdin.flush().await.map_err(|e| {
                McpError::internal_error(format!("bridge flush failed: {e}"), None)
            })?;
        }

        let result = timeout(Duration::from_secs(120), rx)
            .await
            .map_err(|_| McpError::internal_error("bridge call timed out", None))?
            .map_err(|_| McpError::internal_error("bridge response channel closed", None))?;

        result.map_err(|e| McpError::internal_error(e, None))
    }

    pub async fn shutdown(&self) -> Result<(), McpError> {
        let mut guard = self.inner.lock().await;
        if let Some(mut inner) = guard.take() {
            let _ = self_call_shutdown(&mut inner).await;
            let _ = inner.child.kill().await;
        }
        Ok(())
    }
}

async fn self_call_shutdown(inner: &mut BridgeInner) -> Result<(), ()> {
    let msg = json!({ "id": 0u64, "method": "shutdown", "params": {} });
    let line = serde_json::to_string(&msg).unwrap() + "\n";
    let _ = inner.stdin.write_all(line.as_bytes()).await;
    let _ = inner.stdin.flush().await;
    Ok(())
}
