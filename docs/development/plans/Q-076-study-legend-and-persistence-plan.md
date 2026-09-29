# Q-076 implementation plan: Study legend, readout and workspace persistence

> **For implementation agents:** Read the linked spec, `AGENTS.md`, `README.md`, and
> `BOUNDARY.md`. Start this task only with `./work start Q-076 --agent <agent> --worktree`
> after Q-075 is Done and the board plan is approved. Keep changes on that task branch.

**Goal:** Readable study values and studies that survive a restart.
**Architecture:** `StudySet` exposes per-study values at the latest bar and at any bar
index. The legend and readout bind to feed invokables. The workspace schema gains a
per-panel study list with a version migration.
**Tech stack:** Rust, CXX-Qt, Qt 6/QML, `toml`/`serde` workspace schema.
**Spec:** [`../specs/Q-076-study-legend-and-persistence-spec.md`](../specs/Q-076-study-legend-and-persistence-spec.md)

## Ordered implementation

- [ ] **1. Value access.** Add `StudySet` accessors for latest and at-index values and
  status (ready, warm-up, unavailable with reason), and expose them through `BarFeed`
  with a revision so QML rebinds only when values change. Unit test them against the
  study outputs.
- [ ] **2. Legend.** Build `qml/components/StudyLegend.qml` from the Q-075 reference,
  one per pane, with hide, edit and remove actions reusing the picker's edit form.
  Check it against crosshair badges at all four plot edges. Inspect gallery captures as
  you build it.
- [ ] **3. Readout.** Add study values to the crosshair bar readout and its accessible
  text, and guard against stale values on target switch and load.
- [ ] **4. Persistence.** Add the study list to the chart panel node in
  `src/workspace/schema.rs`, raise `CURRENT_SCHEMA_VERSION`, and migrate version 1.
  Restore studies into the feed on workspace load and write them on change. Test the
  migration, the round-trip and dropped invalid entries.
- [ ] **5. Review and handoff.** Record the legend in `docs/design/components.md`. Run
  `env -u WAYLAND_DISPLAY -u DISPLAY make check` and `make bench-frames`. Commit focused
  changes and set Q-076 to In Review through `./work board set` with results.

## Review focus

- The legend and the readout agree with each other and with the drawn line at the same
  bar.
- An old workspace file loads unchanged, and a malformed study entry never blocks a load.
- The legend never shows values from a previous target.
