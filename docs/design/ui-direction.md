# Q Terminal — UI/UX and Dual-Display Design Direction

> **Provenance.** Written outside the repository and vendored here on 2026-09-19 as the
> UI direction of record, amended by its author with the reference-driven sourcing rule in
> §3. Specs cite it by section.
>
> **Superseded sections.** §11's sequencing predates the board: Q-047 and Q-048 are
> complete and Q-049 is in progress, so the design-system work is a retrofit rather than
> something that precedes them. The work this document asks for is specified as Q-051
> (design system), Q-052 (multi-window shell), Q-053 (workspaces and displays) and Q-054
> (gallery and visual baselines). §15's planning questions are answered in
> [`planning-answers.md`](planning-answers.md).

## Purpose

Define the intended UI/UX direction for `q_terminal` before the operations and execution interface expands substantially.

The terminal should be treated as a professional native trading workstation rather than a conventional desktop dashboard.

The immediate goal is not to redesign the application architecture, but to establish a coherent visual and interaction architecture that future work such as Q-047, Q-048, Q-049, order-flow analytics, tape reading, DOM, footprint charts, and volume intelligence can build upon consistently.

---

## 1. Core UI Architecture

The UI should follow this responsibility split:

```text
QML / Qt Quick
    ↓
visual design
layout
interaction
window management
workspace composition

Rust
    ↓
application state
models
execution state
market state
business logic
view-facing state

C++ / Qt Scene Graph
    ↓
specialized high-performance rendering only
candles
large marker sets
heatmaps
footprints
DOM graphics
order-flow visualization

q_core
    ↓
deterministic computational semantics
market analytics
execution semantics
shared research/live calculations
```

### Principle

QML is the primary frontend technology.

Rust and C++ should not be used for ordinary visual composition unless necessary.

C++ should remain restricted primarily to rendering hot paths or Qt integration that cannot reasonably be handled through Rust/QML.

---

## 2. Design Quality Is a First-Class Requirement

A polished interface is an explicit project requirement.

The terminal should not accumulate individually styled QML components without a shared visual language.

Before the execution UI becomes large, establish a reusable design system.

Suggested structure:

```text
qml/
├── theme/
│   ├── Theme.qml
│   ├── Typography.qml
│   ├── Spacing.qml
│   └── Icons.qml
│
├── components/
│   ├── AppButton.qml
│   ├── IconButton.qml
│   ├── Panel.qml
│   ├── SectionHeader.qml
│   ├── StatusBadge.qml
│   ├── Metric.qml
│   ├── DataTable.qml
│   ├── TabBar.qml
│   ├── Toolbar.qml
│   ├── SplitPane.qml
│   └── ConnectionIndicator.qml
```

Avoid repeated hardcoded values such as:

```qml
color: "#1e222d"
font.pixelSize: 12
```

Prefer centralized tokens:

```qml
color: Theme.surface1
font.pixelSize: Theme.fontSm
```

The design system should cover at minimum:

- colors;
- surfaces;
- borders;
- spacing;
- typography;
- numeric typography;
- status colors;
- interaction states;
- selection states;
- focus states;
- table styles;
- panel headers;
- button variants;
- disabled states;
- warnings/errors;
- data freshness/degraded-state presentation.

---

## 3. Visual Design Direction

Q Terminal should visually resemble a modern professional workstation rather than a generic SaaS dashboard.

Desired characteristics:

- high information density;
- restrained visual noise;
- strong hierarchy;
- compact controls;
- precise alignment;
- tabular/monospaced numeric presentation where appropriate;
- dark theme designed for extended use;
- restrained color usage;
- clear status semantics;
- excellent hover/focus/selection states;
- keyboard-friendly interaction;
- resizable panels;
- minimal decorative UI.

Reference categories:

- modern Qt automotive interfaces for rendering and polish;
- MuseScore for mature Qt desktop UI structure;
- Wireshark for dense professional information presentation;
- Quantower for trading workspace composition;
- Bookmap for market microstructure/order-flow visualization.

The goal is not to copy any specific product.

The intended combination is:

```text
Modern Qt polish
       +
professional desktop ergonomics
       +
high information density
       +
trading-specific visualization
```


### Reference-Driven Visual Implementation

Q Terminal should **not visually invent ordinary interface components or screen patterns from scratch**.

For every significant screen, panel, control, table, toolbar, navigation pattern, status surface, order-entry surface, or workspace composition, implementation should begin by identifying high-quality existing references from publicly accessible sources online. Preferred sources include:

- premium UI/component libraries and design systems;
- polished commercial desktop applications;
- professional trading terminals;
- high-quality Figma/community design resources;
- Qt/QML showcases and mature Qt applications;
- well-executed web components whose visual treatment can be translated to QML.

The selected reference should then be **adapted faithfully to Q Terminal's domain, density, interaction model, and design tokens**, rather than asking an implementation agent to invent a new visual treatment.

This is the default design process, not an optional source of inspiration.

```text
Find strong existing reference
        ↓
select the best-fit component/pattern
        ↓
adapt structure + visual treatment to Q Terminal
        ↓
translate into reusable QML primitives
        ↓
validate against the reference
```

Agents should not respond to visual tasks by creating generic buttons, cards, tables, panels, toolbars, tabs, dialogs, status indicators, or layouts from a blank canvas when a stronger existing design can be adapted.

For highly domain-specific surfaces such as DOM, footprint, tape, heatmaps, or execution ladders, the implementation may necessarily be custom, but the **visual language and interaction model should still be derived from strong existing professional references rather than invented ad hoc**.

The goal is to separate:

```text
visual design decisions     ← reference-driven
QML/Rust/C++ implementation ← project-specific
```

---

## 4. Design Workflow

Major QML screens should not be designed ad hoc during implementation.

Preferred workflow:

```text
requirements
    ↓
information architecture
    ↓
reference search / component selection
    ↓
wireframe using selected patterns
    ↓
high-fidelity adaptation / Figma
    ↓
design tokens
    ↓
reusable QML components
    ↓
screen implementation
    ↓
visual comparison against references / refinement
```

For any substantial visual task, the implementation brief should include the selected external reference(s) before coding begins. The agent's job is primarily to adapt and integrate proven visual patterns into Q Terminal, not to originate an unrelated visual system.

Implementation agents should receive all of the following:

- behavioral requirements;
- selected external visual reference(s);
- the intended adaptation into Q Terminal;
- applicable design tokens and reusable QML primitives.

A visual implementation should not begin without a concrete reference direction.

This becomes increasingly important as Q-047 and later terminal features add substantial interface complexity.

---

## 5. Dual-Display as a First-Class Terminal Capability

`q_terminal` should be designed to make excellent use of multi-monitor systems.

However:

> Q should be dual-display aware, not dual-display dependent.

A two-monitor setup should feel like the preferred workstation configuration, while the application must remain fully usable with one monitor.

---

## 6. Multi-Window Architecture

Do not implement dual-screen support as one oversized application window stretched across monitors.

Prefer multiple top-level Qt/QML windows backed by shared Rust state.

Conceptually:

```text
q_terminal
│
├── MarketWindow
│
└── OperationsWindow
```

Both windows consume the same underlying application state:

```text
MarketDataStore
ExecutionStore
RiskState
ConnectionState
       │
       ▼
Shared Rust application state
       │
       ├───────────────┐
       ▼               ▼
 MarketWindow    OperationsWindow
     QML              QML
```

This avoids:

- duplicated terminal processes;
- duplicated stream subscriptions;
- duplicated execution state;
- independent inconsistent UI state.

---

## 7. Proposed Default Dual-Display Layout

### Display 1 — Market / Analysis

Primary purpose:

- understanding current market state;
- charting;
- discretionary analysis;
- future order-flow tooling.

Example:

```text
┌───────────────────────────────────────────────┐
│ Instrument / timeframe / session             │
├───────────────────────────────────────────────┤
│                                               │
│                  Main Chart                   │
│                                               │
│ candles / overlays / executions / signals    │
│                                               │
├──────────────────────────┬────────────────────┤
│ Footprint / Delta        │ Volume Profile     │
├──────────────────────────┴────────────────────┤
│ Tape / Time & Sales                           │
└───────────────────────────────────────────────┘
```

Potential future components:

- main chart;
- volume;
- cumulative delta;
- footprint;
- volume profile;
- tape;
- market intelligence;
- order-flow signals;
- liquidity visualization.

---

### Display 2 — Execution / Operations

Primary purpose:

- execution;
- risk;
- account state;
- deployments;
- operational awareness.

Example:

```text
┌───────────────────────────────────────────────┐
│ Account │ P&L │ Risk │ Connection │ Latency  │
├───────────────────────┬───────────────────────┤
│                       │                       │
│      DOM / Ladder     │      Order Entry      │
│                       │                       │
├───────────────────────┴───────────────────────┤
│ Positions / Orders / Fills                    │
├───────────────────────────────────────────────┤
│ Deployments / Risk Events / Alerts            │
└───────────────────────────────────────────────┘
```

Likely Q-047/Q-048 features should naturally evolve toward this screen.

---

## 8. Workspace Model

Do not hardcode:

```text
Monitor 1 = market
Monitor 2 = execution
```

Instead, introduce the concept of saved workspaces.

Potential examples:

```text
Trading
Execution
Tape Reading
Market Analysis
Operations
Monitoring
```

A workspace should eventually be able to define:

- windows;
- panels;
- panel positions;
- monitor assignment;
- panel sizes;
- selected instrument;
- selected timeframe;
- optional tools;
- visibility state.

Example conceptual configuration:

```toml
[workspace.trading.market_window]
screen = 0
maximized = true

[workspace.trading.operations_window]
screen = 1
maximized = true
```

The exact persistence mechanism should be designed by the planning agent.

---

## 9. Single-Monitor Fallback

The system must degrade cleanly when only one monitor is available.

Desired behavior:

```text
2+ displays
    ↓
restore multi-window workspace

1 display
    ↓
merge the same workspace into panes/tabs/windows
```

No core feature should require a second monitor.

A laptop or single-display user should still have complete functionality.

---

## 10. Display Detection and Restoration

Qt should be used to:

- enumerate screens;
- identify available displays;
- position windows;
- restore previous layout;
- detect removed monitors;
- move inaccessible windows back to an available display.

Saved state should not rely solely on numeric screen indexes because monitor ordering may change.

The planning phase should investigate stable screen identification such as:

- screen name;
- manufacturer/model where available;
- geometry;
- persisted fallback rules.

---

## 11. Relationship With Current Roadmap

Current terminal work should remain the priority.

Do not allow the design-system or multi-display work to derail the Q-047 → Q-050 execution migration.

Recommended sequencing:

```text
Current:
Q-046 complete

Next:
Q-047 Operations Views

Then:
Q-048 Execution Controls

Then:
Q-049 Chart Execution Markers

Then:
Q-050 Remove Execution UI from q_frontend
```

However, before Q-047 grows significantly:

- establish basic visual tokens;
- establish reusable components;
- define shell/window architecture;
- avoid locking the terminal into a single-window architecture that later becomes expensive to undo.

The full configurable workspace system can be deferred.

---

## 12. Future Market-Intelligence Integration

After the execution-terminal migration is complete, the terminal is expected to gain specialized real-time market analysis capabilities.

Candidate systems:

```text
trade classification
volume delta
cumulative delta
volume at price
volume profile
trade velocity
aggressive volume
large-print detection
absorption
exhaustion
sweep detection
bid/ask imbalance
liquidity migration
footprint charts
DOM analytics
tape / time & sales
order-book heatmaps
```

These should generally follow:

```text
Raw market data
      ↓
q_backend ingestion / streaming
      ↓
q_core deterministic analysis
      ↓
contracted output/state
      ↓
q_terminal visualization
```

Important architectural goal:

> Historical research and live terminal analysis should use the same deterministic analytical semantics wherever practical.

For example, order-flow or tape calculations should preferably exist in `q_core`, rather than being independently reimplemented inside QML.

---

## 13. Rendering Strategy for Future Market Tools

Normal interface components:

```text
QML
```

High-volume visual rendering:

```text
Rust
  ↓
C++ / Qt Scene Graph
  ↓
GPU
```

Potential candidates for custom render nodes:

- candlesticks;
- footprint cells;
- dense execution markers;
- volume heatmaps;
- depth heatmaps;
- DOM visualization;
- order-flow bubbles;
- large historical marker sets.

Avoid creating thousands of individual QML objects for high-frequency market graphics.

---

## 14. Domain Logic Boundary

QML should contain as little trading/domain logic as possible.

Bad direction:

```qml
color: pnl > 0 ? "green" : "red"
```

repeated throughout arbitrary components.

Prefer centralized semantic presentation or Rust-backed state.

QML should primarily decide:

- where something appears;
- how it is arranged;
- how it looks;
- how the user interacts with it.

Rust/q_core should determine:

- what the state means;
- calculation results;
- execution state;
- market semantics;
- risk semantics.

---

## 15. Key Planning Questions

The planning agent should determine:

1. What minimum design-system work should happen before Q-047?
2. Should multi-window infrastructure be introduced during Q-047 or immediately afterward?
3. What state belongs globally versus per-window?
4. How should panels be represented?
5. Should dockability/resizing use Qt Quick Controls, custom QML, or another Qt mechanism?
6. How should workspace layouts be serialized?
7. How should monitor identities be persisted robustly?
8. How should single-monitor fallback work?
9. Which components should exist before operations views are implemented?
10. Which future visualization types will require custom scene-graph rendering?
11. How should keyboard commands and focus traversal work across multiple windows?
12. How should visual regression or screenshot testing be incorporated?
13. Whether QML should expose a formal design-token API from the beginning.
14. Whether a Figma component library should mirror the QML component system.
15. How visual references should be selected and attached to implementation tasks.
16. Which external component/design sources should be preferred for recurring UI categories.
17. How visual comparison against the selected references should be incorporated into review.

---

## 16. Constraints

The following existing architectural principles must remain intact:

- `q_terminal` is for live trading and operations, not research workflows.
- `q_terminal` owns no backend processes.
- deterministic computational semantics belong in `q_core`.
- cross-process state is governed by `q_contracts`.
- the terminal consumes backend state rather than inventing competing state models.
- UI expansion must not weaken execution safety or recovery semantics.
- performance-sensitive market rendering should not compromise UI responsiveness.
- significant visual components and screen patterns must be reference-driven rather than designed from a blank canvas.

---

## Desired Outcome

The long-term goal is for Q Terminal to feel like a purpose-built professional trading workstation:

```text
native
fast
dense
precise
multi-monitor aware
highly polished
keyboard friendly
GPU accelerated where necessary
operationally trustworthy
```

The terminal should be visually and ergonomically capable of supporting both:

- professional live execution;
- advanced real-time market intelligence and order-flow analysis.

The planning work should establish the foundation for this without prematurely implementing the entire future workstation.