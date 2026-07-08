# vinys-toolbelt

Personal AI tooling. The repo is also a Claude Code plugin marketplace.

## Conventions

- Plugin components go at the **plugin root** (`skills/`, `agents/`, `hooks/`, `bin/`,
  `.mcp.json`). Only `plugin.json` belongs in `.claude-plugin/`. Components placed
  inside `.claude-plugin/` are silently not loaded.
- Do **not** add a `version` field to any `plugin.json`. Version resolves from the git
  commit SHA, so every commit is picked up by `/plugin update`. Adding `version` would
  mean it must be hand-bumped on every change.
- Every new plugin under `plugins/` needs a matching entry in
  `.claude-plugin/marketplace.json`, or it cannot be installed.
- Reference paths inside plugins with `${CLAUDE_PLUGIN_ROOT}`, never absolute paths.
  This applies to hook commands and `.mcp.json` server commands.
- Hook event names are case-sensitive (`PostToolUse`, not `postToolUse`).
- Scripts must be `chmod +x` with a `#!/usr/bin/env bash` (or equivalent) shebang.

## Skill descriptions

A skill's frontmatter `description` is what Claude matches against to decide whether
to invoke it. Write it as trigger conditions ("Use when the user asks to ...") rather
than as a summary of what the skill contains. A vague description means the skill
never fires.

## Validate

```
claude plugin validate .                # marketplace + every plugin it lists
claude plugin validate ./plugins/<name> # one plugin
```

Run this before committing manifest changes. Expect one warning —
`version: No version specified` — on every plugin. That is the intended state, per the
rule above.

Do **not** use `--strict`. It promotes that warning to an error, so a correctly
configured plugin here always fails under it.

## Out of scope

A local model runtime lives elsewhere on this machine (multi-GB weights). It is a
runtime, not a toolbelt entry — do not vendor it here, and do not name the runtime
or its models in this repo. See "Publishing" below.

## Publishing

This repo is public. `scripts/publish-guard` blocks personal and private details —
real emails, `/home/<user>` paths, credentials, and any mention of the model
vendors and model IDs in use. It runs automatically on commit and on push via
`core.hooksPath = .githooks`.

Commits must be authored with the GitHub noreply address; the guard's history audit
rejects a real email anywhere in the log. Repo-local `user.email` is already set.

Rules live in `.publish-guard.json`. Exempt one line with a trailing
`publish-guard: allow <rule-id>` comment. Prefer rewording over exempting.
