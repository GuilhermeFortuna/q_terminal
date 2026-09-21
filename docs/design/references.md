# Visual reference register

`q_terminal` does not invent ordinary interface components or screen patterns. Every
significant surface — panel, table, toolbar, control, status surface, order-entry
surface, workspace composition — begins from a named external reference, which is then
adapted to the terminal's domain, density and design tokens.

This file is that register. A task's spec names the entries it adapts; a review checks the
result against them. An entry is added here before the visual work it governs begins,
never after.

## How an entry is used

The register sorts entries by *what we actually take from them*, because that decides how
an implementing agent works with each one.

| Class | What we take | How the work goes |
| --- | --- | --- |
| **Ship** | The thing itself — code, values, glyphs, font files — vendored into the repository or the binary | Add the dependency or the asset and use it |
| **Read** | Structure and implementation approach from a component set, or generated output from a design pipeline | Open the source, take the structure, restyle to our tokens |
| **Look** | Layout, density, hierarchy and interaction from a finished product | Study the screen, rebuild the arrangement in our primitives |

## Ship

| Entry | Source | Used for |
| --- | --- | --- |
| Radix Colors | <https://github.com/radix-ui/colors> | The dark colour scales behind `Theme` surface/border/text/accent tokens. Its 12-step scales are built for exactly the surface/border/text separation the terminal needs. |
| Tailwind palette (incumbent) | <https://tailwindcss.com/docs/colors> | The colours already in the codebase are Tailwind's — `#64748b` is slate-500, `#34d399` emerald-400, `#f87171` red-400. Mapped onto named tokens during the Q-051 migration, so most of the migration is a rename rather than a recolour. |
| Inter | <https://github.com/rsms/inter> | UI typeface. Its `tnum` tabular-figures feature is what makes columns of prices align. |
| JetBrains Mono | <https://github.com/JetBrains/JetBrainsMono> | Numeric and identifier typeface: prices, quantities, intent identifiers, log-like rows. |
| Lucide | <https://github.com/lucide-icons/lucide> | Icon set. Consistent 24px grid and stroke weight, and an SVG-per-icon layout that vendors cleanly into a `qrc`. |
| Tabler Icons | <https://github.com/tabler/tabler-icons> | Fallback icon source for glyphs Lucide lacks — ladder, depth, footprint-adjacent shapes. |
| Qt Quick Controls | Qt 6.11, already a dependency | The control substrate. Keyboard navigation, focus handling, interaction states and accessibility come from here; the terminal supplies only the look, as a custom style. |
| KDDockWidgets | <https://github.com/KDAB/KDDockWidgets> | Docking, floating panels, tab merging and layout save/restore, with supported QtQuick bindings. Actively maintained. Q-052's spike was not built and the shell uses `SplitView` panes with custom float, dock, tab and merge instead; see [`../development/notes/Q-052-docking-decision.md`](../development/notes/Q-052-docking-decision.md). |

## Read

| Entry | Source | Used for |
| --- | --- | --- |
| Figma to Qt 1.0 | <https://www.qt.io/blog/figma-to-qt-1.0> | The design→QML pipeline. Converts a Figma frame to a QML project, ships pre-annotated Qt Quick Controls, and carries shared libraries with components and tokens. Replaces the deprecated Qt Bridge for Figma. |
| FluentUI for QML | <https://github.com/zhuzichu520/FluentUI> | A large, maintained QML component set to read and adapt: control structure, state handling, and how a token layer threads through a QQC2-based set. Its *visual* language is consumer Windows and is not adopted. |
| Qaterial | <https://github.com/OlivierLDff/Qaterial> | Secondary QQC2 component source, useful where FluentUI lacks a control. |

## Look

| Entry | What it is referenced for |
| --- | --- |
| Quantower | Workspace composition: how panels, tabs and linked instrument context are arranged across windows and monitors. |
| Bookmap | Market-microstructure visualisation: heatmap axis treatment, colour ramps for liquidity, how time and price axes stay readable at density. |
| Sierra Chart, ATAS, Jigsaw | DOM/ladder and footprint conventions: column ordering, centring behaviour, how working orders and position are drawn on the ladder. |
| Wireshark | Dense professional tables: row height, zebra treatment, column alignment, selection and detail-pane pairing at thousands of rows. |
| MuseScore 4 | Mature Qt desktop structure: toolbar density, panel headers, inspector patterns. |
| Grafana, Linear | Dark-theme hierarchy and restraint at high density; status and badge semantics. |
| Figma community trading kits — TradeStackUI, Trading Orderbook UI Kit Template, PopTrade | Order-entry and order-book visual treatment. Identified by search and not yet opened; confirm each is actually good before a spec cites it. |

## Using this register

1. Identify the UI category the work touches.
2. Pick the entry or entries to adapt, and say in the spec what is taken from each.
3. For a **Look** entry, state the adaptation: the reference's density and arrangement,
   expressed in this project's tokens and primitives.
4. Record the comparison in review: the reference, the result, and the deliberate
   differences.

A category with no entry is a gap to close before its visual work starts, not permission
to design from a blank canvas.
