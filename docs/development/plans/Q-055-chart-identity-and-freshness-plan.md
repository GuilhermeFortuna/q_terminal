# Q-055 implementation plan: Chart identity and data freshness

**Specification:** [`../specs/Q-055-chart-identity-and-freshness-spec.md`](../specs/Q-055-chart-identity-and-freshness-spec.md)  
**Depends on:** Q-049, Q-051, Q-052

## Current system and file map

`qml/panels/ChartPanel.qml` contains `ChartPane` under a generic `PanelFrame` title.
`qml/StatusStrip.qml` has stream/history counters but no symbol. `BarFeed` already exposes
`symbol`, `timeframe`, `last_time`, `connection_state`, `stale`, `data_age_ms`, and history
errors in `src/bar_feed.rs`. `src/chart_target.rs` retargets the shared feed from the
selected deployment, with generation protection. `src/execution_models.rs` exposes the
selected deployment and its list model. `qml/theme/` owns all visual literals.

## Ordered implementation

- [x] 1. On the Q-055 task branch, add a fake-feed test in `tests/ops_views.rs` (or the
  existing chart QML harness) that checks configured and following identities with
  distinct symbol/timeframe pairs. Run that targeted test and confirm it fails.
- [x] 2. Expose a small Rust-facing chart-context projection if the existing QML model
  cannot safely supply the target source and deployment name. Keep source and pending
  retarget distinct from the feed's committed symbol/timeframe. Test its transition from
  configured to following and back in `tests/chart_target.rs`.
- [x] 3. Add `qml/components/ChartIdentity.qml` and mount it in
  `qml/panels/ChartPanel.qml`. Display actual feed symbol/timeframe, candle type, source,
  UTC last-bar time, and a condition label. Bind the empty state in
  `qml/ChartEmptyState.qml` to the same identity/condition so the labels do not disagree.
- [x] 4. Add gallery examples for live, switching, loading, stale, and disconnected
  states in `qml/gallery/`. Record the Quantower, Grafana, and MuseScore adaptations in
  `docs/design/components.md`. Inspect `make gallery-shot` PNGs and adjust spacing,
  truncation, contrast, and focus behavior.
- [x] 5. Test a mid-load deployment switch and a disconnect against the fake stream:
  after the switch, no old candles receive the new settled label; on disconnect, the
  last chart remains visible with `Disconnected` or `Stale`.
- [x] 6. Run `env -u WAYLAND_DISPLAY -u DISPLAY make check`, inspect gallery captures
  at 1920×1080 and 2560×1440, then commit focused changes on the task branch.

## Review focus

- A feed with no timestamp says `Last bar unavailable`, never zero-age live.
- A connected socket with an old bar says `Stale`, never `Live`.
- A long deployment name truncates with a tooltip while symbol and timeframe remain visible.
- Retargeting does not label previous candles as the next target.
- The bottom instrument strip and chart do not show contradictory freshness labels.
