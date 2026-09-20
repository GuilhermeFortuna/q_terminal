# Q-051 implementation plan: Terminal design system

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Specification:** [`../specs/Q-051-terminal-design-system-spec.md`](../specs/Q-051-terminal-design-system-spec.md)  
**Depends on:** Q-049

## Current-system context

`qml/` holds 18 files, ~3 000 lines. `Main.qml` is a bare `Window` that instantiates
`BarFeed`, `ExecutionModels`, `OpsStatus`, `ExecutionControls` and `AppInfo`, and fills
itself with `OpsWorkspace`. `OpsWorkspace.qml` composes `OpsHeader`, `DeploymentList`,
`ChartPane` and `DeploymentDetail`; the detail area tabs over `OrdersTable`,
`FillsTable`, `DecisionsTable`, `RiskTable` and `LedgerTable`. `ConfirmDialog`,
`AccountDialog`, `DeployDialog` and `ResolveDialog` are the only files that currently
import `QtQuick.Controls`; everything else is `Item`/`Rectangle`/`ListView`.

Measured before the work: 150 colour literals (`#64748b` ×49, `#94a3b8` ×26, `#131722`
×19, `#f87171` ×18, `#34d399` ×17), 130 literal `pixelSize` values, and `qmldir`
registers 5 of 18 components. The colours are Tailwind's slate/emerald/red scales used
ad hoc.

`build.rs` declares the QML module with `CxxQtBuilder::new_qml_module(QmlModule::new("qml")
.qml_file(...))`, links `.qt_module("Quick")`, ships `qml/Format.js` through two `qrc`
prefixes, and merges `qml/chart_types.qmltypes` into the generated `plugin.qmltypes`.
`make check` runs `fmt-check lint build qml-lint test contracts-check`; `qml-lint`
currently lints `qml/Main.qml` only. Q-047 added a grep-style test forbidding arithmetic
on money-named fields in QML.

Qt is 6.11.2, cxx-qt 0.10. The development and test environment is Hyprland/Wayland;
`make check` runs with `env -u WAYLAND_DISPLAY -u DISPLAY`, i.e. offscreen.

## Interfaces produced

```
qml/theme/Theme.qml           new: singleton; the only file permitted to hold literals
qml/theme/Palette.qml         new: Radix dark scales, named
qml/theme/Typography.qml      new: role → family, size, weight, features (tnum)
qml/theme/Spacing.qml         new: one scale; row/header/control heights
qml/theme/Semantic.qml        new: role → token resolution (the only conditional colour in the tree)
qml/theme/Icons.qml           new: name → vendored Lucide svg
qml/theme/qmldir              new: singleton registration

qml/components/               new: Panel, SectionHeader, Toolbar, AppButton, IconButton,
                              StatusBadge, ConnectionIndicator, Metric, DataTable,
                              DataTableHeader, DataTableRow, TabBar, SplitPane,
                              EmptyState, AppDialog, AppTextField, AppComboBox
qml/style/                    new: the custom Qt Quick Controls style (Button.qml, TabButton.qml, …)
qml/gallery/Gallery.qml       new: every component × state, roles, type scale, tokens, icons
qml/gallery/GalleryEntry.qml  new: one section, with its register citation
src/gallery.rs                new: registration behind the "gallery" cargo feature;
                              --gallery opens the window, --gallery-shot renders to PNG
Makefile                      new targets: gallery, gallery-shot

assets/fonts/                 new: Inter, JetBrains Mono
assets/icons/                 new: the Lucide subset actually used

src/semantic.rs               new: SemanticRole enum exposed to QML; the store's rows carry a role
tools/token_gate.py           new: the literal/arithmetic gate, run by make check
docs/design/components.md     new: per component — reference adapted, what was taken, what differs
```

## Implementation decisions

- **Qt Quick Controls is the substrate, restyled — not replaced.** The UI direction asks
  for keyboard-friendly interaction and excellent focus, hover and selection states. QQC2
  already implements focus traversal, key handling, interaction states and accessibility;
  writing those by hand across seventeen components is seventeen chances to get them
  subtly wrong. The terminal supplies a custom style, so the look is entirely ours and
  the behaviour is Qt's. This also matches the class-B sources in the register: FluentUI
  for QML, Qaterial and the Figma-to-Qt control library are all QQC2-based, so their
  structure transfers directly. `build.rs` gains `.qt_module("QuickControls2")`.
  Fully custom `Item`-based implementations are reserved for the trading-specific widgets
  — ladder, tape, footprint — where there is nothing to restyle and nothing to source.

- **Design is sourced, not originated.** Before any component is written, its register
  entry is selected and recorded in `docs/design/components.md`. Tokens adapt Radix's
  12-step dark scales because they are built for the surface/border/text separation the
  terminal needs; the existing Tailwind values are mapped onto them, so most of the
  migration is a rename rather than a recolour. Table density, selection and detail-pane
  pairing adapt Wireshark; panel headers and toolbars adapt MuseScore 4; status and badge
  semantics adapt Grafana and Linear. Each is a class-C entry: arrangement and density
  are adapted, assets and code are not.

- **Figma is used where it pays, not as ceremony.** The Figma-to-Qt plugin (free, and the
  supported replacement for the deprecated Qt Bridge) can generate QML from a Figma
  library including its tokens. It is worth a mirror library for the screens Q-052 and
  Q-053 introduce, which do not exist yet. For this task the surfaces already exist and
  the job is a retrofit, so the pipeline is set up and validated on one component
  (`StatusBadge`) to prove the round trip, and the remaining components are written
  directly. The validation result decides whether later screens are designed in Figma
  first; that decision goes in the handoff, with evidence.

- **The semantic layer is where trading meaning stops.** `SemanticRole` is a Rust enum
  carried on the rows the models already expose. `Semantic.qml` is the single file allowed
  to map a role to a token, and the gate forbids conditional colour anywhere else. This is
  invariant 1 applied to presentation: the terminal does not decide that a number is bad,
  it renders the badness the backend reported.

- **The gallery is built early, because it is how the design gets seen.** It lands with
  the first components rather than at the end, so that every later component is made
  against a surface that shows the whole system at once — states side by side, roles side
  by side, the type scale in one column. Working without it means judging a button by
  finding a screen that happens to contain one. It reads the same component-and-state
  enumeration as the interaction-state test, so a component added without a gallery entry
  fails a test rather than quietly going unreviewed, and it sits behind a `gallery` cargo
  feature so it is absent from the operations binary entirely.

- **The gallery renders to files, not only to a window.** `make gallery-shot` writes it
  offscreen with software rasterisation and the vendored fonts. This is what makes the
  feedback loop available to an implementing agent: both agent CLIs in use here can view a
  PNG mid-session — Claude Code through `Read`, codex through `view_image` — but neither
  can look at a window. An agent that cannot see its own output is designing blind
  whatever model is behind it. Q-054 adds comparison against committed baselines on top of
  this capture; the capture itself belongs here, with the thing it photographs.

- **Fonts are vendored, not assumed.** Inter and JetBrains Mono go into the `qrc`. Two
  reasons beyond determinism: tabular figures are what make price columns align, and
  Q-054's screenshot baselines are worthless if the font can shift under them.

- **The gate is a script, not a lint plugin.** `tools/token_gate.py` scans QML for colour
  literals, raw `pixelSize`, raw pixel dimensions and conditional-colour patterns, with
  `qml/theme/` exempt. It absorbs the Q-047 money-arithmetic grep so there is one gate,
  not two. A fixture file containing deliberate violations proves it fails.

- **Migration is per file, with its tests green at each step.** The behaviour of the
  operations workspace is already covered by the Q-047 and Q-048 suites. Those suites are
  the migration's safety net and are not edited: if a migration commit changes what a
  field shows, a test says so.

## Ordered implementation

- [x] 1. Work on the branch `Q-051-terminal-design-system` in `q_terminal`, created from
   `development` by `./work start`. Confirm Q-049 is merged. Commit.
- [x] 2. Select and record references for every component in `docs/design/components.md`,
   before any component is written. Review this against the register as a gate on step 5.
   Commit.
- [x] 3. Vendor the fonts and the Lucide subset; wire the `qrc`;
   add `.qt_module("QuickControls2")`. Prove offscreen that the fonts resolve with system
   font paths emptied, and that tabular figures measure equal. Commit.
- [x] 4. Write `qml/theme/`: `Palette`, `Typography`, `Spacing`, `Icons`, `Theme`,
   `Semantic`, and `src/semantic.rs` with its exhaustive mapping test. Commit.
- [x] 5. Build the gallery shell behind the `gallery` feature, reading the component and
   state enumeration, before the component set is written, with both `make gallery` and
   `make gallery-shot`. Confirm the shot works with no display, since that is how it will
   be used. It is the instrument for the next step, not a report on it. Commit.
- [x] 6. Write `qml/style/` and `qml/components/`, one commit per component, each citing
   its reference, each appearing in the gallery as it lands, and each rendered and looked
   at before it is committed. Add the interaction-state and gallery-completeness tests as
   components land. Commit per component.
- [x] 7. Write `tools/token_gate.py` with its deliberate-violation fixture, wire it into
   `make check`, and extend `qml-lint` to every file in the module. Expect it to fail
   loudly at this point; that is the migration's worklist. Commit.
- [ ] 8. Migrate the 18 existing files, one commit per file, running the Q-047 and Q-048
   suites after each. Finish when the gate is clean. Commit per file.
- [ ] 9. Run `BENCH_EXECUTION_ROWS=10000 make bench-frames` and compare against the Q-047
   figure (p50 8.58 ms, p95 11.56 ms, p99 11.56 ms). Commit the recorded numbers.
- [ ] 10. Update `README.md` and `BOUNDARY.md` (the design system, the gallery feature and
   where its rules are enforced), and complete `docs/design/components.md`. Commit.
- [ ] 11. Run `env -u WAYLAND_DISPLAY -u DISPLAY make check`. Fix, re-run, commit.
- [ ] 12. **Human:** human-verifiable criteria 1–4.

## Validation

- **Unit:** token resolution; semantic role exhaustiveness; font and tabular-figure
  measurement; the gate's own fixture.
- **Integration (headless):** each component in each interaction state; gallery
  completeness and its presence/absence by build; the full Q-047 and Q-048 suites
  unchanged; the Q-035 to Q-038 chart suites; the degraded-state suite.
- **Performance:** `bench-frames` with 10 000 rows, compared against the Q-047 baseline.
- **Static:** `qmllint` over every module file; `tools/token_gate.py`; `make contracts-check`
  (unchanged — this task touches no contract).
- **Manual:** gallery review against the register, from the rendered images and the window; side-by-side reference comparison at
  both resolutions; keyboard-only traversal; live-mode distinctness.

```bash
cd /home/gui/projects/q/q_terminal
env -u WAYLAND_DISPLAY -u DISPLAY make check
python3 tools/token_gate.py --check qml/
BENCH_EXECUTION_ROWS=10000 make bench-frames
env -u WAYLAND_DISPLAY -u DISPLAY make gallery-shot   # the agent's design loop

# human (step 12)
make gallery
make run
```

## Handoff

Give the before/after counts from the gate: colour literals, `pixelSize` values, raw
dimensions, conditional-colour sites. List every component with the register entry it
adapts and what was deliberately changed. Give the frame benchmark's p50, p95 and p99
against the Q-047 baseline. State the result of the Figma-to-Qt round trip on
`StatusBadge` and whether later screens should be designed in Figma first. From the human
steps, give the gallery review result, the four screenshots, the keyboard-traversal result, and anything the
reference comparison flagged as unresolved.
