# Q-056 implementation plan: Manual chart target controls

**Specification:** [`../specs/Q-056-manual-chart-target-controls-spec.md`](../specs/Q-056-manual-chart-target-controls-spec.md)  
**Depends on:** Q-055

## Current system and file map

`src/chart_target.rs::ChartTargeter::select` accepts an optional deployment and always
falls back to the configured pair. `src/startup.rs` connects execution selection to that
targeter. `src/bar_feed.rs` exposes the actual feed target; `src/stream/client.rs`
retargets bar filters. `qml/Main.qml` owns one shared feed and the global selection.
`qml/panels/ChartPanel.qml` will contain Q-055's identity strip. `src/shell/commands.rs`
owns command IDs and shortcuts. `qml/components/AppComboBox.qml` and `AppTextField.qml`
provide keyboard/focus behavior.

## Ordered implementation

- [ ] 1. On the Q-056 task branch, add failing policy tests in `tests/chart_target.rs`:
  following→manual, deployment change during manual, manual→following, rapid target
  changes, and a delayed old-generation history result.
- [ ] 2. Add a Rust-owned `ChartSelection` mode and requested pair adjacent to
  `ChartTargeter` in `src/chart_target.rs`. Keep the committed feed pair separate from the
  requested pair. Route both deployment-follow and manual requests through the same
  generation-safe retargeter and overlay handling. Manual mode clears deployment overlays
  and markers; following mode restores them for the current deployment.
- [ ] 3. Expose chart selection and request methods to QML through the existing CXX-Qt
  bridge in `src/chart_bridge.rs`/`src/startup.rs`. Update `qml/Main.qml` so deployment
  selection changes execution context and asks the chart policy to follow only when in
  Following mode. Keep one feed/subscription across windows.
- [ ] 4. Add `qml/components/ChartTargetPicker.qml` using restyled Qt Quick Controls:
  direct symbol entry, suggestions from configured/deployment/recent successful pairs,
  timeframe choice, explicit apply, and `Follow selected deployment`. Wire it into the
  Q-055 chart strip. Reject blank/malformed input before retarget; report missing history
  and stream failures after retarget without showing the old pair as newly loaded.
- [ ] 5. Add noncolliding chart commands to `src/shell/commands.rs` and their routing in
  `qml/Main.qml`; add focus/shortcut tests to `tests/shell_windows.rs`. Extend gallery
  examples for open, manual, following, invalid, and no-data states. Document the
  Quantower and Qt Controls adaptations in `docs/design/components.md`.
- [ ] 6. Run targeted chart/stream/shell tests, inspect gallery shots at 1920×1080 and
  2560×1440, run `env -u WAYLAND_DISPLAY -u DISPLAY make check`, then commit focused
  changes on the task branch.

## Review focus

- Entering an unsupported symbol does not silently switch the settled chart label.
- A switch with an in-flight old response never paints the old pair under the new pair.
- Manual mode never changes the selected deployment or command enablement.
- Returning to Following uses the latest global selection, not the selection at entry.
- Multiple windows remain views of one shared chart target.
