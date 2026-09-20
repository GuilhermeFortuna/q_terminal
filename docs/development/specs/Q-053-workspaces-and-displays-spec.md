# Q-053: Workspaces and display placement

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Project direction:** [`q_contracts/docs/system-architecture.md` §5.1, §9 invariant 9, §10 Phase 5](https://github.com/GuilhermeFortuna/q_contracts/blob/03214a0760945120c56c2af97a542fcb26da2f1e/docs/system-architecture.md#51-ui-ownership-boundary)  
**UI direction:** [`../../design/ui-direction.md`](../../design/ui-direction.md) §5, §8, §9, §10, §15  
**Depends on:** Q-052  
**Implementation plan:** [`../plans/Q-053-workspaces-and-displays-plan.md`](../plans/Q-053-workspaces-and-displays-plan.md)

## Purpose

Q-052 gives the terminal windows and panels that can be arranged. They are gone on exit,
and the terminal has no idea which physical display anything is on. An operator who
arranges a trading layout across two monitors has to rebuild it every session.

This task makes an arrangement a saved workspace: named, versioned, written to disk,
restored on launch, and bound to displays by an identity that survives the monitors being
reordered, unplugged or replaced. It also decides what happens when the displays a
workspace expects are not there — which, for a workstation that is sometimes a laptop, is
the ordinary case rather than the exception.

It must do this on a Wayland session, where the application is not permitted to place its
own windows at all. That constraint is the centre of the design, not a footnote.

## Requirements

### Workspaces

- A workspace has a name and holds, for each of its windows: which panels it contains,
  their arrangement and sizes, which tab is active, the window's geometry intent, its
  display binding, and the selection context it starts from.
- Workspaces are created, renamed, duplicated, deleted and switched from the UI. Switching
  closes what the current workspace opened and opens what the target describes, without
  touching the stores, the stream subscription or the execution state.
- A workspace is written to a versioned file under the user's configuration directory. The
  format is human-readable and hand-editable, and an unreadable or future-versioned file
  is reported and ignored rather than crashing the terminal or silently discarding the
  operator's layout.
- The terminal ships default workspaces matching the UI direction — at least a
  two-display trading layout and a single-display layout — which are created on first run
  and are never silently overwritten afterwards.
- The last used workspace is restored at launch. A workspace that fails to restore falls
  back to a default, and says so.

### Display identity

- Displays are identified by a fingerprint built from the stable attributes the platform
  reports — name, manufacturer, model, serial where present, and geometry — never by index
  alone, because index order changes between sessions.
- The fingerprint tolerates partial information: a display that reports no serial is still
  identified, with lower confidence, and the resolution rules say what lower confidence
  means.
- A workspace binds each window to a display fingerprint. On restore, bindings resolve in
  order: exact fingerprint match, then a match on the stable attributes minus geometry
  (the same monitor at a different resolution), then a geometry-and-role heuristic, then
  the primary display. Which rule resolved each window is recorded and visible.
- Displays appearing and disappearing while the terminal is running is handled: a window
  whose display is removed is moved to an available display and reported, never left
  unreachable; a display returning re-resolves the binding on request, not automatically
  mid-session.
- Bindings survive heterogeneous displays. Windows are restored proportionally where a
  bound display's resolution has changed, and never restored to a geometry that lies
  outside every available display.

### Placement, and the Wayland constraint

- The Wayland protocol does not let a client position its own windows; a compositor may
  ignore every geometry request the terminal makes. The terminal therefore separates
  **placement intent** — which is always saved — from **placement authority**, which
  depends on the platform.
- Under X11 or XWayland the terminal restores geometry and display binding directly.
- Under Wayland the terminal instead makes itself placeable by the compositor: every
  window carries a stable, per-window application identity and title derived from its
  workspace and composition, so that compositor rules can place it. The terminal exports
  the current workspace as compositor placement rules for the running compositor, which
  the operator installs once.
- The terminal reports its placement mode honestly in the UI: whether it placed the
  windows, or whether the compositor did and how to make the compositor do it. It never
  claims to have restored a layout it did not restore.

### One display

- A workspace whose windows are bound to displays that are not present resolves to a
  single-window arrangement: its panels merge into tabs and panes, preserving identity and
  state, using the merge Q-052 built.
- Merging and splitting are reversible within a session, and the merged arrangement is
  itself saveable as its own workspace.
- No panel and no command is unreachable in the merged arrangement.

## Constraints and non-goals

- **No new panel content, no new data, no new route or topic.**
- **No window management beyond the terminal's own windows.** The terminal places its
  windows or asks the compositor to; it configures nothing else on the system and writes
  no file outside its own configuration directory without the operator asking.
- **No per-workspace backend state.** A workspace describes a view. It never holds
  credentials, deployment definitions, or anything that belongs to the backend.
- **No automatic re-layout on display change** beyond rescuing an unreachable window.
  Rearranging an operator's screen while a position is open is not an improvement.
- **No cloud or cross-machine sync.**

## Acceptance criteria

### Agent-verifiable

1. A workspace round-trips: saved, reloaded, and the restored structure equals the saved
   one — panels, arrangement, sizes, active tabs, bindings and selection context.
2. A file from an older schema version loads with its documented migration; a
   future-versioned, malformed or truncated file is reported and the terminal starts on a
   default workspace. Each case is a test.
3. Fingerprints are computed from a set of fake display descriptions, including one with
   no serial, one renamed, one at a changed resolution and two identical models. Each
   resolves by the expected rule, and the rule used is reported.
4. Removing a bound display at runtime moves the affected window to an available display
   and reports it; no window is left with geometry outside every display. Asserted with
   fake display sets.
5. Restoring a two-display workspace with only one display present produces the merged
   single-window arrangement with every panel present and reachable, and splitting it out
   again restores the two-window arrangement.
6. Under the offscreen and X11 platforms the terminal restores geometry directly; under
   Wayland it does not claim to, sets the per-window application identity and title, and
   emits a compositor rules file whose content matches the saved workspace. The emitted
   rules are asserted against a fixture for the compositor in use.
7. Switching workspaces does not disturb the stores: the fake server counts no new
   subscription, no new snapshot and no interruption of the health cadence across ten
   switches.
8. Every Q-035 to Q-052 test passes.
9. `make check` passes.

### Human-verifiable

1. On the two-monitor workstation (ASUS VG328 1920×1080 and Samsung Odyssey G5 2560×1440),
   a trading workspace is arranged, saved, the terminal restarted, and the layout comes
   back on the correct monitors — directly under XWayland, and via the exported compositor
   rules under Wayland. Both paths are recorded, with screenshots.
2. The monitors are swapped in the compositor's output order and the terminal restarted;
   the layout still lands on the correct physical monitors, and the resolution rule used is
   the exact-fingerprint one.
3. A monitor is unplugged while the terminal is running with a position open. The affected
   window is rescued to the remaining display, nothing becomes unreachable, and no
   execution state is lost.
4. A full session is run on the single-display merged workspace.
