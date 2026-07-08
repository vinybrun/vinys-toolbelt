# vinys-toolbelt

Personal AI tooling, in one place: Claude Code skills, agents, hooks, MCP servers,
standalone scripts, and background services.

The repo doubles as a **Claude Code plugin marketplace**, so the tools here can be
installed into Claude Code directly rather than copy-pasted into `~/.claude/`.

## Install into Claude Code

```
/plugin marketplace add /code/vinys-toolbelt
/plugin install ai-toolbelt@vinys-toolbelt
```

Plugin versions resolve from the git commit SHA (there is no `version` field in
`plugin.json` on purpose), so `/plugin update` picks up every new commit without
a manual version bump.

## Layout

| Path        | What lives here                                                              |
| :---------- | :--------------------------------------------------------------------------- |
| `plugins/`  | Claude Code plugins. Each is installable on its own; listed in the marketplace. |
| `scripts/`  | Standalone CLI scripts, usable from a normal shell without Claude Code.       |
| `services/` | Long-running daemons and their unit files.                                    |
| `mcp/`      | MCP server source code.                                                       |
| `docs/`     | Notes and design docs.                                                        |

`.claude-plugin/marketplace.json` is the marketplace catalog. Every plugin added
under `plugins/` must also get an entry there, or it won't be installable.

## Anatomy of a plugin

Components live at the **plugin root**, not inside `.claude-plugin/`. Only the
manifest goes in `.claude-plugin/`.

```
plugins/ai-toolbelt/
├── .claude-plugin/
│   └── plugin.json     ← manifest, and nothing else
├── skills/<name>/SKILL.md
├── agents/<name>.md
├── hooks/hooks.json
├── bin/                ← executables, added to Claude's Bash PATH
└── .mcp.json           ← MCP servers this plugin provides
```

The `bin/` directory is the useful trick: while the plugin is enabled, anything in
it is invokable as a bare command inside any Bash tool call. That is how a script
in this repo becomes a first-class tool Claude can reach for.

## Adding things

**A skill** — `plugins/ai-toolbelt/skills/<name>/SKILL.md`, with YAML frontmatter
carrying at minimum a `description`. The description is what Claude matches
against when deciding whether to invoke it, so write it as trigger conditions,
not as a summary.

**An agent** — `plugins/ai-toolbelt/agents/<name>.md`.

**An executable** — drop it in `plugins/ai-toolbelt/bin/`, `chmod +x`, done.

**An MCP server** — write it under `mcp/<name>/`, then register it in the
plugin's `.mcp.json`, referencing it via `${CLAUDE_PLUGIN_ROOT}` rather than an
absolute path.

**A new plugin** — create `plugins/<name>/` with its own `.claude-plugin/plugin.json`,
then add an entry to `.claude-plugin/marketplace.json`.

## Publishing safety

This repo is public, so `scripts/publish-guard` guards what leaves it. It blocks
real email addresses, `/home/<user>` paths, API keys and private keys, and mentions
of the model vendors and model IDs in use.

It is wired in through `core.hooksPath = .githooks`, so it runs on every commit
(staged content) and every push (whole tree plus commit metadata). A fresh clone
needs one command to arm it:

```
git config core.hooksPath .githooks
```

Run it by hand any time:

```
./scripts/publish-guard            # tracked files
./scripts/publish-guard --all      # files + every commit's author email
```

Rules live in `.publish-guard.json`. To exempt a line that is a false positive,
append a `publish-guard: allow <rule-id>` comment to it. Rewording beats exempting.

## Validate before committing

```
claude plugin validate .                      # the marketplace + every plugin it lists
claude plugin validate ./plugins/ai-toolbelt  # one plugin
```

Both emit a `version: No version specified` warning and pass. That warning is
expected — see `CLAUDE.md`. Don't pass `--strict` here; it promotes that warning
into an error.
