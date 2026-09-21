# Q-057 implementation plan: True black terminal theme

**Specification:** [`../specs/Q-057-true-black-terminal-theme-spec.md`](../specs/Q-057-true-black-terminal-theme-spec.md)  
**Depends on:** Q-051

## Current system and file map

`qml/theme/Palette.qml` maps most neutral surfaces to blue-gray values; `Theme.qml`
re-exports them. `qml/theme/Semantic.qml` maps state roles to those tokens. Controls in
`qml/components/` and `qml/style/` consume them. `qml/gallery/` and `make gallery-shot`
show component states. `tools/token_gate.py` forbids screen-level literals.

## Ordered implementation

- [ ] 1. On the Q-057 task branch, record a neutral token map in
  `docs/design/components.md`: canvas `#000000`, neutral charcoal surface levels,
  border levels, text levels, and preserved semantic colors. Adapt Radix's stepwise
  surface/contrast approach without retaining its blue-neutral hue.
- [ ] 2. Update only `qml/theme/Palette.qml`, `Theme.qml`, and `Semantic.qml` for the
  neutral ladder and semantic pairings. Keep token names stable where possible to avoid
  unrelated screen changes. Add a focused design-system test in `tests/design_system.rs`
  that asserts the base canvas is black and neutral surfaces have equal RGB components.
- [ ] 3. Capture the gallery with `env -u WAYLAND_DISPLAY -u DISPLAY make gallery-shot`.
  Inspect the PNGs with an image tool. Fix clipped or low-contrast states through tokens
  or reusable components; do not add local screen color literals.
- [ ] 4. Check contrast for text/surface pairs used by controls, tables, warning/critical
  banners, dialogs, and chart axes. Record measured pairs and any documented exception in
  `docs/design/components.md`. Inspect the merged, market, and operations compositions
  at 1920×1080 and 2560×1440.
- [ ] 5. Update fixed-color icons/assets whose strokes disappear on black. Save and
  inspect gallery and screen captures; Q-054 will record baselines after the redesign.
- [ ] 6. Run `env -u WAYLAND_DISPLAY -u DISPLAY make check`, inspect final captures,
  and commit focused changes on the task branch.

## Review focus

- Neutral surfaces and separators remain visible without a blue cast.
- Warning and critical banners retain legible text and distinct labels.
- Selected table rows, focused controls, and disabled buttons remain distinguishable.
- Chart grid/axes do not disappear and positive/negative bars remain distinct.
- Gallery and screen captures document the final theme for Q-054 to baseline later.
