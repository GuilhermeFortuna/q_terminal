# Q-065: Readable chart axes and current-price guide

**Status:** plan awaiting review; status of record is the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)
**Depends on:** Q-062, Q-063, Q-064 (Q-062 and Q-063 Done)
**UI direction:** [`../../design/ui-direction.md`](../../design/ui-direction.md)
**Reference register:** [`../../design/references.md`](../../design/references.md) — Bookmap time/price axes, Quantower chart readout
**Implementation plan:** [`../plans/Q-065-readable-chart-axes-and-price-guide-plan.md`](../plans/Q-065-readable-chart-axes-and-price-guide-plan.md)

## Purpose

The chart currently places only left/right time labels and top/bottom price labels at
the edges of a large plot. Minute labels omit the date and timezone, and the price
axis has no intermediate values. The operator should be able to read time and price
directly from the plot without opening the bar readout or guessing between endpoints.

## Requirements

- Derive a bounded set of major time ticks from actual visible bar timestamps, aligned
  to the same viewport as candles, markers, and overlays. Space labels according to
  measured text width, with no overlap or clipping; prefer useful round boundaries,
  including the start of a local day. Format times in the workstation's local timezone.
  Show the local date at day boundaries and whenever a date-only context is needed;
  show a visible `UTC±HH:MM` offset on the axis. Across a daylight-saving transition,
  repeated local times must remain distinguishable by offset.
- Derive five to seven major price ticks from the visible high/low bounds using
  a stable 1/2/5 × 10ⁿ step. Map tick lines and labels through the viewport's price
  transform, use the feed's price-formatting precision, and reserve enough right-axis
  width for the rendered labels. Avoid a range change or one forming-bar tick causing
  labels to jump unnecessarily. Handle a flat-price view and a very narrow chart.
- Draw quiet grid lines at the major ticks, subordinate to candles, execution markers,
  overlays, and the crosshair. Use at most eight time-tick and seven price-tick QML
  delegates; never create one QML object per bar. The crosshair's time/price badges remain
  legible at plot edges and do not cover axis labels.
- Format the selected bar's timestamp from its existing epoch value in the same local
  timezone and offset as the axis. Keep O/H/L/C numbers and forming status unchanged;
  update visible and accessible readouts together. Target switching, loading, empty
  data, and off-plot pointer state must not show a stale bar under a new identity.
- Add a restrained current-price horizontal guide and right-axis badge from the
  existing feed's latest bar value. It appears only when a valid latest bar exists.
  When the feed is stale or disconnected, the badge explicitly says `Last` and uses
  muted styling; it must not imply a current tradable quote. The guide never performs
  a new market calculation and never covers the crosshair price badge.

## Constraints and acceptance

- Preserve one shared feed, Q-060 navigation, Q-061 crosshair, overlays, and the Q-062
  renderer hot path. No backend route, new data topic, quote semantics, or per-candle
  QML rendering. Put visual sizes and colors in `qml/theme/`; format time from existing
  timestamps and prices through the feed's policy.
- At 1920×1080, 2560×1440, and a narrow chart, tick labels are readable and do not
  overlap. Check two dates in one viewport, a midnight boundary, a timezone-offset
  transition, one-bar and flat-price data, long/high-precision prices, pan/zoom, and
  the crosshair at all four plot edges.
- Captures show a clear hierarchy: candles first, axis values second, grid last. The
  current-price badge reads correctly for fresh and stale/disconnected states.
  Existing chart tests, the full check gate, and `make bench-frames` pass with p95 below
  16 ms.
