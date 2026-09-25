# Q-065 implementation plan: Readable chart axes and current-price guide

> **For implementation agents:** Read the linked spec, `AGENTS.md`, `README.md`, and
> `BOUNDARY.md`. Start this task only with `./work start Q-065 --agent <agent> --worktree`
> after Q-064 is complete and the board plan is approved. Keep changes on that task branch.

**Goal:** Make chart time and price readable across the visible viewport while keeping
the existing chart rendering and interaction budget.
**Architecture:** A pure `qml/ChartAxis.js` helper calculates bounded ticks from the
existing viewport and feed timestamps; QML places ticks and grid lines through the same
viewport transform. The feed remains the source of price precision and selected-bar
values.
**Tech stack:** Qt 6/QML, Rust CXX-Qt feed model, headless chart tests.
**Spec:** [`../specs/Q-065-readable-chart-axes-and-price-guide-spec.md`](../specs/Q-065-readable-chart-axes-and-price-guide-spec.md)

## Ordered implementation

- [ ] **1. Axis data model.** Extract bounded tick generation and formatting from
  `qml/ChartPane.qml` into pure `qml/ChartAxis.js`. Time ticks use `bar_time_at()` and
  visible indices, with a bounded number of timestamp queries;
  price ticks use visible bounds and feed precision. Define the 1/2/5 step, spacing,
  flat-range, and resize behavior explicitly. Test timestamps across midnight and an
  offset change, narrow widths, one bar, empty data, high-precision prices, and
  pan/zoom. Keep calculations off the scene graph paint path.
- [ ] **2. Axis presentation.** Replace the four endpoint labels in `ChartPane.qml`
  with major ticks, local date/time labels, an explicit `UTC±HH:MM` offset, price labels,
  and aligned subdued grid. Use theme tokens for axis width, type, spacing, and line
  color. Measure labels before placing them; keep grid and tick item counts bounded.
  Exercise markers, overlays, crosshair, and plot-edge collision cases in the gallery.
- [ ] **3. Unified readout and price guide.** Format `BarSnapshot.time` into local time
  with the same offset convention for the visual `BarReadout` and accessible text.
  Add the last-bar price line/badge, deriving its value from the existing feed snapshot
  and its status from existing freshness state. The badge reads `Last` when stale or
  disconnected and yields to a crosshair badge on overlap. Test target switch,
  loading/empty state, fresh/stale/disconnected labels, and precision consistency.
- [ ] **4. Review and handoff.** Record the Bookmap and Quantower adaptations in
  `docs/design/components.md`. Inspect gallery and chart captures at 1920×1080,
  2560×1440, and narrow width with both fresh and stale data. Run
  `env -u WAYLAND_DISPLAY -u DISPLAY make check` and `make bench-frames`, preserving
  p95 below 16 ms. Commit focused changes and set Q-065 to In Review through
  `./work board set` with check results and open follow-ups.

## Review focus

- The axis and selected-bar readout agree on local time and offset during DST changes.
- An old last-bar close is never presented as a fresh quote.
- Long or precise values do not overlap, clip, or change the candle transform.
- Grid and labels remain bounded when thousands of bars are visible.
- Crosshair and execution-marker details remain usable at axis edges.
