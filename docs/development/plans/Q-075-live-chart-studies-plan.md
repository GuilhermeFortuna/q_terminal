# Q-075 implementation plan: Live studies on the chart

> **For implementation agents:** Read the linked spec, `AGENTS.md`, `README.md`, and
> `BOUNDARY.md`. Start this task only with `./work start Q-075 --agent <agent> --worktree`
> after Q-074 is Done with a released `q_core` tag and the board plan is approved. Keep
> changes on that task branch.

**Goal:** Operator-selected `q_core` studies on the live chart that update with the
forming bar.
**Architecture:** A `StudySet` beside `BarFeed` owns `q-indicators` streaming states and
emits `OverlaySeries`. `BarFeed` merges them with deployment overlays before
`line_layers`. QML adds a picker that calls feed invokables; it never computes values.
**Tech stack:** Rust, `q-indicators` (Q-074 tag), CXX-Qt, Qt 6/QML, fake stream tests.
**Spec:** [`../specs/Q-075-live-chart-studies-spec.md`](../specs/Q-075-live-chart-studies-spec.md)

## File map

- `Cargo.toml` / `Cargo.lock`: bump `q-qt`, `q-buffers` and `q-io` to the Q-074 tag
  and add `q-indicators` at the same tag.
- `src/studies.rs` (new): `StudyKind`, `StudySpec` (kind, params, palette index),
  `StudySet` (commit, preview, rebuild from `BarColumns`, output as `OverlaySeries`
  aligned to bar open times) and volume and session-key selection for VWAP.
- `src/bar_feed.rs`: own a `StudySet`, drive it from the completed, forming, history,
  target-change and gap-recovery paths, and expose `add_study`, `remove_study` and
  `study_list_json` invokables. Keep deployment overlays in their own vector; merge them
  only when building layers.
- `src/execution/overlays.rs`: allow several independently scaled oscillator
  sub-panes, and accept a colour role index in place of a literal for studies.
- `qml/theme/`: study palette and sub-pane tokens. `qml/components/StudyPicker.qml`
  (new); `qml/ChartPane.qml` toolbar entry.
- `docs/design/references.md`, `docs/design/components.md`, `BOUNDARY.md`.

## Ordered implementation

- [x] **1. Dependency and reference.** Bump the `q_core` crates to the Q-074 tag and add
  `q-indicators`; run `make check` to make sure the bump alone is clean. Add and confirm
  the chart-study **Look** reference in `docs/design/references.md`.
- [x] **2. `StudySet`.** Implement the study set in `src/studies.rs` with unit tests:
  outputs equal the batch kernel over the same bars, preview isolation, rebuild on
  history replacement, VWAP session reset and the mixed-volume unavailable state.
  Document the session-key derivation from the time label.
- [x] **3. Feed wiring.** Drive the study set from every `BarFeed` mutation path.
  Forming updates re-pack only the forming segment's study points where `line_layers`
  allows it; otherwise measure first and keep the full re-pack only if the budget
  holds. Target-reset and delivery integration tests in `bar_feed`; dedicated fake-stream
  gap/timeframe cases remain follow-ups.
- [x] **4. Panes and rendering.** Extend `line_layers` for per-study oscillator panes
  with their own scale and the RSI guide lines. Keep deployment overlay output
  byte-identical when no study is active (existing overlay tests).
- [x] **5. Picker.** Build `StudyPicker.qml` from the confirmed reference with theme
  tokens, keyboard navigation and the eight-study cap. `StudyPicker` is registered in
  `ComponentCatalog.js`; gallery PNG captures per state are follow-ups.
- [ ] **6. Review and handoff.** Update `BOUNDARY.md` and `docs/design/components.md`.
  Run `env -u WAYLAND_DISPLAY -u DISPLAY make check` and `make bench-frames` with eight
  studies. Do the live B3-open check from the spec if the market is open, and otherwise
  say that it is still outstanding. Commit focused changes and set Q-075 to In Review
  through `./work board set` with results and open follow-ups.

## Review focus

- No indicator arithmetic in `q_terminal`: every value comes from a `q_core` state.
- A forming preview never becomes committed state after a replacement, gap or target
  switch.
- VWAP is never computed over mixed volume kinds, and the unavailable reason is
  visible.
- Deployment overlays are unchanged in value and appearance when studies are
  added or removed.
