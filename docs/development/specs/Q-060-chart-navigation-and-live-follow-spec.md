# Q-060: Chart pan, zoom, and return to live

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)
**UI direction:** [`../../design/ui-direction.md`](../../design/ui-direction.md)
**Reference register:** [`../../design/references.md`](../../design/references.md) — Quantower chart navigation, Qt Quick pointer handlers
**Depends on:** Q-038, Q-049
**Implementation plan:** [`../plans/Q-060-chart-navigation-and-live-follow-plan.md`](../plans/Q-060-chart-navigation-and-live-follow-plan.md)

## Purpose

The live chart currently pins the viewport to the newest bar. Let an operator inspect
loaded history without live updates pulling the view away, then return to the live edge
with one clear action.

## Requirements

- Drag horizontally on the plot to pan through **loaded** bars. Wheel input over the
  plot zooms the bar count around the pointer's bar; keyboard `+`/`-` offers the same
  zoom with the viewport center as anchor. Clamp at the first and newest loaded bars,
  with a documented minimum and maximum visible-bar count. Pointer input on axes,
  markers, or overlays must not trigger an execution command.
- Model viewport mode explicitly: `Following live` advances with new bars; the first
  user pan/zoom away from the right edge enters `Inspecting history` and freezes the
  visible bar range as new bars arrive. A visible `Return to live` action resets to the
  newest bars and resumes following. Keep the mode understandable to screen readers.
- Keep bars, deployment markers, and indicators on the same horizontal and vertical
  viewport. A target change resets viewport to Following live for the new pair; stale
  history callbacks must not move the new target's viewport.
- Price bounds fit the **visible** bars, including the forming bar when visible, with
  stable padding. A live forming-bar update may expand bounds when necessary but should
  not oscillate the axis on each tick. Panning to an older region must not use the
  high/low of bars outside that region.
- Time-axis labels must use actual bar timestamps, not `first_time + index × timeframe`,
  because sessions and gaps are not continuous. The right label must reflect the last
  **visible** bar, not always the newest bar.
- Navigation is local presentation state. Keep one shared feed, subscription, target,
  and execution selection; do not persist scroll position or request extra history in
  this task. At the left boundary, make the loaded-history limit clear.

## Constraints and non-goals

- No backend/API changes, new historical pagination, drawings, or order entry.
- Respect the `qml/theme/` token rule and the q_terminal frame-time budget.
- Record the Quantower adaptation in `docs/design/components.md`.

## Acceptance criteria

1. Drag pans and wheel/keyboard input zooms predictably, with the pointer anchor stable
   within one bar and no empty range beyond loaded data.
2. New bars advance a live-following viewport. A viewport inspecting history stays on
   the same bars until Return to live is used.
3. Bars, markers, overlays, grid, price labels, and actual time labels remain aligned
   at pan/zoom extremes and after target changes.
4. Empty, one-bar, narrow-panel, and disconnected states stay usable; left boundary and
   inspecting-history status are visible and accessible.
5. Gallery captures are inspected at 1920×1080 and 2560×1440;
   `env -u WAYLAND_DISPLAY -u DISPLAY make check` and `make bench-frames` pass, with p95
   below 16 ms.
