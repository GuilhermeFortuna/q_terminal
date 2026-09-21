# Q-058: Workstation hierarchy and operations layout

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**UI direction:** [`../../design/ui-direction.md`](../../design/ui-direction.md)  
**Reference register:** [`../../design/references.md`](../../design/references.md) — Quantower (workspace composition), MuseScore 4 (desktop chrome), Wireshark (dense table/empty states), Grafana (operational status)  
**Depends on:** Q-055, Q-057  
**Implementation plan:** [`../plans/Q-058-workstation-hierarchy-and-operations-layout-plan.md`](../plans/Q-058-workstation-hierarchy-and-operations-layout-plan.md)

## Purpose

The merged single-monitor workspace currently devotes large areas to empty deployments
and detail tables, while the chart is short and unnamed health badges overflow the
status panel. Workspace controls and a large compositor notice consume the top of the
window. Make the default composition useful at a glance, including when there are no
deployments and the execution stack is unavailable.

## Requirements

- Make the chart the main visual surface in the merged composition. Put deployments in a
  compact, resizable left pane and execution detail in a shorter, resizable pane below the
  chart. Preserve every panel's ability to move, float, dock, merge, and restore.
- Consolidate workspace actions into one compact toolbar. Keep active workspace name
  visible; group Save/Duplicate and window/selection actions so their purpose is clear.
  Command palette remains keyboard-accessible.
- Replace the full-width compositor instruction banner with a concise placement status
  and a disclosure for its explanation and Export action. Preserve the information and
  export behavior; do not imply that the terminal controls compositor placement.
- Make operations health legible within the available width. Show the highest-severity
  current condition first, with a compact summary and accessible details for stream,
  API, worker, edge, unknown orders, kill switch, and live lock. No badge may be clipped
  or hidden by a panel edge. Unknown heartbeat age must read `unknown`, not `0.0s`.
- Give empty deployments and no-selection details purposeful prompts. When there is no
  selected deployment, suppress empty table chrome and `Load older` controls; keep the
  chart usable in its configured or manual mode. When data returns, populated tables and
  action controls reappear without losing selection or pagination state.
- Keep destructive/live actions visible with consequence labels and existing confirmation
  and enablement behavior. Do not bury a critical state in a transient toast.

## Constraints and non-goals

- No new execution capabilities, market-data calculations, or backend supervision.
- Preserve existing panel identities, saved workspace compatibility, and global versus
  detached selection semantics. Old workspace files must restore without dropping panels.
- Use the Q-057 true-black theme and the existing token/component system.

## Acceptance criteria

1. The default merged layout prioritizes the chart while keeping deployments and detail
   reachable and resizable. Saved pre-Q-058 workspaces still restore with all panels.
2. At 1920×1080 and 2560×1440, the toolbar, placement status, and health indicators do
   not clip or cover chart content, including all degraded health states.
3. With no deployments or selection, the screen gives a clear next action and does not
   devote most of the viewport to empty table furniture.
4. Critical state, kill switch, live lock, and unknown orders remain visible and have
   text labels. Existing confirmations and command enablement are unchanged.
5. Keyboard focus order and accessible names cover toolbar, disclosure, panels, tabs,
   and actions. Gallery and screen captures are inspected at both resolutions.
6. `env -u WAYLAND_DISPLAY -u DISPLAY make check` passes.
