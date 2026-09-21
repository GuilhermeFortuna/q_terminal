# Q-057: True black terminal theme

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**UI direction:** [`../../design/ui-direction.md`](../../design/ui-direction.md)  
**Reference register:** [`../../design/references.md`](../../design/references.md) — Radix Colors (surface/contrast scale), Grafana and Linear (restrained status use)  
**Depends on:** Q-051  
**Implementation plan:** [`../plans/Q-057-true-black-terminal-theme-plan.md`](../plans/Q-057-true-black-terminal-theme-plan.md)

## Purpose

The terminal's current base and panel surfaces are blue-gray. The requested direction is
a pitch-black workstation canvas with neutral, legible hierarchy. Retune the existing
design tokens and component states without changing trading meaning or adding decoration.

## Requirements

- Set the main workspace/chart canvas to true black (`#000000`). Use a compact neutral
  charcoal ladder for panel chrome, controls, menus, table rows, hover/selection, and
  dialogs. No blue undertone in neutral surfaces, borders, or shadows. Keep separators
  quiet but visible and make the current panel/focus state clear.
- Preserve semantic market and operational colors: positive/negative bars, warning,
  critical, live, and disabled. Use saturated color for those meanings and focused
  controls, not large neutral surface fills. A critical condition remains identifiable
  by text/icon as well as color.
- Define accessible text hierarchy for extended use. Primary, secondary, muted,
  disabled, table header, numeric, and status text must stay readable on every mapped
  surface. Check normal text against a 4.5:1 contrast target and larger text/icons
  against a 3:1 target; document any deliberate exception.
- Update `qml/theme/Palette.qml`, `Theme.qml`, and semantic token mappings; ordinary
  screen QML continues consuming tokens only. Replace asset colors that were fixed to the
  previous blue-gray palette. Cover normal, hover, selected, focused, disabled, warning,
  and critical states in the component gallery.
- Keep chart grid lines and axes subdued but visible; candle colors stay distinct.
  Do not lower chart/data legibility to achieve the black aesthetic.

## Constraints and non-goals

- No layout, command, data-flow, or execution-semantic changes. Q-058 owns hierarchy and
  layout; Q-055 and Q-056 own chart context and controls.
- Preserve the Q-051 token gate: literals belong in `qml/theme/`, not screens.
- Q-054 captures committed baselines after this theme and the Q-058 layout settle.
  Until then, inspect gallery and screen captures directly; a test run must not rewrite
  visual expectations.

## Acceptance criteria

1. The workspace and chart canvas render as `#000000`; neutral UI surfaces have no blue
   tint at both workstation resolutions.
2. The component gallery demonstrates readable normal, hover, focus, selected, disabled,
   warning, and critical states. Colors convey no meaning alone.
3. Chart candles, grid, price/time axes, tables, and dialogs remain legible under healthy
   and degraded states. Contrast measurements meet the stated targets or list exceptions.
4. The token gate, gallery capture, and `env -u WAYLAND_DISPLAY -u DISPLAY make check`
   pass. Gallery and screen captures are reviewed before Q-054 records baselines.
