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
| `AppComboBox` | Qaterial, Qt Quick Controls | QQC2 popup/list and keyboard selection structure | Desktop-density popup rows and Lucide chevron asset |

## Asset and pipeline notes

- Radix dark scales provide the colour hierarchy, with incumbent Tailwind colours kept
  where they already express the intended terminal semantics.
- Inter and JetBrains Mono are shipped resources; Lucide supplies the icon subset.
- Figma to Qt 1.0 was assessed using `StatusBadge` as the round-trip specimen. Its
  generated QQC2 structure is useful as a layout handoff, but generated token bindings
  still require a manual pass to target the terminal singleton API. Later greenfield
  screens may start in Figma when a shared library exists; generated QML is not accepted
  directly into the component module.
