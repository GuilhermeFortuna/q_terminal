# Q-075: Live studies on the chart

**Status:** plan awaiting review; status of record is the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)
**Depends on:** Q-074 (q_core streaming chart studies, released tag)
**UI direction:** [`../../design/ui-direction.md`](../../design/ui-direction.md)
**Reference register:** [`../../design/references.md`](../../design/references.md). A chart-study picker entry is added by this task (see Requirements).
**Implementation plan:** [`../plans/Q-075-live-chart-studies-plan.md`](../plans/Q-075-live-chart-studies-plan.md)

## Purpose

The chart draws indicator lines only for a selected deployment, using values from the
backend chart route. For any live symbol and timeframe, the operator should be able to
add EMA, SMA, Bollinger bands, session VWAP with σ bands, RSI and ATR, and watch them
update with the forming bar during the session. Values are computed by `q_core`
streaming studies (Q-074), so the terminal shows exactly what a backtest over the same
bars computes.

## Current system

`BarFeed` (`src/bar_feed.rs`) owns the live `q-qt::BarSeries`, bar open times and HLC,
and `overlay_series: Vec<OverlaySeries>`. `set_overlays` fills that vector from the
deployment chart route. `src/execution/overlays.rs::line_layers` packs price-pane and
oscillator-pane lines into `Layer` vertex buffers in the bar viewport. `OverlayChartItem`
renders those layers in `qml/ChartPane.qml`, sharing the bar viewport. The oscillator
pane takes `OSCILLATOR_SHARE` of the height whenever an oscillator series is present.
Live bars carry the stream's `real_volume`, falling back to `tick_volume`
(`src/stream/arrow_decode.rs`); REST history (`src/history/api_client.rs`) maps the
OHLCV `volume` field to `tick_volume`; lake history reads Parquet columns.

## Requirements

- **Engine.** Depend on `q-indicators` at the Q-074 tag, alongside the other `q_core`
  crates bumped to the same tag. A per-feed study set holds one streaming state per
  active study. Completed bars commit; a forming bar previews on every forming update.
  History load, target change, timeframe change, gap recovery and a completed-bar
  replacement rebuild the states from the loaded bars; they never splice. The terminal
  computes no indicator arithmetic: it calls `q_core` and aligns outputs to bars.
- **Rendering.** Study outputs become `OverlaySeries` in the price or oscillator pane
  and reuse `line_layers` and `OverlayChartItem`. Deployment overlays and user studies
  coexist; a deployment overlay is never replaced by, or merged with, a study. RSI and
  ATR each get their own vertical scale; a fixed RSI 30/70 guide is allowed. NaN
  warm-up breaks the line; it is never drawn as zero.
- **VWAP volume.** VWAP uses one volume kind for the whole session. It uses
  `real_volume` when both the loaded history and the live bars carry it. Otherwise VWAP
  is marked unavailable for that target with a stated reason, instead of mixing kinds or
  silently using tick counts. The session key is the bar's exchange-local calendar
  day, derived from the series' time label; document the derivation.
- **Picker.** A compact chart-toolbar control adds and removes studies, with period,
  source and band multiplier where the study has them, and at most eight active
  studies per chart. Colours come from a theme study palette, not Rust literals added
  by this task. Keyboard and focus follow the Qt Quick Controls substrate. Before
  building the picker, add a sourced **Look** entry for chart-study selection and
  legends (for example Sierra Chart's Studies window or TradingView's indicator
  legend). Confirm it is good and state what is taken from it.
- **Boundary.** Update `BOUNDARY.md` narrowly: the chart may display `q_core` studies
  over its own live bars. No custom formulas, scripting, alerts on study values,
  strategy signals or optimisation.
- **Budget.** A forming update with eight active studies does not rebuild completed
  geometry for bars or studies at a fixed view. `make bench-frames` with studies
  active keeps p95 under 16 ms.

## Constraints and acceptance

- No backend route, stream topic or contract change. Studies are per chart panel
  and held in memory for now; workspace persistence, the legend and study values in
  the crosshair readout are Q-076.
- Tests with the fake stream cover: study outputs equal the batch `q_core` kernel over
  the same bars after history plus live completed bars; a forming preview that never
  leaks into committed state; rebuild on target change, timeframe change and gap
  recovery; VWAP reset at a session change; and VWAP unavailable with mixed volume kinds.
- Gallery captures show the picker, price-pane studies with a deployment overlay, and
  RSI plus ATR panes at 1920×1080 and 2560×1440. They are inspected visually.
- `make contracts-check` and `env -u WAYLAND_DISPLAY -u DISPLAY make check` pass. A
  live check at the B3 open on WIN or WDO M1 with EMA, Bollinger, VWAP and RSI shows the
  lines moving with the forming bar. It is recorded in the review note with a comparison
  against the MT5 terminal's values.
