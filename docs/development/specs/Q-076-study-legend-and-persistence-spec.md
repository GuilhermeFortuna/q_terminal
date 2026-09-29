# Q-076: Study legend, readout and workspace persistence

**Status:** plan awaiting review; status of record is the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)
**Depends on:** Q-075
**UI direction:** [`../../design/ui-direction.md`](../../design/ui-direction.md)
**Reference register:** [`../../design/references.md`](../../design/references.md). Uses the chart-study entry added by Q-075.
**Implementation plan:** [`../plans/Q-076-study-legend-and-persistence-plan.md`](../plans/Q-076-study-legend-and-persistence-plan.md)

## Purpose

After Q-075 the operator can add studies, but they cannot read their values without
guessing from the axis, and a restart loses them. Show each study's current value in a
compact on-chart legend and in the crosshair readout, allow editing and removal from
the legend, and save the study set with the workspace.

## Requirements

- **Legend.** A top-left legend on each pane lists its active studies in the form
  `EMA 21 · 128 455`. It shows the latest value (the forming bar when present) and,
  while the crosshair is active, the value at the inspected bar. Values use the feed's
  price precision for price-pane studies and a fixed precision for oscillators. A study
  in warm-up shows `—`; VWAP unavailable shows its reason in short form. Each row can
  hide, edit or remove its study. The legend never covers the crosshair badges.
- **Readout.** The crosshair bar readout and its accessible text include study values
  for the inspected bar, from the same source as the legend. Target switching or
  loading never shows a previous target's study values.
- **Persistence.** The study set of each chart panel (kind, parameters, visibility,
  palette index) is saved in the workspace file and restored on load. The workspace
  schema version increases, with a migration from version 1 that yields no studies.
  Unknown study kinds or invalid parameters in a file are dropped with a logged
  warning, never a failed load. Studies belong to the panel, not the symbol, and
  survive a target change.

## Constraints and acceptance

- No new indicator, arithmetic, route or contract. Colours and sizes come from
  `qml/theme/`.
- Tests cover schema migration from version 1, round-trip of a saved study set, invalid
  entries dropped on load, legend and readout values equal to the study outputs at the
  latest and the inspected bar, and no stale values across a target switch.
- Gallery captures of the legend (live, crosshair, warm-up and VWAP unavailable) at
  1920×1080 and 2560×1440 are inspected visually.
- `env -u WAYLAND_DISPLAY -u DISPLAY make check` and `make bench-frames` pass with p95
  under 16 ms.
