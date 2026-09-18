# Q-049 implementation plan: Deployment chart with markers

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Specification:** [`../specs/Q-049-deployment-chart-with-markers-spec.md`](../specs/Q-049-deployment-chart-with-markers-spec.md)  
**Depends on:** Q-047

## Current-system context

The chart is the Q-037 and Q-038 slice. `cpp/bar_chart_item.*` hosts
`cpp/bar_chart_node.*`, a `QSGRenderNode` that uploads bar vertices
(`bar_vertex.h`) from a `q_core` `BarSeries`. `qml/Viewport.qml` owns the
live-edge and sticky-price policy, and `qml/ChartPane.qml` draws the axes and
grid. `src/bar_feed.rs` (`BarFeed`) binds the series, history and stream
state, with `feed_set_symbol`, `feed_set_timeframe` and `feed_setup_and_load`
already exposed through `src/chart_bridge.rs` for tests.
`src/history/controller.rs` runs the catalog-driven load (Q-036) for one
symbol and timeframe. The stream client's bar `TopicState`s filter by symbol
and timeframe (`TopicFilter::Bars` after Q-046). `cpp/bar_chart_probe.*` and
`cpp/frame_bench.*` give headless vertex inspection and the frame benchmark.
C++ is limited to scene-graph buffer movement (`BOUNDARY.md`,
architecture §5).

After Q-047 the workspace has a selected deployment (`ExecutionModels.selected_deployment_id`),
and the store has per-deployment recent decisions and fills. The backend's
`GET /api/v1/execution/deployments/{id}/chart` returns `bars` and `indicators`
(`key`, `label`, `pane: price|oscillator`, `color`, `values` aligned to
`bars`) over the evaluator's window. The frontend's placement rules are in
`q_frontend/src/workspaces/execution/liveChartMarkers.ts`. A decision is
matched to the bar whose open equals its close time, or else to
`close − timeframe`. A fill goes on the greatest bar open that is at or before
the fill time. `hold` produces no marker.

## Interfaces produced

```rust
// src/chart_target.rs   (new)
pub struct ChartTarget { symbol: String, timeframe: String, generation: u64 }
pub fn retarget(feed: &mut BarFeedRust, stream: &StreamClient, target: ChartTarget);
    // bumps generation; cancels the history load; swaps the bar TopicFilter; drops any delivery with an older generation

// src/execution/markers.rs   (new; pure)
pub enum MarkerKind { Buy, Sell, Close, Fill }
pub struct Marker { bar_open_ms: i64, price: f64, kind: MarkerKind, label: String }
pub fn decision_markers(decisions: &[DecisionState], bar_opens: &[i64], tf_ms: i64) -> Vec<Marker>;
pub fn fill_markers(fills: &[FillState], bar_opens: &[i64]) -> Vec<Marker>;
    // prices parsed from decimal strings for display placement only

// src/execution/overlays.rs   (new)
pub struct OverlayFetcher;   // on bars.completed for the selected target: one GET .../chart; aligns values to bar opens
```

```
cpp/marker_node.h/.cpp        new: QSGRenderNode drawing marker glyph quads from a vertex buffer (buffer movement only)
cpp/line_series_node.h/.cpp   new: QSGRenderNode drawing overlay polylines from a vertex buffer
cpp/marker_chart_item.*, cpp/overlay_chart_item.*   QQuickItems sharing the Viewport transform with BarChartItem
qml/ChartPane.qml             stacks bars, overlays and markers; oscillator pane below when present; hover tooltip
tests/chart_target.rs         new: criterion 1 (including switch mid-load)
tests/markers.rs              new: criterion 2 (vectors ported from liveChartMarkers.ts), criterion 3
tests/overlays.rs             new: criterion 4
tests/chart_alignment.rs      new: criterion 5 via bar_chart_probe
src/main.rs                   --bench-frames gains --markers N --overlays N
```

## Implementation decisions

- **Retarget by generation.** Every history load and every bar delivery carries
  the target generation it was requested for. `BarFeed` drops anything older than
  the current one. That is the simplest rule that makes "no bar of the previous
  symbol after a switch" hold even when a load or a stream frame is in flight at
  the moment of the switch. The stream connection stays up. Only the bar filters
  change, and the bar topics re-snapshot for the new key.

- **Markers and overlays are render nodes, not QML items.** Two thousand
  `Repeater` delegates would spend the frame budget on item layout. Two small
  nodes that consume vertex buffers built in Rust keep the §4.6 rule (columnar,
  one upload per frame), and keep C++ to buffer movement. Their items read the
  same viewport transform properties as `BarChartItem`, so alignment is
  structural, not re-computed.

- **Placement rules are ported from `liveChartMarkers.ts` as test vectors
  first.** They are presentation rules, not trading semantics, so a terminal
  copy does not break invariant 1. The vectors make the frontend's behaviour the
  oracle while it still exists. Q-050 deletes the frontend copy.

- **Overlays refresh by one REST call per completed bar.** The chart route
  recomputes over the evaluator window with `q_core`, which is exactly the
  values the strategy saw. At M1 that is one request a minute per selected
  deployment. The spec's non-goal records a streamed topic as the alternative, to
  be justified by a measurement.

- **Prices for placement are parsed from decimal strings to `f64` only at the
  vertex boundary.** The store keeps strings (Q-046). Placement needs a screen
  coordinate, not an exact amount.

## Ordered implementation

- [ ] 1. Work on the branch `Q-049-deployment-chart-with-markers` in `q_terminal`,
   created from `development` by `./work start`. Confirm Q-047 is merged.
- [ ] 2. Port the placement rules into `tests/markers.rs` as vectors, including
   close-time and open-time conventions, a fill inside a bar, a fill before the
   first bar, and holds. Implement `markers.rs`. Confirm they pass. Commit.
- [ ] 3. Implement `ChartTarget` and `retarget`, and write `tests/chart_target.rs`
   (criterion 1, including a switch during a slow fake history load and during
   a burst of stream frames). Wire the selection to retarget. Commit.
- [ ] 4. Write `marker_node` and `marker_chart_item`, feed them from the store's
   revision for the selected deployment, and test criterion 3 headlessly.
   Commit.
- [ ] 5. Write `OverlayFetcher`, `line_series_node` and `overlay_chart_item`, add
   the oscillator pane to `ChartPane.qml`, and write `tests/overlays.rs`.
   Commit.
- [ ] 6. Write `tests/chart_alignment.rs` with the probe across a pan and two zoom
   levels. Commit.
- [ ] 7. Add hover tooltips (decision reason; fill side, quantity, price, time).
   Keep `qmllint` clean. Commit.
- [ ] 8. Extend `--bench-frames` with markers and overlays, and record p95.
   Commit.
- [ ] 9. Tick the chart rows of `docs/ops-parity.md`, and update `README.md`.
   Run `env -u WAYLAND_DISPLAY -u DISPLAY make check`. Fix, re-run, commit.
- [ ] 10. **Human:** human-verifiable criteria 1 and 2.

## Validation

- **Unit:** marker placement vectors; generation filtering; overlay alignment.
- **Integration (headless):** retarget during load and stream; markers from
  stream deliveries; overlay refresh on bar completion; alignment through the
  probe.
- **Performance:** `bench-frames` with 2 000 markers and two overlays.
- **Regression:** every Q-035 to Q-048 test; `qmllint`.
- **Manual:** switching between two live deployments; side-by-side screenshot
  with the frontend.

```bash
cd /home/gui/projects/q/q_terminal
env -u WAYLAND_DISPLAY -u DISPLAY make check
RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo test --test markers --test chart_target --test overlays --test chart_alignment
cargo run --release -- --bench-frames --markers 2000 --overlays 2 --bars 500000
```

## Handoff

Give the placement vectors and confirm they match the frontend's rules. Report
the switch-mid-load test and its generation counts. Give the frame benchmark
with markers and overlays (p50, p95, p99). From the human steps, give the
switch observations, the ten-minute p95, and the side-by-side screenshot.
