---
name: orchestrator-loop
description: >
  Session orchestrator: inspect status + workflow skill, spawn workers, manage
  concurrency against hardware, never implement work yourself. Use when the user
  asks to orchestrate, run a session orchestrator, schedule a /loop coordinator,
  farm work to subagents, or runs /orchestrator-loop /loop-orchestrator.
---

# Orchestrator loop (session)

Reusable `/loop` schedule for the session orchestrator.

**Role:** inspect → decide → spawn/stop workers → update status → report.
Do **not** implement feature work yourself.

## Paste-ready (preferred)

Copy everything in the fenced block below into the chat (or run as a `/loop` request):

```text
/loop 10m You are the orchestrator for this session. Do not do anything yourself - inspect what has to be done, spawn workers, and end your turn.

Sources of truth:
- Workflow skill = process only (how work is done: steps, rules, done criteria, what to run). Keep it stateless — never write live status, progress, or run history into the skill.
- Status files = what is done, in progress, blocked, or unfinished.

Each cycle:
1. Read the workflow skill, the status files, and everything currently running (subagents, background commands, monitors, other scheduled work).
2. Check hardware utilization (CPU, RAM, GPU if present, disk/IO when relevant). Prefer simple local signals (e.g. load, free memory, GPU stats when available).
3. Compare process + status + live tasks + hardware. Look for drift, inefficiency, redundant or stale work, wrong approach, unfinished required steps, and poor hardware fit (idle capacity with ready useful work; CPU-bound jobs on a busy CPU while GPU sits idle; GPU-hungry jobs starved by CPU-only fluff; memory pressure; zombies/stragglers).
4. Process change → edit the workflow skill only (still no status in the skill).
   Status change → edit the status files only.
5. After any skill update (and whenever live work no longer matches process + status + sensible hardware use): re-evaluate running tasks. Keep what still fits; stop what is obsolete, redundant, stale, or wrong; if the approach should change, stop the old work and start the correct tasks per the skill.
6. Coordinate concurrency to use the machine hard — but only on work that still makes sense per the skill and status. Start additional useful tasks when capacity is free; avoid thrashing, duplicate jobs, and processes that no longer serve the workflow. Prefer the right device for the job (GPU for GPU-bound work, CPU for CPU-bound, don’t pin useless load on a contended resource).
7. From status + skill, if required work is unfinished or not running, start those tasks (when resources allow) and update the status files.
8. Short report: skill edits (if any), status-file edits, tasks kept/stopped/started, hardware snapshot + scheduling rationale, unfinished gaps closed, next focus.
```

## Defaults

| Field | Value |
|-------|--------|
| Interval | `10m` (change the leading number/unit if needed: `5m`, `1h`, …; min 60s) |
| Recurring | yes (`fire_immediately: true` when scheduled via `/loop`) |
| Auto-expire | scheduled jobs expire after 7 days |
| Cancel | `scheduler_delete <job_id>` (ID is printed when the job is created) |

## Project paths this orchestrator expects

Convention (map to the current project if paths differ):

| Role | Path |
|------|------|
| Workflow skill (process only) | project skill under the agent config skills dir, or the plugin skill in use |
| Live session status | agent config `status/session.md` (e.g. under the project agent home) |
| Other status bits | agent config `status/*` (progress lists, PIDs, unit trackers) |

If the project has no status dir yet, create `status/session.md` under the project agent home with goal / phase / in-progress / blocked / next — still never put that into the workflow skill.

## Notes

- Do **not** put live progress into the workflow skill — status files only.
- The orchestrator must **not** implement work itself; it only inspects, spawns workers, updates status, and reports.
- To change cadence only: re-run `/loop` with a different interval (and cancel the old job if still active).
- Pair with heavy process skills (e.g. `ui-viewport-qa`) that stay stateless while this loop owns coordination.
