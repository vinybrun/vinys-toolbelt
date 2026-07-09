//! Pretty-printing helpers for MCP structures.

use colored::Colorize;
use rmcp::model::{
    CallToolResult, ContentBlock, GetPromptResult, Prompt, ReadResourceResult, Resource,
    ResourceContents, ResourceTemplate, ServerInfo, Tool,
};
use serde_json::Value;

pub struct Printer {
    pub json: bool,
}

impl Printer {
    pub fn new(json: bool) -> Self {
        Self { json }
    }

    pub fn out_json(&self, v: &(impl serde::Serialize + ?Sized)) {
        match serde_json::to_string_pretty(v) {
            Ok(s) => println!("{s}"),
            Err(e) => eprintln!("serialize error: {e}"),
        }
    }

    pub fn info(&self, info: &ServerInfo) {
        if self.json {
            self.out_json(info);
            return;
        }
        println!("{}", "Server".bold().cyan());
        let si = &info.server_info;
        println!("  name:        {}", si.name);
        println!("  version:     {}", si.version);
        if let Some(t) = &si.title {
            println!("  title:       {t}");
        }
        if let Some(instr) = &info.instructions {
            let short = if instr.len() > 400 {
                format!("{}…", &instr[..400])
            } else {
                instr.clone()
            };
            println!("  instructions:");
            for line in short.lines() {
                println!("    {}", line.dimmed());
            }
        }
        println!("{}", "Capabilities".bold().cyan());
        let caps = &info.capabilities;
        println!(
            "  tools: {}  prompts: {}  resources: {}  logging: {}  completions: {}",
            flag(caps.tools.is_some()),
            flag(caps.prompts.is_some()),
            flag(caps.resources.is_some()),
            flag(caps.logging.is_some()),
            flag(caps.completions.is_some()),
        );
        println!("  protocol:    {:?}", info.protocol_version);
    }

    pub fn tools(&self, tools: &[Tool], filter: Option<&str>) {
        let tools: Vec<&Tool> = tools
            .iter()
            .filter(|t| {
                filter.is_none_or(|f| {
                    let f = f.to_lowercase();
                    t.name.to_lowercase().contains(&f)
                        || t.description
                            .as_ref()
                            .is_some_and(|d| d.to_lowercase().contains(&f))
                })
            })
            .collect();
        if self.json {
            self.out_json(&tools);
            return;
        }
        println!(
            "{} ({} tool{})",
            "Tools".bold().cyan(),
            tools.len(),
            if tools.len() == 1 { "" } else { "s" }
        );
        for t in tools {
            println!("  {} {}", "•".green(), t.name.bold());
            if let Some(d) = &t.description {
                let one = d.lines().next().unwrap_or("").trim();
                if !one.is_empty() {
                    println!("      {}", one.dimmed());
                }
            }
        }
    }

    pub fn tool_schema(&self, tool: &Tool) {
        if self.json {
            self.out_json(tool);
            return;
        }
        println!("{} {}", "Tool".bold().cyan(), tool.name.bold());
        if let Some(d) = &tool.description {
            println!("{d}");
        }
        println!("{}", "inputSchema:".bold());
        println!(
            "{}",
            serde_json::to_string_pretty(&tool.input_schema).unwrap_or_default()
        );
        if let Some(out) = &tool.output_schema {
            println!("{}", "outputSchema:".bold());
            println!("{}", serde_json::to_string_pretty(out).unwrap_or_default());
        }
    }

    pub fn tool_result(&self, result: &CallToolResult) -> bool {
        let is_err = result.is_error.unwrap_or(false);
        if self.json {
            self.out_json(result);
            return !is_err;
        }
        let header = if is_err {
            "Tool error".bold().red()
        } else {
            "Tool result".bold().green()
        };
        println!("{header}");
        for block in &result.content {
            match block {
                ContentBlock::Text(t) => {
                    // try pretty-print if JSON
                    if let Ok(v) = serde_json::from_str::<Value>(&t.text) {
                        println!("{}", serde_json::to_string_pretty(&v).unwrap_or(t.text.clone()));
                    } else {
                        println!("{}", t.text);
                    }
                }
                ContentBlock::Image(img) => {
                    println!(
                        "[image mime={} bytes≈{}]",
                        img.mime_type,
                        img.data.len()
                    );
                }
                ContentBlock::Audio(a) => {
                    println!("[audio mime={}]", a.mime_type);
                }
                ContentBlock::ResourceLink(link) => {
                    println!("[resource link] {} ({})", link.name, link.uri);
                }
                ContentBlock::Resource(res) => {
                    println!("[embedded resource] {res:?}");
                }
                _ => {
                    println!("[{block:?}]");
                }
            }
        }
        if let Some(sc) = &result.structured_content {
            println!("{}", "structuredContent:".bold());
            println!("{}", serde_json::to_string_pretty(sc).unwrap_or_default());
        }
        !is_err
    }

    pub fn resources(&self, items: &[Resource]) {
        if self.json {
            self.out_json(items);
            return;
        }
        println!(
            "{} ({} resource{})",
            "Resources".bold().cyan(),
            items.len(),
            if items.len() == 1 { "" } else { "s" }
        );
        for r in items {
            println!("  {} {}", "•".green(), r.uri.bold());
            println!("      name: {}", r.name);
            if let Some(d) = &r.description {
                println!("      {}", d.dimmed());
            }
        }
    }

    pub fn resource_templates(&self, items: &[ResourceTemplate]) {
        if self.json {
            self.out_json(items);
            return;
        }
        println!("{}", "Resource templates".bold().cyan());
        for r in items {
            println!("  {} {}", "•".green(), r.uri_template.bold());
            if let Some(d) = &r.description {
                println!("      {}", d.dimmed());
            }
        }
    }

    pub fn resource_contents(&self, result: &ReadResourceResult) {
        if self.json {
            self.out_json(result);
            return;
        }
        println!("{}", "Resource".bold().cyan());
        for c in &result.contents {
            match c {
                ResourceContents::TextResourceContents { uri, text, mime_type, .. } => {
                    println!("  uri:  {uri}");
                    if let Some(m) = mime_type {
                        println!("  mime: {m}");
                    }
                    if let Ok(v) = serde_json::from_str::<Value>(text) {
                        println!("{}", serde_json::to_string_pretty(&v).unwrap_or(text.clone()));
                    } else {
                        println!("{text}");
                    }
                }
                ResourceContents::BlobResourceContents { uri, blob, mime_type, .. } => {
                    println!("  uri:  {uri}");
                    if let Some(m) = mime_type {
                        println!("  mime: {m}");
                    }
                    println!("  blob: {} chars (base64)", blob.len());
                }
                _ => {
                    println!("  {c:?}");
                }
            }
        }
    }

    pub fn prompts(&self, items: &[Prompt]) {
        if self.json {
            self.out_json(items);
            return;
        }
        println!(
            "{} ({} prompt{})",
            "Prompts".bold().cyan(),
            items.len(),
            if items.len() == 1 { "" } else { "s" }
        );
        for p in items {
            println!("  {} {}", "•".green(), p.name.bold());
            if let Some(d) = &p.description {
                println!("      {}", d.dimmed());
            }
            if let Some(args) = &p.arguments {
                let names: Vec<_> = args.iter().map(|a| a.name.as_str()).collect();
                println!("      args: {}", names.join(", ").dimmed());
            }
        }
    }

    pub fn prompt_result(&self, result: &GetPromptResult) {
        if self.json {
            self.out_json(result);
            return;
        }
        println!("{}", "Prompt".bold().cyan());
        if let Some(d) = &result.description {
            println!("  {}", d.dimmed());
        }
        for (i, msg) in result.messages.iter().enumerate() {
            println!("  [{}] {:?}", i, msg.role);
            match &msg.content {
                ContentBlock::Text(t) => println!("{}", t.text),
                other => println!("{other:?}"),
            }
        }
    }

    pub fn error(&self, err: &impl std::fmt::Display) {
        if self.json {
            self.out_json(&serde_json::json!({ "error": err.to_string() }));
        } else {
            eprintln!("{} {err}", "error:".bold().red());
        }
    }

    pub fn ok(&self, msg: &str) {
        if self.json {
            self.out_json(&serde_json::json!({ "ok": true, "message": msg }));
        } else {
            println!("{} {msg}", "ok".bold().green());
        }
    }
}

fn flag(on: bool) -> colored::ColoredString {
    if on {
        "yes".green()
    } else {
        "no".dimmed()
    }
}

/// Extract plain text from a tool result (joined).
pub fn tool_result_text(result: &CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|b| b.as_text().map(|t| t.text.clone()))
        .collect::<Vec<_>>()
        .join("\n")
}
