# Q-061: Chart crosshair and bar readout

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)
**UI direction:** [`../../design/ui-direction.md`](../../design/ui-direction.md)
**Reference register:** [`../../design/references.md`](../../design/references.md) — Quantower inspection readout, Bookmap time/price axis treatment
**Depends on:** Q-059, Q-060
**Implementation plan:** [`../plans/Q-061-chart-crosshair-and-readability-plan.md`](../plans/Q-061-chart-crosshair-and-readability-plan.md)

## Purpose

While inspecting live or historical bars, an operator needs to identify the exact bar,
time, price, and OHLC values without guessing from the axes. Add an accurate crosshair
and compact readout on top of Q-060's navigable viewport.

## Requirements

- Hovering or moving keyboard focus over the plot shows a crosshair at the nearest
  **visible** bar and the pointer price. Show that bar's actual timestamp and O/H/L/C,
  with a forming-bar indicator when appropriate. Values come from the feed, not from
  pixel-derived market calculations. Use the feed's price formatting/precision policy;
  do not hardcode two decimals for all symbols.
- The crosshair tracks pan, zoom, and target changes using Q-060's viewport. It hides
  outside the plot, on empty data, or while a target is switching. It must not make a
  previous target's values appear under a new target's identity.
- Preserve decision/fill marker hover details. If the pointer is over a marker, its
  existing tooltip remains reachable while the bar readout remains visible.
- Provide keyboard access: chart focus plus left/right arrows selects adjacent visible
  bars; Escape clears the selection. The readout and selected bar have accessible text.
  Ensure typing-to-target from Q-059 still takes priority for printable keys.
- Grid and axis styling stay subordinate to candles and trading overlays. Price and
  time labels remain legible at narrow panel sizes and the two workstation resolutions.

## Constraints and non-goals

- No volume pane, indicator calculations, drawings, order entry, or new backend route.
- Keep one feed and the existing renderer/frame budget; use theme tokens for visuals.
- Record reference adaptations in `docs/design/components.md`.

## Acceptance criteria

1. For a known bar, hover and keyboard selection show its true timestamp and exact
   O/H/L/C, at the correct bar index after pan and zoom.
2. Empty, loading, disconnect, and retarget states never show a misleading readout.
3. Marker hover details and crosshair coexist; keyboard selection is accessible and
   stops at the visible range edges.
4. Gallery captures at 1920×1080 and 2560×1440 show readable chart and narrow-panel
   states; `env -u WAYLAND_DISPLAY -u DISPLAY make check` and `make bench-frames` pass,
   with p95 below 16 ms.
