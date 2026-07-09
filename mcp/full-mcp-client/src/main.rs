//! full-mcp-client — complete CLI/REPL MCP client on rmcp.

mod config;
mod coverage;
mod display;
mod repl;
mod session;

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::{Config, builtin_preset};
use crate::session::{ConnectOptions, Session, build_arguments};

#[derive(Parser, Debug)]
#[command(
    name = "mcp-client",
    version,
    about = "Complete MCP client (stdio + HTTP): tools, resources, prompts, tasks, REPL",
    long_about = None
)]
struct Cli {
    /// Spawn a local MCP server: program name (remaining -- args go after --stdio-arg)
    #[arg(long, value_name = "PROGRAM")]
    stdio: Option<String>,

    /// Extra args for --stdio program (repeatable). Prefer: --stdio cargo -- -q -p ...
    #[arg(long = "stdio-arg", value_name = "ARG")]
    stdio_args: Vec<String>,

    /// Everything after bare `--` becomes the stdio command (program + args)
    #[arg(last = true)]
    raw_command: Vec<String>,

    /// Streamable HTTP MCP endpoint
    #[arg(long, value_name = "URL")]
    http: Option<String>,

    /// HTTP header K:V (repeatable)
    #[arg(long = "header", value_name = "K:V")]
    headers: Vec<String>,

    /// Environment variable for stdio child K=V (repeatable)
    #[arg(long = "env", value_name = "K=V")]
    env: Vec<String>,

    /// Working directory for stdio child
    #[arg(long)]
    cwd: Option<PathBuf>,

    /// Built-in preset: browser, counter, dev-tools, task
    #[arg(long, value_name = "NAME")]
    preset: Option<String>,

    /// Machine-readable JSON on stdout
    #[arg(long)]
    json: bool,

    /// Config TOML path (defaults to ~/.config/mcp-client/config.toml if present)
    #[arg(long)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show server implementation and capabilities
    Info,
    /// Tool operations
    Tools {
        #[command(subcommand)]
        action: ToolsCmd,
    },
    /// Resource operations
    Resources {
        #[command(subcommand)]
        action: ResourcesCmd,
    },
    /// Prompt operations
    Prompts {
        #[command(subcommand)]
        action: PromptsCmd,
    },
    /// Completions for prompt/resource arguments
    Complete {
        #[command(subcommand)]
        action: CompleteCmd,
    },
    /// Task (long-running invocation) helpers
    Task {
        #[command(subcommand)]
        action: TaskCmd,
    },
    /// Run a batch script of tool calls (JSON file)
    Script {
        path: PathBuf,
        /// Stop on first tool error (default true)
        #[arg(long, default_value_t = true)]
        stop_on_error: bool,
    },
    /// Interactive REPL
    Repl,
    /// Run systematic browser MCP tool/resource/prompt coverage
    Coverage {
        /// Site origin under test (default http://127.0.0.1:8877)
        #[arg(long, default_value = "http://127.0.0.1:8877")]
        base_url: String,
    },
}

#[derive(Subcommand, Debug)]
enum ToolsCmd {
    /// List tools
    List {
        #[arg(long, short)]
        filter: Option<String>,
    },
    /// Show a tool's schema
    Schema { name: String },
    /// Call a tool
    Call {
        name: String,
        /// JSON object of arguments
        #[arg(long)]
        args: Option<String>,
        /// key=value (JSON value or string); repeatable
        #[arg(long = "arg")]
        arg: Vec<String>,
        /// Invoke as SEP-1319 task and poll
        #[arg(long)]
        task: bool,
        /// Poll interval ms when --task
        #[arg(long, default_value_t = 250)]
        poll_ms: u64,
        /// Timeout seconds when --task
        #[arg(long, default_value_t = 600)]
        timeout: u64,
    },
}

#[derive(Subcommand, Debug)]
enum ResourcesCmd {
    List,
    Read { uri: String },
    Templates,
    Subscribe { uri: String },
    Unsubscribe { uri: String },
}

#[derive(Subcommand, Debug)]
enum PromptsCmd {
    List,
    Get {
        name: String,
        #[arg(long)]
        args: Option<String>,
        #[arg(long = "arg")]
        arg: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
enum CompleteCmd {
    Prompt {
        name: String,
        argument: String,
        value: String,
    },
    Resource {
        uri: String,
        argument: String,
        value: String,
    },
}

#[derive(Subcommand, Debug)]
enum TaskCmd {
    List,
    Get { id: String },
    Cancel { id: String },
}

#[derive(Debug, Deserialize)]
struct ScriptFile {
    steps: Vec<ScriptStep>,
}

#[derive(Debug, Deserialize)]
struct ScriptStep {
    tool: String,
    #[serde(default)]
    arguments: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    task: bool,
    #[serde(default)]
    stop_on_error: Option<bool>,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    init_tracing(cli.json);

    match run(cli).await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(1)
        }
    }
}

fn init_tracing(json: bool) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        if json {
            EnvFilter::new("warn")
        } else {
            EnvFilter::new("info")
        }
    });
    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_target(false),
        )
        .init();
}

async fn run(cli: Cli) -> Result<ExitCode> {
    let file_cfg = if let Some(ref p) = cli.config {
        Config::load(Some(p))?
    } else {
        Config::try_load_default()
    };
    let json = cli.json || file_cfg.json;

    let opts = resolve_connect(&cli, &file_cfg, json)?;
    let session = Session::connect(opts)
        .await
        .context("failed to connect to MCP server")?;

    let code = match cli.command {
        Commands::Info => {
            if let Some(info) = session.server_info() {
                session.printer.info(info.as_ref());
            } else {
                bail!("server did not return initialize info");
            }
            ExitCode::SUCCESS
        }
        Commands::Tools { action } => match action {
            ToolsCmd::List { filter } => {
                let tools = session.list_tools().await?;
                session.printer.tools(&tools, filter.as_deref());
                ExitCode::SUCCESS
            }
            ToolsCmd::Schema { name } => {
                let tool = session.find_tool(&name).await?;
                session.printer.tool_schema(&tool);
                ExitCode::SUCCESS
            }
            ToolsCmd::Call {
                name,
                args,
                arg,
                task,
                poll_ms,
                timeout,
            } => {
                let arguments = build_arguments(args.as_deref(), &arg)?;
                let result = if task {
                    session
                        .call_tool_as_task(
                            &name,
                            arguments,
                            poll_ms,
                            Duration::from_secs(timeout),
                        )
                        .await?
                } else {
                    session.call_tool(&name, arguments).await?
                };
                let ok = session.printer.tool_result(&result);
                if ok {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(3)
                }
            }
        },
        Commands::Resources { action } => match action {
            ResourcesCmd::List => {
                let items = session.list_resources().await?;
                session.printer.resources(&items);
                ExitCode::SUCCESS
            }
            ResourcesCmd::Read { uri } => {
                let r = session.read_resource(&uri).await?;
                session.printer.resource_contents(&r);
                ExitCode::SUCCESS
            }
            ResourcesCmd::Templates => {
                let items = session.list_resource_templates().await?;
                session.printer.resource_templates(&items);
                ExitCode::SUCCESS
            }
            ResourcesCmd::Subscribe { uri } => {
                session.subscribe(&uri).await?;
                session.printer.ok(&format!("subscribed to {uri}"));
                ExitCode::SUCCESS
            }
            ResourcesCmd::Unsubscribe { uri } => {
                session.unsubscribe(&uri).await?;
                session.printer.ok(&format!("unsubscribed from {uri}"));
                ExitCode::SUCCESS
            }
        },
        Commands::Prompts { action } => match action {
            PromptsCmd::List => {
                let items = session.list_prompts().await?;
                session.printer.prompts(&items);
                ExitCode::SUCCESS
            }
            PromptsCmd::Get { name, args, arg } => {
                let arguments = build_arguments(args.as_deref(), &arg)?;
                let r = session.get_prompt(&name, arguments).await?;
                session.printer.prompt_result(&r);
                ExitCode::SUCCESS
            }
        },
        Commands::Complete { action } => match action {
            CompleteCmd::Prompt {
                name,
                argument,
                value,
            } => {
                let vals = session.complete_prompt(&name, &argument, &value).await?;
                if json {
                    session.printer.out_json(&vals);
                } else {
                    for v in vals {
                        println!("{v}");
                    }
                }
                ExitCode::SUCCESS
            }
            CompleteCmd::Resource {
                uri,
                argument,
                value,
            } => {
                let vals = session.complete_resource(&uri, &argument, &value).await?;
                if json {
                    session.printer.out_json(&vals);
                } else {
                    for v in vals {
                        println!("{v}");
                    }
                }
                ExitCode::SUCCESS
            }
        },
        Commands::Task { action } => match action {
            TaskCmd::List => {
                let v = session.list_tasks().await?;
                session.printer.out_json(&v);
                ExitCode::SUCCESS
            }
            TaskCmd::Get { id } => {
                let v = session.get_task(&id).await?;
                session.printer.out_json(&v);
                ExitCode::SUCCESS
            }
            TaskCmd::Cancel { id } => {
                let v = session.cancel_task(&id).await?;
                session.printer.out_json(&v);
                ExitCode::SUCCESS
            }
        },
        Commands::Script {
            path,
            stop_on_error,
        } => {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("read script {}", path.display()))?;
            let script: ScriptFile = serde_json::from_str(&text).context("parse script JSON")?;
            let mut failed = false;
            for (i, step) in script.steps.iter().enumerate() {
                if !json {
                    eprintln!("→ step {}: {}", i + 1, step.tool);
                }
                let result = if step.task {
                    session
                        .call_tool_as_task(
                            &step.tool,
                            step.arguments.clone(),
                            250,
                            Duration::from_secs(600),
                        )
                        .await
                } else {
                    session
                        .call_tool(&step.tool, step.arguments.clone())
                        .await
                };
                match result {
                    Ok(r) => {
                        let ok = session.printer.tool_result(&r);
                        if !ok {
                            failed = true;
                            if step.stop_on_error.unwrap_or(stop_on_error) {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        session.printer.error(&e);
                        failed = true;
                        if step.stop_on_error.unwrap_or(stop_on_error) {
                            break;
                        }
                    }
                }
            }
            if failed {
                ExitCode::from(3)
            } else {
                ExitCode::SUCCESS
            }
        }
        Commands::Repl => {
            repl::run_repl(&session).await?;
            ExitCode::SUCCESS
        }
        Commands::Coverage { base_url } => {
            let code = coverage::run_browser_coverage(&session, &base_url).await?;
            ExitCode::from(code as u8)
        }
    };

    session.shutdown().await.ok();
    Ok(code)
}

fn resolve_connect(cli: &Cli, file_cfg: &Config, json: bool) -> Result<ConnectOptions> {
    // Highest priority: explicit --http / --stdio / raw -- command
    if let Some(url) = &cli.http {
        return Ok(ConnectOptions {
            stdio: None,
            http: Some(url.clone()),
            env: parse_kv(&cli.env)?,
            headers: parse_headers(&cli.headers)?,
            cwd: cli.cwd.clone(),
            json,
            quiet: json,
        });
    }

    if let Some(program) = &cli.stdio {
        let mut cmd = vec![program.clone()];
        cmd.extend(cli.stdio_args.clone());
        // also allow trailing -- args
        if !cli.raw_command.is_empty() {
            cmd.extend(cli.raw_command.clone());
        }
        return Ok(ConnectOptions {
            stdio: Some(cmd),
            http: None,
            env: parse_kv(&cli.env)?,
            headers: Default::default(),
            cwd: cli.cwd.clone(),
            json,
            quiet: json,
        });
    }

    if !cli.raw_command.is_empty() {
        return Ok(ConnectOptions {
            stdio: Some(cli.raw_command.clone()),
            http: None,
            env: parse_kv(&cli.env)?,
            headers: Default::default(),
            cwd: cli.cwd.clone(),
            json,
            quiet: json,
        });
    }

    let preset_name = cli
        .preset
        .clone()
        .or_else(|| file_cfg.default_preset.clone());
    if let Some(name) = preset_name {
        if let Some(p) = file_cfg.presets.get(&name) {
            let mut opts = ConnectOptions::from_preset(p, json);
            // merge CLI env/cwd overrides
            for (k, v) in parse_kv(&cli.env)? {
                opts.env.insert(k, v);
            }
            if let Some(c) = &cli.cwd {
                opts.cwd = Some(c.clone());
            }
            return Ok(opts);
        }
        if let Some(p) = builtin_preset(&name) {
            let mut opts = ConnectOptions::from_preset(&p, json);
            for (k, v) in parse_kv(&cli.env)? {
                opts.env.insert(k, v);
            }
            if let Some(c) = &cli.cwd {
                opts.cwd = Some(c.clone());
            }
            return Ok(opts);
        }
        bail!("unknown preset '{name}' (try: browser, counter, dev-tools, task)");
    }

    bail!(
        "no connection specified. Use one of:\n  \
         --preset browser\n  \
         --stdio <program> --stdio-arg ...\n  \
         --http <url>\n  \
         -- <program> [args...]"
    )
}

fn parse_kv(pairs: &[String]) -> Result<std::collections::HashMap<String, String>> {
    let mut m = std::collections::HashMap::new();
    for p in pairs {
        let (k, v) = p
            .split_once('=')
            .with_context(|| format!("expected K=V, got {p}"))?;
        m.insert(k.to_string(), v.to_string());
    }
    Ok(m)
}

fn parse_headers(pairs: &[String]) -> Result<std::collections::HashMap<String, String>> {
    let mut m = std::collections::HashMap::new();
    for p in pairs {
        let (k, v) = p
            .split_once(':')
            .with_context(|| format!("expected K:V header, got {p}"))?;
        m.insert(k.trim().to_string(), v.trim().to_string());
    }
    Ok(m)
}
