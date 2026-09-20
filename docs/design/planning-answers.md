# Answers to the UI direction's planning questions

[`ui-direction.md`](ui-direction.md) §15 asks seventeen questions of the planning phase.
Each is answered here, with the task and the document where the decision is carried out.
Where a question cannot be answered without evidence, the answer says what will be
measured and by which step.

### 1. What minimum design-system work should happen before the operations views grow?

All of Q-051, and it is now a retrofit: the operations views already exist and hold about
150 colour literals and 130 literal type sizes across eighteen files. The minimum is the
token layer, the semantic layer, the primitive set, the migration of every existing file,
and the gate that stops literals returning. Anything less leaves the phase-5 surfaces
copying what is there.

### 2. Should multi-window infrastructure come during the operations work or immediately after?

Immediately after, as Q-052, and before any phase-5 market surface. The single-window
assumption is currently only one file deep — `Main.qml` owns the stores — which is the
cheapest it will ever be to undo. Tape, DOM and footprint panels would each deepen it.

### 3. What state belongs globally versus per window?

Global: the stores, connection and health, execution, risk and account state, the kill
switch, the command registry and its enablement. Per window: which panels it holds, their
arrangement and sizes, the active tab, scroll and expansion state. Selection is a named
context, global by default so that a deployment selected in one window retargets the
other's chart, with an explicit, visible per-window detach. Q-052.

### 4. How should panels be represented?

As identified, self-contained content units with a stable string identifier, a title, an
icon and a singleton-or-many rule. Windows are compositions of panels. The identifier is
what Q-053 persists, so it is chosen in Q-052 and tested to survive a move between
windows.

### 5. Should docking use Qt Quick Controls, custom QML, or another mechanism?

KDDockWidgets, subject to a spike that is the first implementation step of Q-052. It is
mature, supports QtQuick, and gives floating, docking, tab-merge and layout save/restore
rather than having them written by hand. The check before the task depends on it is that
it composes with a cxx-qt-owned engine and the custom scene-graph chart item. The
fallback, if the spike fails, is `SplitView` panes with custom float and dock.

Separately, and not in tension with this: ordinary controls are Qt Quick Controls
restyled, not rebuilt, so that keyboard navigation, focus handling, interaction states and
accessibility come from the substrate. Fully custom implementations are reserved for the
trading-specific widgets — ladder, tape, footprint — where there is nothing to restyle.

### 6. How should workspace layouts be serialised?

Versioned TOML under the XDG configuration directory, one file per workspace plus a
last-used pointer, hand-editable, with real migrations. An unreadable or future-versioned
file is renamed aside and reported, never discarded. Q-053.

### 7. How should monitor identities be persisted robustly?

As a fingerprint of the attributes the platform reports — connector name, manufacturer,
model, serial where present, geometry — kept separately rather than hashed together, so
resolution can degrade in a defined order: exact match, stable attributes minus geometry,
a geometry-and-role heuristic, then the primary display. The rule that fired is recorded
and shown. Index order is never used alone. Q-053.

### 8. How should single-monitor fallback work?

A workspace whose bound displays are absent resolves to the merged single-window
arrangement: panels collapse into tabs and panes, keeping identity and state, reversibly,
and the merged arrangement is itself saveable. The merge is built in Q-052 and used by
Q-053. No panel and no command is unreachable in it.

### 9. Which components should exist before the operations views are implemented?

They already exist as ad-hoc code, so the question becomes which the migration must
produce: `Panel`, `SectionHeader`, `Toolbar`, `AppButton` and variants, `IconButton`,
`StatusBadge`, `ConnectionIndicator`, `Metric`, `DataTable` with header, row, cell and
selection, `TabBar`, `SplitPane`, `EmptyState`, `AppDialog`, and restyled text field and
combo box. Q-051.

### 10. Which future visualisation types will require custom scene-graph rendering?

The ones that draw thousands of primitives per frame: footprint cells, depth and volume
heatmaps, the DOM's liquidity column, dense execution markers over long history, and
order-flow bubbles. The tape is a virtualised list until measurement says otherwise. The
existing bar render node is the precedent and the budget is §4.6's 16 ms. None of this is
in the current batch; it is recorded so the shell does not foreclose it.

### 11. How should keyboard commands and focus traversal work across windows?

Commands are a registry in Rust — identifier, label, enablement rule, shortcut — rendered
by every window and enabled from global state, so the kill switch disables a command
everywhere at once. Application shortcuts work from whichever window has focus; panel
shortcuts only when that panel has focus; a collision is a startup error, not a silent
precedence rule. Focus traversal stays within a window, and dialogs are window-modal so a
confirmation never freezes the other window's live chart. Q-052.

### 12. How should visual regression or screenshot testing be incorporated?

Three layers. Statically, the Q-051 token gate in `make check`. While the design is being
made, Q-051's gallery renders every component in every state on one page, so an agent or a
reviewer can see the whole system at once instead of hunting for a screen that contains
the component. As a regression check, Q-054 renders that gallery and every screen
offscreen in a pinned environment — software rasterisation,
vendored fonts with system font paths excluded, fixed sizes and device pixel ratio — at
both workstation resolutions, and compares against committed baselines with a written diff
image. Acceptance is one explicit command that reports what it rewrote; a test run never
rewrites a baseline. The baselines come after the shell and workspaces
because baselines captured before layouts settle are baselines that get discarded; the
gallery does not, because it is a design instrument rather than a regression check.

### 13. Should QML expose a formal design-token API from the beginning?

Yes, and it is enforced rather than encouraged: a `Theme` singleton is the only place a
literal may appear, and the gate fails the build on a colour literal, a raw type size or a
raw pixel dimension anywhere else. Q-051.

### 14. Should a Figma component library mirror the QML component system?

For new screens, yes; for this batch, not yet. The supported pipeline is the Figma to Qt
plugin — free, the replacement for the deprecated Qt Bridge, and it carries shared
libraries with components and tokens. Q-051 sets it up and validates the round trip on one
component, because the surfaces it touches already exist and the job is a retrofit. The
validation decides whether Q-052's and Q-053's new screens are designed in Figma first,
and that decision is made on the recorded evidence.

### 15. How should visual references be selected and attached to implementation tasks?

Through [`references.md`](references.md). An entry is added before the visual work it
governs begins. A spec names the entries it adapts; the first implementation step of a
visual task is recording, per component, the reference selected and what is taken from it,
in `docs/design/components.md`, and that record gates the component work. A category with
no entry is a gap to close, not permission to design from a blank canvas.

### 16. Which external sources should be preferred for recurring UI categories?

The register sorts them by what may be taken. Vendored: Radix Colors for the scales, Inter
and JetBrains Mono for text and numerics, Lucide with Tabler as fallback for icons, Qt
Quick Controls as the substrate, KDDockWidgets for docking. Generated: the Figma to Qt
pipeline, and FluentUI for QML and Qaterial as component implementations to read.
Observed: Quantower for workspace composition, Bookmap for microstructure visualisation,
Sierra Chart, ATAS and Jigsaw for ladder and footprint conventions, Wireshark for dense
tables, MuseScore 4 for Qt desktop structure, Grafana and Linear for dark hierarchy and
status semantics.

### 17. How should comparison against references be incorporated into review?

Each visual task carries a human-verifiable criterion that compares the result against its
adapted references at both workstation resolutions, and `docs/design/components.md`
records, per component, the reference, what was taken and what was deliberately changed. A
divergence is either fixed or written down as intentional. Q-051's gallery makes the comparison a single
surface to review rather than a walk through the application.
