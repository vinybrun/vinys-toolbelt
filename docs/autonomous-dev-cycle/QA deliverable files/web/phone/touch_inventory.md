# Phone touch inventory (real device) — 2×2 matrix

- at: 2026-07-11T07:39:14.658Z
- url: https://intrusting-games.github.io/rusty-dasher/
- model: 2412DPC0AG
- cells: portrait_browsing, portrait_fullscreen, landscape_browsing, landscape_fullscreen

## Matrix

| | browsing (Chrome UI) | fullscreen |
|--|----------------------|------------|
| **portrait** | `portrait_browsing` | `portrait_fullscreen` |
| **landscape** | `landscape_browsing` | `landscape_fullscreen` |

Orientation forced via `adb` (`accelerometer_rotation=0`, `user_rotation`).
Fullscreen via `document.documentElement.requestFullscreen()` after load.

## Criteria

- Min hit target: 48 CSS px (fatty-finger)
- Min gap stick/dash: 12 CSS px
- Whole-phone screencaps: `screenshots/web/phone/{cell}_*.png`
- Input: Chrome CDP on real device (no Puppeteer)

## Inventory

| Cell | Screen | Control | Worked | Fatty-finger | Notes |
|------|--------|---------|--------|--------------|-------|
| portrait_browsing | playing(layout) | virtual_stick | geometry | good | 375x691 center=(105,573) hitR=63.9 |
| portrait_browsing | playing(layout) | dash_button | geometry | good | center=(281,573) hitR=43.5 gap=68.8 |
| portrait_browsing | boot | dismiss CTA | yes | large — good |  |
| portrait_browsing | menu | primary confirm | no event | center — good |  |
| portrait_browsing | mode_select | START | tapped → playing expected | START band | play CSS 375x691 portrait_layout=true |
| portrait_browsing | playing | virtual stick | no events | good | events=0 |
| portrait_browsing | playing | DASH | no events | good |  |
| portrait_browsing | playing | multi stick+DASH | weak | gap ok | gap=68.8 |
| portrait_fullscreen | playing(layout) | virtual_stick | geometry | good | 375x834 center=(105,692) hitR=63.9 |
| portrait_fullscreen | playing(layout) | dash_button | geometry | good | center=(281,692) hitR=43.5 gap=68.8 |
| portrait_fullscreen | boot | dismiss CTA | yes | large — good |  |
| portrait_fullscreen | menu | primary confirm | no event | center — good |  |
| portrait_fullscreen | mode_select | START | tapped → playing expected | START band | play CSS 375x834 portrait_layout=true |
| portrait_fullscreen | playing | virtual stick | no events | good | events=0 |
| portrait_fullscreen | playing | DASH | no events | good |  |
| portrait_fullscreen | playing | multi stick+DASH | weak | gap ok | gap=68.8 |
| landscape_browsing | playing(layout) | virtual_stick | geometry | good | 747x279 center=(74,145) hitR=69.2 |
| landscape_browsing | playing(layout) | dash_button | geometry | good | center=(672,145) hitR=52.6 gap=475.8 |
| landscape_browsing | boot | dismiss CTA | yes | large — good |  |
| landscape_browsing | menu | primary confirm | no event | center — good |  |
| landscape_browsing | mode_select | START | tapped → playing expected | START band | play CSS 747x279 portrait_layout=false |
| landscape_browsing | playing | virtual stick | no events | good | events=0 |
| landscape_browsing | playing | DASH | no events | good |  |
| landscape_browsing | playing | multi stick+DASH | weak | gap ok | gap=475.8 |
| landscape_fullscreen | playing(layout) | virtual_stick | geometry | good | 834x375 center=(83,195) hitR=89.9 |
| landscape_fullscreen | playing(layout) | dash_button | geometry | good | center=(750,195) hitR=63.8 gap=513.5 |
| landscape_fullscreen | boot | dismiss CTA | yes | large — good |  |
| landscape_fullscreen | menu | primary confirm | no event | center — good |  |
| landscape_fullscreen | mode_select | START | tapped → playing expected | START band | play CSS 834x375 portrait_layout=false |
| landscape_fullscreen | playing | virtual stick | no events | good | events=0 |
| landscape_fullscreen | playing | DASH | no events | good |  |
| landscape_fullscreen | playing | multi stick+DASH | weak | gap ok | gap=513.5 |

## E2E results

- PASS **devtools**: Chrome/149.0.7827.200
- PASS **portrait_browsing/orientation**: 375x691
- PASS **portrait_browsing/browsing**: not fullscreen
- PASS **portrait_browsing/boot**: 
- FAIL **portrait_browsing/stick**: []
- FAIL **portrait_browsing/dash**: 
- FAIL **portrait_browsing/multi**: 
- PASS **portrait_fullscreen/orientation**: 375x834
- PASS **portrait_fullscreen/fullscreen**: active
- PASS **portrait_fullscreen/boot**: 
- FAIL **portrait_fullscreen/stick**: []
- FAIL **portrait_fullscreen/dash**: 
- FAIL **portrait_fullscreen/multi**: 
- PASS **landscape_browsing/orientation**: 747x279
- PASS **landscape_browsing/browsing**: not fullscreen
- PASS **landscape_browsing/boot**: 
- FAIL **landscape_browsing/stick**: []
- FAIL **landscape_browsing/dash**: 
- FAIL **landscape_browsing/multi**: 
- PASS **landscape_fullscreen/orientation**: 834x375
- PASS **landscape_fullscreen/fullscreen**: active
- PASS **landscape_fullscreen/boot**: 
- FAIL **landscape_fullscreen/stick**: []
- FAIL **landscape_fullscreen/dash**: 
- FAIL **landscape_fullscreen/multi**: 

## Summary

- results: 13/25 pass
- inventory: 16/32 pass
- open_bads: 28
