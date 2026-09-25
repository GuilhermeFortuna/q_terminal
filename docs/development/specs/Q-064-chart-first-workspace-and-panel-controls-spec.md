# Q-064: Chart-first workspace, panel controls, and quiet UI copy

**Status:** plan awaiting review; status of record is the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)
**Depends on:** Q-062 and Q-063 (Done)
**UI direction:** [`../../design/ui-direction.md`](../../design/ui-direction.md)
**Reference register:** [`../../design/references.md`](../../design/references.md) — Quantower workspace composition, MuseScore 4 compact desktop controls, Grafana status hierarchy
**Implementation plan:** [`../plans/Q-064-chart-first-workspace-and-panel-controls-plan.md`](../plans/Q-064-chart-first-workspace-and-panel-controls-plan.md)

## Purpose

The current first-run workspace shows the chart beside empty operations and a Tape
placeholder. A full-width compositor notice, routine feed counters, and repeated status
sentences reduce the space and attention available to the chart. Make the first-run
terminal a chart-first workstation with fast, discoverable access to Operations and Tape.
An operator who saved a different workspace must get that workspace back on restart.

## Behavior and layout

- Ship a bundled `Chart` workspace containing one window and the existing `chart` panel
  only. Use it when there is no recorded last-used workspace, including a first launch.
  Do not overwrite an existing named workspace or replace a recorded last-used layout.
  Keep `Single monitor` and `Trading` available. A saved chart workspace persists panel
  visibility through the existing window-tree format; no new workspace schema is needed.
- Keep one compact top control row with the active workspace, **Operations** and **Tape**
  buttons, and access to the command palette. Move Save, Duplicate, window arrangement,
  selection scope, and placement details into an accessible secondary menu.
  The core controls remain visible at narrow window widths. No permanent placement banner
  is shown. The menu keeps the current placement mode, explanation, restore reports, and
  compositor-rule export; it must not imply the application placed Wayland windows.
- The Operations button and `Ctrl+Shift+O` toggle the existing `status`, `deployments`,
  and `detail` panels together. Opening them from the Chart workspace docks a resizable
  group below the chart at 25% of the content height. The group
  keeps its existing internal panel functions, selection behavior, confirmations, and
  command enablement. The Tape button and `Ctrl+Shift+T` toggle the existing `tape`
  placeholder in a resizable right pane at 20% of the width. Neither
  action creates a second feed, poller, or execution store.
- A toggle hides its group if visible in any window of the active workspace; otherwise it
  opens the group in the invoking window. Hiding a group removes its panel IDs from the
  active layout, preserving the persistent panel objects for reopening. Existing saved
  layouts stay intact until the operator explicitly toggles a group. On reopening, use
  the standard dock positions above; Save persists the new arrangement. Commands that
  target a hidden Operations panel reveal Operations before focusing its action. Closing
  the last extra group returns keyboard focus to the chart. Show button state and shortcut
  in accessible names/tooltips and list both commands in the command palette. Shortcuts
  must not collide with existing registered commands or chart typing/navigation.

## Copy and status hierarchy

- The default screen shows the actual symbol and timeframe, one market-data condition,
  and any actionable problem. Remove the repeated `Configured` label in configured mode;
  retain the source when following a deployment or while a retarget is pending. Hide
  `Last bar unavailable` during ordinary initial loading; show a concise missing-data
  state if loading ends without bars. This intentionally narrows Q-055's always-visible
  source and last-bar copy while retaining its underlying facts and accessible details.
- Routine `Applied`, `Dropped`, `Gaps`, and `REST` counters move out of the primary view
  into a diagnostics disclosure. A nonzero gap or drop that affects trust is summarized
  as a problem in the chart condition, with the exact counters available in diagnostics.
  Keep connection and freshness states distinct; never label a disconnected or stale
  feed `Live`.
- When Operations is hidden, show a single compact, clickable alert chip only for a
  critical or degraded execution condition. Its text names the highest-severity cause
  (for example, `Worker unavailable`, `Unknown orders: 2`, or `Kill switch engaged`),
  and opening it reveals Operations. The chart header does not repeat the full operations
  health dashboard. Keep critical states, live lock, unknown-order count, and consequences
  of trading actions explicit inside Operations.
- Shorten verbose OpsHeader banners and no-selection/empty-state prose to a state and,
  where applicable, the next action. Keep technical cause, timestamps, and recovery
  details in a disclosure or tooltip, not in a full-width repeated sentence. The Tape
  placeholder says only `Tape data is not available yet`; it must not pretend to show
  live trades. Preserve detailed accessible text and actionable error information.

## Constraints and acceptance

- Use existing panel IDs, command registry, workspace format, and design tokens. QML owns
  presentation; Rust owns layout structure and semantic conditions. Do not add backend
  routes, data calculations, or new trading actions. Existing saved workspaces, detached
  selection, multi-window composition, and Wayland placement honesty remain valid.
- A fresh configuration opens one chart window without Operations, Tape, instrument
  strip, or placement banner. A recorded last-used workspace restores its exact panels.
  Open/close, restart, Save, switch-workspace, and multi-window cases keep panel identity
  and chart state. Existing execution commands remain reachable when Operations is hidden.
- At 1920×1080, 2560×1440, and a narrow window, the chart remains dominant and all
  core controls, critical alerts, and focus indicators remain usable. Healthy status
  avoids repetitive text; degraded status still names its cause. Inspect gallery and
  application captures against the references, then run the repository check gate.
