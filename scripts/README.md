# scripts

Standalone CLI scripts that stand on their own outside Claude Code.

## aibox

Disposable rootless-podman sandboxes that clone a repo and run a coding agent CLI
inside it, one container per agent, on a shared bridge network so sandboxes can
reach each other by name. `aibox --help` for usage.

Nothing machine- or vendor-specific lives in the script. Which agent gets installed,
which models it runs, and — importantly — **where sandbox data lives** all come from
`~/.config/aibox/config`, which is never committed:

```bash
AIBOX_ROOT=/path/to/sandbox/data      # podman store + every env folder hang off this
AIBOX_AGENT_HOME=.your-agent          # dotdir bind-mounted into each container's $HOME
AIBOX_INSTALL_CMD='curl -fsSL https://example.com/install.sh | bash'
AIBOX_UPDATE_CMD='your-agent update'
AIBOX_CMDS=("your-agent -m model-a" "your-agent -m model-b")
```

`AIBOX_ROOT` matters: the store is tens of gigabytes and must not follow the script.
Without it, `ROOT` falls back to the resolved script directory, so a checked-in and
symlinked `aibox` would create a brand-new empty store next to this README and lose
sight of every existing container. Set it once, in the config.

Environment variables override the config file, so `AIBOX_ROOT=/tmp/x aibox ls` works
for a one-off.

## enable-kvm

Loads `kvm_intel`/`kvm_amd` with nested virtualisation and persists it across reboots,
so x86_64 emulators get hardware acceleration inside aibox containers. Needs root:
`sudo ./scripts/enable-kvm`.

## Wiring scripts up

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
