# Autonomous deploy / QA cycle

Merged process for shipping UI with local matrix QA, honest review, then deploy.

## Sources (2026-07-17 merge)

| Source | What it contributed |
|--------|---------------------|
| **1st-rust-game** | Pipeline per matrix unit; CAPTURE vs REVIEW; **deep multi-role review (R1→R3)**; **`*.review.json` sidecars** (discovery/adversary fields); SIM-* + fairness; capture-bound suite timing; richest criteria |
| **Compre Barato Alagoas** | **Baseline vs full matrix**; missing runners must be **built**, not skipped; stack map pattern; layer-aware A1; `app-input-e2e` for non-game UIs; VPS/Flutter path examples |

## Layout

| Path | Role |
|------|------|
| `skills/ui-viewport-qa/` | Canonical process (plugin + docs) |
| `skills/game-input-e2e/` | Game input surface rules |
| `skills/app-input-e2e/` | Product/app input surface rules |
| `orchestrator/orchestrator-loop.md` | Session orchestrator `/loop` |
| `preferences files/qa_*.json` | Default reference matrix + criteria (from rust game) |
| `preferences files/from-*/` | Per-source snapshots |
| `status files/from-*/` | Live status examples (not process) |

## Install (Claude Code)

```
/plugin marketplace add /code/vinys-toolbelt
/plugin install ai-toolbelt@vinys-toolbelt
```

Skills live under `plugins/ai-toolbelt/skills/`.

## Using in a project

1. Copy or depend on the skills; map the **stack map** in `ui-viewport-qa` to your paths.
2. Keep project-local `qa_matrix.json` + `qa_success_criteria.json` (adapt from `preferences files/`).
3. Live status only under project agent `status/` — never in the skill.
4. A7 requires **review** (`*.review.json` + rollups), not suite exit 0 alone.

Media binaries (PNG/webm) are intentionally **not** versioned here.
