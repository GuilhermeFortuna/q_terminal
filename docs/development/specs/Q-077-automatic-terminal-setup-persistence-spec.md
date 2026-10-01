# Q-077: Automatic terminal setup persistence

**Status:** written spec and plan awaiting human review; status of record is the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2).
**Batch:** 13 — persistent terminal setup and live market analysis
**Depends on:** Q-076
**Implementation plan:** [Plan](../plans/Q-077-automatic-terminal-setup-persistence-plan.md)

## Purpose

Restore the operator’s last terminal setup automatically on every launch.

## Current system

Workspace schema version 2 saves named window layouts, deployment selection and studies. Main.qml saves on explicit workspace save/switch; it does not persist the manual chart target or reliably save before final-window removal. WorkspaceStore currently writes files directly. ChartTarget already separates manual and deployment-following targets.

## Required behavior

- Extend the existing workspace system, not a second settings store. Version 3 adds chart target/mode and viewport preferences keyed by stable chart panel id. Retain the existing layout, placement, selection and study fields.
- Save symbol, canonical timeframe, manual/following mode, followed deployment id, last successful manual target, study kinds/parameters/visibility/palette, window composition, panel visibility, splitter sizes, active tabs and chart zoom (visible bar count). Do not save transient dialogs, crosshair position, fetched bars, credentials or execution actions.
- Save accepted changes automatically after a 500 ms single-shot debounce. Invalid edits and failed symbol requests never replace the committed target. A dirty revision stays dirty until the write succeeds.
- Flush synchronously before workspace switching and before destroying the final window. Capture the usable layout before close_window removes it; closing one of several windows saves the remaining layout. Do not save an empty workspace during application teardown.
- Write workspace and last-used state using temporary siblings, file flush/sync and atomic rename; sync the parent directory on supported local filesystems. Write the workspace before advancing last-used state. A failure preserves the old valid file and surfaces an actionable error without blocking the terminal.
- Restore the last workspace before the initial history load/subscription target is chosen. Restore studies before data arrives. Suppress autosave during restore, then enable it after a valid state is applied.
- Initial target precedence per field: an explicitly supplied nonempty Q_TERMINAL_SYMBOL/Q_TERMINAL_TIMEFRAME; saved committed target; terminal.toml; built-in defaults. Q-078 removes launcher-injected target defaults. Startup overrides are session-only until an operator target edit, avoiding replacement of the saved target merely by launching with an override. Any explicit target override selects Manual mode for that session; delayed deployment restoration cannot override it.
- Resume at live prices with the saved visible-bar count; do not restore historical pan. A saved followed deployment resolves after the execution snapshot is confirmed; if missing or archived, select the saved manual target and show why. Startup never resumes or starts a deployment.
- Migrate version 1 through version 2 and version 2 to version 3. Missing target fields use config defaults without erasing studies. Drop invalid optional chart settings individually with a warning. Preserve future-version files unchanged and use a recovery workspace with a new unused name, so subsequent autosave cannot overwrite them.
- Backend unavailability does not prevent local restoration. Retain requested symbol/timeframe with a loading/unavailable state and retry; never replace it with PETR4 because validation cannot currently reach the API.

## Interfaces and ownership

Rust workspace and chart-target state remain authoritative; QML signals schedule saves and render errors.
Add typed WorkspaceChartPreferences in src/workspace/schema.rs and autosave orchestration in src/workspace/autosave.rs. Expose capture_chart_preferences, restore_chart_preferences and flush_pending through WorkspaceController/ChartTarget bridges. Restore resolves config overrides before subscribing; future Q-082/Q-084 add typed optional settings with subsequent migrations.
Keep display placement under the existing resolver and Wayland compositor rules; restoring layout does not promise client-controlled Wayland window coordinates.

## Acceptance criteria

1. Change symbol/timeframe, study parameters/visibility/palette, layout and zoom; reopen without an explicit save and obtain the same configuration at the latest bars.
2. A burst of edits writes once after 500 ms; closing before that deadline or switching workspace flushes the accepted state. Final-window close never replaces layout with an empty window list.
3. Version 1/2 files migrate with studies and layout intact; malformed optional settings fall back individually; corrupt/future files retain the existing recovery protection.
4. Inject write, sync and rename failures: the prior file remains parseable, dirty state remains retryable, and the operator sees the save failure.
5. Explicit overrides, manual edits, missing followed deployments, unavailable API startup and delayed execution snapshots follow the stated precedence without stale-target delivery or execution commands.
6. Workspace save/restore preserves the existing single stream/shared stores. Offscreen make check passes.

## Implementation boundary

This issue authorizes only its listed deliverable after written-plan approval and
`./work start Q-077 --agent <agent> --worktree`. Dependencies must be Done.
Preserve the existing execution controls and research/operations ownership boundaries.
No live-order activation, new backend-process ownership or unrelated refactoring.
Use generated contracts and commit/tag pins; never edit vendored code by hand.
