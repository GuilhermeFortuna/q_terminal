# Q-061 implementation plan: Chart crosshair and bar readout

> **For agentic workers:** Read the linked spec and `AGENTS.md`/`BOUNDARY.md` first. Use the task branch created by `./work start Q-061 --agent <agent> --worktree`. Track each step with its checkbox.

**Goal:** Expose exact bar values through pointer and keyboard inspection without disturbing chart navigation.
**Architecture:** The feed supplies an indexed read-only bar snapshot; `ChartPane.qml` maps pointer or keyboard selection through Q-060's viewport and renders crosshair/readout. All values and time are read from the selected bar.
**Tech stack:** Rust CXX-Qt, Qt 6/QML, headless chart tests.
**Spec:** [`../specs/Q-061-chart-crosshair-and-readability-spec.md`](../specs/Q-061-chart-crosshair-and-readability-spec.md)

## Current system and file map

`src/bar_feed.rs` retains time and H/L/C per bar but only exposes bar time to C++ probes.
`qml/ChartPane.qml` has a `HoverHandler` for marker tooltips, fixed top/bottom price
labels, and no bar readout. Q-060 supplies the viewport range and actual time-axis
labels; Q-059 supplies chart-focused printable-key behavior.

## Ordered implementation

- [x] 1. Write feed/query tests for completed and forming bars, first and last indices,
  invalid indices, target reset, and exact decimal formatting. Run first and verify the
  missing indexed readout API fails these tests.
- [x] 2. Expose one indexed, read-only bar snapshot from `src/bar_feed.rs` to QML via
  the existing bridge. Include timestamp, O/H/L/C, forming status, and validity. Keep
  the representation bounded to one selected bar, with no bulk copy on every pointer
  move. Define precision from the existing feed/symbol policy or, if none exists,
  document and test a value-preserving decimal display rule before wiring the UI.
- [x] 3. Add crosshair and compact readout to `qml/ChartPane.qml` using theme tokens.
  Map pointer x to the nearest visible index and y to the viewport price; use the
  indexed query for time and O/H/L/C. Hide it for empty data and while retargeting.
  Keep marker tooltip hit testing and its existing detail text reachable.
- [x] 4. Add left/right/Escape keyboard behavior on the focused chart canvas, clamped
  to Q-060's visible range, without consuming Q-059 printable input. Provide accessible
  selected-bar text and test focus transfer to/from header controls.
- [x] 5. Extend `cpp/chart_cxx.cpp`, `tests/test_chart_bridge.rs`, and gallery states to
  verify bar-index mapping before/after pan/zoom, retarget clearing, empty/disconnected
  behavior, marker overlap, keyboard boundaries, and narrow-panel layout. Document
  Quantower/Bookmap adaptations in `docs/design/components.md`.
- [x] 6. Run `env -u WAYLAND_DISPLAY -u DISPLAY make gallery-shot`, inspect both
  workstation sizes, run `env -u WAYLAND_DISPLAY -u DISPLAY make check`, and
  `make bench-frames` (p95 below 16 ms). Commit focused changes on the Q-061 task
  branch and set the board to In Review with results and open follow-ups.

## Review focus

- A target switch clears prior pair values before the new bar is read.
- A forming bar is labelled as forming, not completed.
- Marker tooltip remains usable under the crosshair.
- Keyboard selection never escapes the visible range.
- Price formatting preserves meaningful precision for non-two-decimal symbols.
