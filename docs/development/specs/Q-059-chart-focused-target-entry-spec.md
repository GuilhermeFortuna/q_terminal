# Q-059: Chart-focused symbol and timeframe entry

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)
**UI direction:** [`../../design/ui-direction.md`](../../design/ui-direction.md)
**Reference register:** [`../../design/references.md`](../../design/references.md) — Quantower chart context, Qt Quick Controls keyboard and focus behavior
**Depends on:** Q-056
**Implementation plan:** [`../plans/Q-059-chart-focused-target-entry-plan.md`](../plans/Q-059-chart-focused-target-entry-plan.md)

## Purpose

An operator looking at the live chart should be able to change its symbol or timeframe by
typing immediately, without first finding the header controls or using a shortcut. Keep
Q-056's visible picker as the discoverable alternative and reuse its manual target path.

## Requirements

- A click on the chart canvas gives it keyboard focus. Alphanumeric input while that canvas
  has focus opens a compact target prompt containing the first typed character. Never
  intercept typing in a text field, combo box, popup, command palette, or another panel.
- Accept a symbol alone (`PETR4`), a timeframe alone (`5m`), or a symbol followed by a
  timeframe (`PETR4 5m`). A single value preserves the current effective value of the
  other field. Normalize symbol case and whitespace; use the timeframe list accepted by
  Q-056. If a token could be both, prefer an exact supported timeframe match and explain
  that choice in the prompt.
- The prompt shows its parsed symbol and timeframe before applying. Enter submits through
  the existing chart context request; Escape cancels without changing the target;
  Backspace edits; ordinary text editing and focus indication come from Qt Quick Controls.
  A click away closes the prompt without applying.
- Reuse Q-056's validation, errors, generation-safe retargeting, Manual/Following modes,
  and current chart identity. A rejected pair keeps the previous settled chart visible.
  No new market-data subscription or independent chart target is created.
- The existing `Ctrl+Shift+C` symbol command and visible picker continue to work. The
  prompt has an accessible name, instructions, visible focus, and a recoverable error.

## Constraints and non-goals

- No instrument catalog, broker lookup, order entry, or persistence of the typed draft.
- No chart navigation or crosshair in this task; Q-060 and Q-061 own those changes.
- Use existing theme tokens and restyled Qt Quick Controls. Record the reference
  adaptation in `docs/design/components.md`.

## Acceptance criteria

1. With chart canvas focus, typing each of `PETR4`, `5m`, and `PETR4 5m` opens the prompt
   and Enter requests the expected pair. Escape and click-away leave the target unchanged.
2. Typing in another control or panel never opens the chart prompt. Switching windows
   does not direct typing to a hidden chart.
3. Invalid input reports the attempted pair, keeps the settled chart and identity, and
   can be corrected without reopening the prompt.
4. Rapid submitted changes keep Q-056's generation isolation and one shared feed.
5. Keyboard focus, accessible names, and open/error states are verified in gallery
   captures at 1920×1080 and 2560×1440.
6. `env -u WAYLAND_DISPLAY -u DISPLAY make check` passes.
