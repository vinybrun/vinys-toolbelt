# mcp

Source code for MCP servers, one directory per server.

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
