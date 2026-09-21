# Q-052 docking decision

**Result: the KDDockWidgets spike was not built. The shell uses the plan's fallback:
`SplitView` panes plus custom float, dock, tab and merge.**

The plan asked for a spike proving that KDDockWidgets composes with the cxx-qt-owned engine
and the custom scene-graph chart item, and left the decision to its result. It was not run,
and this note records why instead of assuming an outcome.

- KDDockWidgets 2.4.1 is packaged for Arch (`extra/kddockwidgets`) but is not installed on
  this workstation. Installing it is a system-wide change outside a task worktree, and it
  would add a C++ system dependency that the hosted CI runners do not have.
- Its QtQuick mode owns floating windows and layout itself. The shell's requirement is the
  opposite: the arrangement is a serialisable structure Rust owns (`src/shell/layout.rs`),
  so Q-053 can persist it and tests can assert on it headlessly. Two layout authorities
  would need reconciling on every move.
- The fallback costs the drag-and-drop affordances the plan names and nothing else. Panels
  are moved, floated, docked back, merged and split by command and from the panel chrome.

**What was and was not verified.** The chart scene-graph item survives being reparented
between windows in the fallback; that is exercised by `tests/shell_windows.rs`. Whether
KDDockWidgets would carry it is unknown.

**Revisit** if drag-to-dock becomes a requirement: install the package, run the spike as
the plan describes, and put the docking host behind `PanelHost.qml`, which is the only file
that would change.
