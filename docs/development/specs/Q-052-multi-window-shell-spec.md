# Q-052: Multi-window shell over shared state

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Project direction:** [`q_contracts/docs/system-architecture.md` §4.6, §5.1, §9 invariants 1 and 9, §10 Phase 5](https://github.com/GuilhermeFortuna/q_contracts/blob/03214a0760945120c56c2af97a542fcb26da2f1e/docs/system-architecture.md#51-ui-ownership-boundary)  
**UI direction:** [`../../design/ui-direction.md`](../../design/ui-direction.md) §5, §6, §7, §9, §15  
**Reference register:** [`../../design/references.md`](../../design/references.md)  
**Depends on:** Q-051  
**Implementation plan:** [`../plans/Q-052-multi-window-shell-plan.md`](../plans/Q-052-multi-window-shell-plan.md)

## Purpose

The terminal is one `Window` that instantiates its own stores and fills itself with one
workspace. That shape is the thing the UI direction warns against locking in: a second
monitor would have to be served either by stretching one window across both, or by a
second process with its own stream subscription, its own execution state and its own idea
of what is selected.

This task replaces the single window with a shell: several top-level windows composed of
relocatable panels, all reading one set of stores owned by Rust, with one stream
subscription, one execution state and one command path however many windows are open. It
introduces the panel as the unit of composition, decides what is global and what is
per-window, and makes the whole thing work on one monitor as well as on two.

It does not yet persist anything. A workspace opened here is gone when the terminal
exits; saving, restoring and placing on physical displays is Q-053.

## Requirements

### Shared state, one subscription

- The stores — bar feed, execution models, ops status, execution controls, app info — are
  created once, owned by Rust, and handed to every window. No window creates its own.
- Opening, closing or reopening any number of windows produces exactly one stream
  connection, one health poller and one execution store. Closing a window never tears down
  state another window is using, and never cancels a subscription the remaining windows
  need.
- Every window sees the same execution state at the same revision. A deployment that
  changes lifecycle changes in both windows in the same frame.
- Closing the last window exits the application. Closing any other window does not.

### Panels and windows

- A panel is an identified, self-contained content unit — deployments, detail tables,
  chart, header, account, and the placeholders phase 5 will fill. A panel declares its
  identity, its title, its icon and whether it may appear more than once.
- A window is a composition of panels. The shell ships two named compositions: a market
  window (instrument context and chart) and an operations window (status, deployments,
  detail, account), matching the UI direction's two displays.
- Panels can be moved between windows, floated into a window of their own, docked back,
  and merged into tabs, without losing their state and without the underlying subscription
  changing.
- Panes resize by dragging, and a window remembers its own pane sizes for as long as it is
  open.

### Global versus per-window state

- Global: connection and health, execution state, risk state, account state, the kill
  switch, and the command path. These have exactly one value in the process.
- Per-window: which panels it holds, their arrangement and sizes, which tab is active, and
  scroll and expansion state.
- Instrument and deployment selection is a named context that is global by default, so
  that selecting a deployment in the operations window retargets the market window's
  chart, as Q-049 established within one window. A window may detach from the global
  context and hold its own selection; a detached window shows that it is detached.

### Commands, keyboard and focus

- Commands are registered once, with an identifier, a label, an enablement rule and a
  shortcut. Every window exposes the same set. A command's enablement is evaluated from
  global state, so a command disabled by the kill switch is disabled in every window at
  once.
- Application-wide shortcuts work from whichever window has focus. Panel-scoped shortcuts
  work only when that panel has focus. The two sets do not collide, and a collision is a
  startup error rather than a silent precedence rule.
- Focus traversal stays within a window. A dialog is modal to the window that raised it,
  never to the application, so a confirmation in the operations window never blocks the
  market window's chart.
- Every command is reachable by keyboard, and a command palette lists them with their
  shortcuts.

### One monitor

- The shell works fully with one window. Every panel is reachable, by tab or by pane, and
  no capability requires a second window to exist.
- A composition can be asked to merge: the panels of several windows collapse into one
  window's tabs and panes, preserving panel identity and state, and can be split out
  again.

### Budget

- With both compositions open, the chart live and a large execution store, p95 frame time
  stays under 16 ms per window on the target machine (§4.6). A second window costs a
  second render loop, not a second copy of the data.

## Constraints and non-goals

- **No persistence, no saved workspaces, no display placement.** Q-053. The shell must
  expose enough structure for Q-053 to serialise, and must not assume it.
- **No new panel content.** The panels are the surfaces Q-047 to Q-049 already built, plus
  empty placeholders where phase 5 will add tape, DOM and footprint. Nothing new is drawn.
- **No new data, route or topic.**
- **No second process, ever.** Multi-window is one process with one subscription. A second
  terminal process is the failure mode this task exists to prevent.
- **No research surface.** The boundary in §5.1 and `BOUNDARY.md` is unchanged: more
  windows is not permission for an editor, an optimizer or a result browser.

## Acceptance criteria

### Agent-verifiable

1. With one, two and four windows open headlessly, the fake server counts exactly one
   WebSocket subscription, one snapshot request and one health poll cadence, and the
   counts are unchanged after a window is closed and reopened.
2. An event applied to the store is visible in every open window at the same revision,
   asserted across two windows in one test.
3. Closing a non-last window leaves every remaining window functional, with its stores
   intact; closing the last window exits. Both are asserted headlessly.
4. A panel moved from one window to another keeps its identity and its state — selection,
   scroll position, active tab — asserted before and after the move.
5. Detaching a window from the global selection context gives it an independent selection,
   and the global context is unaffected; reattaching adopts the global selection.
6. The command registry is enumerable, every command has a label, an enablement rule and a
   unique shortcut, and a duplicate shortcut fails at startup. A command disabled by the
   kill switch reports disabled from every window.
7. Merging two compositions into one window preserves every panel and its state, and
   splitting them out restores the arrangement, asserted headlessly.
8. p95 frame time per window with both compositions open, the chart live and
   `BENCH_EXECUTION_ROWS=10000`, stays under 16 ms.
9. Every Q-035 to Q-051 test passes, including the token gate and the ops suites.
10. `BOUNDARY.md` describes the shell and restates that window count does not widen scope.
11. `make check` passes.

### Human-verifiable

1. Both compositions run for thirty minutes with a live worker and two paper deployments,
   one window per monitor, and the states match the backend's snapshot at every check.
   Commands issued from either window take effect once.
   Command: `make run`
2. Every §8.1 degraded state is produced with both windows open, and both windows show it
   consistently. Screenshots of both windows in a healthy state and in a worker-down state
   are recorded.
3. The whole workspace is driven for ten minutes from the keyboard alone, across both
   windows, including a confirmation dialog and the command palette.
4. The single-monitor path is used for a full session on one display, and nothing is
   unreachable.
