---
name: ui-viewport-qa
description: >
  Mandatory full screens×formats visual and input QA for web/game UIs. LOCAL
  suite (build → exhaustive e2e VIDEO → full viewport matrix → open every PNG →
  critiques → PRE-PROD gate) MUST pass before any push. Real USB phone testing
  is AFTER deploy (or optional debug against local dist — never a push
  substitute). Use on UI/UX, layout, menus, HUD, touch, scaling, boot overlay,
  screenshots, viewport QA, test matrix, or /ui-viewport-qa. Pair with
  game-input-e2e for control surface rules.
---

# UI / UX viewport QA (mandatory)

Battle-tested process for shipping a web/WASM game UI across phones, tablets,
and desktops. Script names (`e2e_inputs.mjs`, `viewport_shots.mjs`,
`qa_matrix.json`) and artifact paths are the reference layout — map them to the
current project when it differs. Pair with **`game-input-e2e`**.

## Skill is stateless

This file is **process only**. It must **not** record run state, progress, open
BADs, “we already regressed,” or the current matrix size. Do not append session
notes here. Live facts live in:

- `scripts/qa_matrix.json` — screens, formats, `expected_cells`
- `screenshots/viewports/matrix_critique.md` — per-cell **PNG** critiques
- `screenshots/web/e2e/video_critique.md` — per-recording **video** critiques
- `screenshots/web/e2e/results.json`, phone `results.json` — last run outputs
- Chat / PR / commit messages — what this session did

Agents: re-read JSON and artifacts each run; never assume prior run progress.

## Chain rule — always start the next task

**Every task ends by checking this skill for the next step in the current phase.**

1. Finish the current step fully (exit 0 / required artifacts / review done).
2. Look at [Ship order](#ship-order-read-this-first--do-not-reorder) (and the
   phase A/B/C checklists). Identify the **next** required step that is not yet
   satisfied by artifacts on disk.
3. **If a next step exists → start it immediately** (same turn or spawn a
   background/subagent task). Do **not** stop to “wait for the user” after a
   successful intermediate step (e.g. e2e videos done is not “done”).
4. **If this was the last step of the phase** → either enter the next phase
   (A7 PASS → B; B success + phone present → C) or report completion.
5. **If the step failed** → enter the fix loop; after patch, restart from the
   required phase A steps — then again apply this chain rule.

### Explicit anti-stop points (do not end the session here)

| Just finished | Must still do next |
|---------------|--------------------|
| A2 build | A3 serve → A4a e2e run |
| A4a e2e scripts exit 0 (videos on disk) | **A4b video review** (separate from PNG matrix) |
| A4b video review PASS | A5 viewport matrix PNGs |
| A5 matrix PNGs on disk | **A6 matrix PNG review** + critiques (not the videos) |
| A6 matrix critiques written | A7 pre-prod review |
| A7 PASS | Phase B push (if shipping) |
| B2 deploy success | B3 live smoke; if phone → Phase C |
| C phone fail | Phase A fix loop, not stop |

Stopping after “recordings exist” without **A4b video review**, or treating video
review as the matrix PNG review, is a process failure.

## Two different reviews (do not merge them)

| | **A4b — E2E video review** | **A6 — Matrix PNG review** |
|--|----------------------------|----------------------------|
| **What** | Playthrough / input recordings | Static layout screenshots |
| **Produced by** | `e2e_inputs.mjs` (+ phone `e2e_phone.mjs` in C) | `viewport_shots.mjs` |
| **Artifacts** | `screenshots/web/e2e/recordings/*.webm` (and `stills/` extracts) | `screenshots/viewports/{format}_{screen}.png` |
| **Unit of review** | One recording per format×input path (kb/mouse/touch) | One PNG per matrix **cell** (format × screen) |
| **Written output** | `screenshots/web/e2e/video_critique.md` | `screenshots/viewports/matrix_critique.md` |
| **Catches** | Input lag, wrong transitions, stick miss mid-play, flicker, dead controls | Layout, clipping, wrong chrome, form-factor copy, HUD placement |
| **Does not replace** | Matrix PNGs | E2E videos |

**Videos are not “included in” the PNG matrix review.** Both are required. You may
use extracted stills *from* a video for A4b, but those stills are not matrix cells
and do not go in `matrix_critique.md`.

## Ship order (read this first — do not reorder)

Three **phases**. Only phase A unlocks push. Phone work never replaces phase A.

```text
PHASE A — LOCAL ONLY (blocks push until every box is true)
  A1. cargo test / cargo check
  A2. ./scripts/web-build.sh          # wait for finish; fresh dist/
  A3. serve dist                      # http://127.0.0.1:8080/
  A4a. node scripts/e2e_inputs.mjs    # ALL formats → VIDEO files
  A4b. Review every e2e VIDEO (or stills) → video_critique.md
  A5.  node scripts/viewport_shots.mjs # ALL expected_cells → PNGs only
  A6.  Open EVERY matrix PNG → matrix_critique.md
  A7.  PRE-PROD REVIEW: matrix BAD none + video BAD none (or user-accepted)
  ── only after A7 PASS may you commit + push ──

PHASE B — PUSH + PAGES (only after A7 PASS)
  B1. git commit + git push
  B2. gh run watch pages.yml until success
  B3. smoke live URL HTTP 200

PHASE C — REAL PHONE (only if adb device present; after B2 success)
  C1. node scripts/e2e_phone.mjs against LIVE Pages (2×2 video)
  C2. Review phone videos + touch_inventory (phone video review, not matrix PNGs)
  C3. Any phone FAIL → back to PHASE A (patch), not “phone-only push”
```

### What is **not** enough to push

| Action | Unlocks push? |
|--------|----------------|
| `cargo test` / `cargo check` only | **No** |
| Web build only | **No** |
| One format / one screen smoke | **No** |
| Phone USB smoke against **local** `dist` (adb reverse) | **No** — useful debug only |
| Phone against **live** before phase A finished | **No** |
| E2E exit 0 without matrix critiques / pre-prod gate | **No** |
| Full phase A (A1–A7) PASS | **Yes** → then phase B |

Phone against local `dist` is **debug only**. It never unlocks push. Only phase A
does.

Pairs with **`game-input-e2e`**. Matrix source of truth: **`scripts/qa_matrix.json`**
(read `expected_cells` / formats from that file — do not hardcode counts in
memory or treat this skill as a live status log).

---

## Phase A checklist (required before push)

You are not allowed to `git push` until **all** of these are true:

1. **Build finished** — `./scripts/web-build.sh` exit 0; wait as long as needed.
2. **A4a — E2E run** — `node scripts/e2e_inputs.mjs` exit 0 on **every** format.
   Exhaustive surface (all screens/modes/difficulties/inputs, ≥20s play).
   Videos under `screenshots/web/e2e/recordings/`. Handheld = device emulation.
3. **A4b — E2E video review** — open/review each recording (or extracted stills)
   and write `screenshots/web/e2e/video_critique.md` (see [E2E video review](#a4b--e2e-video-review-required)).
   **Not** the same as matrix PNG critique.
4. **A5 — Matrix PNGs** — one PNG per `expected_cells` under `screenshots/viewports/`.
5. **A6 — Matrix PNG review** — open every matrix PNG; write
   `matrix_critique.md` (see [Matrix PNG review](#a6--matrix-png-review-required)).
6. **A7 — PRE-PROD REVIEW PASS** — no unaccepted BADs in **either** critique file.

Partial matrices (e.g. only menu@1080p) do **not** count.

---

## Phase B / C (after push only)

7. **Commit + push** — only after phase A.
8. **Deploy watcher** — Pages workflow success (build + deploy jobs).
9. **Real USB phone (if connected)** — `e2e_phone.mjs` on **LIVE** URL after deploy.
   Failures send you back to phase A. Optional: during phase A you may smoke the
   phone against **local** dist for faster iteration; that never skips A4–A7.

---

## Parallelism & tasks (required working style)

Parallelize **inside** a phase when independent. Do **not** parallelize away the
phase barriers (especially **do not push while phase A is incomplete**).

### Safe to parallelize

| Tracks | Why |
|--------|-----|
| Multiple tool calls (reads, greps, batch PNG opens) | No dependency |
| `cargo test` while reading UI code | Independent |
| Critique batches of matrix PNGs | Embarrassingly parallel |
| After serve: investigate code while e2e runs | Overlap wait |
| After **push**: `gh run watch` while drafting the report | Phase B idle time |
| Subagents for split critique / code search | Bounded fan-out |

### Must stay serial

- **Build → then** e2e / viewport against that `dist/`
- **Phase A complete → then** push → **then** deploy watch → **then** live phone
- **Patch → rebuild → full phase A retest** in the fix loop

### Phone + local (do not confuse)

| Goal | How | Counts as phase A? |
|------|-----|---------------------|
| Fast touch/debug loop | Phone → local `dist` via `adb reverse` | **No** — debug only |
| Ship proof on PC | Emulated devices in `e2e_inputs` / `viewport_shots` (`phone_rodin*`) | **Yes** (part of A4/A5) |
| Ship proof on real device | Phone → **LIVE** after deploy (`e2e_phone`) | Phase **C** only |

### Anti-patterns (explicit)

- **Stopping after an intermediate step** (e.g. e2e videos written) without
  checking ship order and spawning the next task
- **Pushing after phone smoke without full local e2e + matrix + critiques**
- Treating adb reverse local test as “prod verified”
- Waiting on trunk with nothing else in flight when critiques could run
- Parallel full `viewport_shots` + `e2e_inputs` if the machine thrashs (sequence heavy Chrome)
- Parallel pushes / conflicting fix branches without a merge plan

---

## A4b — E2E video review (required)

**Separate from matrix PNGs.** Runs after A4a produces recordings; before or after
A5 is fine as long as A4b completes before A7 — prefer **before A5** so input bugs
are caught early.

### Where

[`screenshots/web/e2e/video_critique.md`](../../screenshots/web/e2e/video_critique.md)

### Format (one line per recording)

```text
VIDEO {format_id}_{keyboard|mouse|touch}: GOOD: <what works in the playthrough> | BAD: <transient/input issue or "none">
```

Examples:

```text
VIDEO phone_rodin_chrome_touch: GOOD: modes cycle, stick moves player, dash cooldown shows | BAD: none
VIDEO phone_landscape_touch: GOOD: PSP grips, play 20s | BAD: status "Dash 0.4s" clips bottom field border mid-run
VIDEO laptop_hd_mouse: GOOD: no stick chrome, point-to-move + right-dash | BAD: none
```

### How to review

- Prefer opening extracted stills under `screenshots/web/e2e/stills/{recording}/`
  (several frames across the timeline) **or** sample the `.webm` if needed.
- Listing `recordings/` is **not** review.
- Cover **every** recording named in `results.json` / on disk for this run.

### Video checklist (A4b)

- [ ] Boot dismiss works; canvas receives input after
- [ ] Mode select: modes and difficulties actually change (not stuck)
- [ ] Start enters play; ≥20s of play is visible (not frozen black)
- [ ] Move works (keys / mouse drag / stick); dash works (Space / right-click / DASH)
- [ ] Handheld: stick+DASH respond; desktop: **no** virtual stick chrome
- [ ] No panic / blank / stuck boot mid-run
- [ ] No obvious flicker, wrong screen flashes, or control dead-zones
- [ ] Status/HUD during play not unusable (clipping mid-run is a BAD)

`BAD` not `none` → ship blocker → fix loop (re-run A4a+A4b at minimum).

---

## A6 — Matrix PNG review (required)

**Layout-only static cells.** Not a substitute for video review.

### Where

[`screenshots/viewports/matrix_critique.md`](../../screenshots/viewports/matrix_critique.md)

### Format (one line per matrix cell)

```text
CRITIQUE {format_id}_{shot_suffix}: GOOD: <what works> | BAD: <what fails or is weak, or "none">
```

Examples:

```text
CRITIQUE phone_landscape_04_playing: GOOD: stick+DASH outside field, zoom readable | BAD: status "Dash READY" tight on bottom border
CRITIQUE laptop_hd_02_menu: GOOD: keyboard control copy, no touch chrome | BAD: none
```

### Rules

- **GOOD** and **BAD** both required (use `BAD: none` only when clean).
- Open **each** matrix PNG with the image tool (not directory listing).
- User may accept residual BADs in writing; document that at ship time.

### Matrix PNG checklist (A6)

**All screens**
- [ ] No clipped text; panels inside the canvas
- [ ] Readable type (esp. phone landscape / laptop_hd)
- [ ] No huge empty layout or overlapping controls
- [ ] No panics / blank / frozen boot frame
- [ ] Form-factor: **desktop** = no stick/DASH chrome; **handheld** = chrome while playing

**Boot** — progress/CTA sensible; short landscape OK  
**Menu** — desktop: WASD copy; phone/tablet: stick+DASH copy  
**Mode select** — modes/difficulty readable; no overflow  
**Playing** — HUD legible; handheld Game Boy/PSP chrome outside field; desktops full field no stick  
**Game over** — stats + hints match form factor  

`BAD` not `none` → ship blocker → fix loop.

---

## Pre-prod critique review (required before push)

**Gate name:** critique review. Hard stop between “tests green” and `git push`.

### What you must do

1. Open **both** `screenshots/web/e2e/video_critique.md` **and**
   `screenshots/viewports/matrix_critique.md`.
2. Collect every line in **either** file where `BAD:` is not exactly `none`.
3. **If both lists are empty** → review **PASS**. Proceed to commit/push.
4. **If any BAD remains** → review **FAIL**. Do **not** push. Fix loop:

```text
START OF FIX LOOP
  1. Patch code for every open BAD (video and/or matrix).
  2. cargo test / cargo check
  3. ./scripts/web-build.sh          # wait for finish
  4. ensure dist served
  5. node scripts/e2e_inputs.mjs     # A4a
  6. A4b rewrite video_critique.md
  7. node scripts/viewport_shots.mjs # A5
  8. A6 open PNGs + rewrite matrix_critique.md
  9. Return to this pre-prod review gate
END LOOP — until every BAD is "none" (or user-accepted in writing)
```

### Explicit prohibitions

- **Do not** push “and fix later.”
- **Do not** push after phone-only or local-dist phone smoke without phase A.
- **Do not** treat e2e exit 0 alone as ship-ready if **video or matrix** critiques have BADs.
- **Do not** skip A4b because A6 PNG review “looks fine.”
- **Do not** skip A6 because “videos already cover it.”
- **Do not** delete BAD lines to silence the gate.
- **Do not** leave headless Chrome/Puppeteer orphans.

### Review checklist output (put in final report)

```text
PRE-PROD REVIEW: PASS | FAIL
open_bads_video: N
open_bads_matrix: N
(if FAIL) next_action: patch + full retest from suite start
(if PASS) proceeding_to: commit / push / deploy watch
```

---

## Master test matrix (main reference)

**File:** `scripts/qa_matrix.json`

Whenever you **add/remove a screen or format**, you **must**:

1. Update `scripts/qa_matrix.json` (`screens[]`, `formats[]`, `expected_cells`,
   and `selection_rationale` for new sizes)
2. Ensure `scripts/viewport_shots.mjs` and `scripts/e2e_inputs.mjs` still load
   the matrix (they import the JSON — do not hardcode stale lists in scripts)
3. Re-run full phase A and inspect **all** cells (including new ones)

### How to read the matrix (stateless)

**Always open `scripts/qa_matrix.json`.** Do not rely on remembered counts or a
pasted table in this skill.

- **Screens** — `screens[]` (shot suffixes, labels)
- **Formats** — `formats[]` (CSS width/height, dpr, touch, expected_class)
- **Cell count** — `expected_cells` (must equal `screens.length * formats.length`)
- **Why a size exists** — `selection_rationale`

Sizes are **CSS viewports** (logical px), not physical panel pixels. Classification:
`src/ui_scale.rs` → `classify_viewport`.

**Lab formats** (if present in JSON, e.g. `phone_rodin*`): PC device-emulation
repro of a real Chrome-tab CSS size for iteration without waiting on Pages.

Artifact: `screenshots/viewports/{format_id}_{shot_suffix}.png`

`viewport_shots.mjs` writes `screenshots/viewports/matrix_results.json` and
**exits non-zero** if any expected file is missing/empty.

**Game over capture:** shots use `http://127.0.0.1:8080/?qa_matrix=1` so the game
forces Game Over after a short play (`world::qa_matrix_force_gameover`). Normal
players without that query are unaffected.

### Why resolution criteria exist (durable rules)

1. **CSS viewport, not panel pixels** — browsers report logical size (DPR-scaled).
2. **Market share / common devices** — phones, tablets, desktops, budget laptops.
3. **Form-factor boundaries** — e.g. 1024×768 tablet vs 1366×768 laptop.
4. **DPI / OS scaling** — e.g. 1080p at 125% Windows scale.
5. **Orientation** — portrait + landscape for handhelds.
6. **High end** — QHD + 4K so UI does not become huge or sparse.

---

## Builds — wait as long as needed

WASM / Trunk release builds can take **many minutes**. Rules:

1. Start `./scripts/web-build.sh` (local default: wasm-fast). Wait for finish.
   Optional ship-like: `./scripts/web-build.sh --release`. Use a **high or
   unlimited** timeout (e.g. 15–30+ minutes).
2. If the tool backgrounds the process, **poll until exit** — do not abandon.
3. Only after **exit code 0** and a fresh `dist/` may you serve and test.
4. Do **not** run matrix/e2e against a stale `dist/` after code changes.
5. `cargo check` / `cargo test` first is fine for fast Rust errors; it does **not**
   replace the web build for screenshot QA.

---

## Full suite commands

### Phase A — local (required before push)

```bash
# A1
cargo test -q && cargo check

# A2 — WAIT for completion (can be long)
./scripts/web-build.sh

# A3
./scripts/web-serve-dist.sh   # http://127.0.0.1:8080/

# A4a — exhaustive E2E + VIDEO, ALL formats (device emulation on handheld)
node scripts/e2e_inputs.mjs
# → screenshots/web/e2e/recordings/*.webm + stills/ + results.json

# A4b — REVIEW VIDEOS (separate critique file; not matrix PNGs)
# open stills or webm → write screenshots/web/e2e/video_critique.md

# A5 — layout matrix PNGs only, ALL expected_cells
node scripts/viewport_shots.mjs
# → screenshots/viewports/{format}_{screen}.png + matrix_results.json

# A6 — REVIEW MATRIX PNGs → matrix_critique.md
# A7 — PRE-PROD: video_critique + matrix_critique both clean
```

Optional during A (debug only, **not** a ship gate): point the USB phone at local
dist (`adb reverse tcp:8080 tcp:8080`) to reproduce touch bugs faster. Still must
finish A4a–A7 before push.

### Phase B — after A7 PASS

```bash
git add … && git commit && git push -u origin HEAD
gh run list --workflow=pages.yml --branch main -L 3
gh run watch <run-id> --exit-status
```

### Phase C — after Pages success, if phone connected

```bash
node scripts/e2e_phone.mjs
# LIVE URL, 2×2 video: screenshots/web/phone/recordings/*.mp4 + touch_inventory.md
```

### Exhaustive E2E surface (required — game is simple)

Every e2e path (keyboard / mouse / touch / phone cell) **must** include:

| Surface | Must exercise |
|---------|----------------|
| Boot | Dismiss CTA |
| Menu | Confirm; **swap stick/DASH** (handheld) |
| Mode select | **All 4 modes** (Classic, Zen, Survival, Timed); **all 4 difficulties** (Easy→Insane); START; back |
| Playing | Move (keys / mouse drag / stick); dash (Space / right-click / DASH); **≥20 seconds** continuous play |
| Game over / exit | Confirm again and/or Esc/back when reachable |

### Fail / fix loop

```text
PHASE A FAIL or PHASE C phone FAIL:
  patch → rebuild → A4a e2e → A4b video review → A5 matrix PNGs
  → A6 matrix PNG review → A7 PRE-PROD (both critique files)
  ── only then push (B) ──
  ── then if phone: C live e2e_phone ──
```

If review fails, go back to **patch** — not to push. Do not ship partial green.
Fast PC repro: **`phone_rodin_chrome`** device-emulation format. Confirm on real
phone only after a proper phase B deploy (or local reverse for debug).

---

## Real phone (ADB + Chrome CDP) — phase C

**When (ship path):** authorized `adb devices` **and** phase B Pages deploy for
the commit under test has succeeded. Target = **LIVE** Pages URL.

**When (debug only):** phone against local `dist` via `adb reverse` **during**
phase A. Useful for touch mapping. **Does not** authorize push.

Skip cleanly if no device (unless `PHONE_REQUIRE=1`).

**Why phase C exists:** PC emulation misses browser chrome, gesture bar, real
DPR, and some touch bugs. It **adds** confidence after local suite + deploy; it
does **not** replace phase A.

### 2×2 matrix (required on device)

Force **both** orientations and **both** Chrome presentations. Do not only test
the phone’s current pose.

| | **browsing** (normal Chrome: address bar + tabs) | **fullscreen** (`requestFullscreen`) |
|--|--------------------------------------------------|--------------------------------------|
| **portrait** | `portrait_browsing` | `portrait_fullscreen` |
| **landscape** | `landscape_browsing` | `landscape_fullscreen` |

- **Orientation:** `adb` disables auto-rotate and sets `user_rotation` (0 portrait /
  1 landscape). Restored after the run.
- **Fullscreen vs browsing:** browsing = normal Chrome chrome; fullscreen =
  `document.documentElement.requestFullscreen()` after load (re-try after a tap
  if the browser requires a gesture).
- Each cell: **adb screenrecord** of whole chain on LIVE + calibrated **adb taps**
  (real OS touches). CDP only for navigate/evaluate (Android Chrome CDP touch is
  unreliable).
- Artifacts: `screenshots/web/phone/recordings/{cell}.mp4` + `touch_inventory.md`.

Optional filter: `PHONE_CELLS=portrait_browsing,landscape_fullscreen node scripts/e2e_phone.mjs`

### Rules

1. **No Puppeteer on the device path.** CDP (`scripts/cdp.mjs`) for DevTools only;
   **input via `adb shell input`** (calibrated CSS→physical).
2. **Video, not stills, is primary** — `adb shell screenrecord` for each 2×2 cell
   for the full exhaustive scenario (catch transients).
3. **LIVE URL** default: `https://intrusting-games.github.io/rusty-dasher/`.
4. **Exhaustive per cell** — all modes, all difficulties, swap, START, ≥20s play
   stick+dash; fatty-finger notes in inventory.
5. **All four cells** when a phone is connected (unless `PHONE_CELLS` / user skip).

### Touch inventory (must cover, each cell)

| Screen | Controls |
|--------|----------|
| Boot | Dismiss CTA |
| Menu | Confirm; swap stick/DASH |
| Mode select | **All 4 modes**; **all 4 difficulties**; START |
| Playing | Stick drag; DASH; **≥20s play** |
| Game over | Again / two-finger menu when reached |

**Fatty-finger criteria:** hit diameter ≥ **48 CSS px**; stick↔dash gap ≥ **12 CSS px**.

### Commands

```bash
adb devices -l
node scripts/e2e_phone.mjs
# Artifacts: screenshots/web/phone/recordings/*.mp4, touch_inventory.md, results.json
```

Review each cell’s **video**; treat inventory FAILs as ship blockers.

---

## Device emulation (PC phone formats)

Handheld matrix cells (`touch: true`) **must** use
`scripts/device_emulation.mjs` (`page.emulate` mobile UA + metrics + touch), and
drive the **full user chain** (goto site → boot → menu → mode → play → game over).
Do **not** treat “setViewport width/height only” as phone QA.

---

## Phase B: push (only after A7 PASS)

Only after **PRE-PROD REVIEW: PASS** (zero unaccepted BADs) **and** full A4–A6:

1. Commit source, matrix scripts, screenshots, and **`matrix_critique.md`**.
2. `git push -u origin HEAD` (usually `main`).
3. Pages CI (`.github/workflows/pages.yml`) rebuilds
   `https://intrusting-games.github.io/rusty-dasher/`.

**Do not push** if: phase A incomplete, matrix incomplete, critiques missing,
e2e failed/missing videos, screenshots not inspected, **or** any critique BAD
without user acceptance, **or** you only validated on a phone against local dist.

---

## Phase B continued: deploy watcher

```bash
gh run list --workflow=pages.yml --branch main -L 3
gh run watch <run-id> --exit-status
```

Both **trunk build** and **deploy** must succeed. Smoke live URL HTTP 200.
If CI fails: fix → full **phase A** again → push → watch.

**Then phase C** if phone connected: live `e2e_phone.mjs`; inventory FAIL →
phase A, not a silent ship.

---

## Do not ship if

- **Phase A incomplete**
- Build skipped or still running when tests “passed”
- Fewer than **expected_cells** matrix screenshots
- Any matrix cell not visually inspected **or** missing a matrix CRITIQUE line
- E2E not run on every format **or** A4b video review skipped / missing `video_critique.md`
- E2E skipped full surface (not all modes/difficulties/controls or &lt;20s play)
- E2E has no video recordings
- Phone/tablet tested only as resized desktop windows (no device emulation)
- **Pushed after phone smoke / adb reverse only** (no full local matrix + e2e)
- Phone connected for phase C but real-device step skipped without reason / user skip
- Phone touch inventory has unaccepted FAILs (fix locally, redeploy)
- **Pre-prod critique review not run, or any unaccepted `BAD` still open**
- Wrong control copy for PC/laptop vs phone/tablet
- Laptop sizes (esp. 1366×768) classified or rendered as handheld
- Never pushed after true phase A PASS, or push without deploy success
- Pushed “knowing” about open BADs “to fix later”

---

## Reporting when done

1. **Phase A:** build + e2e summary + **A4b** path to `video_critique.md` +
   matrix_results + **A6** path to `matrix_critique.md`
2. Confirmation that **all** e2e recordings **and** all matrix PNGs were reviewed
3. **PRE-PROD REVIEW: PASS** (`open_bads_video` + `open_bads_matrix`); residual only if user-accepted
4. **Phase B:** commit hash + push + Pages run id/URL + **success** + live URL
5. **Phase C (if phone):** inventory + phone video path — or “no device / skipped”

## Related

- Matrix JSON: `scripts/qa_matrix.json`
- Device emulation: `scripts/device_emulation.mjs`
- Recording: `scripts/record.mjs` (CDP screencast → ffmpeg)
- Shots (layout matrix PNGs): `scripts/viewport_shots.mjs`
- E2E run: `scripts/e2e_inputs.mjs` → `screenshots/web/e2e/recordings/`
- E2E video critique: `screenshots/web/e2e/video_critique.md`
- Matrix PNG critique: `screenshots/viewports/matrix_critique.md`
- Real phone (2×2 video): `scripts/e2e_phone.mjs` → `screenshots/web/phone/recordings/`
- Phone inventory: `screenshots/web/phone/touch_inventory.md`
- Input rules: sibling skill `game-input-e2e`
- Scale: `src/ui_scale.rs` (`ViewportClass` / `classify_viewport`)
- Pages: `.github/workflows/pages.yml`
