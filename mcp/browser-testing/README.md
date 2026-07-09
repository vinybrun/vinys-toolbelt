# Browser Testing MCP (`rmcp-browser-testing`)

Full browser automated testing suite for AI agents: **Rust MCP server + Playwright Node bridge**.

Factored out of a working tree of [`modelcontextprotocol/rust-sdk`][sdk], where it lived
as an `examples/servers` overlay. It is now a standalone crate depending on published
`rmcp`, and builds on its own.

[sdk]: https://github.com/modelcontextprotocol/rust-sdk

## Quick start

```bash
# 1) Install bridge deps + Chromium (once)
cd browser-bridge && npm install && npx playwright install chromium && cd ..

# 2) Build and run the MCP server
cargo run --bin browser-testing-mcp
```

Register it with any MCP client. For Claude Code, in the plugin's `.mcp.json` or your
own `.mcp.json`:

```json
{
  "mcpServers": {
    "browser": {
      "command": "cargo",
      "args": ["run", "--quiet", "--manifest-path", "mcp/browser-testing/Cargo.toml",
               "--bin", "browser-testing-mcp"],
      "env": { "MCP_BROWSER_ALLOWED_HOSTS": "127.0.0.1,localhost" }
    }
  }
}
```

Build a release binary and point `command` straight at it to avoid paying `cargo run`
startup on every MCP handshake.

## Driving it from the CLI

`../full-mcp-client` speaks to this server (or any MCP server) over stdio:

```bash
mcp-client --stdio ./target/debug/browser-testing-mcp info
mcp-client --stdio ./target/debug/browser-testing-mcp tools list
mcp-client --stdio ./target/debug/browser-testing-mcp tools call browser_info
```

## Agent workflow

1. `browser_info` — config + capabilities  
2. `browser_install_deps` — if first run  
3. `fixture_server_start` — local demo app on :8765  
4. `browser_launch` — chromium context + page  
5. `goto` → `snapshot` → `click` / `fill` → `expect_*`  
6. `run_suite` path=`browser-tests/smoke.yaml`  
7. `a11y_scan`, `visual_diff`, `tracing_*`, `list_artifacts`  

## Safety

- Default host allowlist: `localhost`, `127.0.0.1`  
- `MCP_BROWSER_ALLOW_ALL_HOSTS=1` to disable (careful)  
- Paths are workspace-relative  
- `file://` blocked  
- Secrets redacted in tool output  

## Env vars

| Variable | Purpose |
|----------|---------|
| `MCP_BROWSER_WORKSPACE` | Workspace root (default cwd) |
| `MCP_BROWSER_ARTIFACTS` | Artifact directory |
| `MCP_BROWSER_BRIDGE` | Path to `bridge.mjs` |
| `MCP_BROWSER_ALLOWED_HOSTS` | Comma-separated hosts |
| `MCP_BROWSER_ALLOW_ALL_HOSTS` | `1` = any host |
| `MCP_BROWSER_BASE_URL` | Default base URL |
| `MCP_BROWSER_HEADLESS_ONLY` | Force headless (CI) |

## Declarative suites

YAML under `browser-tests/`:

```yaml
name: my-flow
base_url: http://127.0.0.1:8765
tags: [smoke]
steps:
  - action: goto
    url: /
  - action: expect_text
    text: Hello
  - action: click
    role: button
    name: Submit
```

## Resources

- `browser://session`, `browser://guide`, `browser://config`  
- `artifact://...` for run reports, screenshots, traces  

## Prompts

`explore_and_map_app`, `write_e2e_for_flow`, `debug_failing_test`, `a11y_review`, `smoke_critical_paths`, `flaky_triage`
