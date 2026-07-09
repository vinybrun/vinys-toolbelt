//! Interactive REPL for exploring an MCP server.

use anyhow::Result;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use serde_json::{Map, Value};

use crate::session::{Session, build_arguments};

const HELP: &str = r#"
Interactive MCP client — commands:

  help                         Show this help
  info                         Server info & capabilities
  tools [filter]               List tools (optional filter)
  schema <tool>                Show tool input schema
  call <tool> [json-args]      Call a tool
  task <tool> [json-args]      Call tool as long-running task
  resources                    List resources
  read <uri>                   Read a resource
  templates                    List resource templates
  subscribe <uri>              Subscribe to resource updates
  unsubscribe <uri>            Unsubscribe
  prompts                      List prompts
  prompt <name> [json-args]    Get a prompt
  complete-prompt <n> <a> <v>  Complete prompt argument
  complete-res <uri> <a> <v>   Complete resource argument
  tasks                        List tasks (if supported)
  task-get <id>                Get task status
  task-cancel <id>             Cancel a task
  /json on|off                 Toggle JSON output
  quit | exit                  Leave the REPL
"#;

pub async fn run_repl(session: &Session) -> Result<()> {
    let mut rl = DefaultEditor::new()?;
    let history = dirs::data_local_dir()
        .map(|d| d.join("mcp-client").join("history.txt"));
    if let Some(ref h) = history {
        let _ = std::fs::create_dir_all(h.parent().unwrap());
        let _ = rl.load_history(h);
    }

    println!("Connected. Type `help` for commands, `quit` to exit.");
    if let Some(info) = session.server_info() {
        println!(
            "Server: {} v{}",
            info.server_info.name, info.server_info.version
        );
    }

    loop {
        let line = match rl.readline("mcp> ") {
            Ok(l) => l,
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(e) => {
                session.printer.error(&e);
                break;
            }
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let _ = rl.add_history_entry(line);
        if let Err(e) = handle_line(session, line).await {
            session.printer.error(&e);
        }
        if matches!(line, "quit" | "exit" | "q") {
            break;
        }
    }

    if let Some(ref h) = history {
        let _ = rl.save_history(h);
    }
    Ok(())
}

async fn handle_line(session: &Session, line: &str) -> Result<()> {
    let mut parts = shell_split(line);
    if parts.is_empty() {
        return Ok(());
    }
    let cmd = parts.remove(0);
    match cmd.as_str() {
        "help" | "?" => {
            print!("{HELP}");
        }
        "quit" | "exit" | "q" => {}
        "info" => {
            if let Some(info) = session.server_info() {
                session.printer.info(info.as_ref());
            } else {
                session.printer.error(&"no server info");
            }
        }
        "tools" => {
            let filter = parts.first().map(|s| s.as_str());
            let tools = session.list_tools().await?;
            session.printer.tools(&tools, filter);
        }
        "schema" => {
            let name = parts
                .first()
                .ok_or_else(|| anyhow::anyhow!("usage: schema <tool>"))?;
            let tool = session.find_tool(name).await?;
            session.printer.tool_schema(&tool);
        }
        "call" | "task" => {
            let name = parts
                .first()
                .ok_or_else(|| anyhow::anyhow!("usage: {cmd} <tool> [json-args]"))?
                .clone();
            let args_json = parts.get(1).map(|s| s.as_str());
            let arguments = build_arguments(args_json, &[])?;
            let result = if cmd == "task" {
                session
                    .call_tool_as_task(
                        &name,
                        arguments,
                        250,
                        std::time::Duration::from_secs(600),
                    )
                    .await?
            } else {
                session.call_tool(&name, arguments).await?
            };
            let ok = session.printer.tool_result(&result);
            if !ok {
                anyhow::bail!("tool returned isError");
            }
        }
        "resources" => {
            let items = session.list_resources().await?;
            session.printer.resources(&items);
        }
        "read" => {
            let uri = parts
                .first()
                .ok_or_else(|| anyhow::anyhow!("usage: read <uri>"))?;
            let r = session.read_resource(uri).await?;
            session.printer.resource_contents(&r);
        }
        "templates" => {
            let items = session.list_resource_templates().await?;
            session.printer.resource_templates(&items);
        }
        "subscribe" => {
            let uri = parts
                .first()
                .ok_or_else(|| anyhow::anyhow!("usage: subscribe <uri>"))?;
            session.subscribe(uri).await?;
            session.printer.ok(&format!("subscribed to {uri}"));
        }
        "unsubscribe" => {
            let uri = parts
                .first()
                .ok_or_else(|| anyhow::anyhow!("usage: unsubscribe <uri>"))?;
            session.unsubscribe(uri).await?;
            session.printer.ok(&format!("unsubscribed from {uri}"));
        }
        "prompts" => {
            let items = session.list_prompts().await?;
            session.printer.prompts(&items);
        }
        "prompt" => {
            let name = parts
                .first()
                .ok_or_else(|| anyhow::anyhow!("usage: prompt <name> [json-args]"))?
                .clone();
            let args_json = parts.get(1).map(|s| s.as_str());
            let arguments = build_arguments(args_json, &[])?;
            let r = session.get_prompt(&name, arguments).await?;
            session.printer.prompt_result(&r);
        }
        "complete-prompt" => {
            if parts.len() < 3 {
                anyhow::bail!("usage: complete-prompt <name> <argument> <value>");
            }
            let vals = session
                .complete_prompt(&parts[0], &parts[1], &parts[2])
                .await?;
            if session.printer.json {
                session.printer.out_json(&vals);
            } else {
                for v in vals {
                    println!("  {v}");
                }
            }
        }
        "complete-res" | "complete-resource" => {
            if parts.len() < 3 {
                anyhow::bail!("usage: complete-res <uri> <argument> <value>");
            }
            let vals = session
                .complete_resource(&parts[0], &parts[1], &parts[2])
                .await?;
            if session.printer.json {
                session.printer.out_json(&vals);
            } else {
                for v in vals {
                    println!("  {v}");
                }
            }
        }
        "tasks" => {
            let v = session.list_tasks().await?;
            session.printer.out_json(&v);
        }
        "task-get" => {
            let id = parts
                .first()
                .ok_or_else(|| anyhow::anyhow!("usage: task-get <id>"))?;
            let v = session.get_task(id).await?;
            session.printer.out_json(&v);
        }
        "task-cancel" => {
            let id = parts
                .first()
                .ok_or_else(|| anyhow::anyhow!("usage: task-cancel <id>"))?;
            let v = session.cancel_task(id).await?;
            session.printer.out_json(&v);
        }
        "/json" => {
            // printer.json is not mutable through &Session — document as start-time only
            println!("JSON mode is set at process start with --json (REPL inherits it).");
        }
        other => {
            // allow: call-like shorthand  toolname {"a":1}
            if other.contains('{') {
                anyhow::bail!("unknown command: {other} (try: call <tool> '{{...}}')");
            }
            // if first token looks like a tool name present on server, call it
            if let Ok(tool) = session.find_tool(other).await {
                let args_json = parts.first().map(|s| s.as_str());
                let arguments = build_arguments(args_json, &[])?;
                let result = session.call_tool(&tool.name, arguments).await?;
                let ok = session.printer.tool_result(&result);
                if !ok {
                    anyhow::bail!("tool returned isError");
                }
            } else {
                anyhow::bail!("unknown command: {other} (type help)");
            }
        }
    }
    Ok(())
}

/// Minimal shell-like split respecting simple double quotes.
fn shell_split(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => in_quotes = !in_quotes,
            ' ' | '\t' if !in_quotes => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            '\\' if in_quotes => {
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[allow(dead_code)]
fn merge_json_arg(raw: Option<&str>) -> Result<Option<Map<String, Value>>> {
    build_arguments(raw, &[])
}
