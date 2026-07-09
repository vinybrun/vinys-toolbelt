//! Full browser automated testing MCP server.
//!
//! Architecture: Rust MCP surface + Playwright Node bridge (`browser-bridge/bridge.mjs`).

mod artifacts;
mod bridge;
mod config;
mod safety;
mod server;
mod suite;
mod visual;

pub use server::BrowserTesting;
