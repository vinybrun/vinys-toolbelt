# mcp

Source code for MCP servers, one directory per server.

## What's here

| Directory          | What it is                                                                    |
| :----------------- | :---------------------------------------------------------------------------- |
| `browser-testing/` | Browser automation MCP server — 86 tools over a Playwright bridge. `cargo run --bin browser-testing-mcp` |
| `full-mcp-client/` | Interactive MCP client (stdio + HTTP): tools, resources, prompts, REPL, coverage runs. |

Both were factored out of a working tree of `modelcontextprotocol/rust-sdk`, where they
lived as untracked `examples/` overlays. They now depend on published `rmcp` and build
standalone. The client can spawn the server directly, which is the quickest smoke test:

```bash
cargo build --manifest-path browser-testing/Cargo.toml
cargo run --manifest-path full-mcp-client/Cargo.toml --bin mcp-client -- \
  --stdio ../browser-testing/target/debug/browser-testing-mcp tools list
```

A server here is wired into Claude Code by declaring it in a plugin's `.mcp.json`.
Use `${CLAUDE_PLUGIN_ROOT}` so the entry keeps working regardless of where the
plugin is installed from:

```json
{
  "mcpServers": {
    "example": {
      "command": "node",
      "args": ["${CLAUDE_PLUGIN_ROOT}/../../mcp/example/index.js"]
    }
  }
}
```

Test a server standalone before wiring it up. `claude --debug` surfaces
initialization errors when it fails to start.
