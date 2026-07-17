---
name: ui-viewport-qa
description: >
  Mandatory full screens×formats visual and input QA for this Bevy game. LOCAL
  suite: build → unified e2e (VIDEO + matrix PNGs per matrix unit) with
  pipeline review → PRE-PROD gate, then push. Phase A handhelds use Android
  emulators (full-display adb screenrecord + adb shell input touches). Desktop
  stays Chrome/Puppeteer. Physical USB phone is Phase C after Pages deploy.
  Use on UI/UX, layout, menus, HUD, touch, scaling, boot overlay, screenshots,
  viewport QA, or /ui-viewport-qa.
---

# UI / UX viewport QA (mandatory)

## Skill is stateless

This file is **process only**. It must **not** record run state, progress, open
BADs, “we already regressed,” or the current matrix size. Do not append session
notes here. Live facts live in:

- `status/session.md` — session goal, phase, in-progress / blocked / next
  (orchestrator + agents update this; never put this content in the skill)
- `scripts/qa_matrix.json` — screens, formats, `expected_cells`
- **`scripts/qa_success_criteria.json`** — **PASS/FAIL authority** (visual,
  input, video, artifact, phase gates, per-screen checklists, criterion IDs).
  Open **before** writing any CRITIQUE/VIDEO line. Skills describe process;
  this file decides what counts as GOOD vs BAD.
- `screenshots/viewports/matrix_critique.md` — per-cell **PNG** critiques (rollup)
- `screenshots/web/e2e/video_critique.md` — per-recording **video** critiques (rollup)
- **Per-artifact review sidecars** — `*.review.json` next to each PNG/video
  (see [Per-artifact review files](#per-artifact-review-files-required))
- `screenshots/web/e2e/results.json`, phone `results.json` — last run **capture** outputs
- Chat / PR / commit messages — what this session did

Agents: re-read JSON (matrix **and** success criteria), status, and artifacts
each run; never assume prior run progress.


## Project stack map (fill in per repo)

Process is stack-agnostic. Every project keeps a short map (here or in status)
so agents do not invent paths.

| Cycle concept | Typical locations (pick one per project) |
|---------------|------------------------------------------|
| Product UI | game `src/` + `dist/`; Flutter `frontend/`; SPA `web/` |
| Matrix JSON | `scripts/qa_matrix.json` or `e2e/qa_matrix.json` |
| **PASS/FAIL criteria** | `scripts/qa_success_criteria.json` or `e2e/qa_success_criteria.json` |
| Desktop e2e | `scripts/e2e_inputs.mjs`, `e2e/full.js`, etc. |
| Live post-deploy | project smoke (`e2e_phone`, `npm run live`, …) |
| Unit tests (A1) | `cargo test` / `pytest` / `flutter test` — **layer-aware** |
| Web build (A2) | `./scripts/web-build.sh` / `flutter build web` / `npm run build` |
| Deploy (Phase B) | Pages `pages.yml`, VPS `deploy.yml`, etc. |
| Input sibling skill | `game-input-e2e` (games) or `app-input-e2e` (apps) |

**Rule:** map commands in this skill to the current repo. Do not hardcode a
foreign stack. Do not drop product-UI unit tests from A1 when that layer moves.

## Baseline vs full matrix (do not conflate) — and do not skip runners

| Layer | What it is | Gate? |
|-------|------------|--------|
| **Baseline capture** | Project’s primary local suite + post-deploy live smoke | **Always required** before/after ship |
| **Criteria critiques** | Open success-criteria JSON; deep R1→R3 + **`*.review.json`** on **this-run** artifacts | **Always required** (CAPTURE_OK ≠ review) |
| **A1 units** | Layer-aware tests for changed code | **Yes** for changed layers |
| **A2 build** | Fresh shippable web/app artifact | **Yes** when UI ships |
| **Priority / debug subset** | Fast path only | **Not a substitute** for full matrix residual close |
| **Full matrix** | Every `screens[]` × `formats[]` + VIDEO + deep A4b∥A6 + sidecars | **Required target** for residual close / full visual QA |

### Full matrix + runners (common failure mode)

1. **`qa_matrix.json` → `expected_cells` is the real matrix**, not a wishlist.
2. **Missing or incomplete matrix runners are not a free pass.** Agents **must
   install, implement, or finish those runners**, then run the full matrix and
   review under criteria. Do **not** close residual by documenting “no runners.”
3. **“Until runners land” means: make them land.** Not passive wait or A7 without pixels.
4. **Priority subset is debug/speed only** — never label it “full matrix done.”
5. **Baseline still always required.** Full matrix never replaces baseline + live
   + deep criteria reviews.
6. **Do not** claim A7 on suite exit 0 alone. **Do not** invent CRITIQUE lines
   without artifacts.
7. **Do not** remove product-UI tests from A1/A2 for the stack that owns the UI.


## Capture is the bottleneck — analysis is deep, parallel, and bounded

**Generating** screenshots, e2e videos, emulator `screenrecord`, and phone 2×2
recordings dominates real cost (build, Chrome/AVD, encode, device). Once
artifacts exist, **review must not be cheap**: spend agent turns, open many
frames, run discovery + checklist + adversary. Do **not** skim to save tokens
or force a fast green A7.

### Parallel analysis (keep capture as the limiter)

Review **must** run **in parallel with capture and with other reviews** (pipeline
per matrix unit; multi-agent R1/R2/R3). Because analysis **fans out**:

- A **single** review worker (or one unit’s full R1→R3 chain) **may take longer
  wall-clock than a single capture unit** — that is normal and allowed.
- Capture often runs with modest `CONCURRENCY` (sometimes effectively serial on
  one AVD); analysis can still stack many readers. **Suite wall-clock should
  still be dominated by capture + build**, not by a long serial review queue.
- Orchestrators: **prefer more concurrent review workers** over one deep serial
  reviewer when hardware allows, so thoroughness does not invent a new
  end-to-end bottleneck.

| Layer | Cost reality | Rule |
|-------|--------------|------|
| Capture (A4 / phone) | Highest wall-clock; hard to redo | Produce once per pass; pipeline review as units land |
| Analysis (A4b / A6 / C2 / A7) | Agent-time, parallelizable | **Deep and thorough**; fan out so **suite** time stays capture-bound |
| Fix + rebuild | High again | Only after honest BADs; recapture affected units |

### Depth vs suite ceiling (grain of salt)

**Depth first:** never drop geometry scans, multi-frame video, OPEN-*, or R3
adversary just to “go faster.”

**Suite ceiling (soft, wall-clock):** the **parallel analysis phase** for a pass
should not **systematically** outlast the **capture phase** of that same pass
as the thing that holds the ship. In other words:

- Aim: *while* the capture pool is still running, analysis of finished units
  keeps up; after the last capture exits, only a **short trailing review tail**
  remains (finishing in-flight R2/R3, A7 rollup) — not a second full-duration
  serial “now we finally look at everything.”
- **OK:** one unit’s adversary pass longer than that unit’s capture journey.
- **OK:** phone cell review longer than that cell’s screenrecord if other cells
  capture/review overlap.
- **Not OK:** waiting until *all* capture is done, then starting a long serial
  analysis that alone exceeds the whole capture window — that reintroduces the
  old batch-only anti-pattern and makes analysis the bottleneck.
- **Not OK:** infinite re-review loops, decorative extra agents, or re-opening
  the entire matrix three more times “for confidence” without new evidence after
  R3 already agreed — thrash without gain.
- If analysis is falling behind capture: **scale up review fan-out** (more
  workers), do **not** shallow the checks. If hardware is saturated: finish
  capture, keep reviews draining in parallel as capacity frees; still avoid
  thrifty rubber-stamps.

Live concurrency N belongs only in status files. This skill mandates **deep
parallel analysis under a capture-dominated suite timeline**, not a fixed minute
budget per file.

### Forbidden thrift on analysis

- One mid-play still as the only video evidence for a ≥20s recording  
- Directory listing + prior `BAD: none` → ship  
- Single agent writing 105 `all_good` sidecars in one burst without image-tool opens  
- Skipping menu/mode/game_over frames because “we care about play”  
- Skipping post-dash / mid-play / late-play frames because “journey looked fine early”  
- Collapsing discovery into “walk ids I already know” only  
- Preferring a fast green A7 over a slow red one when pixels look wrong  
- Serializing all review after capture “to keep it simple” when workers could fan out  

### Required generosity on analysis

- Prefer **more stills, more opens, more parallel agents** over fewer  
- Prefer **finding one real BAD** over closing the gate early  
- Prefer **updating criteria** when a new failure class appears over ignoring it  
- Prefer **keeping review pool full** whenever finished units exist and capture
  is still running (or has just finished with a short tail)

## Vocabulary — CAPTURE vs REVIEW (do not conflate)

Agents, orchestrators, and status files **must** keep these layers separate.
Conflating them is why prior cycles rubber-stamped ship while screenshots still
had nested borders, glyph tofu, and dead chrome.

| Term | Who produces it | What it means | What it does **not** mean |
|------|-----------------|---------------|---------------------------|
| **CAPTURE_OK** (scripts may still print `PASS name` for history — treat as capture) | `e2e_inputs.mjs`, `e2e_emulator_matrix.mjs`, `e2e_phone.mjs`, `viewport_shots.mjs` | Automation step succeeded: file written, state reached, steps counted, matrix cell non-empty, exit 0 | “Looks good”, “criteria passed”, “ship ready” |
| **results.json / emulator_results.json `ok: true`** | Same runners | Same as CAPTURE_OK for that named check | Visual or play-quality acceptance |
| **N/N passed** in suite summary | Same runners | N capture checks ok / total checks | N units **reviewed** clean |
| **A5 VERIFY** | `VERIFY_ONLY=1 viewport_shots` | All `expected_cells` files **exist** and non-empty | Images were opened or look correct |
| **A4b / A6 review** | Agents (prefer multi-role R1→R3, parallel across units) open media deeply + write **per-file `*.review.json`** + rollups | Human-grade judgment: discovery **and** criteria checklist **and** adversary | Capture succeeded; one-glance checklist |
| **`*.review.json` sidecar** | Agent only, after analyzing that file | Proof **this** artifact was opened (multi-frame if video), discovery+checklist (+adversary), `verdict` | CAPTURE_OK; empty file; copy-paste without open |
| **CRITIQUE / VIDEO line `BAD: none`** | A4b or A6 only | Discovery found nothing material **and** every applicable criterion passed **and** adversary agrees on this-run pixels | Script CAPTURE_OK; “ids I know are fine” |
| **A7 PRE-PROD PASS** | Agent gate after all deep reviews | This-run sidecars (with discovery/adversary evidence) + rollups + no unaccepted BAD/OPEN-* | Suite exit 0; post-capture serial skim only |

### Hard rules

1. **Capture scripts never perform visual review.** They must not write
   `matrix_critique.md` / `video_critique.md` / `*.review.json` and must not claim A7.
2. **Status / chat language:** say **“capture complete (CAPTURE_OK)”** or
   **“suite exit 0”** for runners; say **“A4b/A6 reviewed; BAD: none”** only after
   **per-file review sidecars** + critique lines exist with opened-image evidence.
   Never write “A4 PASS” alone.
3. **Pipeline:** capture finish → **then** A4b ∥ A6. Artifact on disk is necessary
   for review, not sufficient for ship.
4. **Review of a file is complete only when** media was opened deeply (multi-frame
   for video), discovery + checklist (+ adversary on ship path) ran, and
   **`*.review.json`** was written. Rollup without sidecar is **incomplete**.
5. **A7 inputs are sidecars + critiques + criteria + rubber-stamp spot-check**,
   not `results.json` alone. Suite `failed: 0` is a **prerequisite**, not the gate.
6. If a worker says “15/15 PASS open_bads none” without deep sidecars + opened
   media evidence, treat as **unproven** until re-checked.
7. **Analysis runs in parallel** with capture and other reviews so capture remains
   the suite limiter; do not thrift depth to “beat the clock” when workers are idle.

### Anti-pattern (this was the real failure mode)

```text
BAD:  e2e 240/240 PASS  →  write BAD: none on every critique  →  A7 PASS  →  push
GOOD: e2e CAPTURE_OK 240/240  →  open each PNG/video  →  write *.review.json per file
      →  CRITIQUE/VIDEO rollup  →  only then A7 if no open BADs
```

### Anti-pattern (checklist-only / cheap analysis)

```text
BAD:  open criteria → tick known ids → if nothing matches, all_good → ship
      (misses lateral panel clip, Dash text on stick, etc. until a human reports)
GOOD: DISCOVER freeform issues on pixels first (no id required)
   → MAP issues to criterion ids (or OPEN-*/propose criteria edit)
   → CHECKLIST every id for that screen (must not skip)
   → ADVERSARIAL second pass whose job is to find one fail
   → only then all_good / BAD: none
```

Checklist compliance alone is **not** product taste. Criteria are a **floor**
(you must check every listed id) **and** a **growing ontology** (new visual
failure classes must become new ids). They are **not** a ceiling that forbids
reporting something ugly without a pre-existing id.

## Multi-role visual review (required — use multiple agents)

Review of a matrix unit (or phone cell) is **not** one skim. Prefer **separate
subagents** when the unit has multiple PNGs + a video (width is parallel-eligible).
A single agent may only combine roles if it still produces **all** role outputs
below and does not skip depth to “finish the unit.”

### Roles

| Role | Goal | Output | Parallel? |
|------|------|--------|-----------|
| **R1 Discover** | Adversarial product eye: what looks wrong, clipped, overlapping, unfair, unreadable, wrong state, dead chrome — **before** optimizing for known ids | Freeform issue list with frame/file pointers (paths + timestamps) | ∥ R1 of other units; serial before that unit’s R2 if one agent |
| **R2 Map + checklist** | Map each issue to `qa_success_criteria.json` id; walk **full** `review_checklist_by_screen` (+ fairness / SIM for playing); write sidecars + rollup lines | `*.review.json` + CRITIQUE/VIDEO lines; proposed criteria edits if needed | After R1 for same unit; ∥ other units |
| **R3 Adversary** | Independent pass whose **only success metric is finding a miss** R1/R2 left as `all_good` / `BAD: none` | Confirm or escalate; never “agree to ship faster” | After R2 for same unit (or same-time on a **held-out** still set) |
| **R4 Criteria steward** (when any OPEN-* / new class) | Edit `scripts/qa_success_criteria.json` (new id, known_fail_examples, checklist membership) | Criteria PR/diff in-tree; re-run R2 on affected artifacts | Serial with shared criteria file writer |

**Orchestrator default:** for each finished matrix unit, spawn **R1 ∥ (optional
second discover on video vs PNGs)** → **R2** → **R3**. Do not wait for the full
matrix before starting R1 on unit U. Keep **many units’ review chains in flight**
while capture continues so suite wall-clock stays capture-bound; a single unit’s
R1→R3 may outlast that unit’s capture. Analysis wall-time >> one tool call is
fine — shallow parallel is still thrift.

### R1 Discover — how to look (mandatory geometry + chrome scan)

When the image tool (or stills) is open, agents **must** actively scan — not
only name the screen:

1. **Full frame edges** — all four sides of the **game canvas** and of any
   **panel blue border**: is the border complete, or cut on left/right/top/bottom?
2. **Text vs edges** — any glyph clipped by panel, canvas, safe area, or field border?
3. **Overlaps** — status/HUD text on stick or DASH; START on help; score under chrome;
   controls inside field; labels through borders.
4. **State truth** — filename/inventory claim vs visible screen (`V-STATE-MATCH`).
5. **Form factor** — stick/DASH only on handheld play; desktop has none; copy matches.
6. **Entities** — stars/hazards/player inside blue play rect (not in deck/grips/HUD).
7. **Time (video)** — sample **early, mid, late**, and **after dash / after mode change**,
   not one representative frame. Prefer ≥6 stills across a ≥20s play segment when
   stills exist; extract more with ffmpeg if needed (analysis cost OK).
8. **Transient HUD** — cooldown strings (`Dash 0.Xs`), combo, banners — appear only
   sometimes; seek them; if present over controls → issue.
9. **Ugly without a name** — if it would make a player wince and no id fits, still
   record it (see OPEN-* below). Do **not** drop it to keep the gate green.

R1 does **not** need criterion ids. Plain language is required.

### Mapping issues (R2) — ids, OPEN-*, criteria growth

For each R1 issue:

1. Prefer an existing id in `qa_success_criteria.json` (visual / input / video /
   fairness / SIM / artifact).
2. If none fits: use provisional id **`OPEN-{SHORT-SLUG}`** in the sidecar
   `issues[]` and rollup BAD line (severity: treat as **blocker** until mapped or
   user-accepted). Example: `OPEN-PANEL-LATERAL-CLIP`, `OPEN-DASH-TEXT-ON-STICK`.
3. **Same turn or immediate follow-up:** R4 (or R2 if combined) **must** edit
   `scripts/qa_success_criteria.json` to:
   - add a durable criterion id (or extend an existing check text),
   - add `known_fail_examples` pointing at **this-run** artifact paths/stills,
   - add the id to the correct `review_checklist_by_screen` / fairness / SIM lists,
   - bump `version` / `updated` when the schema meaningfully grows.
4. Re-map OPEN-* → stable id on the sidecar before calling A7 PASS (or leave OPEN-*
   as open BAD — either blocks ship).

**Never:** drop an OPEN-* issue because “criteria didn’t list it.”  
**Never:** `all_good` while OPEN-* or unmapped freeform issues remain for that file.

### R3 Adversary — anti-rubber-stamp

R3 opens **at least**:

- every matrix PNG for the unit that R2 marked `all_good`, and  
- ≥3 video stills (or mid-play samples) for recordings marked `all_good`, and  
- any `known_fail_examples` paths that match this format/screen  

R3 asks: “Would a careful human fail this?” If yes → force `verdict: issues`,
update rollup, block A7. R3 **fails the process** if it only re-reads R2’s
summary without re-opening media.

When many units claim all_good, orchestrator may run **one R3 batch** that
spot-checks a **risk-weighted** set (all phone portrait menu/mode/playing, all
short landscape, any format with prior OPEN-*, random 10% of desktop) — but
**ship-critical handhelds and any unit R1 flagged** always get full R3.

## Per-artifact review files (required)

**Format: JSON** (machine-checkable at A7; one file per reviewed media asset).

Rollup markdown (`matrix_critique.md` / `video_critique.md`) is **not enough**.
For **every** matrix PNG and **every** primary e2e recording that counts toward
ship, the reviewing agent must produce a sidecar **after** analyzing that file.

### Paths (sidecar next to artifact, same basename)

| Artifact | Review sidecar |
|----------|----------------|
| `screenshots/viewports/{format}_{shot}.png` | `screenshots/viewports/{format}_{shot}.review.json` |
| `screenshots/web/e2e/recordings/{name}.webm` or `.mp4` | `screenshots/web/e2e/recordings/{name}.review.json` |
| Phase C phone `screenshots/web/phone/recordings/{cell}.mp4` | `screenshots/web/phone/recordings/{cell}.review.json` |

Optional stills used only as A4b helpers may also get sidecars under
`screenshots/web/e2e/stills/.../*.review.json` when they are the sole evidence
opened; **primary** gate is matrix PNG + primary recording sidecars.

### Schema (`*.review.json`)

```json
{
  "artifact": "screenshots/viewports/phone_landscape_04_playing.png",
  "artifact_kind": "matrix_png",
  "reviewed_at": "2026-07-17T21:00:00Z",
  "reviewer": "agent",
  "review_roles_completed": ["discover", "map_checklist", "adversary"],
  "run_start_unix": 1784321756,
  "opened": true,
  "frames_or_stills_opened": [
    "screenshots/viewports/phone_landscape_04_playing.png"
  ],
  "discovery_notes": [
    "Full blue field border visible; stick left grip clear of field; no HUD on DASH"
  ],
  "verdict": "all_good",
  "summary": "all good",
  "issues": [],
  "criteria_checked": ["V-STATE-MATCH", "V-PLAY-SINGLE-BORDER", "V-PLAY-CONTROLS-OUTSIDE-FIELD", "V-PLAY-HUD-CLEAR", "V-OVERLAP"],
  "sim_scenarios_checked": [],
  "adversary_reviewed": true
}
```

**With issues:**

```json
{
  "artifact": "screenshots/web/e2e/recordings/phone_landscape_touch.mp4",
  "artifact_kind": "e2e_video",
  "reviewed_at": "2026-07-17T21:05:00Z",
  "reviewer": "agent",
  "review_roles_completed": ["discover", "map_checklist", "adversary"],
  "run_start_unix": 1784321756,
  "opened": true,
  "frames_or_stills_opened": [
    "stills/t05.jpg",
    "stills/t30.jpg",
    "stills/t45.jpg",
    "stills/t60.jpg",
    "stills/t75.jpg"
  ],
  "discovery_notes": [
    "t75: grey 'Dash 0.5s' between stick and DASH button",
    "t75: red hazard below blue play border over control deck"
  ],
  "verdict": "issues",
  "summary": "Dash cooldown text on chrome; hazard OOB below field",
  "issues": [
    { "id": "V-PLAY-HUD-CLEAR", "detail": "Dash 0.5s between stick and DASH ~t75" },
    { "id": "V-PLAY-ENTITIES-IN-BOUNDS", "detail": "hazard fully below play border t75" }
  ],
  "criteria_checked": ["VID-JOURNEY", "V-PLAY-HUD-CLEAR", "V-OVERLAP", "V-PLAY-ENTITIES-IN-BOUNDS", "SIM-DASH-VISUAL-FEEDBACK"],
  "sim_scenarios_checked": ["SIM-PLAY-STICK-NORMAL", "SIM-DASH-VISUAL-FEEDBACK"],
  "adversary_reviewed": true
}
```

| Field | Rules |
|-------|--------|
| `opened` | Must be **true**. Agent must have used image tool / stills / video sample on **this** artifact. |
| `frames_or_stills_opened` | Paths or timestamps actually opened. Video: **≥3** times; prefer **≥6** for ≥20s play. Missing/empty → incomplete. |
| `discovery_notes` | R1 freeform findings (may be empty array only after a real geometry scan that found nothing). |
| `review_roles_completed` | `discover`, `map_checklist`, `adversary` as applicable. Ship path needs all three (or combined pass that still re-opened media for adversary). |
| `adversary_reviewed` | **true** only after R3 re-opened media (not rubber-stamp of R2 text). |
| `verdict` | **`all_good`** or **`issues`** only. |
| `summary` | If all good: short phrase like **`all good`**. If issues: human-readable what’s wrong. |
| `issues` | Empty when `all_good`; else ≥1 `{ id, detail }` with criterion, `SIM-*`, or `OPEN-*`. |
| `run_start_unix` | This QA pass so stale sidecars do not count. |

### When to write

1. **R1:** Open media (PNG image tool; many stills / sample for video). Geometry + chrome scan → discovery notes.
2. **R2:** Map → ids / OPEN-*; full screen checklist + SIM/fairness as applicable.
3. Write/update **`*.review.json`** (update again after R3).
4. Rollup CRITIQUE/VIDEO line must match sidecar.
5. **R3:** Re-open media; flip to `issues` if miss found.
6. **R4** if OPEN-* / new class → edit criteria before A7 PASS.

### Completeness rules

- Review of **F** incomplete until **F.review.json** has `opened: true`, valid
  `verdict`, discovery evidence (`discovery_notes` and/or `frames_or_stills_opened`),
  and adversary done on ship path.
- A7 needs this-run sidecars for all expected matrix cells + required recordings.
- `verdict: issues` ⇔ rollup must not say `BAD: none`.
- `verdict: all_good` ⇔ rollup `BAD: none` only if criteria + discovery + adversary agree.
- **Do not** invent sidecars without opening media. **Do not** batch-write
  105 `all_good` files from a directory listing.
- **Do not** shallow analysis to save time when capture already paid the cost —
  fan out workers instead so suite time stays capture-bound.

### Anti-patterns

- Writing only rollup markdown with no per-file sidecars
- One sidecar for a whole format folder instead of one per PNG/video
- `opened: false` / missing fields while claiming review done
- Stale `run_start_unix` reused as this-run proof
- `all_good` with empty `frames_or_stills_opened` or no discovery pass
- Skipping R3 because “R2 already looked”
- Suppressing OPEN-* because no criterion id existed yet
- One still for an entire ≥20s video
- Serial review-only phase longer than the capture phase because reviews never
  started until capture fully exited

## Chain rule — always start the next task

**Every task ends by checking this skill for the next step in the current phase.**

1. Finish the current step fully (exit 0 / required artifacts / review done).
2. Look at [Ship order](#ship-order-read-this-first--do-not-reorder) (and the
   phase A/B/C checklists). Identify the **next** required step that is not yet
   satisfied by artifacts on disk.
3. **If a next step exists → start it immediately** (same turn or spawn a
   background/subagent task). Do **not** stop to “wait for the user” after a
   successful intermediate step (e.g. e2e videos done is not “done”).
4. **If this was the last step of the phase** → enter the next phase
   immediately when the gate for that edge is satisfied
   (A7 PASS → B; B2/B3 success + physical phone present → C). Report as you go.
5. **If the step failed** → enter the fix loop; after patch, restart from the
   required phase A steps — then again apply this chain rule.

### Trust the gates — do not wait for the user

If the **workflow criteria for the next phase are met** (artifacts on disk, exit
codes, critiques with no open BADs, A7 PASS, etc.), agents and the orchestrator
**must proceed** to the next required step without asking for permission or
waiting for a human “go ahead.”

- **Trust honest work.** When A7 is a true PASS (both critique files clean,
  matrix complete, reviews done), start **Phase B** (commit + push + deploy
  watch) in the same session. Do **not** park at “ready for Phase B — awaiting
  user.”
- The same applies across **all** anti-stop points: intermediate success is not
  a stopping point; **gate failure** is.
- Still **never** skip or weaken a gate (do not push with open BADs, incomplete
  matrix, or missing reviews). Trust does not mean optimism without evidence —
  it means **evidence satisfied → continue**.
- Only pause for the user when the skill/status truly cannot decide (missing
  credentials, ambiguous product acceptance of residual BADs the user must
  accept in writing, or an explicit user order to hold).

### Explicit anti-stop points (do not end the session here)

| Just finished | Must still do next |
|---------------|--------------------|
| A2 build | A3 serve → A4 pipeline (capture + review) |
| One **matrix unit** **capture** done (CAPTURE_OK only) | **Immediately** start **deep A4b + A6 (R1→R3)** for that unit — do **not** wait for other units |
| A4b/A6 for unit U while other units still capturing | Keep **review fan-out full** on finished units; keep capture pool full |
| All matrix units **captured** (CAPTURE_OK) | **Not done** until every unit has deep sidecars + rollups + adversary (short trailing drain only) |
| All units captured **and** all deep reviews written | A5 verify → **A7** pre-prod |
| A7 PASS (review, not capture) | **Phase B immediately** (commit + push + Pages watch) — do not wait for user |
| B2 deploy success | B3 live smoke; if physical phone → Phase C |
| C phone fail | Phase A fix loop, not stop |

Stopping after “recordings exist” or “suite N/N PASS” without **reviews**,
batching all reviews only after every unit finishes, treating video review as
the matrix PNG review, treating **CAPTURE_OK as A7**, or **idling after a true
A7 review PASS waiting for a human OK**, is a process failure.

### Matrix unit (work atom)

A **matrix unit** is one entry in `scripts/qa_matrix.json` → `formats[]`: a
specific **format id + CSS resolution** (width×height, plus dpr/touch class).
Examples: `phone_portrait` (390×844), `laptop_hd` (1366×768), `4k` (3840×2160).

- Capture atom: one e2e journey for that unit → one primary recording (+ stills)
  + all screen matrix PNGs for that format id (`{format_id}_{01_boot…05_game_over}`).
- Review atom: as soon as **that unit’s** artifacts exist, review **that unit**
  (video path + its matrix PNG cells). Do not wait for the full matrix.

## Capture once, review twice — **pipeline per matrix unit**

### Bad vs required workflow

| | Workflow |
|--|----------|
| **BAD (do not)** | Run e2e for **all** matrix units → only **then** review every video and every PNG |
| **REQUIRED** | For **each matrix unit** (format id + resolution): when that unit’s capture finishes, **immediately** start evaluating its **video** and its **matrix PNGs**, **while** other units are still capturing and/or being reviewed |

Agents must **overlap capture and review**. The moment unit U’s recording and
matrix PNGs land on disk, spawn or start A4b(U) ∥ A6(U). Do not idle until the
whole `e2e_inputs` pool exits.

### Unified production (A4 capture) — avoid duplicate work

**Default:** one journey per **matrix unit** produces **both** continuous VIDEO
and **clear matrix PNGs** (quality holds at each screen). Do **not** run a full
serial `viewport_shots` walk after e2e reloaded the same game for every screen
again.

| Concern | How A4 capture handles it |
|---------|---------------------------|
| Duplicate loads | Single cold-load per matrix unit for matrix + primary video path |
| Parallel capture | `CONCURRENCY` matrix units at once (env-tuned; orchestrator/session sets the number from hardware headroom + quality) |
| Desktop path | Chrome + Puppeteer; CDP screencast → webm (`record.mjs`) |
| Handheld path | **Android emulator** + **`adb shell screenrecord`** (full display) + **`adb shell input`** touches |
| Pipeline review | As **each** unit finishes capture → immediate A4b + A6 for **that** unit |
| PNG quality | **Quality hold** (`MATRIX_HOLD_MS`, default ~450ms settle + short idle) then PNG — not a random video frame |
| Video quality | Recording continues through holds and play; holds are brief pauses, not freezes that hide bugs |
| Separate analysis | Reviews stay split (below); still **start** as soon as the unit is ready |

`viewport_shots.mjs` is a **fallback** for missing matrix PNGs (verify-only or
missing-only). Prefer one producer journey per unit (desktop: `e2e_inputs`;
handheld: emulator+adb runner).

### Two different reviews (do not merge them)

| | **A4b — E2E video review** | **A6 — Matrix PNG review** |
|--|----------------------------|----------------------------|
| **What** | Playthrough / input over time | Static layout at settled screens |
| **Produced by** | Same A4 journey for that unit (recording) | Same A4 journey (quality-hold PNGs) |
| **Artifacts** | `screenshots/web/e2e/recordings/*.webm` (+ `stills/`) | `screenshots/viewports/{format}_{screen}.png` |
| **Unit of review** | One recording per matrix unit × input path | One PNG per screen **cell** of that matrix unit |
| **When to start** | **As soon as that unit’s** recording/stills exist | **As soon as that unit’s** matrix PNGs exist |
| **Written output** | Per recording: `recordings/{name}.review.json` **+** line in `video_critique.md` | Per PNG: `viewports/{cell}.review.json` **+** line in `matrix_critique.md` |
| **Catches** | Lag, transitions, stick miss mid-play, flicker, dead controls | Layout, clipping, wrong chrome, form-factor copy, HUD |
| **Does not replace** | Matrix PNGs / A6 | E2E videos / A4b |

**Videos are not “included in” the PNG review.** Complementary, both required.
Extracted video stills help A4b only — they are **not** matrix cells.
**Sidecar `*.review.json` is mandatory proof** that the agent analyzed that file.

### Pipeline loop (required mental model)

```text
A4 PIPELINE (matrix units from qa_matrix.json formats[]):
  start capture pool (CONCURRENCY units at a time)
  whenever matrix unit U finishes capture (video + PNGs on disk):
      IMMEDIATELY fan out analysis (do not wait for other captures):
        A4b R1→R2→R3 on video  ∥  A6 R1→R2→R3 on that unit's PNGs
        (prefer separate subagents; deep multi-frame / geometry OK)
      keep review workers busy on every finished unit while capture continues
  when all units captured AND all deep reviews (sidecars + rollups + adversary) done:
      A5 verify matrix complete → A7 pre-prod gate
  trailing review after last capture should be a short drain of in-flight R2/R3,
  not a brand-new serial analysis of the whole matrix
```

If the capture script runs as one process for all units, **do not** sit idle on
that process: watch for per-unit artifacts (new `recordings/*`, new
`viewports/{format_id}_*.png`, log lines) and start **deep** review for each
finished unit as it appears. Subagents are preferred so **capture keeps moving**
and **analysis stays parallel** (suite bottleneck = capture, not a review queue).

## Ship order (read this first — do not reorder)

Three **phases**. Only phase A unlocks push. Phone work never replaces phase A.

```text
PHASE A — LOCAL ONLY (blocks push until every box is true)
  A1. cargo test / cargo check
  A2. ./scripts/web-build.sh          # wait for finish; fresh dist/
  A3. serve dist                      # http://127.0.0.1:17880/ (RUSTY_PORT; adb reverse same port)
  A4.  PIPELINE — per matrix unit (format id + CSS resolution):
        Desktop / laptop formats → Chrome + Puppeteer (e2e_inputs path)
        Handheld / touch formats → Android emulator + adb (required; see below)
        For EACH unit, as soon as its capture finishes:
          A4b open VIDEO → {name}.review.json + video_critique.md   ⎫ simultaneous
          A6  open PNGs  → {cell}.review.json + matrix_critique.md  ⎭ with each other
        …while other units are still capturing / being reviewed
        Do NOT wait for all units before starting any review
        Review incomplete without per-file *.review.json
  A5.  VERIFY matrix complete (viewport_shots.mjs VERIFY_ONLY or missing-only)
  A7.  PRE-PROD: every artifact has this-run *.review.json + rollup lines;
       no open BADs (or user-accepted)
  ── only after A7 PASS may you commit + push ──

PHASE B — PUSH + PAGES (only after A7 PASS)
  B1. git commit + git push
  B2. gh run watch pages.yml until success
  B3. smoke live URL HTTP 200

PHASE C — PHYSICAL USB PHONE (only if real adb device present; after B2 success)
  C1. node scripts/e2e_phone.mjs against LIVE Pages (2×2 video)
  C2. Review each phone cell video as it finishes (same pipeline idea) + inventory
  C3. Any phone FAIL → back to PHASE A (patch), not “phone-only push”
```

### What is **not** enough to push

| Action | Layer | Unlocks push? |
|--------|-------|----------------|
| `cargo test` / `cargo check` only | build | **No** |
| Web build only | build | **No** |
| One format / one screen smoke | capture | **No** |
| E2E / emulator suite **exit 0** / `N/N PASS` / `failed: 0` | **CAPTURE_OK only** | **No** |
| A5 VERIFY 105 files present | file presence | **No** (not visual review) |
| Physical USB phone smoke against **local** `dist` (adb reverse) | debug | **No** |
| Phone against **live** before phase A finished | process | **No** |
| Handheld matrix covered only by Chrome `page.emulate` / resized desktop | wrong capture path | **No** |
| Critiques all say `BAD: none` without opening this-run images | rubber-stamp | **No** |
| Missing `*.review.json` sidecars for PNGs/videos | incomplete review | **No** |
| Full phase A: capture + **A4b+A6 review (sidecars + rollups)** + A5 + **A7 review PASS** | capture **and** review | **Yes** → phase B |

Physical USB against local `dist` is **debug only** and never unlocks push. Phase A
handheld ship proof is the **Android emulator** path (below), not Puppeteer-only
device emulation.

Pairs with **`game-input-e2e`** (games) or **`app-input-e2e`** (apps). Matrix source of truth: **`scripts/qa_matrix.json`**
(read `expected_cells` / formats from that file — do not hardcode counts in
memory or treat this skill as a live status log). **PASS/FAIL criteria source
of truth: `scripts/qa_success_criteria.json`** (criterion ids, checklists,
severity, anti-patterns).

---

## Phase A checklist (required before push)

You are not allowed to `git push` until **all** of these are true:

1. **Build finished** — `./scripts/web-build.sh` exit 0; wait as long as needed.
2. **A4 capture — Unified per matrix unit** — every `formats[]` entry gets exhaustive
   surface + **quality-hold matrix PNGs** + VIDEO. **Desktop** formats: Chrome +
   Puppeteer (`e2e_inputs` / `CAPTURE_MATRIX=1`). **Handheld / touch** formats:
   **Android emulator** with full-display **`adb shell screenrecord`** and OS-level
   touches via **`adb shell input`** (see [Phase A handheld — Android emulator](#phase-a-handheld--android-emulator-required)).
   Chrome `page.emulate` alone is **not** ship-valid for handhelds. Parallel units
   via `CONCURRENCY` (level set outside this skill — orchestrator/session from
   hardware + quality, not a fixed N here).
3. **A4 pipeline review** — for **each** matrix unit, **as soon as that unit’s**
   artifacts exist: **A4b** video review **and** **A6** matrix PNG review for
   that unit (simultaneous with each other and with other units’ capture/review).
   For **each** media file opened: write **`*.review.json`** sidecar, then update
   rollup lines in `video_critique.md` / `matrix_critique.md`. Do **not**
   defer all review until the full capture job ends.
4. **A5 — Matrix present** — all `expected_cells` PNGs exist (usually already from A4;
   `VERIFY_ONLY=1 node scripts/viewport_shots.mjs` or missing-only capture).
5. **A7 — PRE-PROD REVIEW PASS** — every required PNG/video has a this-run
   `*.review.json` with `opened: true`; rollup lines exist; no unaccepted BADs.

Partial matrices (e.g. only menu@1080p) do **not** count.

---

## Phase B / C (after push only)

7. **Commit + push** — only after phase A.
8. **Deploy watcher** — Pages workflow success (build + deploy jobs).
9. **Physical USB phone (if connected)** — `e2e_phone.mjs` on **LIVE** URL after
   deploy (Phase C). Failures send you back to phase A. Optional USB smoke against
   **local** dist is debug only; it never skips A4–A7 and does **not** replace
   Phase A **Android emulator** handheld coverage.

---

## Parallel vs serial (required working style)

This section is the **authority** for when agents may fan out work vs wait.
It also governs **future edits to this skill and related scripts**: every new
step must be classified with the criteria below and listed in the task map.

### How to apply when editing this skill or the suite

When you add, rename, split, or merge a step:

1. **Name the step** (e.g. A4c) and its **inputs** (what must exist) and
   **outputs** (what it writes).
2. **Run the criteria table** against every edge to other steps (before / after /
   same-time). Record the criterion numbers in the task map.
3. **Update the task map** in this file — do not leave classification only in chat.
4. **Update ship order + chain-rule anti-stop table** if the new step sits on the
   critical path.
5. **If two steps produce the same artifact kind** (e.g. two full game walks) →
   apply criterion **4** (merge or make the second verify-only). Prefer **one
   producer, many consumers**.
6. **If two steps only read different finished artifacts** → mark **parallel**
   (criterion **5**) and say so in the chain rule (e.g. “A4b ∥ A6”).
7. **Never** mark a step parallel across a **phase barrier** (criterion **3**).
8. Keep this skill **stateless**: document **which** work may run concurrently and
   the `CONCURRENCY` env knob; do **not** prescribe a fixed pool size or live N.
   put live run results / current N only in status files / critique / `results.json`.

**Rule of thumb:** parallelize *width* (matrix units, capture∥review pipeline,
critique batches, wait+work). Serialize *depth* across **barriers** (build →
start capture; all units reviewed → gate → push → deploy → live phone).  
**Do not** serialize “all capture then all review” — that is the bad workflow.

### Decision criteria (classify every edge A → B)

Apply in order. First true row wins for that edge.

| # | Criterion | Question | If true → |
|---|-----------|----------|-----------|
| 1 | **Artifact dependency** | Does B need files/exit codes only A produces (fresh `dist/`, **this unit’s** recording/PNGs, complete critique files for the gate)? | **SERIAL** for that edge — A finishes first (unit-scoped when possible) |
| 2 | **Shared mutable product state** | Do both write the same paths, same git tip, or same live deploy? | **SERIAL** (or one designated writer) |
| 3 | **Phase / ship barrier** | Does the edge cross A→B→C or “may push / may claim ship”? | **SERIAL** — never parallelize away a gate |
| 4 | **Same work twice** | Does the second step cold-load the game only to recreate what the first already wrote? | **DO NOT RUN** as a full walk — merge into one producer or **verify-only** |
| 5 | **Independent consumers** | Do A and B only *read* different finished artifacts (no writes to each other)? | **PARALLEL** OK |
| 6 | **Independent work units** | Same step body, different formats/files, no shared write conflict? | **PARALLEL** with a **cap** (`CONCURRENCY=<N>`, tuned outside this skill) |
| 7 | **Wait overlap** | Is one side pure wait (build, encode, `gh run watch`)? | **PARALLEL** with useful other work on *already finished* units only |
| 8 | **Resource thrash** | Do heavy Chrome/GPU/ffmpeg/**emulator** jobs degrade (OOM, CDP timeouts, black frames, AVD stalls)? | **Lower concurrency** or **SERIAL**; quality beats speed |
| 9 | **Capture quality** | Must a PNG be a settled screen (not a random mid-animation frame)? | Hold/settle **inside** the journey; parallelize **formats**, not “second full load for screenshots” |

### What may run in parallel

| Work | How | Criteria |
|------|-----|----------|
| **Matrix units inside A4 capture** | `CONCURRENCY=<N>` pool (desktop: browser+CDP record; handheld: emulator+adb screenrecord/input + quality-hold PNGs) | 6; cap via 8 |
| **Video encode of unit X** while **unit Y** still plays | Same pool; natural overlap | 6, 7; watch 8 |
| **Review of finished unit U ∥ capture of other units** | **Required pipeline** — start deep A4b(U)+A6(U) (R1→R3) the moment U’s artifacts exist | 5, 6, 7 |
| **A4b(U) video review ∥ A6(U) matrix PNG review** | Same unit, two artifact trees; each may use its own R1/R2/R3 workers | 5 |
| **Reviews of different finished units** | Multiple readers / subagents; keep pool full so suite stays capture-bound | 5, 6 |
| **R1 discover ∥ other units’ R1/R2** | Freeform discovery is read-only on media | 5, 6 |
| **R3 adversary batch** on risk-weighted all_good units | After those units’ R2; may ∥ trailing capture | 5, 6, 7 |
| **A1 cargo test ∥** read code / draft notes | No capture artifacts shared | 5, 7 |
| **A2 build wait ∥** review *prior-run* artifacts / read code | Wait overlap only — **do not** ship on old capture | 7 |
| **A5 verify** while trailing unit reviews finish | Verify is read-only check of PNG presence | 5 (after those PNGs exist) |
| **B2 deploy watch ∥** draft final ship report | Wait + writing, no new capture | 7 |
| **Local adb-reverse phone debug ∥** phase A capture | Debug only; never ship proof | debug, not 3 |

### What must stay serial (barriers)

| Order | Why |
|-------|-----|
| **A1 → A2** (tests before / with build is fine; build before capture is required) | 1 — fresh `dist/` |
| **A2 build complete → A3 serve → start A4 capture** | 1 — HTTP + current WASM |
| **Capture of unit U complete → start A4b(U) and A6(U)** | 1 — need **that unit’s** this-run video + PNGs (not the whole matrix) |
| **All units captured + all unit reviews complete → A7** | 1, 3 — full critique coverage |
| **A7 PASS → B1 push → B2 deploy → B3 live smoke** | 2, 3 |
| **B2/B3 success → C live phone** | 1, 3 — package matches shipped commit |
| **Any FAIL / open BAD → patch → rebuild → full A4 pipeline again → A7** | 1, 2 — no capture on broken/stale build |
| **Inside one unit’s journey:** boot → menu → mode → play → game over holds | 1, 9 — screen order and settle times |

**Main fan-out:** capture pool of matrix units **plus** per-unit A4b ∥ A6 as each
unit lands. **Barrier is not** “A4 capture process exit 0 before any review.”

### Task map (quick reference)

| Step / pair | Mode | Criteria |
|-------------|------|----------|
| A1 `cargo test` ∥ explore code | **Parallel** | 5, 7 |
| A2 build | **Serial before A4 capture** | 1 |
| A2 wait ∥ draft from *prior* run / read code | **Parallel** | 7 (not ship-valid) |
| A3 serve | **Serial before A4 capture** | 1 |
| A4 capture matrix units (`CONCURRENCY=<N>`) | **Parallel (capped)** | 6, 8 |
| A4 video + matrix PNGs for one **matrix unit** | **One journey (serial screens)** | 4, 9 — unified producer |
| A4 capture (this run) ∥ full `viewport_shots` re-walk | **Forbidden** | 4 |
| **Review unit U ∥ capture other units** | **Required parallel (pipeline)** | 5, 6, 7 |
| A4b(U) ∥ A6(U) for same finished unit | **Parallel** | 5 |
| R1 → R2 → R3 within one artifact | **Serial roles** (same file); depth OK if suite fans out | 1 |
| Reviews of different finished units | **Parallel** (prefer many workers) | 6 |
| R4 criteria file edit | **Serial** writer on `qa_success_criteria.json` | 2 |
| A5 verify | After PNGs exist; may ∥ trailing reviews | 1, 5 |
| A7 pre-prod gate | **Serial after all units reviewed** (incl. adversary) | 1, 3 |
| Batch-all-review after all-capture | **Forbidden** (makes analysis the suite bottleneck) | anti-pattern |
| B1 push | **Serial after A7 PASS** | 2, 3 |
| B2 deploy watch ∥ report draft | **Parallel** | 7 |
| B3 live smoke | **Serial after B2** | 1 |
| C phone live | **Serial after B2/B3** | 1, 3 |
| C phone adb-reverse local ∥ A4 | **Parallel as debug only** | never A7/C proof |
| Fix-loop patch ∥ capture of old build | **Serial** | 1, 2 |
| All capture then all review (batch-only) | **Forbidden anti-pattern** | wastes time; hides BADs late |

### Env / knobs

| Setting | Role | Notes |
|---------|------|-------|
| `CONCURRENCY` | Parallel matrix-unit pool size for capture | **Not fixed in this skill.** Orchestrator/session sets N from hardware headroom + quality. This skill only states that independent matrix units **may** run concurrently. Lower on criterion **8** thrash (black frames, Chrome crashes, CDP timeouts, encode thrash). |
| `CAPTURE_MATRIX` | `1` (on) typical for ship path | `0` only for video-only debug; then A5 must fill missing PNGs (still avoid a full duplicate walk if possible). |
| `MATRIX_HOLD_MS` | Quality-hold settle (~450 typical) | Increase if PNGs are mid-transition (criterion **9**); never “fix” flaky stills by skipping holds. |

### Phone / emulator / desktop (do not confuse)

| Goal | How | Counts as phase A ship proof? |
|------|-----|-------------------------------|
| Desktop / laptop matrix units | Chrome + Puppeteer (`e2e_inputs`) | **Yes** (desktop path) |
| Handheld / touch matrix units | **Android emulator** + **`adb shell screenrecord`** + **`adb shell input`** against local served `dist` (usually `adb reverse`) | **Yes** — **required** for handhelds |
| Fast debug on USB handset | Physical phone → local `dist` via `adb reverse` | **No** — debug only |
| Ship proof on physical handset | Physical phone → **LIVE** after deploy (`e2e_phone`) | Phase **C** only |
| Chrome `page.emulate` / resized window only | Puppeteer device metrics | **No** for handheld ship proof (may assist desktop-only or throwaway debug) |

### Anti-patterns (explicit)

- **Treating suite `PASS` / exit 0 / N/N as visual review or A7** — that is CAPTURE_OK only
- **Calling full matrix “optional / aspirational residual” because runners are missing** — install/build the runners, then run all `expected_cells`
- **Closing residual with priority/debug subset only** and labeling it done
- **Writing `BAD: none` because capture succeeded** or because a prior critique said so
- Parallelizing across a **ship barrier** (push while A4/A7 still open) — criterion **3**
- **Stopping after an intermediate step** without chain rule / next task
- **Waiting for the user after a true gate PASS** (e.g. A7 review PASS) instead of
  starting the next phase
- **Waiting for every matrix unit to finish capture before any A4b/A6** — the
  old bad workflow; review each unit as soon as **that unit** is done
- **Serial full `viewport_shots` after full e2e** (duplicate cold loads) — criterion **4**
- Treating adb reverse on a **physical** handset as prod verified (debug only)
- Claiming Phase A handheld pass with **Chrome device emulation only** (no Android emulator / no full-display screenrecord / no `adb shell input`)
- `CONCURRENCY` so high that quality collapses (black frames, timeouts) — criterion **8**; pool size is tuned outside this skill
- Parallel writers on the same critique file without merge discipline — criterion **2**
- Parallel pushes / conflicting branches without a plan — criterion **2**
- Adding a new skill step **without** updating this task map and criteria refs
- “Screenshots while video runs” implemented as a **second full page load** instead of quality holds inside A4 capture — criteria **4**, **9**
- **Cheap analysis** (one still, checklist-only, no adversary) because “capture already took long enough”
- **Serial review phase** that alone exceeds the capture phase because reviews waited for full capture exit
- Dropping R1 discovery / multi-frame video / OPEN-* to meet an arbitrary stopwatch while workers sit idle

---

## A4b — E2E video review (required)

**Separate from matrix PNG review.** Starts **per matrix unit** as soon as that
unit’s recording (and stills, if any) exist — **not** only after the full
capture job exits. May run **in parallel with A6 for the same unit**, and in
parallel with capture/review of **other** units. Every unit must be covered
before A7. Use multi-role review (R1 discover → R2 map/checklist → R3 adversary).
**Depth over thrift**; fan out so the suite does not wait on one serial video critic.

### Where

[`screenshots/web/e2e/video_critique.md`](../../screenshots/web/e2e/video_critique.md)

### Format (one line per recording)

```text
VIDEO {format_id}_{keyboard|mouse|touch}: GOOD: <what works in the playthrough> | BAD: none
VIDEO {format_id}_{keyboard|mouse|touch}: GOOD: <…> | BAD: {criterion_id}: <detail>
```

Examples:

```text
VIDEO phone_rodin_chrome_touch: GOOD: modes cycle, stick moves, dash cooldown | BAD: none
VIDEO phone_landscape_touch: GOOD: PSP grips, play ≥20s | BAD: V-PLAY-SINGLE-BORDER: nested frames; VID-HUD-USABLE: Dash 0.4s on bottom border
VIDEO laptop_hd_mouse: GOOD: no stick chrome, point-to-move + right-dash | BAD: none
```

### How to review

- **Authority:** [`scripts/qa_success_criteria.json`](../../scripts/qa_success_criteria.json)
  → `input_criteria`, `video_criteria`, and the same visual ids when stills show
  layout bugs (`VID-VISUAL-SAME-AS-MATRIX`). Criteria are a **floor**; R1 may
  still find OPEN-* issues not yet listed.
- Open extracted stills under `screenshots/web/e2e/stills/{recording}/` across
  **early / mid / late / post-dash** (prefer **≥6** for ≥20s play). Extract more
  with ffmpeg if thin — analysis cost is OK when parallelized. Sample the
  `.webm`/`.mp4` when stills are insufficient.
- Listing `recordings/` is **not** review.
- Cover **each** recording **when that matrix unit finishes**:
  1. **R1** freeform discovery (geometry, transient HUD, motion feedback).
  2. **R2** map + full applicable checklists → sidecar + VIDEO line.
  3. **R3** adversary re-open (≥3 stills if R2 said all_good).
  4. **R4** if OPEN-* → grow criteria file.
- Confirm before A7: this-run sidecar (with frames + adversary) **and** VIDEO line.

### Video checklist (A4b)

Use criterion ids from `qa_success_criteria.json` (non-exhaustive):
`I-BOOT-DISMISS`, `I-MODE-CYCLE`, `I-START-PLAY`, `I-MOVE`, `I-DASH`,
`I-PLAY-DURATION`, `VID-JOURNEY`, `VID-INPUT-WORKS`, `VID-NO-FLICKER`,
`VID-HUD-USABLE`, `VID-VISUAL-SAME-AS-MATRIX`, **`SIM-VIDEO-MATCHES-INPUT`**,
**`SIM-NORMAL-PATH-COVERED`**, **`SIM-EDGE-NO-PANIC`**, **`SIM-STICK-VISUAL-FEEDBACK`**,
**`SIM-DASH-VISUAL-FEEDBACK`** (+ any `V-PLAY-*` visible in stills).

**Input simulation (required):** open
`scripts/qa_success_criteria.json` → **`input_simulation_scenarios`**. For the
recording’s modality, walk `review_checklist_input_sim` (keyboard_desktop /
mouse_desktop / touch_handheld). Compare video/stills to each scenario’s
`expected_video_outcome` (normal **and** edge). Cite **`SIM-*`** ids on BAD
lines when outcomes fail. CAPTURE_OK step counts do **not** prove motion/dash
feedback.

`BAD` not `none` → ship blocker → fix loop (re-run A4 pipeline for affected
units at minimum; full matrix if the bug is systemic).

---

## A6 — Matrix PNG review (required)

**Layout-only static cells.** Not a substitute for video review. Starts **per
matrix unit** as soon as that unit’s screen PNGs exist — **in parallel with
A4b** for the same unit and with other units still capturing.

### Where

[`screenshots/viewports/matrix_critique.md`](../../screenshots/viewports/matrix_critique.md)

### Format (one line per matrix cell)

```text
CRITIQUE {format_id}_{shot_suffix}: GOOD: <what works> | BAD: none
CRITIQUE {format_id}_{shot_suffix}: GOOD: <what works> | BAD: {criterion_id}: <detail> [; {criterion_id}: ...]
```

Examples:

```text
CRITIQUE phone_landscape_04_playing: GOOD: stick+DASH outside field | BAD: V-PLAY-SINGLE-BORDER: nested blue frames; V-PLAY-ENTITIES-IN-BOUNDS: star past right edge; V-GLYPH-TOFU: · as boxes
CRITIQUE laptop_hd_02_menu: GOOD: keyboard control copy, no touch chrome | BAD: none
```

### Rules

- **Authority:** [`scripts/qa_success_criteria.json`](../../scripts/qa_success_criteria.json)
  — open it; walk **full** `review_checklist_by_screen` for that shot; cite
  criterion **ids** on every non-`none` BAD. Also run **R1 discovery** first
  (panel laterals, text clip, overlaps) — checklist alone is insufficient.
- **GOOD** and **BAD** both required (`BAD: none` only when discovery + every
  applicable criterion + adversary agree).
- Open **each** matrix PNG with the image tool (not directory listing). Fan out
  PNG reviews across agents when a unit has many cells.
- Write **`screenshots/viewports/{format}_{shot}.review.json`** after R1/R2;
  update after R3 (`opened`, frames, discovery_notes, verdict, issues).
- Append/update **CRITIQUE** rollup (must match sidecar).
- Review a unit’s cells **when that unit’s capture finishes**, not only after
  the whole matrix — keeps suite capture-bound.
- User may accept residual BADs in writing **by criterion id** (or OPEN-*).
- Filename is not state proof (`*_04_playing` showing menu → `V-STATE-MATCH` /
  `A-LABEL-TRUTH` FAIL).
- **No sidecar = that PNG is not reviewed**, even if a CRITIQUE line exists.

### Matrix PNG checklist (A6)

**Do not improvise a shorter list.** Use
`review_checklist_by_screen` in `qa_success_criteria.json` for the shot’s
screen id. High-signal blockers (non-exhaustive):

**All screens** — `V-CLIP-TEXT`, `V-READABLE`, `V-GLYPH-TOFU`, `V-BLANK-PANIC`,
`V-STATE-MATCH`, `V-FORM-FACTOR-COPY`  
**Menu / mode / GO** — `V-GHOST-FIELD`, `V-PANEL-IN-CANVAS`, `V-MODE-START-CLEAR`  
**Playing** — `V-PLAY-SINGLE-BORDER`, `V-PLAY-NO-SIDE-DIM-SLABS`,
`V-PLAY-ENTITIES-IN-BOUNDS`, `V-PLAY-NO-WEIRD-POLYGONS`,
`V-PLAY-CONTROLS-OUTSIDE-FIELD` / `V-PLAY-DESKTOP-NO-STICK`, `V-PLAY-HUD-CLEAR`  
**Playing fairness / usability (F-\*)** — also walk
`review_checklist_fairness` in `qa_success_criteria.json`: handheld
`F-PLAY-AREA-HANDHELD`, `F-ENTITY-CSS-SIZE`, `F-CROSS-TIME`, `F-STICK-SIZE`,
`F-DASH-SIZE`, `F-SPEED-FEEL`, `F-DENSITY`; desktop `F-NO-DESKTOP-REGRESS` (+
`F-SPEED-FEEL`). Priority formats: `phone_rodin_chrome`, short-height landscape,
desktop baseline. Mid-play stills for density. Desktop is baseline — do not
regress it to fix phones.

`BAD` not `none` → ship blocker → fix loop.

---

## Pre-prod critique review (required before push)

**Gate name:** critique review (**A7**). Hard stop between “capture green” and
`git push`.

**A7 is a REVIEW gate, not a capture gate.** Suite exit 0, `results.json`
`failed: 0`, `emulator_results` N/N, and A5 “105 files present” are all
**CAPTURE / presence** facts. They are required **inputs** to A7, not A7 itself.

### What you must do

1. Confirm capture prerequisites (CAPTURE_OK): expected recordings + matrix
   PNGs on disk; suite failed count 0 for the ship paths used this run.
2. Confirm **per-artifact review sidecars**: every expected matrix PNG and every
   required recording has a sibling **`*.review.json`** with `opened: true`,
   valid `verdict`, this-run `run_start_unix` (or mtime), discovery evidence
   (`frames_or_stills_opened` / `discovery_notes`), and **adversary** completed
   on ship-path artifacts (no OPEN-* left unmapped unless accepted).
3. Open **`scripts/qa_success_criteria.json`** plus **both** critique rollups.
4. Confirm **every** matrix cell and required recording has a this-run
   CRITIQUE/VIDEO line.
5. Collect every rollup line where `BAD:` is not exactly `none`, and every
   sidecar `verdict: issues` / OPEN-*.
6. **Rubber-stamp detector (required, may fan out):** re-open
   `known_fail_examples` paths and a risk-weighted sample of `all_good`
   handheld menu/mode/playing stills with the image tool. Fail A7 if examples
   still match while critiques claim clean.
7. Confirm review was **pipelined** (not a single post-capture serial skim of
   the whole matrix) — status/logs/worker history or staggered sidecar mtimes.
8. **If** capture OK, deep reviews complete, BAD lists empty, rubber-stamp clean
   → **PRE-PROD REVIEW: PASS** → Phase B immediately.
9. **Else** → **PRE-PROD REVIEW: FAIL**. Do **not** push. Fix loop:

```text
START OF FIX LOOP
  1. Patch code for every open BAD (video and/or matrix).
  2. cargo test / cargo check
  3. ./scripts/web-build.sh          # wait for finish
  4. ensure dist served
  5. A4 PIPELINE: desktop via e2e_inputs; handheld via Android emulator + adb
     screenrecord + adb shell input (CONCURRENCY=<N> as hardware allows)
     — as EACH matrix unit finishes: A4b + A6 for that unit immediately
  6. VERIFY_ONLY=1 node scripts/viewport_shots.mjs               # A5 verify
  7. Confirm every unit has *.review.json sidecars + critique lines; rewrite stale
  8. Return to this pre-prod review gate
END LOOP — until every BAD is "none" (or user-accepted in writing)
```

### Explicit prohibitions

- **Do not** push “and fix later.”
- **Do not** push after phone-only or local-dist phone smoke without phase A.
- **Do not** treat e2e / emulator **exit 0**, **N/N PASS**, or **CAPTURE_OK** as
  ship-ready or as A7 — those are capture only.
- **Do not** treat A5 “files present” as visual inspection.
- **Do not** treat rollup CRITIQUE/VIDEO alone as complete without **`*.review.json`**.
- **Do not** write `*.review.json` without opening that artifact (`opened` must be true).
- **Do not** copy `BAD: none` / `all_good` from a previous run without re-opening this-run artifacts.
- **Do not** skip A4b because A6 PNG review “looks fine.”
- **Do not** skip A6 because “videos already cover it.”
- **Do not** wait for the entire capture suite before reviewing finished units.
- **Do not** re-run a full second matrix walk when A4 already wrote all cells.
- **Do not** delete BAD lines or sidecars to silence the gate.
- **Do not** leave headless Chrome/Puppeteer orphans.
- **Do not** wait for the user after a true **PRE-PROD REVIEW: PASS** — proceed to
  Phase B (commit + push + deploy watch) per the chain rule.
- **Do not** pass A7 on checklist-only reviews without discovery + adversary.
- **Do not** create a long serial analysis-only phase after capture when reviews
  could have run in parallel during capture.

### Review checklist output (put in final report)

```text
CAPTURE: OK | FAIL   (suite exit / artifacts present — not visual)
A5_presence: OK | FAIL
sidecar_reviews: OK | FAIL   (this-run *.review.json, opened:true, frames listed)
discovery_and_adversary: OK | FAIL
open_OPEN_star_or_unmapped: N
PRE-PROD REVIEW (A7): PASS | FAIL
open_bads_video: N
open_bads_matrix: N
review_evidence: multi-frame / geometry scan this run (yes/no)
pipeline_review_during_capture: yes | no
(if FAIL) next_action: patch + full retest from suite start
(if PASS) proceeding_to: commit / push / deploy watch
```

---

## Master test matrix (main reference)

**File:** `scripts/qa_matrix.json`  
**PASS/FAIL criteria:** `scripts/qa_success_criteria.json` (required companion)

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

**Lab formats** (if present in JSON, e.g. `phone_rodin*`): still **handheld** —
Phase A ship proof uses the **Android emulator** path at that CSS size. Puppeteer
emulation is not a substitute for ship-valid lab handhelds.

Artifact: `screenshots/viewports/{format_id}_{shot_suffix}.png`

`viewport_shots.mjs` writes `screenshots/viewports/matrix_results.json` and
**exits non-zero** if any expected file is missing/empty.

**Game over capture:** shots use `http://127.0.0.1:17880/?qa_matrix=1` (default
local port; override `RUSTY_PORT` / `E2E_URL`) so the game forces Game Over after
a short play (`world::qa_matrix_force_gameover`). Normal players without that
query are unaffected.

### Why resolution criteria exist (durable rules)

1. **CSS viewport, not panel pixels** — browsers report logical size (DPR-scaled).
2. **Market share / common devices** — phones, tablets, desktops, budget laptops.
3. **Form-factor boundaries** — e.g. 1024×768 tablet vs 1366×768 laptop.
4. **DPI / OS scaling** — e.g. 1080p at 125% Windows scale.
5. **Orientation** — portrait + landscape for handhelds.
6. **High end** — QHD + 4K so UI does not become huge or sparse.

---


### Reference layout note

Script names (`e2e_inputs.mjs`, `viewport_shots.mjs`, `qa_matrix.json`) and
paths under `screenshots/` are the **reference layout** from the cycle that
proved this process. Map them to the current project. The **process** (pipeline,
multi-role deep review, sidecars, CAPTURE vs REVIEW, emulator handhelds,
baseline vs full matrix) is mandatory; exact filenames are not.

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
./scripts/web-serve-dist.sh   # http://127.0.0.1:17880/

# A4 PIPELINE — per matrix unit (format id + CSS resolution):
# Desktop / laptop (keyboard+mouse):
CAPTURE_MATRIX=1 CONCURRENCY=<N> node scripts/e2e_inputs.mjs
# Handheld / touch (REQUIRED for ship): Android emulator + adb
#   - boot AVD(s); adb reverse tcp:17880 tcp:17880
#   - full-display: adb shell screenrecord …
#   - touches: adb shell input tap|swipe … (not CDP/Puppeteer touch)
#   - same pipeline: as each unit finishes → A4b ∥ A6 immediately
# (Runner may be e2e_phone-style tooling pointed at local dist + emulator, or a
#  dedicated emulator matrix script — process rules above are authoritative.)

# → as each unit finishes:
#     recordings for that unit + viewports/{format_id}_*.png (+ stills if any)
#     IMMEDIATELY: A4b open video → recordings/{name}.review.json + VIDEO line
#                  A6  open PNGs  → viewports/{cell}.review.json + CRITIQUE line
#     (while other units still capture / review)
#     Review incomplete without per-file *.review.json

# A5 — verify matrix complete (no-op capture if A4 filled all cells)
VERIFY_ONLY=1 node scripts/viewport_shots.mjs
# If missing cells only: CONCURRENCY=<N> node scripts/viewport_shots.mjs

# A7 — PRE-PROD: every artifact has this-run *.review.json + clean rollups
```

Optional during A (debug only, **not** a ship gate): physical USB handset via
`adb reverse tcp:17880 tcp:17880`. Still must finish full A4–A7 (including **emulator**
handhelds) before push.

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
  patch → rebuild → A4 PIPELINE
    (CONCURRENCY=<N> capture; as each matrix unit finishes → A4b ∥ A6 for that unit)
  → A5 verify → A7 PRE-PROD
  ── only then push (B) ──
  ── then if phone: C live e2e_phone ──
```

If review fails, go back to **patch** — not to push. Do not ship partial green.
Fast handheld repro: Android **emulator** profile matching the failing format
(full `screenrecord` + `adb shell input`). Confirm on a **physical** phone only
after a proper phase B deploy (or local reverse for debug — not a ship gate).

---

## Physical USB phone (ADB + Chrome CDP) — phase C

**When (ship path):** authorized **physical** `adb devices` (not only an emulator)
**and** phase B Pages deploy for the commit under test has succeeded. Target =
**LIVE** Pages URL.

**When (debug only):** physical phone against local `dist` via `adb reverse`
**during** phase A. Useful for handset-specific bugs. **Does not** authorize push
and **does not** replace Phase A **Android emulator** handheld coverage.

Skip cleanly if no physical device (unless `PHONE_REQUIRE=1`).

**Why phase C exists:** a **real handset** still differs from an **emulator**
(thermal, real SoC GPU, OEM Chrome, gesture bar, display pipeline). It **adds**
confidence after local suite + deploy; it does **not** replace Phase A (including
required emulator handheld A4).

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

Review each phone cell’s **video as that cell finishes** with the same **deep
multi-role** process (R1→R2→R3, multi-frame stills, OPEN-* allowed). Do not wait
for all four cells before reviewing the first; fan out so phone capture stays
the limiter. Inventory FAILs and visual BADs are ship blockers.

---

## Phase A handheld — Android emulator (required)

For every matrix format with **`touch: true`** (phones/tablets in
`scripts/qa_matrix.json`), Phase A ship proof **must** use an **Android Virtual
Device (AVD) / Android emulator**, not desktop Chrome device-emulation alone.

### Required capture stack

| Layer | Requirement |
|-------|-------------|
| Runtime | **Android emulator** (AVD) booted and visible to `adb devices` as an emulator |
| App under test | Chrome (or system WebView browser) on the emulator loading **local** `dist` (typically `adb reverse tcp:17880 tcp:17880` → `http://127.0.0.1:17880/`) |
| Video | **Full-display** recording via **`adb shell screenrecord`** (entire emulator screen, including browser chrome / system UI as shown — not a Puppeteer CDP canvas-only screencast) |
| Input | **OS-level simulated touches** via **`adb shell input`** (`tap`, `swipe`, etc.), calibrated CSS → physical coordinates. **Do not** rely on Chrome CDP / Puppeteer touch injection for ship-valid handheld play |
| Navigate / diagnose | CDP (`scripts/cdp.mjs` or equivalent) may open URLs and evaluate JS; **input for play must stay on adb** |
| Journey | Full user chain: boot → menu (swap) → mode select (all modes + difficulties) → START → ≥20s stick+DASH play → game over when in scope |
| Matrix PNGs | Quality-hold stills per screen for that format (from the same journey when practical), written under `screenshots/viewports/{format_id}_*.png` |
| Pipeline | Same A4b ∥ A6 rules: review each unit’s video + PNGs as soon as that unit finishes |

### Explicitly insufficient for Phase A handheld

- Puppeteer `page.emulate` / `device_emulation.mjs` alone
- Resized desktop Chrome window without an emulator
- CDP/Puppeteer-synthesized touch as the only input path
- Recording only the WebGL canvas (CDP screencast) when the requirement is **full phone display** `screenrecord`

`scripts/device_emulation.mjs` may still exist for **throwaway** layout experiments; it
does **not** satisfy Phase A handheld ship criteria.

### Desktop formats (unchanged path)

Non-touch / desktop / laptop formats continue to use Chrome + Puppeteer
(`scripts/e2e_inputs.mjs`, `scripts/record.mjs` CDP screencast → webm) with keyboard
and mouse paths. Do not force those through the Android emulator.

### Concurrency

Multiple emulator instances or sequential AVD profiles may run under
`CONCURRENCY=<N>` as hardware allows (orchestrator tunes N). Prefer quality over
stacking unstable AVDs (criterion **8**).

### Artifacts (handheld)

| Kind | Typical location |
|------|------------------|
| Full-display videos | Under e2e/phone capture trees as produced by the emulator runner (e.g. `screenshots/web/e2e/recordings/` or `screenshots/web/phone/` — keep paths consistent per run and document in status) |
| Matrix PNGs | `screenshots/viewports/{format_id}_{shot_suffix}.png` |
| Critiques | `video_critique.md` + `matrix_critique.md` (same gate as desktop units) |

---

## Phase B: push (only after A7 PASS)

Only after **PRE-PROD REVIEW: PASS** (zero unaccepted BADs) **and** full A4
pipeline (every matrix unit captured and reviewed):

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
- Any matrix cell not **visually inspected** (A6) **or** missing a matrix CRITIQUE line
  **or** missing `{cell}.review.json` sidecar
- E2E not run on every format **or** A4b video review skipped / missing `video_critique.md`
  **or** missing `{recording}.review.json` sidecar
- E2E skipped full surface (not all modes/difficulties/controls or &lt;20s play)
- E2E has no video recordings
- **Only CAPTURE_OK** (suite exit 0 / N/N) without completed A4b+A6 sidecars+A7 review
- Phone/tablet tested only as resized desktop windows or Chrome `page.emulate` (no **Android emulator** path)
- Handheld A4 without **full-display** `adb shell screenrecord` and **`adb shell input`** touches
- **Pushed after physical-phone adb reverse only** (no full local matrix + e2e, including emulator handhelds)
- Physical phone connected for phase C but real-device step skipped without reason / user skip
- Phone touch inventory has unaccepted FAILs (fix locally, redeploy) — inventory PASS lines are often CAPTURE_OK; still need video review for Phase C
- **Pre-prod critique review not run, or any unaccepted `BAD` still open**
- Wrong control copy for PC/laptop vs phone/tablet
- Laptop sizes (esp. 1366×768) classified or rendered as handheld
- Never pushed after true phase A **review** PASS, or push without deploy success
- Pushed “knowing” about open BADs “to fix later”

---

## Reporting when done

1. **Capture summary (CAPTURE_OK):** suite exit codes, artifact counts — label
   explicitly as capture, not review
2. **Phase A review:** confirmation that A4b+A6 were **pipelined per matrix unit**
   (not batch-only) + paths to `video_critique.md` and `matrix_critique.md`
3. Confirmation that **all** e2e recordings **and** all matrix PNGs were **opened
   and reviewed** with **`*.review.json` sidecars** (not merely listed)
4. **PRE-PROD REVIEW: PASS|FAIL** with `sidecar_reviews` + `open_bads_video` +
   `open_bads_matrix`; residual only if user-accepted by criterion id
5. **Phase B:** commit hash + push + Pages run id/URL + **success** + live URL
6. **Phase C (if physical phone):** inventory + phone video **review** + sidecars — or “no device / skipped”
7. Confirmation that Phase A **handheld** units used **Android emulator** + full-display
   **screenrecord** + **adb shell input** (not Chrome-emulation-only)

## Related

- Matrix JSON: `scripts/qa_matrix.json`
- **Success criteria (PASS/FAIL):** `scripts/qa_success_criteria.json`
- Desktop device helpers (non-ship handheld): `scripts/device_emulation.mjs`
- Desktop recording: `scripts/record.mjs` (CDP screencast → ffmpeg)
- Handheld recording / input: **`adb shell screenrecord`**, **`adb shell input`** on **Android emulator** (Phase A) or physical device (Phase C)
- Shots (layout matrix PNGs): `scripts/viewport_shots.mjs`
- Desktop E2E: `scripts/e2e_inputs.mjs` → `screenshots/web/e2e/recordings/`
- Emulator/physical phone tooling: `scripts/e2e_phone.mjs` / `scripts/cdp.mjs` (adapt for local emulator + reverse in Phase A)
- E2E video critique: `screenshots/web/e2e/video_critique.md`
- Matrix PNG critique: `screenshots/viewports/matrix_critique.md`
- **Per-file reviews:** `screenshots/viewports/*.review.json`, `screenshots/web/e2e/recordings/*.review.json`
- Physical phone artifacts: `screenshots/web/phone/recordings/`, `touch_inventory.md`
- Input rules: sibling skill `game-input-e2e` or `app-input-e2e`
- Scale: `src/ui_scale.rs` (`ViewportClass` / `classify_viewport`)
- Pages: `.github/workflows/pages.yml`
