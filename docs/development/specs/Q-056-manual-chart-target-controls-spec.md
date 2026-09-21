# Q-056: Manual chart target controls

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**UI direction:** [`../../design/ui-direction.md`](../../design/ui-direction.md)  
**Reference register:** [`../../design/references.md`](../../design/references.md) — Quantower (linked chart context), Qt Quick Controls (picker behavior), Linear (command discovery)  
**Depends on:** Q-055  
**Implementation plan:** [`../plans/Q-056-manual-chart-target-controls-plan.md`](../plans/Q-056-manual-chart-target-controls-plan.md)

## Purpose

The operator cannot choose a symbol or timeframe in the terminal. The chart currently
follows a deployment or the configured startup pair. Give the operator a visible way to
browse market bars independently of execution selection, with an explicit way to resume
following the deployment.

## Requirements

- The chart identity strip provides keyboard-accessible symbol and timeframe controls.
  Show the current pair as the controls' labels, even before interaction. A symbol can be
  entered directly; the picker suggests the configured symbol, deployment symbols, and
  recent successfully loaded symbols. The picker does not imply that suggestions are an
  exhaustive instrument catalog. Timeframe choices use formats accepted by the existing
  history and stream routes; show the effective value on the chart.
- Define exactly two chart modes: `Following deployment` and `Manual`. Choosing a pair
  enters Manual mode. `Follow selected deployment` returns to Following mode. With no
  deployment selected, Following mode uses the configured startup pair and says
  `Configured` in Q-055's identity strip.
- Selecting a deployment while in Manual mode changes execution detail selection but
  does not retarget the chart. Show the divergence clearly in the chart strip. Returning
  to Following mode immediately retargets to the current global deployment selection.
- An invalid/unsupported pair leaves the prior chart visible and shows a specific
  recoverable error, with the attempted pair named. An unavailable valid pair shows an
  honest no-data state after loading. Do not claim success merely because the input was
  accepted. Keep old and new targets distinct during the load.
- Retarget through the existing generation-safe path. Preserve the single shared chart
  feed and one stream subscription; all windows containing that chart reflect one target.
  Manual mode is session state and does not alter deployment, execution, or workspace
  persistence state.
- Add chart target actions to the existing command registry with noncolliding shortcuts
  and meaningful disabled reasons. Use Qt Quick Controls for picker input and focus.

## Constraints and non-goals

- No order entry, execution command, or broker symbol discovery API is added.
- No second chart or independent per-window feed is created in this task.
- The backend and `q_core` remain the authority for market and execution semantics.

## Acceptance criteria

1. A user can enter a symbol and choose a timeframe from the chart without editing TOML
   or restarting. The chart header always shows the actual displayed pair and mode.
2. Manual mode survives deployment changes during the session; returning to Following
   mode targets the current selected deployment or configured pair.
3. Rapid target changes, a delayed history response, and a stream disconnect cannot
   put bars from one pair under another pair's settled identity.
4. Invalid input and unavailable data are distinguishable, visible, and recoverable.
5. The picker and commands work by keyboard, retain visible focus, and have accessible
   names. Gallery captures at both workstation resolutions show open and error states.
6. `env -u WAYLAND_DISPLAY -u DISPLAY make check` passes.
