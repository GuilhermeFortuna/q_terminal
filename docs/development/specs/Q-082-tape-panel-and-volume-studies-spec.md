# Q-082: Tape panel and volume studies

**Status:** written spec and plan awaiting human review; status of record is the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2).
**Batch:** 13 — persistent terminal setup and live market analysis
**Depends on:** Q-077, Q-080, Q-081
**Implementation plan:** [Plan](../plans/Q-082-tape-panel-and-volume-studies-plan.md)

## Purpose

Show the current session’s trade tape and configurable volume studies beside the live chart.

## Current system

The shell has a tape placeholder and one shared stream. StudySet/OverlayChartItem already render price and oscillator studies; workspace schema becomes version 3 in Q-077. The terminal currently subscribes to bar/execution topics, not trade events.

## Required behavior

- Pin all q_core crates to the Q-081 published tag and vendor Q-079 contracts. Add a single process-level TradeFeed per active chart symbol using the existing stream connection; tape panel instances share it. A timeframe-only change rebuilds bar grouping without another source subscription.
- Subscribe/buffer trades and trades.status on the shared stream, obtain a frozen session snapshot, stream its immutable pages into VolumeState and then apply higher sequences. Deduplicate transport batches by epoch/seq and records by contracted identity, never by equal tick values. Abort and restart on token/source generation changes.
- While backfill loads, show progress and provisional/partial coverage; do not present full-session CVD as complete. Replay a retained gap; on expiry/source correction, clear derived output and load a fresh session snapshot. Display source coverage, volume field/unit and classified share separately.
- Store current-session trade history as bounded temporary columnar chunks, not unbounded per-trade Rust/QML objects. Reuse them for timeframe/threshold rebuilds; cap at 1 GiB per active feed. On capacity exhaustion keep prior UI state visibly incomplete and offer retry; no silent truncation or false session completeness.
- Activate the existing tape placeholder. Show newest-first time, price, volume, side and large-print indication through a virtualized model. Retain at most 1000 displayed rows, independent of full-session calculation history. Default minimum volume 0 and side filter All; filters affect display only, never delta or rate.
- Add delta, cumulative_delta, trade_rate and large_prints study kinds to the existing picker; retain the total maximum of eight active studies. Delta/CVD/rate have independent panes; large prints use bounded chart markers (latest 1000). Unknown prints use the neutral semantic role.
- Align trade bar bounds with chart intervals in UTC, converting naive exchange-local bar labels explicitly. Earlier sessions outside loaded tape history show unavailable gaps. Tick-based rows are never substituted for actual trade volume.
- Picker, legend and inspected-bar readout expose quantity units, parameters and coverage. Maintain the bounded current-session bar aggregate outputs for crosshair inspection and marker tooltips. Indicator computations call Q-081 only.
- Extend workspace schema to version 4 with typed volume-study parameters and per-tape-panel display filters; migrate v1/v2/v3 without erasing existing studies/layout. Persist settings through Q-077 autosave; a restart recomputes market-derived state from session history.
- Freeze market totals as stale on disconnect and show their timestamp. Warm-up, all-unknown sides, missing volume/unit, unavailable source, capacity limits and mixed generations have explicit states. Forming bar aggregates update without rebuilding completed chart geometry.
- Narrowly update BOUNDARY.md for built-in tape analytics, not user-authored formulas or execution signals. Register Quantower Time & Sales as a sourced Look entry before visual implementation; adapt its compact newest-first columns, row limit and unknown-side neutrality using Q tokens.

## Interfaces and ownership

Rust TradeFeed owns snapshot/replay, identity tracking, temporary chunks and VolumeState; QML owns arrangement.
Add src/trades/{feed.rs,history.rs,model.rs}, a typed trade sink/decoder to src/stream, and qml/panels/TapePanel.qml. Extend StudySpec with typed volume parameters rather than overloading period/num_std. Existing trade topic/schema/API metadata comes only from Q-079; calculations are Q-081’s API.

## Acceptance criteria

1. Fake session history plus live trades yields exactly the Q-081 outputs, including equal timestamps, page/live seam, duplicate replay and source correction.
2. Reconnect, missing history, 202 progress, 410 expiry and trades.status changes produce truthful coverage and rebuild behavior with no old-target values.
3. Tape has 1000 bounded virtualized rows; changing its filters leaves studies unchanged. Threshold/timeframe edits rebuild from full cached session records, not just displayed tape.
4. Volume studies/markers, units, classification coverage, legend and crosshair agree with kernel outputs; unknown-only volume has no directional recommendation.
5. Saved target, volume parameters, tape filters and layout restore automatically from v4; older workspaces migrate with studies intact.
6. Gallery captures at both resolutions are inspected; make contracts-check, offscreen make check and make bench-frames pass with tape/studies active and p95 < 16 ms.

## Implementation boundary

This issue authorizes only its listed deliverable after written-plan approval and
`./work start Q-082 --agent <agent> --worktree`. Dependencies must be Done.
Preserve the existing execution controls and research/operations ownership boundaries.
No live-order activation, new backend-process ownership or unrelated refactoring.
Use generated contracts and commit/tag pins; never edit vendored code by hand.

## References

[Quantower Time & Sales](https://help.quantower.com/quantower/analytics-panels/time-and-sales): adapt compact trade columns, bounded newest-first rows and neutral unknown-side presentation.
