# Q-052 implementation plan: Multi-window shell over shared state

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Specification:** [`../specs/Q-052-multi-window-shell-spec.md`](../specs/Q-052-multi-window-shell-spec.md)  
**Depends on:** Q-051

## Current-system context

`qml/Main.qml` is a bare `Window` that declares `BarFeed`, `ExecutionModels`, `OpsStatus`,
`ExecutionControls` and `AppInfo` as children, with `property var` overrides so tests can
inject fakes, and fills itself with `OpsWorkspace`. Because the stores are window
children, a second `Main.qml` would mean a second stream client, a second health poller
and a second execution store. That is the coupling this task removes.

`src/startup.rs::run_slice` builds the engine and loads the module, and opens the window
in degraded states when configuration or the API is missing. `src/ops_session.rs` wires
the session; `src/execution/store.rs` holds `ExecutionStore` behind `ExecutionHandle` with
a revision counter; `src/execution/health.rs` runs the single 2 s poller;
`src/execution_controls.rs` issues commands with `Idempotency-Key`. `cpp/chart_cxx.cpp`
provides the `beforeSynchronizing` hook that coalesces model resets to one per frame —
per window, once there is more than one.

Headless tests (`tests/degraded_states.rs`, `tests/slice_end_to_end.rs`,
`tests/ops_views.rs`) drive the app offscreen against a fake server that counts requests.
After Q-051 the tokens, primitives and token gate exist, and `.qt_module("QuickControls2")`
is linked.

## Interfaces produced

```rust
// src/shell/mod.rs          new
pub struct Shell;                       // owns the stores; creates and tracks windows
#[qobject] ShellController {
    #[qproperty(i32, window_count)]
    #[qinvokable] fn open_window(&self, composition: QString) -> QString   // returns window id
    #[qinvokable] fn close_window(&self, id: QString)
    #[qinvokable] fn move_panel(&self, panel: QString, to_window: QString)
    #[qinvokable] fn merge_into(&self, window: QString)
    #[qinvokable] fn split_out(&self, panel: QString)
}
// src/shell/panels.rs       new: panel registry — id, title, icon, singleton-or-many
// src/shell/context.rs      new: SelectionContext (global by default, per-window detach)
// src/shell/commands.rs     new: command registry — id, label, enablement, shortcut;
//                                duplicate shortcut is a startup error
```

```
qml/shell/ShellWindow.qml        new: the top-level window; hosts a composition
qml/shell/PanelHost.qml          new: docking/tab host
qml/shell/PanelFrame.qml         new: panel chrome — title, icon, actions, focus ring
qml/shell/CommandPalette.qml     new
qml/compositions/MarketWindow.qml      new
qml/compositions/OperationsWindow.qml  new
qml/compositions/MergedWindow.qml      new: the single-monitor composition
qml/panels/*.qml                 the Q-047–Q-049 surfaces, wrapped as panels
tests/shell_windows.rs           new: criteria 1–7
```

## Implementation decisions

- **The stores move from QML to Rust ownership, and are exposed as context properties.**
  This is the one change that makes everything else possible, and it is mechanical: the
  declarations leave `Main.qml`, `Shell` constructs them once, and each `ShellWindow` gets
  them as context properties. The existing test-injection seams are preserved by
  constructing `Shell` with fakes rather than by overriding window properties, so the
  Q-047 and Q-048 suites keep working with a one-line change to their harness.

- **Docking uses KDDockWidgets, and the first step is a spike that proves it.** It is the
  mature answer for floating, docking, tab-merging and layout save/restore, it supports
  QtQuick, and it is actively maintained. The one thing to check before the rest of the
  task depends on it is that it composes with a cxx-qt-owned engine and a custom
  scene-graph item without fighting for the window. If the spike fails, the fallback is
  `SplitView`-based panes plus custom float/dock, which costs the drag-and-drop
  affordances and nothing else; the spike's result decides, and the decision is recorded
  rather than assumed.

- **Panels are identified by a stable string, not by object identity.** Everything Q-053
  will persist keys off that identifier: which panels a window holds, which tab is active,
  what is selected. Choosing it now, and testing that a moved panel keeps it, is what
  makes the next task serialisation rather than redesign.

- **Selection is a context object, global by default.** Q-049 established that selecting a
  deployment retargets the chart. Across windows that is the behaviour worth keeping, so
  the context lives in Rust and both windows read it. Detaching is per-window and visible
  in the window's chrome, because a silently detached window showing stale selection is an
  operational hazard, not a feature.

- **Commands are data, not QML.** A registry in Rust with identifier, label, enablement
  and shortcut lets enablement reuse `execution/enablement.rs` unchanged, lets every window
  render the same palette, and makes a duplicate shortcut a startup failure instead of a
  precedence surprise. QML renders the registry; it does not define commands.

- **Dialogs are window-modal.** Qt gives this directly, and it matters: an application-modal
  confirmation in the operations window would freeze the market window's live chart, which
  is exactly the wrong thing to do while a position is open.

- **Frame coalescing becomes per-window.** The `beforeSynchronizing` hook in
  `cpp/chart_cxx.cpp` is currently one hook for one window. It becomes one per window, each
  coalescing its own model resets against the shared store revision, so a second window
  costs a second render loop and no extra data.

- **Nothing is saved.** The shell exposes its arrangement as a serialisable structure and
  stops there. Q-053 adds the file, the schema, the display identity and the placement.

## Ordered implementation

- [ ] 1. Work on the branch `Q-052-multi-window-shell` in `q_terminal`, created from
   `development` by `./work start`. Confirm Q-051 is merged. Commit.
- [ ] 2. KDDockWidgets spike: build it against Qt 6.11 with the cxx-qt engine, host one
   existing panel and the scene-graph chart item, float it, dock it, tab it. Record the
   result and, if it fails, the fallback. Commit the spike and its write-up.
- [ ] 3. Move the stores into `Shell`, expose them as context properties, and adapt the
   test harnesses. Every Q-047 and Q-048 test must pass with one window before any second
   window exists. Commit.
- [ ] 4. Implement the panel registry and wrap the Q-047–Q-049 surfaces as panels, keeping
   their behaviour. Commit per panel.
- [ ] 5. Implement `ShellWindow`, `PanelHost` and `PanelFrame`, and the two compositions.
   Add criteria 1–3 tests as soon as two windows can open. Commit.
- [ ] 6. Implement the selection context with detach, and criterion 5. Commit.
- [ ] 7. Implement the command registry, the palette, window-modal dialogs and the
   shortcut-collision startup error, and criterion 6. Commit.
- [ ] 8. Implement panel move, float, dock, merge and split, and criteria 4 and 7. Commit.
- [ ] 9. Make frame coalescing per-window, then record p95 per window with both
   compositions open and 10 000 rows (criterion 8). Commit the numbers.
- [ ] 10. Update `BOUNDARY.md`, `README.md` and `docs/design/components.md` (panel chrome,
   palette, and their references). Commit.
- [ ] 11. Run `env -u WAYLAND_DISPLAY -u DISPLAY make check`. Fix, re-run, commit.
- [ ] 12. **Human:** human-verifiable criteria 1–4.

## Validation

- **Unit:** panel registry; command registry and collision detection; selection context
  attach/detach; composition merge and split as pure structure.
- **Integration (headless):** subscription counting across window counts; cross-window
  revision equality; window lifecycle; panel move with state; enablement from every window.
- **Performance:** `bench-frames` per window with both compositions open and 10 000 rows.
- **Regression:** every Q-035 to Q-051 test, the degraded-state suite, the token gate,
  `qmllint`, `make contracts-check`.
- **Manual:** thirty-minute two-window session against the backend snapshot; the §8.1 stop
  sequence with both windows open; ten minutes keyboard-only; a full single-monitor session.

```bash
cd /home/gui/projects/q/q_terminal
env -u WAYLAND_DISPLAY -u DISPLAY make check
RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo test --test shell_windows
BENCH_EXECUTION_ROWS=10000 make bench-frames

# human (step 12)
cd /home/gui/projects/q && ./dev up execution
cd /home/gui/projects/q/q_terminal && make run
```

## Handoff

Give the spike's result: whether KDDockWidgets carried the scene-graph chart item, and
the fallback if it did not. Give the subscription counts at
one, two and four windows. Give p95 per window with both compositions open, against the
Q-051 figure. List every panel with its identifier, and say which are singletons. Give the
command registry's size and its shortcut map. From the human steps, give the thirty-minute
comparison, what each service stop showed in both windows, the keyboard-only result, the
single-monitor session, and the four screenshots.
