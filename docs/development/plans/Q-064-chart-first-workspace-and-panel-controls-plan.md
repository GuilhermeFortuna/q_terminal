# Q-064 implementation plan: Chart-first workspace, panel controls, and quiet UI copy

> **For implementation agents:** Read the linked spec, `AGENTS.md`, `README.md`, and
> `BOUNDARY.md`. Start this task only with `./work start Q-064 --agent <agent> --worktree`
> after the board plan is approved. Keep changes on that task branch.

**Goal:** Start new installations on the chart and make Operations, Tape, placement
details, and diagnostics available without filling the default screen with panel chrome
or repetitive messages.
**Architecture:** Keep the existing Rust shell layout and command registry as the source
of panel visibility. QML renders a compact toolbar and semantic summaries; the same
panel objects and shared stores serve every arrangement.
**Tech stack:** Rust, CXX-Qt, Qt 6/QML.
**Spec:** [`../specs/Q-064-chart-first-workspace-and-panel-controls-spec.md`](../specs/Q-064-chart-first-workspace-and-panel-controls-spec.md)

## Ordered implementation

- [ ] **1. First-run workspace.** In `src/workspace/schema.rs` add the bundled `Chart`
  workspace with a single `chart` node; in `src/workspace/store.rs` select it only when
  `last_used` is absent. Keep the existing bundled workspaces and versioned schema.
  Cover fresh config, recorded last-used layout, unreadable last-used fallback, and
  workspace round-trip in `tests/workspaces.rs` and store tests. Confirm old workspace
  files restore unchanged.
- [ ] **2. Group visibility and commands.** Add shell layout operations that insert or
  remove the three Operations IDs as one group below the chart and `tape` at the right;
  prune empty splits without dropping any other panel. Expose toggle and visible-state
  methods through `src/shell_controller.rs`. Register `operations.toggle` on
  `Ctrl+Shift+O` and `tape.toggle` on `Ctrl+Shift+T` in `src/shell/commands.rs` and route
  them in `qml/Main.qml`. A visible group anywhere is hidden; a hidden group opens in the
  invoking window. Have execution commands reveal hidden Operations before forwarding.
  Test command uniqueness, docking weights, repeated toggles, existing multi-window
  layouts, save/restore, and panel identity in `tests/shell_windows.rs`.
- [ ] **3. Compact shell chrome.** Replace the top `PlacementNotice` row in
  `qml/shell/ShellWindow.qml` with a focused `WorkspaceMenu.qml` toolbar: persistent
  workspace, Operations, Tape, and palette controls; secondary menu for workspace,
  window, selection, placement, and diagnostics actions. Preserve compositor export,
  placement reports, and keyboard focus. Show active toggle state and shortcut tooltips;
  return focus to the chart after a group closes. Capture both workstation sizes and a
  narrow window to check for clipping.
- [ ] **4. Quiet primary copy.** In `ChartIdentity.qml`, `StatusStrip.qml`,
  `OpsHeader.qml`, `ChartEmptyState.qml`, deployment empty states, and
  `PlaceholderPanel.qml`, apply the spec's hierarchy. Move routine feed counters to
  diagnostics; keep nonzero trust problems and one actionable market-data condition
  visible. Collapse technical explanations without deleting their accessible or
  diagnostic detail. Add the critical OpsStatus chip when Operations is hidden; clicking
  it reveals that group. Update gallery examples for healthy, stale, disconnected,
  worker unavailable, unknown orders, kill switch, and empty Tape states.
- [ ] **5. Review and handoff.** Record the Quantower, MuseScore, and Grafana adaptations
  in `docs/design/components.md`. Inspect captured windows and gallery states at both
  workstation sizes and a narrow width. Run
  `env -u WAYLAND_DISPLAY -u DISPLAY make check`; check the chart and split-pane frame
  budget with `make bench-frames`. Commit focused changes and set Q-064 to In Review
  through `./work board set` with check results and open follow-ups.

## Review focus

- A saved Trading or Single monitor workspace does not silently become Chart.
- Hiding Operations does not make kill-switch, flatten, or account actions unreachable.
- A healthy screen stays quiet; stale market data and critical execution state remain
  distinguishable and explicit.
- A narrow toolbar retains the workspace and both panel controls without clipping.
- Wayland placement text stays honest and compositor export remains reachable.
