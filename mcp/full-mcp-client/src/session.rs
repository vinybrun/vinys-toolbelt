//! MCP session: connect over stdio or HTTP and expose high-level operations.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use rmcp::{
    ClientHandler, ServiceExt,
    model::{
        CallToolRequestParams, CallToolResult, CancelTaskParams, ClientCapabilities, ClientInfo,
        ClientRequest, GetPromptRequestParams, GetPromptResult, GetTaskParams,
        GetTaskPayloadParams, Implementation, PaginatedRequestParams, ProgressNotificationParam,
        Prompt, ReadResourceRequestParams, ReadResourceResult, Request, Resource,
        ResourceTemplate, ResourceUpdatedNotificationParam, ServerInfo, ServerResult,
        SubscribeRequestParams, TaskMetadata, TaskStatus, Tool, UnsubscribeRequestParams,
    },
    service::{NotificationContext, RoleClient, RunningService},
    transport::{ConfigureCommandExt, StreamableHttpClientTransport, TokioChildProcess},
};
use serde_json::{Map, Value};
use tokio::process::Command;

use crate::config::PresetConfig;
use crate::display::Printer;

/// Handles progress / resource notifications from the server.
#[derive(Clone, Default)]
pub struct NotifyingClient {
    pub progress_log: Arc<Mutex<Vec<String>>>,
    pub quiet: bool,
}

impl ClientHandler for NotifyingClient {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::new(
            ClientCapabilities::default(),
            Implementation::new("full-mcp-client", env!("CARGO_PKG_VERSION")),
        )
    }

    async fn on_progress(
        &self,
        params: ProgressNotificationParam,
        _ctx: NotificationContext<RoleClient>,
    ) {
        let msg = format!(
            "progress token={:?} {}/{} {}",
            params.progress_token,
            params.progress,
            params
                .total
                .map(|t| t.to_string())
                .unwrap_or_else(|| "?".into()),
            params.message.as_deref().unwrap_or("")
        );
        if let Ok(mut g) = self.progress_log.lock() {
            g.push(msg.clone());
        }
        if !self.quiet {
            eprintln!("… {msg}");
        }
    }

    async fn on_resource_updated(
        &self,
        params: ResourceUpdatedNotificationParam,
        _ctx: NotificationContext<RoleClient>,
    ) {
        if !self.quiet {
            eprintln!("… resource updated: {}", params.uri);
        }
    }

    async fn on_resource_list_changed(&self, _ctx: NotificationContext<RoleClient>) {
        if !self.quiet {
            eprintln!("… resource list changed");
        }
    }

    async fn on_tool_list_changed(&self, _ctx: NotificationContext<RoleClient>) {
        if !self.quiet {
            eprintln!("… tool list changed");
        }
    }

    async fn on_prompt_list_changed(&self, _ctx: NotificationContext<RoleClient>) {
        if !self.quiet {
            eprintln!("… prompt list changed");
        }
    }
}

pub struct Session {
    pub service: RunningService<RoleClient, NotifyingClient>,
    pub printer: Printer,
}

pub struct ConnectOptions {
    pub stdio: Option<Vec<String>>,
    pub http: Option<String>,
    pub env: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub cwd: Option<PathBuf>,
    pub json: bool,
    pub quiet: bool,
}

impl ConnectOptions {
    pub fn from_preset(p: &PresetConfig, json: bool) -> Self {
        Self {
            stdio: p.stdio.clone(),
            http: p.http.clone(),
            env: p.env.clone(),
            headers: p.headers.clone(),
            cwd: p.cwd.as_ref().map(PathBuf::from),
            json,
            quiet: json,
        }
    }
}

impl Session {
    pub async fn connect(opts: ConnectOptions) -> Result<Self> {
        let handler = NotifyingClient {
            progress_log: Arc::new(Mutex::new(Vec::new())),
            quiet: opts.quiet,
        };

        let service = if let Some(url) = opts.http {
            let _ = opts.headers; // reserved for custom reqwest client later
            let transport = StreamableHttpClientTransport::from_uri(url.as_str());
            handler
                .serve(transport)
                .await
                .context("connect streamable HTTP MCP server")?
        } else if let Some(cmd) = opts.stdio {
            if cmd.is_empty() {
                bail!("stdio command is empty");
            }
            let program = cmd[0].clone();
            let args = cmd[1..].to_vec();
            let env = opts.env.clone();
            let cwd = opts.cwd.clone();
            let command = Command::new(&program).configure(|c| {
                c.args(&args);
                for (k, v) in &env {
                    c.env(k, v);
                }
                if let Some(dir) = &cwd {
                    c.current_dir(dir);
                }
            });
            let transport =
                TokioChildProcess::new(command).context("spawn stdio MCP server")?;
            handler
                .serve(transport)
                .await
                .context("initialize MCP session over stdio")?
        } else {
            bail!("provide --stdio, --http, or --preset");
        };

        Ok(Self {
            service,
            printer: Printer::new(opts.json),
        })
    }

    pub fn server_info(&self) -> Option<std::sync::Arc<ServerInfo>> {
        self.service.peer_info()
    }

    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        Ok(self.service.list_all_tools().await?)
    }

    pub async fn find_tool(&self, name: &str) -> Result<Tool> {
        let tools = self.list_tools().await?;
        tools
            .into_iter()
            .find(|t| t.name == name)
            .with_context(|| format!("tool not found: {name}"))
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Option<Map<String, Value>>,
    ) -> Result<CallToolResult> {
        let mut params = CallToolRequestParams::new(name.to_string());
        if let Some(args) = arguments {
            params = params.with_arguments(args);
        }
        Ok(self.service.call_tool(params).await?)
    }

    /// Call a tool as a long-running task (SEP-1319), poll until terminal, return result.
    pub async fn call_tool_as_task(
        &self,
        name: &str,
        arguments: Option<Map<String, Value>>,
        poll_ms: u64,
        timeout: Duration,
    ) -> Result<CallToolResult> {
        let mut params = CallToolRequestParams::new(name.to_string()).with_task(TaskMetadata::new());
        if let Some(args) = arguments {
            params = params.with_arguments(args);
        }
        let create = self
            .service
            .send_request(ClientRequest::CallToolRequest(Request::new(params)))
            .await?;
        let ServerResult::CreateTaskResult(create) = create else {
            if let ServerResult::CallToolResult(r) = create {
                return Ok(r);
            }
            bail!("expected CreateTaskResult or CallToolResult, got {create:?}");
        };
        let task_id = create.task.task_id.clone();
        if !self.printer.json {
            eprintln!("task {task_id} status={:?}", create.task.status);
        }
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > timeout {
                bail!("task {task_id} timed out after {timeout:?}");
            }
            tokio::time::sleep(Duration::from_millis(poll_ms)).await;
            let info = self
                .service
                .send_request(ClientRequest::GetTaskRequest(Request::new(
                    GetTaskParams::new(task_id.clone()),
                )))
                .await?;
            let ServerResult::GetTaskResult(info) = info else {
                bail!("expected GetTaskResult, got {info:?}");
            };
            if !self.printer.json {
                eprintln!("task {task_id} status={:?}", info.task.status);
            }
            match info.task.status {
                TaskStatus::Completed => break,
                TaskStatus::Failed => bail!("task {task_id} failed"),
                TaskStatus::Cancelled => bail!("task {task_id} cancelled"),
                _ => {}
            }
        }
        let payload = self
            .service
            .send_request(ClientRequest::GetTaskPayloadRequest(Request::new(
                GetTaskPayloadParams::new(task_id),
            )))
            .await?;
        let call_result: CallToolResult = match payload {
            ServerResult::CallToolResult(r) => r,
            ServerResult::CustomResult(c) => serde_json::from_value(c.0)
                .context("decode task payload as CallToolResult")?,
            other => bail!("unexpected task result: {other:?}"),
        };
        Ok(call_result)
    }

    pub async fn list_resources(&self) -> Result<Vec<Resource>> {
        Ok(self.service.list_all_resources().await?)
    }

    pub async fn list_resource_templates(&self) -> Result<Vec<ResourceTemplate>> {
        Ok(self.service.list_all_resource_templates().await?)
    }

    pub async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult> {
        Ok(self
            .service
            .read_resource(ReadResourceRequestParams::new(uri))
            .await?)
    }

    pub async fn subscribe(&self, uri: &str) -> Result<()> {
        Ok(self
            .service
            .subscribe(SubscribeRequestParams::new(uri))
            .await?)
    }

    pub async fn unsubscribe(&self, uri: &str) -> Result<()> {
        Ok(self
            .service
            .unsubscribe(UnsubscribeRequestParams::new(uri))
            .await?)
    }

    pub async fn list_prompts(&self) -> Result<Vec<Prompt>> {
        Ok(self.service.list_all_prompts().await?)
    }

    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<Map<String, Value>>,
    ) -> Result<GetPromptResult> {
        let mut params = GetPromptRequestParams::new(name);
        if let Some(args) = arguments {
            params = params.with_arguments(args);
        }
        Ok(self.service.get_prompt(params).await?)
    }

    pub async fn complete_prompt(
        &self,
        prompt: &str,
        argument: &str,
        value: &str,
    ) -> Result<Vec<String>> {
        Ok(self
            .service
            .complete_prompt_simple(prompt, argument, value)
            .await?)
    }

    pub async fn complete_resource(
        &self,
        uri: &str,
        argument: &str,
        value: &str,
    ) -> Result<Vec<String>> {
        Ok(self
            .service
            .complete_resource_simple(uri, argument, value)
            .await?)
    }

    pub async fn cancel_task(&self, task_id: &str) -> Result<Value> {
        let result = self
            .service
            .send_request(ClientRequest::CancelTaskRequest(Request::new(
                CancelTaskParams::new(task_id),
            )))
            .await?;
        Ok(serde_json::to_value(result)?)
    }

    pub async fn get_task(&self, task_id: &str) -> Result<Value> {
        let result = self
            .service
            .send_request(ClientRequest::GetTaskRequest(Request::new(
                GetTaskParams::new(task_id),
            )))
            .await?;
        Ok(serde_json::to_value(result)?)
    }

    pub async fn list_tasks(&self) -> Result<Value> {
        let result = self
            .service
            .send_request(ClientRequest::ListTasksRequest(
                rmcp::model::RequestOptionalParam::with_param(PaginatedRequestParams::default()),
            ))
            .await?;
        Ok(serde_json::to_value(result)?)
    }

    pub async fn shutdown(self) -> Result<()> {
        self.service.cancel().await?;
        Ok(())
    }
}

/// Merge `--args JSON` and repeated `--arg k=v` into a map.
pub fn build_arguments(
    args_json: Option<&str>,
    kv_pairs: &[String],
) -> Result<Option<Map<String, Value>>> {
    let mut map = Map::new();
    if let Some(j) = args_json {
        let v: Value = serde_json::from_str(j).context("--args must be JSON object")?;
        match v {
            Value::Object(o) => map.extend(o),
            Value::Null => {}
            other => bail!("--args must be a JSON object, got {other}"),
        }
    }
    for pair in kv_pairs {
        let (k, v) = pair
            .split_once('=')
            .with_context(|| format!("--arg expects key=value, got {pair}"))?;
        let parsed =
            serde_json::from_str::<Value>(v).unwrap_or_else(|_| Value::String(v.to_string()));
        map.insert(k.to_string(), parsed);
    }
    if map.is_empty() {
        Ok(None)
    } else {
        Ok(Some(map))
    }
}
