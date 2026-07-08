# scripts

Standalone CLI scripts that stand on their own outside Claude Code.

Symlink them onto your shell `PATH`:

```
ln -s /code/vinys-toolbelt/scripts/<name> ~/.local/bin/<name>
```

If a script is also something Claude should be able to invoke directly, symlink it
into a plugin's `bin/` as well — anything there joins the Bash tool's `PATH` while
the plugin is enabled:

```
ln -s ../../../scripts/<name> plugins/ai-toolbelt/bin/<name>
```

Keep the source of truth here; `bin/` holds links, not copies.
