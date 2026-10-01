# Workspaces and display placement (Q-053)

Named, versioned workspaces live under `~/.config/q_terminal/workspaces/`. The last used
workspace is recorded in `state.toml`. Default workspaces **Trading** (two displays) and
**Single monitor** are created on first run and never overwritten.

## Platform measurement

| Environment | Display identity | Placement authority |
|---|---|---|
| `env -u WAYLAND_DISPLAY -u DISPLAY` (offscreen) | One virtual screen from Qt | **Direct** — `setX`/`setY`/`resize` honoured |
| `QT_QPA_PLATFORM=xcb` (XWayland) | Full `QScreen` attributes | **Direct** — geometry restored by the terminal |
| Native Wayland (Hyprland) | Full `QScreen` attributes | **Compositor** — terminal sets title/class and exports Hyprland rules |

Under Wayland the protocol does not let clients position windows. The terminal saves placement
intent in every workspace file, sets a stable per-window application identity
(`q_terminal-<workspace>-<composition>`), and can export Hyprland `windowrulev2` lines.
`placement_mode` in the UI is `direct`, `compositor`, or `none`.

XWayland (`QT_QPA_PLATFORM=xcb`) restores geometry directly at the cost of Wayland scaling
and input handling. Use it when you want the terminal to place windows without compositor rules.

## Schema (version 3)

```toml
schema_version = 3
name = "Trading"

[[windows]]
composition = "market"
display = { name = "HDMI-A-1", manufacturer = "ASUS", model = "VG328", geometry = { x = 0, y = 0, width = 1920, height = 1080 }, device_pixel_ratio = 1.0, primary = true }
geometry = { x = 0, y = 0, width = 1920, height = 1080 }
detached = false
# root = <panel tree>

[selection]
global = ""
detached = {}

# Panel study sets:
# [study_sets]
# chart = [{ kind = "ema", period = 21, source = "close", num_std = 2.0, visible = true, palette_index = 0 }]

# Panel chart preferences (Q-077):
[chart_preferences.chart]
symbol = "PETR4"
timeframe = "1m"
mode = "manual" # "manual" or "following"
last_manual_symbol = "PETR4"
last_manual_timeframe = "1m"
visible_bars = 200
```

Version 1 and 2 files migrate forward seamlessly to version 3. Invalid or unknown study or
chart-preference entries fall back per-setting without rejecting the remainder of the workspace.

## Automatic persistence and crash resilience (Q-077)

Terminal setup changes are persisted automatically without requiring manual save actions:

- **Debounced autosave:** Layout mutations, study configuration changes, panel toggles,
  chart target changes, and viewport zoom level adjustments are coalesced and committed after
  a 500ms debounce interval.
- **Synchronous flush:** Closing the application, shutting down, or switching workspaces
  synchronously flushes pending dirty state to disk.
- **Non-destructive window close:** Closing windows in a multi-window session preserves the
  remaining layout. Closing the final window flushes the active layout before teardown,
  preventing empty layout files from being written.
- **Atomic write & directory sync:** Saves write to a temporary file (`.<name>.toml.tmp-PID-UUID`),
  flush and sync the file descriptor and parent directory, and atomically rename over the target.
- **Preservation & Recovery workspace:** Unreadable or corrupt files are renamed aside to
  `.bad-<timestamp>`. Future-version files (schema version > 3) are preserved completely
  untouched on disk, and the terminal opens on a newly created `Recovery` workspace
  (`Recovery 2`, etc.), preventing older versions from corrupting newer configs.

## Startup precedence and session overrides

On startup, initial target resolution follows a strict field-by-field precedence hierarchy:

1. **Explicit environment variables (`Q_TERMINAL_SYMBOL`, `Q_TERMINAL_TIMEFRAME`):**
   Non-empty environment variables override the active session target. Crucially, they apply
   as a **session-only override**: the committed target on disk is not overwritten, and the
   UI indicates the divergence note until the operator explicitly requests a manual target.
2. **Saved committed target:** The target (`symbol`, `timeframe`, `mode`, `followed_deployment_id`)
   saved in the active workspace's chart preferences.
3. **Configuration file (`terminal.toml`):** Configured symbol and timeframe.
4. **Hardcoded defaults:** `PETR4` · `1m`.

### Delayed followed-deployment fallback

When the saved workspace specifies Following mode targeting a deployment ID:
- The terminal begins loading history immediately for the deployment's symbol and timeframe.
- When the execution stream snapshot is confirmed, if the deployment is missing, archived,
  or cannot be resolved, the terminal automatically falls back to the saved `last_manual`
  target and displays an explanatory diagnostic note in the chart header.
- The saved workspace configuration on disk retains the operator's preference until the
  operator explicitly retargets.

## Display resolution order

1. **Exact** — name, manufacturer, model, serial, geometry and DPR match
2. **Stable minus geometry** — same monitor at a different resolution (geometry scaled proportionally)
3. **Geometry and role** — size and primary-flag heuristic
4. **Primary** — fallback to the primary display

The rule used is logged and surfaced in workspace reports. Identical models without serial
resolve by connector name at low confidence.

## Single display

A multi-window workspace with only one display available merges into one window (Q-052
`merge_into`), preserving every panel. Splitting out restores the two-window arrangement
within the session.

## Compositor rules

Use **Export compositor rules** in the shell toolbar, or call
`WorkspaceController.export_compositor_rules()`. Rules are written next to the workspace
files as `<workspace>.hyprland.conf`.
