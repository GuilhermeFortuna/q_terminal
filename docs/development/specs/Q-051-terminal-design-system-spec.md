# Q-051: Terminal design system

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Project direction:** [`q_contracts/docs/system-architecture.md` §4.6, §5.1, §9 invariant 1, §10 Phase 5](https://github.com/GuilhermeFortuna/q_contracts/blob/03214a0760945120c56c2af97a542fcb26da2f1e/docs/system-architecture.md#51-ui-ownership-boundary)  
**UI direction:** [`../../design/ui-direction.md`](../../design/ui-direction.md) §2, §3, §4, §14  
**Reference register:** [`../../design/references.md`](../../design/references.md)  
**Depends on:** Q-049  
**Implementation plan:** [`../plans/Q-051-terminal-design-system-plan.md`](../plans/Q-051-terminal-design-system-plan.md)

## Purpose

Phase 4 built the operations workspace fast, and it shows. Eighteen QML files hold
roughly 3 000 lines with about 150 hardcoded colour literals — `#64748b` alone appears
49 times — and 130 literal `pixelSize` values. Five of the eighteen components are
registered in `qmldir`. There is no `Theme`, no shared control set, and every table
restates its own row height, header treatment and status colours. Profit and loss is
coloured by a comparison written inline in QML, in more than one place.

Phase 5 adds market and order-flow surfaces, and a multi-window shell after it. Each of
those multiplies whatever discipline exists now. This task establishes that discipline
first: a token layer, a set of reusable primitives built on Qt Quick Controls, a semantic
layer that keeps trading meaning out of QML, and a mechanical gate that keeps raw values
from coming back. It changes how the terminal is built, and it changes how the terminal
looks only where the reference adaptation says it should.

The visual treatment is not invented here. It is adapted from the entries named in the
reference register, per the UI direction's reference-driven rule.

## Requirements

### Tokens

- A `Theme` singleton exposes colour, spacing, radius, border, elevation, duration and
  typography tokens. No screen reads a raw value.
- Colour tokens are semantic, not literal: surfaces by depth, borders by emphasis, text
  by rank, plus accent, and the status family (positive, negative, warning, critical,
  neutral, stale, disabled). Each is defined once, in one place, against the Radix dark
  scales, with the Tailwind values currently in the tree mapped onto them so that the
  migration is a rename wherever the existing colour was already right.
- Typography tokens name roles — UI text by size rank, and a numeric family with tabular
  figures for prices, quantities and identifiers. Inter and JetBrains Mono are vendored
  into the binary; the terminal never depends on a system font being installed.
- Spacing and sizing tokens come from one scale. Row heights, header heights, gutters and
  control heights are tokens, not per-file constants.
- A light theme is not required. The token layer must not assume one is impossible.

### Semantic layer

- QML never decides what a value means. `color: pnl > 0 ? "green" : "red"` and every
  variant of it is removed. A row or metric carries a semantic role supplied by the Rust
  side or by one mapping function, and the component resolves that role to a token.
- The roles cover at least: direction of a signed figure, order and deployment lifecycle,
  reconciliation state, health state, broker mode, and data freshness. Each role has one
  visual treatment, defined once.
- Live broker mode stays visually unmistakable, as Q-047 requires. That treatment becomes
  a token-backed role rather than an ad-hoc colour.

### Primitives

- A component set exists under one module and is registered in `qmldir`, built as a custom
  Qt Quick Controls style so that keyboard navigation, focus handling, interaction states
  and accessibility come from the substrate rather than being written per component.
- The set covers at minimum: `Panel`, `SectionHeader`, `Toolbar`, `AppButton` with its
  variants, `IconButton`, `StatusBadge`, `ConnectionIndicator`, `Metric`, `DataTable` with
  its header, row, cell and selection treatment, `TabBar`, `SplitPane`, `EmptyState`,
  `AppDialog`, and a text-field and combo-box restyle.
- Every component states its interaction states: rest, hover, pressed, focused,
  selected, disabled, and where it applies, degraded or stale. Focus is always visible.
- Each component's spec names the register entry it adapts and what was taken from it.

### Migration

- All eighteen existing QML files are migrated onto the tokens and primitives. The
  operations workspace keeps every behaviour Q-047 and Q-048 gave it — every field, every
  degraded state, every command and confirmation — and every existing test keeps passing
  unchanged.
- The icon set replaces whatever glyphs, text arrows or coloured dots currently stand in
  for icons.

### Gate

- A check in `make check` fails on a colour literal, a raw `pixelSize`, or a raw pixel
  dimension outside the theme directory, and on arithmetic over money- or quantity-named
  fields in QML, which Q-047 introduced as a grep-style test and this task subsumes.
- `qmllint` passes with no warnings over every QML file, not only `Main.qml`.

## Constraints and non-goals

- **No new window, no docking, no workspace persistence.** Q-052 and Q-053.
- **No new data, no new route, no new topic.** This task adds no request and changes no
  payload. It is a presentation change over the state Q-046 to Q-049 already hold.
- **No behaviour change in the operations workspace.** A field that was shown is still
  shown, in the same place, with the same source.
- **No screenshot baselines.** Q-054, once the shell has stopped moving layouts.
- **No frame-budget regression.** The chart path and the render node are untouched.
- **No trading logic anywhere in QML** (invariant 1), which is the point of the semantic
  layer.

## Acceptance criteria

### Agent-verifiable

1. The token gate reports zero colour literals, zero raw `pixelSize` and zero raw pixel
   dimensions outside `qml/theme/`, over all QML files. It is part of `make check`, and it
   fails when a literal is reintroduced, proven by a deliberate violation in the test.
2. `qmllint` passes with no warnings over every file listed in the QML module, and every
   component is registered in `qmldir`.
3. Every component in the set renders headlessly in each of its declared interaction
   states, and a test asserts that each state resolves to a distinct token — no state is
   silently identical to another.
4. The semantic mapping is tested exhaustively: every role has exactly one treatment, and
   a test fails if a role is added without one. No QML file contains a comparison against
   zero, a colour name, or a lifecycle string literal.
5. Fonts resolve from the vendored resources with the system font directories emptied in
   the test environment, and numeric text uses tabular figures, asserted by measuring that
   two different digit strings of equal length have equal advance width.
6. Every Q-035 to Q-049 test passes unchanged, including the degraded-state suite, the
   ops-views suite and the execution-controls suite.
7. `BENCH_EXECUTION_ROWS=10000 make bench-frames` stays within the §4.6 budget: p95 under
   16 ms, and no worse than the figure Q-047 recorded.
8. `docs/design/references.md` names a register entry for every component in the set, and
   `docs/design/components.md` records, per component, the reference adapted, what was
   taken and what was deliberately changed.
9. `make check` passes.

### Human-verifiable

1. The operations workspace is compared side by side against the adapted references at
   both monitor resolutions (1920×1080 and 2560×1440). Screenshots of the deployments
   list, a detail table, the header in a healthy state and the header in a worker-down
   state are recorded in the handoff.
   Command: `make run`
2. Keyboard-only operation: every control in the workspace is reachable by tab traversal
   with a visible focus indicator, and every dialog can be confirmed and dismissed without
   a mouse.
3. The live-mode treatment is confirmed still unmistakable against a paper deployment on
   the same screen.
