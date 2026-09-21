# Q-055: Chart identity and data freshness

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**UI direction:** [`../../design/ui-direction.md`](../../design/ui-direction.md)  
**Reference register:** [`../../design/references.md`](../../design/references.md) — Quantower (chart context), Grafana (status hierarchy), MuseScore 4 (compact panel header)  
**Depends on:** Q-049, Q-051, Q-052  
**Implementation plan:** [`../plans/Q-055-chart-identity-and-freshness-plan.md`](../plans/Q-055-chart-identity-and-freshness-plan.md)

## Purpose

The chart currently shows candles under a generic “Chart” title. The instrument panel
reports transport and history counters, but not the symbol or timeframe. An operator
cannot tell what series is on screen, whether it follows a deployment, or whether the
latest displayed bar is current. The chart must identify its target even when it is
empty, loading, stale, or disconnected.

## Requirements

- Put a compact, persistent identity strip inside the chart panel: actual feed symbol,
  actual feed timeframe, `Candles`, target source (`Configured` or `Following: <deployment
  name>`), and last bar time with an explicit timezone. Do not derive the symbol from the
  selected row when the feed has not retargeted yet. While a retarget is in flight, show
  `Switching to <symbol> · <timeframe>` without mislabelling old candles.
- State the market-data condition separately from operations health: `Live`, `Loading`,
  `Stale <age>`, `Disconnected`, or a specific history/load error. `Live` requires a
  connected feed and fresh bars; a connected socket alone is insufficient. If the
  latest bar time is unknown, display `Last bar unavailable` rather than `0.0s`.
- Keep the target identity and reason visible in the chart's empty state. If the stream
  disconnects, retain the last rendered candles with a clear unconfirmed/stale state.
- Show the effective target and freshness in the chart panel itself; the bottom
  instrument strip may keep diagnostics but is not the only source of context.
- Use theme tokens and existing Qt Quick Controls. Chart semantics (source and freshness)
  are supplied by Rust-facing state; QML arranges and styles them. Keep keyboard focus and
  accessible names on any new interactive control.

## Constraints and non-goals

- Q-056 owns changing the target. This task is read-only.
- Preserve one shared `BarFeed`, the current deployment-follow behavior, and the
  generation guard that excludes old-target bars after retargeting.
- Do not compute market data or execution values in QML.
- Keep live trading and operations inside `BOUNDARY.md`.

## Acceptance criteria

1. With no deployment selected, the chart identifies the configured symbol and timeframe.
   With a deployment selected, it identifies that deployment and the actual feed target.
2. Across retarget, loading, empty history, live data, stale data, and disconnection, the
   identity remains visible and the condition is accurate. No previous symbol's candles
   appear under the next symbol's settled label.
3. The last-bar time includes its timezone; an unavailable timestamp is named as such.
4. At 1920×1080 and 2560×1440, the identity strip is legible without covering candles
   or clipping the target name. The component gallery includes its important states.
5. `env -u WAYLAND_DISPLAY -u DISPLAY make check` passes; the gallery is captured and
   inspected at both workstation resolutions.
