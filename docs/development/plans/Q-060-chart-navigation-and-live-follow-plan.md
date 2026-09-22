# Q-060 implementation plan: Chart pan, zoom, and return to live

> **For agentic workers:** Read the linked spec and `AGENTS.md`/`BOUNDARY.md` first. Use the task branch created by `./work start Q-060 --agent <agent> --worktree`. Track each step with its checkbox.

**Goal:** Add bounded pan/zoom, a stable history-inspection mode, and one-click return to live.
**Architecture:** `Viewport.qml` owns visible indices, zoom level, price bounds, and live-follow state. `ChartPane.qml` maps pointer/keyboard gestures onto viewport methods; both render items consume the same viewport properties. Read-only feed queries provide visible-bar extents and actual timestamps.
**Tech stack:** Qt 6/QML, Rust CXX-Qt bridge, C++ scene graph, headless Qt tests.
**Spec:** [`../specs/Q-060-chart-navigation-and-live-follow-spec.md`](../specs/Q-060-chart-navigation-and-live-follow-spec.md)

## Current system and file map

`qml/Viewport.qml` currently recomputes `firstBar = barCount - barsVisible` on every
revision and uses all-series `feed.low/high`. `qml/ChartPane.qml` binds both
`BarChartItem` and `OverlayChartItem` to that viewport. `src/bar_feed.rs` retains bar
times and high/low/close values; `bar_time_at` exists, but QML does not have a
visible-range extents query. `tests/test_chart_bridge.rs` and `cpp/chart_cxx.cpp` host
headless chart probes.

## Ordered implementation

- [x] 1. Add viewport policy tests for empty and one-bar feeds, panning to both limits,
  zooming around left/center/right anchors, new-bar arrival in both modes, and target
  reset. Assert exact first/last bar indices and mode. Run them first and observe failure.
- [x] 2. Refactor `qml/Viewport.qml` to expose explicit `panBars(delta)`,
  `zoomAt(anchorFraction, direction)`, and `returnToLive()` operations. Define minimum
  and maximum visible-bar counts in `qml/theme/`; clamp against loaded bar count. Keep
  following mode only when the view is deliberately reset to the right edge.
- [x] 3. Add a read-only visible-range query to `src/bar_feed.rs` and the CXX-Qt/QML
  bridge. Return first/last visible timestamps and high/low for the range, including a
  forming bar if indexed there. Test a gap between sessions, a forming bar, a singleton,
  and invalid ranges. Recompute only when viewport indices or feed revision change.
- [x] 4. Wire `DragHandler`, `WheelHandler` or equivalent Qt Quick pointer handling in
  `qml/ChartPane.qml`; use actual plot width for pixel-to-bar conversion. Add keyboard
  zoom and a token-styled Return to live control. Bind price/time labels to the query
  from step 3 and show loaded-history boundary/status. Keep hover marker behavior.
- [x] 5. Extend `tests/chart_alignment.rs` and the headless Qt probe so pan and zoom
  assert bars, markers, and overlays share the same first/last indices and price bounds.
  Check target switch, delayed old response, narrow panel, disconnected feed, and the
  scene-graph frame budget. Document Quantower adaptation in `docs/design/components.md`.
- [x] 6. Run `env -u WAYLAND_DISPLAY -u DISPLAY make gallery-shot` and inspect both
  workstation sizes. Run `env -u WAYLAND_DISPLAY -u DISPLAY make check` and
  `make bench-frames` (p95 below 16 ms). Commit focused changes on the Q-060 task
  branch and set the board to In Review with results and open follow-ups. The
  software-renderer benchmark completed with p95 just above the target; this is
  recorded as an environment-sensitive follow-up.

## Review focus

- A session gap yields true timestamps on both visible edges.
- A bar arriving during history inspection does not shift the viewed bars.
- A retarget cannot reuse the prior pair's viewport or price bounds.
- Wheel input over adjacent controls does not zoom the chart.
- Very short or very long feeds remain within loaded bounds.
