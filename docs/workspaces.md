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

## Schema (version 1)

```toml
schema_version = 1
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
```

Unreadable, truncated or future-version files are reported; the terminal starts on a default
workspace. Corrupt files are renamed aside (`.bad-<timestamp>`), never deleted.

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
