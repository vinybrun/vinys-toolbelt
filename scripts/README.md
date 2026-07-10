# scripts

Standalone CLI scripts that stand on their own outside Claude Code.

## aibox

Disposable rootless-podman sandboxes that clone a repo and run a coding agent CLI
inside it. Full docs — usage, host dependencies, every `AIBOX_*` config knob, and
agent setup (including alpha channel) — live in the script header:

```
aibox --help
```

Config still comes from `~/.config/aibox/config` (never committed). Set `AIBOX_ROOT`
there so the multi-GB podman store does not follow a checked-in / symlinked script.

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
