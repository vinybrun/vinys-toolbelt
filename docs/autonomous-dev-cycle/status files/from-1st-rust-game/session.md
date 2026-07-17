# Session status

Last update: **2026-07-17 criteria worker — fairness_usability F-* added (v5)**

## USER MANDATE
**OPEN:** Usability + fairness across formats. Desktop playability is good — **do not change desktop**. Mobile/tablet focus (resize, inputs, sizes, speeds, field area).

## Workers
| Role | subagent_id | Duty | Status |
|------|-------------|------|--------|
| Fairness metrics | `019f7201-4e13-7741-8088-da9c0cba821b` | Quantify area/size/cross-time; write fairness_usability_report.md | (orchestrator) |
| Criteria prefs | `019f7201-4e13-7741-8088-daa8c3eaaa09` | Add F-* criteria to qa_success_criteria.json + skill pointer | **DONE** (v5) |
| Mobile visual review | `019f7201-4e13-7741-8088-dab77b940a3f` | Open handheld playing shots; *.review.json + usability notes | (orchestrator) |

## Criteria worker deliverable
- `scripts/qa_success_criteria.json` **version 5** — section `fairness_usability` + `review_checklist_fairness`
- F-* ids: `F-PLAY-AREA-HANDHELD`, `F-ENTITY-CSS-SIZE`, `F-CROSS-TIME`, `F-STICK-SIZE`, `F-DASH-SIZE`, `F-SPEED-FEEL`, `F-DENSITY`, `F-NO-DESKTOP-REGRESS`
- Wired into `review_checklist_by_screen.playing` + A7 gate language
- Skill pointers: `ui-viewport-qa` A6 playing checklist; `game-input-e2e` fairness note
- Fairness report file was **not** present at write time; bands from code (`view_height_for`, chrome insets, PLAYER_SPEED)
- **Not committed** — leave dirty for orchestrator

## Constraints
- Orchestrator does **not** implement analysis/patches
- Product patches only after reports if clearly mobile-only
- Phase A fix worker from phone C may still be separate

## Next
Wait for remaining workers; then spawn mobile-only fix if reports demand it


## Worker event — fairness criteria COMPLETE
- Worker `019f7201-4e13-7741-8088-daa8c3eaaa09` **DONE**
- `qa_success_criteria.json` → **v5** with F-* fairness_usability (not committed yet)
- Ids: F-PLAY-AREA-HANDHELD, F-ENTITY-CSS-SIZE, F-CROSS-TIME, F-STICK-SIZE, F-DASH-SIZE, F-SPEED-FEEL, F-DENSITY, F-NO-DESKTOP-REGRESS
- Skill pointers in ui-viewport-qa A6 + game-input-e2e
- Usability trio: metrics DONE, criteria DONE, visual DONE
- Still: Phase A fix `019f71f8-…` for phone C BADs
- Next: after Phase A fix finishes, spawn **mobile-only feel** worker for R1–R5 (class-gated) if not already in fix scope; then commit criteria + product
