# full-mcp-client (`mcp-client`)

A **complete MCP client** built on the official Rust SDK (`rmcp`). Use it to explore, script, and interactively drive any MCP server over **stdio** or **streamable HTTP**.

## Features

| Area | Support |
|------|---------|
| Transports | stdio child process, streamable HTTP |
| Tools | list, schema, call, filter; optional **task** invocation + poll |
| Resources | list, read, templates, subscribe |
| Prompts | list, get (with arguments) |
| Completions | prompt / resource argument completion |
| Progress | prints progress notifications |
| Modes | one-shot CLI, JSON output, interactive **REPL**, batch **script** |
| Presets | `browser` → browser-testing MCP server in this repo |

## Build

```bash
cargo build -p full-mcp-client --release
# binary: target/release/mcp-client
```

## Quick start

### Against the browser testing server (this monorepo)

```bash
# install bridge deps once
cd examples/servers/browser-bridge && npm install && npx playwright install chromium

# REPL
cargo run -p full-mcp-client -- --preset browser repl

# one-shot
cargo run -p full-mcp-client -- --preset browser info
cargo run -p full-mcp-client -- --preset browser tools list
cargo run -p full-mcp-client -- --preset browser tools call browser_info
```

### Custom stdio server

```bash
cargo run -p full-mcp-client -- \
  --stdio cargo -- -q -p mcp-server-examples --example servers_counter_stdio \
  tools list

# env vars for the child
cargo run -p full-mcp-client -- \
  --stdio node -- server.js \
  --env API_KEY=secret \
  tools call ping
```

### HTTP server

```bash
cargo run -p full-mcp-client -- --http http://127.0.0.1:8000/mcp tools list
```

## Commands

```
mcp-client [connection opts] <command>

Connection:
  --stdio <program> [args...]     spawn MCP server
  --http <url>                    streamable HTTP endpoint
  --header K:V                    HTTP headers (repeatable)
  --env K=V                       child env (stdio)
  --cwd <path>                    child working directory
  --preset browser|counter|dev-tools
  --json                          machine-readable output
  --config <path>                 load defaults from TOML

Subcommands:
  info                            server capabilities & implementation
  tools list [--filter TEXT]
  tools schema <name>
  tools call <name> [--args JSON] [--arg key=value]... [--task] [--poll-ms N]
  resources list
  resources read <uri>
  resources templates
  resources subscribe <uri>
  prompts list
  prompts get <name> [--args JSON]
  complete prompt <name> <argument> <value>
  complete resource <uri> <argument> <value>
  script <path.json>              run a batch of tool calls
  repl                            interactive shell
```

## REPL

```
mcp-client --preset browser repl

mcp> help
mcp> info
mcp> tools
mcp> call browser_launch {"headless":true}
mcp> call goto {"url":"http://127.0.0.1:8765/"}
mcp> call snapshot
mcp> resources
mcp> read browser://session
mcp> prompts
mcp> prompt smoke_critical_paths
mcp> /json on
mcp> quit
```

## Batch script format

`script.json`:

```json
{
  "steps": [
    { "tool": "browser_info", "arguments": {} },
    { "tool": "browser_launch", "arguments": { "headless": true } },
    { "tool": "goto", "arguments": { "url": "http://127.0.0.1:8765/" } },
    { "tool": "expect_text", "arguments": { "text": "Fixture" }, "stop_on_error": true }
  ]
}
```

```bash
mcp-client --preset browser script ./script.json
```

## Config file (optional)

`~/.config/mcp-client/config.toml` or `--config`:

```toml
default_preset = "browser"
json = false

[presets.my-server]
stdio = ["node", "/path/to/server.js"]
env = { NODE_ENV = "development" }
cwd = "/path/to/project"

[presets.remote]
http = "https://mcp.example.com/mcp"
headers = { Authorization = "Bearer ${TOKEN}" }
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | success |
| 1 | runtime / protocol error |
| 2 | bad CLI usage |
| 3 | tool returned `isError` |

## Browser MCP coverage

Exercise **every** advertised tool (plus resources/prompts) against a running site:

```bash
# terminal 1 — demo site with coverage harness
python3 -m http.server 8877 --bind 127.0.0.1 \
  --directory examples/servers/demo-site

# terminal 2
cargo run -p full-mcp-client -- --preset browser coverage \
  --base-url http://127.0.0.1:8877
```

Optional env for skipped tools:

| Env | Enables |
|-----|---------|
| `MCP_COVERAGE_CDP` | `browser_connect_cdp` (Chrome remote debugging URL) |
| `MCP_COVERAGE_PLAYWRIGHT=1` | `run_playwright_tests` |

Report JSON: `examples/servers/browser-artifacts/coverage-report.json`
