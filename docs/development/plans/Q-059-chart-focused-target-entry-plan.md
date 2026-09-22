# Q-059 implementation plan: Chart-focused symbol and timeframe entry

> **For agentic workers:** Read the linked spec and `AGENTS.md`/`BOUNDARY.md` first. Use the task branch created by `./work start Q-059 --agent <agent> --worktree`. Track each step with its checkbox.

**Goal:** Let chart-focused typing request a symbol, timeframe, or both through Q-056's existing target path.
**Architecture:** The chart canvas owns only the keystroke-to-prompt interaction; a small parser turns the draft into a pair. `ChartContext::request_target` remains the authority for validation and retargeting.
**Tech stack:** Qt 6/QML, Rust CXX-Qt bridge, headless Qt tests.
**Spec:** [`../specs/Q-059-chart-focused-target-entry-spec.md`](../specs/Q-059-chart-focused-target-entry-spec.md)

## Current system and file map

`qml/panels/ChartPanel.qml` composes `ChartIdentity` and `ChartPane`.
`qml/components/ChartTargetPicker.qml` owns the visible Q-056 symbol/timeframe controls and
calls `context.request_target`. `qml/shell/PanelFrame.qml` currently focuses the panel on
click. `src/chart_context.rs` owns manual/following state and target errors. Existing
`Ctrl+Shift+C` routing is in `src/shell/commands.rs` and `qml/Main.qml`.

## Ordered implementation

- [x] 1. Add a focused parser unit (prefer `src/chart_target_input.rs` if parsing cannot
  stay small and clear in QML) with table cases for `PETR4`, `5m`, `PETR4 5m`, lowercase,
  extra whitespace, blank input, two symbols, and unsupported timeframes. Assert that a
  one-token input preserves the other effective field and that errors name the input.
  Run its targeted test and observe failure before implementation.
- [x] 2. Add `qml/components/ChartTargetPrompt.qml` using `AppTextField` and existing
  theme tokens. It owns draft text, parsed preview, Enter, Escape, and click-away only;
  `ChartPane` or `ChartPanel` provides the effective pair and calls the existing
  `request_target` on submission. Keep focus in the field while an error is corrected.
- [x] 3. Route canvas key input in `qml/ChartPane.qml`: a printable alphanumeric key
  starts the prompt with that character; key handling is active only when the chart
  canvas has focus. Make canvas click focus it, without stealing focus from header
  controls. Preserve `Ctrl+Shift+C` and visible picker behavior.
- [x] 4. Extend the headless chart/QML probe in `cpp/chart_cxx.cpp`, its declarations,
  and `tests/test_chart_bridge.rs` or a dedicated chart interaction test. Assert focused
  `PETR4`, `5m`, and combined input request the right pair; Escape, click-away, invalid
  input, typing in another field, and focus in another window do not retarget.
- [x] 5. Add open and error examples to `qml/gallery/`; document the Quantower context
  and Qt Controls focus adaptations in `docs/design/components.md`. Run
  `env -u WAYLAND_DISPLAY -u DISPLAY make gallery-shot`, inspect both workstation sizes,
  then run `env -u WAYLAND_DISPLAY -u DISPLAY make check`. Commit focused changes on
  the Q-059 task branch and set the board to In Review with checks and any follow-ups.

## Review focus

- Text fields and the command palette keep every typed character.
- A timeframe-only entry cannot accidentally become a symbol.
- Rejected input leaves the settled identity and old bars consistent.
- Hidden or unfocused charts never receive keystrokes from another window.
- Prompt closure never applies an unsubmitted draft.
