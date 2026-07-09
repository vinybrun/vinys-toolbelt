//! Full browser automated testing MCP server (stdio).
//!
//! ```bash
//! cargo run --bin browser-testing-mcp
//!
//! MCP_BROWSER_ALLOW_ALL_HOSTS=1 cargo run --bin browser-testing-mcp
//!
//! npx @modelcontextprotocol/inspector cargo run --bin browser-testing-mcp
//! ```

mod browser_testing;

use anyhow::Result;
use browser_testing::BrowserTesting;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::{self, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let workspace = std::env::var("MCP_BROWSER_WORKSPACE")
        .map(std::path::PathBuf::from)
        .or_else(|_| std::env::current_dir())
        .expect("workspace");

    tracing::info!(
        workspace = %workspace.display(),
        "starting rmcp browser testing server"
    );

    let service = BrowserTesting::new(workspace)
        .serve(stdio())
        .await
        .inspect_err(|e| {
            tracing::error!("serving error: {e:?}");
        })?;

    service.waiting().await?;
    Ok(())
}
