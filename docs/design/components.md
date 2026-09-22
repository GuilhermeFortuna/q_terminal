# Terminal component reference adaptations

This register is the implementation record for Q-051. It complements
[`references.md`](references.md): that file identifies approved sources, while this one
records what each terminal component actually adapts. All components retain Qt Quick
Controls behaviour for focus, keyboard input and accessibility.

| Component | Reference | Adapted | Deliberate terminal change |
| --- | --- | --- | --- |
| `Panel` | MuseScore 4 | Recessed work area, quiet one-pixel frame and compact title edge | Deeper surface hierarchy for dense market data; no floating-panel chrome until Q-052 |
| `SectionHeader` | MuseScore 4 | Small, high-contrast section title and restrained divider | Uppercase label and numeric font option suit instrument identifiers |
| `Toolbar` | MuseScore 4 | Compact horizontal command strip with grouped actions | Uses terminal spacing and no consumer-style oversized command icons |
| `AppButton` | FluentUI for QML, Qt Quick Controls | QQC2 button substrate and explicit hover, press, focus and disabled states | Compact heights; destructive and live variants are visually unambiguous |
| `IconButton` | FluentUI for QML, Lucide | QQC2 icon-button behaviour and Lucide's consistent stroke grid | Always exposes a text tooltip and visible keyboard focus |
| `StatusBadge` | Grafana, Linear | Restrained filled badge with semantic foreground/background pairing | Live and critical roles use stronger borders; labels remain readable without colour |
| `ConnectionIndicator` | Grafana | Small status mark paired with text | Never relies on an unlabelled coloured dot; stale and degraded remain distinct |
| `Metric` | Grafana | Ranked label/value typography and compact grouping | Numeric values use vendored JetBrains Mono and tabular figures |
| `DataTable` | Wireshark | Dense virtualised rows, fixed header and detail-friendly selection | Dark zebra surfaces replace Wireshark's platform palette |
| `DataTableHeader` | Wireshark | Compact uppercase column labels and clear column boundaries | Token-backed height and spacing; no sortable affordance until sorting exists |
| `DataTableRow` | Wireshark | Tight row height, zebra treatment, hover and selection | Strong visible focus ring supports keyboard row navigation |
| `DataTableCell` | Wireshark | Ellided, aligned table content | Numeric/identifier mode uses JetBrains Mono with tabular figures |
| `TabBar` | MuseScore 4, Qt Quick Controls | Compact desktop tab strip on QQC2 tab behaviour | Accent underline instead of a raised document-tab silhouette |
| `SplitPane` | Qt Quick Controls, Quantower | Native split-view resizing and dense workspace division | Handle is intentionally quiet until hovered or focused |
| `EmptyState` | Linear | Short title, muted explanation and optional action | Kept compact so an empty operational panel does not resemble onboarding |
| `AppDialog` | FluentUI for QML, Qt Quick Controls | Native dialog focus scope, buttons and modal keyboard behaviour | Strong live/critical warning treatment is token-backed and consequence-first |
| `AppTextField` | FluentUI for QML | QQC2 editing, selection and focus behaviour with a token-backed frame | Dense control height and numeric-family option |
| `PanelFrame` | Quantower, MuseScore 4 | Quantower's panel as the unit of a workspace, with a per-panel title and focus state; MuseScore's compact panel header | Focus ring is the only chrome that moves with keyboard focus; the float/dock action is one icon button, no drag handle (see the docking note) |
| `PanelHost` | Quantower, Qt Quick Controls | Splits and tab groups arranged per window; native `SplitView` resizing and `TabBar` keyboard behaviour | A slot borrows a persistent panel item by reparenting rather than creating one, so a moved panel keeps its state; KDDockWidgets was not adopted |
| `CommandPalette` | Linear | A single filtered list, shortcut at the right edge, restraint at density | Commands come from the Rust registry, disabled ones stay listed with the reason; modal to its own window only |
| `ShellWindow` chrome | Quantower | A window-level strip for the workspace actions and a visible detached-selection banner | The banner is warning-coloured: a silently detached window is an operational hazard |
| `Workstation toolbar` | MuseScore 4, Quantower | One compact desktop strip groups workspace persistence, window arrangement, selection scope, and command discovery | The active workspace stays visible in the picker; command palette remains an explicit keyboard-accessible action rather than consuming permanent chrome |
| `Placement notice` | Quantower | Concise workspace-placement state with an on-demand explanation | It never suggests Q Terminal can place compositor windows; rule export remains available in the disclosure |
| `Operations health` | Grafana | Severity-ranked operational summary with wrapped, labelled health facts | Critical state stays persistently visible, every health field remains text-labelled, and unavailable worker heartbeat is shown as `unknown` |
| `Deployment empty states` | Wireshark | Dense, action-oriented empty and no-selection prompts | Table chrome and paging controls disappear until a deployment is selected; the chart remains independent and usable |
| `AppComboBox` | Qaterial, Qt Quick Controls | QQC2 popup/list and keyboard selection structure | Desktop-density popup rows and Lucide chevron asset |
| `ChartIdentity` | Quantower, Grafana, MuseScore 4 | Quantower's chart context line (symbol, timeframe, series type); Grafana's ranked freshness badge separate from transport health; MuseScore's compact two-line panel header | Target source and pending retarget stay distinct from the feed's committed symbol; long deployment names truncate with tooltip; condition never treats a connected socket alone as live |
| `ChartTargetPicker` | Quantower, TradingView | Symbol direct entry and suggestions popup, timeframe selector, explicit apply button, and follow-deployment restoration | Single shared feed model across windows; suggestions categorized into Configured, Deployment, and Recent; keyboard navigation (Enter/Return, Esc, arrows) and inline recoverable error chip |

## Asset and pipeline notes

- Radix dark scales provide the colour hierarchy, with incumbent Tailwind colours kept
  where they already express the intended terminal semantics.
- Inter and JetBrains Mono are shipped resources; Lucide supplies the icon subset.
- The Q-052 panel icons (`activity`, `list`, `table`, `chart-candlestick`, `external-link`,
  `arrow-down-to-line`) are Lucide glyphs with the stroke fixed to the `textSecondary` colour (`#b8b8b8`, Q-057):
  Qt renders `currentColor` black, which vanishes on the dark surfaces, and the software
  backend cannot recolour an image at runtime.
- Figma to Qt 1.0 was assessed using `StatusBadge` as the round-trip specimen. Its
  generated QQC2 structure is useful as a layout handoff, but generated token bindings
  still require a manual pass to target the terminal singleton API. Later greenfield
  screens may start in Figma when a shared library exists; generated QML is not accepted
  directly into the component module.

## Q-057 true-black neutral token map

Radix's stepwise surface/contrast approach, with the blue hue removed: every neutral has
equal RGB components (asserted in `tests/design_system.rs`). Semantic colours are unchanged.

| Level | Tokens |
| --- | --- |
| Canvas | `surfaceSunken`, `surfaceBase` `#000000` (workspace, chart, controls at rest) |
| Surfaces | `surfaceRaised` `#0f0f0f`, `surfaceOverlay` `#171717`, `surfaceElevated` `#1f1f1f`, `surfaceSelected` `#2a2a2a`, `surfaceHover` `#333333` |
| Borders | `borderSubtle` `#2a2a2a` (grid, separators), `borderDefault` `#3d3d3d`, `borderStrong` `#666666` |
| Text | `textMuted` `#8c8c8c`, `textTertiary` `#9e9e9e`, `textSecondary` `#b8b8b8`, `textPrimary` `#e2e2e2`, `textStrong` `#fafafa` |

Measured contrast (WCAG, base/raised/overlay/elevated/selected/hover):

| Foreground | Ratios |
| --- | --- |
| `textMuted` | 6.2 / 5.7 / 5.3 / 4.9 / 4.3 / 3.8 |
| `textTertiary` (axes, table header) | 7.8 / 7.2 / 6.7 / 6.2 / 5.4 / 4.7 |
| `textSecondary` | 10.6 / 9.7 / 9.0 / 8.3 / 7.2 / 6.4 |
| `textPrimary` | 16.2 / 14.8 / 13.8 / 12.7 / 11.1 / 9.8 |
| `warning` / `negative` / `positive` | all >= 4.6 on every surface (warning 7.6+, positive 6.6+) |
| `critical` | 5.4 / 4.9 / 4.6 / 4.2 / 3.7 / 3.2 |
| Banners: `warningText` on `warningSurface`, `negativeText` on `negativeSurface`, white on `criticalSurface` | 13.5, 14.8, 10.0 |

Exceptions: `textMuted` on selected/hover (4.3, 3.8) is used only for secondary or disabled
text, which is exempt from the 4.5 target but stays above 3:1. `critical` as text on
elevated/selected/hover (4.2 / 3.7 / 3.2) meets only the 3:1 large-text target on hover; critical
states are always paired with a text label and the `criticalSurface` banner (10.0), never colour
alone. Grid and separators (`borderSubtle`, 1.5:1 on black) are deliberately quiet graphics.

Icons `activity`, `list`, `table`, `chart-candlestick`, `external-link` and `arrow-down-to-line`
use `#b8b8b8`; the execution overlay separator uses `#2a2a2a`.
