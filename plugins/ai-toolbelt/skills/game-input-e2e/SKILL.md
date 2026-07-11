---
name: game-input-e2e
description: >
  Enforce full keyboard, mouse, and touch support on every game screen. Use when
  changing UI, menus, input, touch controls, web boot overlay, HUD, or window
  scaling; when the user mentions controls, keyboard, mouse, touch, e2e, input,
  or playtesting; or runs /game-input-e2e. Pair with ui-viewport-qa for ship gate.
---

# Game input E2E

Battle-tested on a Bevy WASM game (keyboard + mouse + touch chrome). Paths
like `scripts/`, `src/ui_scale.rs`, and mode/difficulty names are the reference
layout — map them to the current project's equivalents when elsewhere.

## Hard rule

**Every screen must work with keyboard, mouse, AND touch.** No dead-ends where
only one input works. Treat regressions as blockers before claiming "done".

## Parallelism & tasks

Work **in parallel when independent** — multiple tool calls per turn, batched
screenshot evaluation, subagents for split research/critique, overlap long
builds with other useful work. Stay **efficient and reasonable**: don’t
parallelize dependent steps (build before input e2e; fix before retest), don’t
thrash the machine with duplicate full Chrome matrix runs, cap subagent fan-out.
Full rules live in **`ui-viewport-qa`** → *Parallelism & tasks*.

**Chain rule (required):** when any step finishes, re-read **`ui-viewport-qa`**
ship order and **start the next required step** (or spawn it). Do not stop after
e2e videos, matrix shots, or other intermediate artifacts alone.

## Target surfaces (visual + layout)

The game **must look and play well** on every format in
`scripts/qa_matrix.json` (phones, tablets, **budget laptops 1366×768**, scaled
laptops, 1080p, QHD, 4K). Logical classes:

| Class | Typical CSS size | Notes |
|-------|------------------|-------|
| **Desktop4k** | 2560×1440, 3840×2160 | Comfortable UI, not huge |
| **Desktop1080** | 1280×720 … 1920×1080, **1366×768**, 1536×864 | Keyboard/mouse; **no** touch chrome |
| **Tablet** V/H | 768×1024, 1024×768, 820×1180 | Stick + DASH chrome while playing |
| **Phone** V/H | 360×800, 390×844, 430×932 + landscapes | Stick + DASH; Game Boy / PSP chrome |

Classification lives in `src/ui_scale.rs` (`ViewportClass` / `classify_viewport`).
Use `UiScale.class` for layout branches — not one-off magic aspect ratios.
**Never** treat 1366×768 laptops as tablet.

Play bounds stay aspect-correct with equal margins (`PlayBounds`). Scale
**text/panels**, not gameplay unfairness.

## Screens checklist

| Screen | Keyboard | Mouse | Touch |
|--------|----------|-------|-------|
| Web boot overlay | Enter / Space dismisses + focuses canvas | Click dismisses | Tap dismisses |
| Main menu | Enter / Space / NumpadEnter → modes; Esc quits (native) | Click → modes | Tap → modes |
| Mode select | Up/Down or W/S (or K/J) mode; Left/Right difficulty; Enter/Space start; Esc back | Click top/bottom thirds mode; sides difficulty; center start | Same via tap |
| Playing | WASD/arrows move; **Space** dash; Esc menu | Hold/drag = **point-to-move**; **right-click** dash | **Virtual stick** moves; **DASH button** dashes (Game Boy / PSP chrome) |

**On-screen help text must match the form factor** (`UiScale.class` / `is_handheld()`):
- **Desktop / laptop (including 1366×768):** show WASD + arrows + Space. Do not lead with touch-only copy.
- **Phone / tablet:** show stick + DASH button. Do not lead with WASD/keyboard.
| Game over | Enter/Space again; Esc menu | Click again; left-edge back | Tap again; left edge / two-finger back |

### Playing controls (detail)

- **Handheld chrome** (phone/tablet while playing):
  - **Portrait = Game Boy**: play screen on top; bottom deck with stick (left) + DASH (right).
  - **Landscape = PSP**: stick on left grip, DASH on right grip, screen in the middle.
  - Stick is analog (direction + strength). DASH is a dedicated button — not second-finger.
  - Controls live **outside** the playfield so they never cover the character.
- **Desktop mouse**: point-to-move toward cursor; right-click dash.
- **Desktop keyboard**: WASD/arrows + Space dash.

## When you change input or UI

1. **Shared helpers** in `src/ui_scale.rs`:
   - `confirm_just_pressed` — Enter, NumpadEnter, Space
   - `back_just_pressed` — Escape, Backspace
   - `menu_up_just_pressed` / `menu_down_just_pressed`
   - `ViewportClass` / `classify_viewport` / `UiScale.class`
2. Menu systems must use those helpers **plus** `TouchControls` flags.
3. Web boot is HTML (`index.html`) — keyboard handlers must dismiss overlay and
   **focus the canvas** so Bevy receives keys.
4. **Text/UI scale** via `UiScale` + `ScaledText` / `ScaledPanel`. Design bases
   assume FixedVertical 1080 world units; small windows increase scale so text
   stays readable. Never hardcode unscaled fonts for HUD/menus.
5. Difficulty label slots must scale with `design` width (phone must not clip
   EASY…INSANE).

## Visual QA (required with this skill)

Full **screens × formats** matrix + exhaustive e2e is enforced by
**`ui-viewport-qa`** (see its **Ship order** section; that skill is **stateless
process only** — live matrix counts live in `scripts/qa_matrix.json`). It is
authoritative for when you may push.

**Short version of ship order:**

1. **Phase A (local, before any push):**  
   build → e2e run (videos) → **video review** (`video_critique.md`) →  
   viewport matrix PNGs → **PNG review** (`matrix_critique.md`) → pre-prod gate  
   Videos and matrix PNGs are **two separate reviews** (see ui-viewport-qa).  
2. **Phase B:** commit/push → watch Pages deploy  
3. **Phase C (if USB phone):** live `e2e_phone` 2×2 video — **not** a substitute for A  

Phone smoke against local `dist` is **debug only**. Do not push after that alone.

## Exhaustive E2E + video (required)

The game is simple — **do not** run a partial happy path. Every e2e must cover:

- **All screens:** boot, menu, mode select, playing, game over / exit
- **All modes:** Classic, Zen, Survival, Timed
- **All difficulties:** Easy, Normal, Hard, Insane
- **All primary inputs** for the path (keyboard / mouse / touch stick+DASH / swap)
- **≥20 seconds of play** with movement and dash

**Record video** of each scenario (`scripts/record.mjs` → webm; phone →
`adb screenrecord` mp4). Review recordings (or stills extracted from them) to
catch **transient** bugs — screenshots alone are not enough for e2e.

```bash
# PHASE A (required before push)
./scripts/web-build.sh
./scripts/web-serve-dist.sh
node scripts/e2e_inputs.mjs      # all formats + video
node scripts/viewport_shots.mjs  # all matrix cells
# then critiques + pre-prod gate — see ui-viewport-qa

# PHASE C only after push + Pages success, if phone connected
node scripts/e2e_phone.mjs
```

## Do not ship if

- **Phase A incomplete** (e2e/matrix/critiques/gate skipped) — including
  “works on phone via adb reverse” without full local suite
- Any screen only advances with mouse (keyboard stuck)
- Boot overlay ignores Enter/Space
- Canvas not focused after boot (keys do nothing)
- Panic on play (`SystemTime` / other WASM traps)
- HUD/menu text unreadable on phone portrait or phone landscape
- Layout broken on tablet portrait/landscape
- Touch cannot start a run, use the virtual stick, or press DASH
- Difficulty labels overflow the menu panel on narrow designs

## Fairness note

Play bounds stay aspect-correct and equal-margin; do **not** break fairness when
fixing UI.
