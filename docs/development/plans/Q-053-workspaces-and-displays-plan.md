# Q-053 implementation plan: Workspaces and display placement

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Specification:** [`../specs/Q-053-workspaces-and-displays-spec.md`](../specs/Q-053-workspaces-and-displays-spec.md)  
**Depends on:** Q-052

## Current-system context

After Q-052 the shell owns the stores, panels carry stable identifiers, windows are
compositions of panels, and `ShellController` can open, close, move, merge and split.
Arrangement exists as a serialisable structure and nothing writes it.

The development and operating environment is Hyprland on Wayland, with `DISPLAY=:1`
available through XWayland, Qt 6.11.2, and two heterogeneous monitors: `HDMI-A-1`, an
ASUS VG328 at 1920×1080, and `DP-3`, a Samsung Odyssey G5 at 2560×1440 reporting serial
`HX5Y903636`. Both at scale 1. `make check` runs offscreen with `env -u WAYLAND_DISPLAY
-u DISPLAY`.

The constraint that shapes this task: **a Wayland client cannot position its own
windows.** This is the protocol's design, not a Qt limitation and not a bug with a
workaround — `QWindow::setPosition` and `move()` are ignored by the compositor. Qt exposes
display identity fine (`QScreen::name`, `manufacturer`, `model`, `serialNumber`,
`geometry`, `devicePixelRatio`, and `QGuiApplication::screenAdded`/`screenRemoved`), so
identification works everywhere; only placement is gated by the platform.

## Interfaces produced

```rust
// src/workspace/mod.rs        new
pub struct WorkspaceStore;              // load, save, list, switch; versioned schema
#[qobject] WorkspaceController {
    #[qproperty(QString, active_workspace)]
    #[qproperty(QString, placement_mode)]      // "direct" | "compositor" | "none"
    #[qinvokable] fn save(&self, name: QString)
    #[qinvokable] fn switch_to(&self, name: QString)
    #[qinvokable] fn duplicate(&self, name: QString, as_name: QString)
    #[qinvokable] fn remove(&self, name: QString)
    #[qinvokable] fn export_compositor_rules(&self) -> QString   // path written
}
// src/workspace/schema.rs     new: versioned TOML model + migrations
// src/display/identity.rs     new: ScreenFingerprint, resolution rules, confidence
// src/display/placement.rs    new: platform placement strategy
// src/display/compositor.rs   new: rules emitters (Hyprland first)
```

```
~/.config/q_terminal/workspaces/<name>.toml    the saved workspaces
~/.config/q_terminal/workspaces/state.toml     last used workspace
qml/shell/WorkspaceMenu.qml     new: create, rename, duplicate, delete, switch
qml/shell/PlacementNotice.qml   new: placement mode, and how to install compositor rules
tests/workspaces.rs             new: criteria 1, 2, 7
tests/display_identity.rs       new: criteria 3, 4, 5, 6
docs/workspaces.md              new: the schema, the resolution rules, the Wayland path
```

## Implementation decisions

- **Placement intent is saved everywhere; placement authority is platform-dependent.**
  This is the whole design. The workspace file always records which display a window
  belongs on and what geometry it wants. Under X11 or XWayland the terminal applies that
  directly. Under Wayland it cannot, so it does the thing Wayland actually intends: it
  makes each window identifiable — a stable per-window application identity and title,
  derived from workspace and composition — and lets the compositor place it by rule. The
  terminal generates those rules from the saved workspace so the operator does not write
  them by hand. Hyprland's rule syntax is the first emitter because it is the environment
  in use; the emitter is behind a trait so a second compositor is a new file, not a
  redesign.

- **The terminal never lies about what it did.** `placement_mode` is surfaced in the UI.
  A workspace restored under Wayland without compositor rules installed shows that the
  panels are correct and the placement is the compositor's, with the one action needed to
  fix it. Silently reopening everything on the primary monitor and calling it a restored
  layout is the behaviour to avoid.

- **XWayland is offered, not imposed.** Running with `QT_QPA_PLATFORM=xcb` gives direct
  placement today on this machine, at the cost of Wayland's scaling and input handling.
  The plan measures both on the real two-monitor setup and records the trade-off rather
  than picking for the operator; `./dev` and the launcher documentation state how to choose.

- **Fingerprints are attributes plus confidence, not a hash.** A hash of every attribute
  breaks the moment a resolution changes, which is exactly when the operator most wants
  the layout back. The fingerprint keeps the attributes separately so the resolution rules
  can degrade in a defined order — exact, stable-minus-geometry, geometry-and-role
  heuristic, primary — and report which rule fired. Two identical monitors of the same
  model with no serial are the genuinely ambiguous case; they resolve by connector name
  and are reported as low confidence.

- **TOML under the XDG config directory, versioned, with migrations.** It matches the
  UI direction's sketch, it is hand-editable, and a schema version with real migrations
  means a layout survives the format changing. A file that cannot be read is renamed
  aside and reported, never deleted: the operator's layout is their work.

- **Switching a workspace touches windows only.** The stores, the subscription, the health
  poller and the execution state belong to `Shell` and are untouched by a switch. Criterion
  7 counts that at the fake server, because a workspace switch that silently resubscribes
  would be a regression of the whole point of Q-052.

- **Display changes rescue, they do not rearrange.** A removed display moves only the
  windows that became unreachable. Re-resolving on a display's return is an explicit
  action. Rearranging an operator's screens mid-session while a position is open is not a
  feature.

- **Fake display sets for tests.** Real monitors cannot be plugged and unplugged in CI, so
  the resolution rules take a display description list as input and the tests drive them
  directly. The two real monitors' attributes are among the fixtures, including the
  Odyssey G5's serial and the resolution difference.

## Ordered implementation

- [ ] 1. Work on the branch `Q-053-workspaces-and-displays` in `q_terminal`, created from
   `development` by `./work start`. Confirm Q-052 is merged. Commit.
- [ ] 2. Measure the platform reality first: enumerate `QScreen` attributes under Wayland,
   XWayland and offscreen on the real two-monitor setup, and confirm what placement each
   honours. Record it in `docs/workspaces.md`; it is the evidence the rest of the task
   rests on. Commit.
- [ ] 3. Implement `ScreenFingerprint` and the resolution rules with the fake display
   fixtures, including the two real monitors. Criteria 3 and 4. Commit.
- [ ] 4. Implement the versioned schema, `WorkspaceStore`, and load/save round-trip with
   the malformed, truncated and future-version cases. Criteria 1 and 2. Commit.
- [ ] 5. Implement `placement.rs` with the direct strategy, and restore under X11 and
   offscreen. Commit.
- [ ] 6. Implement `compositor.rs` with the Hyprland emitter and its fixture test, and the
   Wayland strategy with honest `placement_mode` reporting. Criterion 6. Commit.
- [ ] 7. Implement `WorkspaceMenu` and `PlacementNotice` on the Q-051 primitives, and the
   switch path with criterion 7. Commit.
- [ ] 8. Implement single-display resolution onto Q-052's merge, and criterion 5. Ship the
   default two-display and single-display workspaces, created on first run only. Commit.
- [ ] 9. Write `docs/workspaces.md` fully — schema, rules, the Wayland path and the
   XWayland trade-off — and update `README.md`, `BOUNDARY.md` and `./dev` guidance. Commit.
- [ ] 10. Run `env -u WAYLAND_DISPLAY -u DISPLAY make check`. Fix, re-run, commit.
- [ ] 11. **Human:** human-verifiable criteria 1–4, on the real two-monitor setup.

## Validation

- **Unit:** fingerprinting and each resolution rule; schema migrations; rules emission
  against a fixture; geometry clamping against available displays.
- **Integration (headless):** workspace round-trip; corrupt and future-version handling;
  display add/remove with fake sets; single-display merge and split; switch cadence at the
  fake server.
- **Platform:** restore under offscreen and X11; under Wayland, identity and emitted rules
  rather than geometry.
- **Regression:** every Q-035 to Q-052 test, the token gate, `qmllint`,
  `make contracts-check`.
- **Manual:** the real two-monitor restore on both paths; output reorder; live unplug with
  a position open; a full single-display session.

```bash
cd /home/gui/projects/q/q_terminal
env -u WAYLAND_DISPLAY -u DISPLAY make check
RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo test --test workspaces --test display_identity

# human (step 11)
make run                              # Wayland: compositor rules path
QT_QPA_PLATFORM=xcb make run          # XWayland: direct placement path
hyprctl monitors -j                   # record the display set used
```

## Handoff

Give the platform measurement from step 2: what each of Wayland, XWayland and offscreen
reported and honoured. Give the resolution rule that fired for each human scenario,
including after the output reorder. Give the workspace schema version and the migrations
written. State the XWayland trade-off as measured, with a recommendation. From the human
steps, give the restore result on both paths with screenshots, what the live unplug did,
and the single-display session result.
