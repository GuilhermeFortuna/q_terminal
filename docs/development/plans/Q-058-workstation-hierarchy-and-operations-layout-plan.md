# Q-058 implementation plan: Workstation hierarchy and operations layout

**Specification:** [`../specs/Q-058-workstation-hierarchy-and-operations-layout-spec.md`](../specs/Q-058-workstation-hierarchy-and-operations-layout-spec.md)  
**Depends on:** Q-055, Q-057

## Current system and file map

`src/shell/layout.rs::composition("merged")` supplies the default split weights.
`qml/shell/ShellWindow.qml` stacks `PlacementNotice`, `WorkspaceMenu`, and a second
`Toolbar` before `PanelHost`. `qml/OpsHeader.qml` places long badges in one horizontal
row; the screenshot shows clipping. `qml/DeploymentList.qml` and
`qml/DeploymentDetail.qml` own their empty/no-selection states. `qml/shell/PanelHost.qml`
and the workspace controller persist panel arrangement.

## Ordered implementation

- [ ] 1. On the Q-058 task branch, add layout tests in `tests/shell_windows.rs` for the
  new merged weights, all panels present, resizability, and restoration of a saved
  pre-Q-058 window tree. Run the targeted tests and confirm the new expectations fail.
- [ ] 2. Change only the default merged composition in `src/shell/layout.rs` so the chart
  receives the majority of the right pane and the detail pane starts shorter. Preserve
  panel IDs and loaded workspace layouts. Test split/merge and old-workspace restoration.
- [ ] 3. Combine workspace and shell actions into one compact toolbar in
  `qml/shell/ShellWindow.qml` and `WorkspaceMenu.qml`. Keep active workspace and the
  command palette visible. Convert `PlacementNotice.qml` to a one-line status with a
  keyboard-operable disclosure containing the explanation and export button.
- [ ] 4. Refactor `qml/OpsHeader.qml` into a severity-first status summary plus a details
  view that exposes every existing health field and action. Render `unknown` when worker
  heartbeat is unavailable. Add healthy, offline, reconciliation, kill-switch, and
  live-locked examples to the gallery and check for clipping at both resolutions.
- [ ] 5. Update `qml/DeploymentList.qml` and `qml/DeploymentDetail.qml` empty states:
  show the next action, hide empty table headers/load-older affordances when no selection,
  and restore them when selected. Verify with fake models and the existing operations
  tests that pagination and command enablement are unchanged.
- [ ] 6. Record Quantower, MuseScore, Wireshark, and Grafana adaptations in
  `docs/design/components.md`. Inspect merged, market, and operations screenshots at
  1920×1080 and 2560×1440; retain captures for Q-054's later baseline work. Run
  `env -u WAYLAND_DISPLAY -u DISPLAY make check`, then commit focused
  changes on the task branch.

## Review focus

- Narrow operations panel does not clip a critical badge or action.
- Unavailable heartbeat is labelled unknown, never displayed as zero seconds.
- A no-selection screen keeps the chart usable and offers a clear deployment action.
- Existing saved workspaces and detached selection semantics still work.
- Destructive controls retain confirmation and their current enablement rules.
