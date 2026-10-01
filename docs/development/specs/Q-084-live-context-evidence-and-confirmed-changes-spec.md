# Q-084: Live context, evidence and confirmed changes

**Status:** written spec and plan awaiting human review; status of record is the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2).
**Batch:** 13 — persistent terminal setup and live market analysis
**Depends on:** Q-077, Q-082, Q-083
**Implementation plan:** [Plan](../plans/Q-084-live-context-evidence-and-confirmed-changes-plan.md)

## Purpose

Explain the active chart’s current market context with editable settings and confirmed changes.

## Current system

Batch 12 supplies study picker/legend/crosshair. Q-077 adds autosave, Q-082 adds v4 persisted volume tools and tape coverage, and Q-083 supplies pure context outputs. The context section follows the chart target without becoming a research editor.

## Required behavior

- Pin all q_core crates to one published tag containing both Q-081 and Q-083, whichever task is released last. Maintain one ContextState per active chart, independent of visible StudySet settings. Hidden/removed chart studies do not affect analysis.
- Add a collapsible context section within the chart, not another shell panel. Summary has four separate trend/momentum/volatility/VWAP readings; expanding exposes values, thresholds, readiness and the selected target. Show Q-082 volume coverage beside the volume evidence, without blending it into a score.
- Default profile and exact category semantics are Q-083’s. The settings popover edits those periods/thresholds with Apply/Cancel; valid Apply increments an analysis revision and replays current chart history, previews the forming bar and saves automatically. Invalid edits change nothing and show field errors.
- ContextState commits completed bars and previews forming bars. Rebuild on history load, target/timeframe change, corrected completed bars, gap recovery and config edits. Avoid whole-history recomputation on each forming update; at most one view projection per UI frame.
- Latest readings show Provisional when a forming bar exists and Confirmed otherwise. Crosshair inspection projects the output for that bar, with bar time and analysis revision. Switching targets/loading never shows the old target’s values.
- Correct the shared session-day helper used by studies/context: convert TimeLabel::Utc to America/Sao_Paulo before extracting the date; exchange-local naive labels are already local. Add timezone-aware conversion using chrono/chrono-tz, not a hard-coded offset. Keep the calendar-day session policy from Q-080. Missing/mixed actual volume makes VWAP unavailable with the reason.
- Retain at most 100 confirmed change events in memory for the current target/config revision. Track changes in trend category, RSI zone, volatility category and VWAP side/extension. One event per changed family per completed bar, ordered trend → momentum → volatility → VWAP, with before/after evidence, bar time and revision.
- Suppress events on initial warm-up, unavailable-to-ready transitions, forming previews, history replay and rebuild. A rebuild clears the event list and seeds the last complete categories. Duplicate bar/stream delivery creates no event. There are no notification sounds, desktop alerts or execution commands.
- On disconnect or stale bars keep last values with stale status/time; disable new confirmed events until recovery establishes continuity. Other families remain inspectable when VWAP or tape coverage alone is unavailable.
- Extend workspace schema to version 5 with typed per-chart analysis profile and collapsed state; migrate v1–v4 retaining targets, price/volume studies and tape settings. Reset invalid profiles to defaults with a warning while retaining the rest of the workspace.
- Use the registered TradingView indicator-legend pattern for compact name/value/evidence hierarchy and MuseScore inspector pattern for grouped settings. Use existing theme tokens and Qt Quick Controls; record sourced adaptations before coding.
- Narrow BOUNDARY.md to permit built-in explainable live context and its in-memory confirmed change list. Exclude strategy authoring, custom scripts, optimization, model calls, multiple-timeframe feeds and order recommendations.

## Interfaces and ownership

Rust ContextController owns ContextState, per-bar outputs, analysis revision and confirmed events; QML owns section/layout/editing.
Add src/context/{state.rs,controller.rs,events.rs}, expose typed profile validation and projected current/at-bar outputs through CXX-Qt, and add qml/components/MarketContextSection.qml. Use Q-083 types for category/evidence, Q-082 coverage and Q-077 autosave; do not duplicate classifications in QML.

## Acceptance criteria

1. Context latest/inspected values equal Q-083 outputs for history plus live bars; forming previews never mutate committed output or create events.
2. Repeated updates, replay, corrected bars, target switches and settings edits produce neither stale readings nor historical event bursts. Real new-bar category changes produce one correctly ordered event per family.
3. Apply/Cancel, invalid profiles, analysis revisions and all v1–v4 migrations preserve the correct settings; close/reopen restores v5 configuration automatically at live prices.
4. UTC/local session boundaries, unavailable VWAP, partial tape, zero ATR, warm-up and disconnected/stale states show accurate evidence/status without affecting execution.
5. Gallery captures show summary, evidence, settings errors, inspected bar and degraded states at both resolutions and are visually inspected.
6. Offscreen make check and make bench-frames pass with context plus eight chart studies and tape active, p95 < 16 ms.

## Implementation boundary

This issue authorizes only its listed deliverable after written-plan approval and
`./work start Q-084 --agent <agent> --worktree`. Dependencies must be Done.
Preserve the existing execution controls and research/operations ownership boundaries.
No live-order activation, new backend-process ownership or unrelated refactoring.
Use generated contracts and commit/tag pins; never edit vendored code by hand.
