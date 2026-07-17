---
name: game-input-e2e
description: >
  Enforce full keyboard, mouse, and touch support on every game screen for this
  Bevy project. Use when changing UI, menus, input, touch controls, web boot
  overlay, HUD, or window scaling; when the user mentions controls, keyboard,
  mouse, touch, e2e, input, or playtesting; or runs /game-input-e2e.
---

# Game input E2E (RustyDasher)

## Hard rule

**Every screen must work with keyboard, mouse, AND touch.** No dead-ends where
only one input works. Treat regressions as blockers before claiming "done".

## Parallelism & tasks

**Authority:** **`ui-viewport-qa`** → section *Parallel vs serial* (decision
criteria 1–9, task map, “how to apply when editing”, defaults, anti-patterns).

**Short form — parallel OK:**
- Matrix units (format id + CSS resolution) inside unified capture (`CONCURRENCY=<N>`, tuned by orchestrator/session)
- **Pipeline:** as soon as **one matrix unit** finishes capture → **A4b ∥ A6 for
  that unit**, while other units still capture/review (do **not** batch all
  reviews only after the full suite)
- Critique batches; cargo/build **wait** overlapping read-only work
- Local adb-reverse phone **only as debug**, never as ship proof

**Short form — must stay serial:**
- Build → serve → start A4 capture; **per unit** capture-done → that unit’s
  reviews; **all** units reviewed → A7 → push → deploy → live phone
- Patch → rebuild → full retest (no capture against stale/broken `dist/`)
- Screen order + quality holds **inside** one matrix unit’s journey

**Never:** two full game walks (e2e then full `viewport_shots`); parallel across
ship gates; second page load just for screenshots (use A4 quality holds);
**wait for every unit before reviewing any finished unit**.

**When editing skills/scripts:** classify each new edge with criteria 1–9 and
update the **task map** in `ui-viewport-qa` — do not leave parallelism only in chat.

**Chain rule (required):** when any step finishes, re-read **`ui-viewport-qa`**
ship order and **start the next required step** (or spawn it). When a matrix
unit’s video + PNGs land, **immediately** start that unit’s reviews. Do not
stop after intermediate artifacts alone. **Trust the gates:** if A7 is a true
PASS (critiques clean, matrix complete), proceed to Phase B (commit/push/deploy
watch) without waiting for a human “go ahead.” Only pause when a gate fails or
the skill cannot decide (e.g. user-accepted residual BADs).

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
process only** — live matrix counts live in `scripts/qa_matrix.json`;
**PASS/FAIL checks** live in `scripts/qa_success_criteria.json`). Process skill
+ criteria file together are authoritative for when you may push.

### CAPTURE vs REVIEW (same as ui-viewport-qa)

- **e2e / emulator / phone script `PASS` or `CAPTURE_OK` / exit 0** = automation
  wrote artifacts or steps ran — **not** “looks good.”
- **Ship requires A4b + A6** (open videos/PNGs, write critiques) then **A7**
  (no open BADs under criteria). Never treat suite N/N as A7.

**Short version of ship order:**

1. **Phase A (local, before any push):**  
   build → **unified** e2e per **matrix unit** (video **and** quality-hold matrix
   PNGs) with **pipeline review** — as each unit finishes, **immediately** A4b
   video + A6 PNG review while others still run → pre-prod when all units done.  
   - **Desktop / laptop:** Chrome + Puppeteer (`e2e_inputs`).  
   - **Handheld / touch:** **Android emulator** + full-display **`adb shell
     screenrecord`** + OS-level touches via **`adb shell input`** (not Chrome
     `page.emulate` alone; see `ui-viewport-qa`).  
2. **Phase B:** commit/push → watch Pages deploy  
3. **Phase C (if physical USB phone):** live `e2e_phone` on Pages — **not** a
   substitute for A (and not a substitute for Phase A emulator handhelds)

Do **not** double-walk the game (full e2e then full serial viewport). Do **not**
defer all review until every unit finished. Physical-phone adb reverse against
local `dist` is **debug only**.

## Exhaustive E2E + video (required)

The game is simple — **do not** run a partial happy path. Every e2e must cover:

- **All screens:** boot, menu, mode select, playing, game over / exit
- **All modes:** Classic, Zen, Survival, Timed
- **All difficulties:** Easy, Normal, Hard, Insane
- **All primary inputs** for the path (keyboard / mouse / touch stick+DASH / swap)
- **≥20 seconds of play** with movement and dash

**Record video** of each scenario:
- Desktop: `scripts/record.mjs` (CDP screencast → webm)
- Handheld Phase A: **`adb shell screenrecord`** on an **Android emulator** (full display)
- Phase C physical phone: **`adb shell screenrecord`** on the handset

Review recordings (or stills) for **transient** bugs — screenshots alone are not
enough for e2e. Handheld **play input** on Android paths must use **`adb shell
input`**, not CDP/Puppeteer touch alone.

```bash
# PHASE A (required before push)
./scripts/web-build.sh && ./scripts/web-serve-dist.sh
# Desktop formats:
CAPTURE_MATRIX=1 CONCURRENCY=<N> node scripts/e2e_inputs.mjs
# Handheld formats (required): boot AVD → adb reverse → screenrecord + adb shell input
# As EACH matrix unit finishes: A4b + A6 for that unit (pipeline) — see ui-viewport-qa
VERIFY_ONLY=1 node scripts/viewport_shots.mjs
# A7 pre-prod when every unit reviewed

# PHASE C only after push + Pages success, if physical phone connected
node scripts/e2e_phone.mjs
```

## Do not ship if

- Any **blocker** (or unaccepted **major**) criterion in
  `scripts/qa_success_criteria.json` still fails on this-run artifacts
- **Phase A incomplete** (e2e/matrix/critiques/gate skipped) — including
  handhelds only via Chrome device-emulation, or “works on USB phone via adb
  reverse” without full local suite (desktop + **emulator** handhelds)
- Any screen only advances with mouse (keyboard stuck)
- Boot overlay ignores Enter/Space
- Canvas not focused after boot (keys do nothing)
- Panic on play (`SystemTime` / other WASM traps)
- HUD/menu text unreadable on phone portrait or phone landscape
- Layout broken on tablet portrait/landscape
- Touch cannot start a run, use the virtual stick, or press DASH
- Difficulty labels overflow the menu panel on narrow designs
- Nested playfield borders, side dim slabs over the field, entities outside the
  blue rect, ghost playfield under menus, START covering help, or glyph tofu
  (`·` boxes) — see `V-PLAY-*`, `V-GHOST-FIELD`, `V-MODE-START-CLEAR`,
  `V-GLYPH-TOFU` in the criteria file
- Phase C inventory with `events=0` / open_bads > 0 left unfixed (`I-EVENTS-NONZERO`)

## Fairness note

Play bounds stay aspect-correct and equal-margin; do **not** break fairness when
fixing UI.
