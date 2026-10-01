# Q-082 implementation plan: Tape panel and volume studies

> **For implementation agents:** Read the linked spec and repository instructions.
> Use superpowers:executing-plans when that skill is available. Start only through
> `./work start Q-082 --agent <agent> --worktree` after written-plan approval and
> completed dependencies. Implement this task natively; delegation requires separate authorization.

**Goal:** Show the current session’s trade tape and configurable volume studies beside the live chart.
**Architecture:** Rust TradeFeed owns snapshot/replay, identity tracking, temporary chunks and VolumeState; QML owns arrangement.
**Spec:** [Specification](../specs/Q-082-tape-panel-and-volume-studies-spec.md)
**Status:** written plan awaiting human review.

## Global constraints

- The linked spec defines the interface, defaults and acceptance criteria; do not widen scope.
- Use the existing repository toolchain and canonical checks without resource-slice wrappers.
- Commit focused changes on the task branch. Never push, merge or change protected branches.
- Report unavailable prerequisites with the documented board workflow; do not substitute shortcuts.

## Ordered implementation

- [ ] 1. Vendor Q-079, pin Q-081 and add fake trade snapshots/pages/status to the existing server harness. Add tests/trade_feed.rs for subscribe/buffer/history/watermark joins, duplicates, expiry and target generations.
- [ ] 2. Implement typed trade decode/sink and src/trades feed/history modules with bounded columnar cache and shared process ownership. Test source correction, cache limit, disconnect, late pages and timeframe-only rebuild.
- [ ] 3. Project Q-081 outputs into study panes/readout and bounded large-print markers. Add tests/volume_studies.rs for exact kernel agreement, UTC/local bar alignment, threshold edits and historical unavailable regions.
- [ ] 4. Register the Quantower tape reference in docs/design/references.md; adapt the tape panel with a virtualized model and display-only filters. Inspect gallery captures incrementally and record adaptations in components.md.
- [ ] 5. Add v4 typed volume-study/tape settings and migrations on Q-077 autosave. Test close/reopen with changed threshold, row filters and layout; keep existing price-study serialization valid.
- [ ] 6. Extend frame benchmark fixtures for a dense trade burst while candles and studies are active; document the exact fixture load. Update BOUNDARY.md/README/workspaces docs and run the gates before review.

## Review focus

- Display filters cannot modify analytics; filter tests compare unchanged kernel aggregates.
- Source/history pages arrive after retarget; generation tests discard every old-symbol response.
- Two tape instances share one feed; shell test counts subscriptions and history requests.
- Same-millisecond prints and replay duplicates use different deduplication levels; seam fixtures prove both cases.
- Missing/limited history is visible despite plausible CVD numbers; coverage/gallery tests inspect the degraded labels.

## Validation and handoff

Run focused trade-feed, volume-study, migration and shell tests, then `make contracts-check`, `env -u WAYLAND_DISPLAY -u DISPLAY make check` and `make bench-frames`. Inspect `env -u WAYLAND_DISPLAY -u DISPLAY make gallery-shot` images at 1920×1080 and 2560×1440. A read-only WIN/WDO session walkthrough compares representative tape prints and totals; no order submission is required.

Record acceptance results, exact dependency pins, any manual evidence and open follow-ups.
Commit the final changes, then run `./work board set Q-082 in-review -m "<changes; checks and results; follow-ups>"`
from the workspace root. The human owns integration and any required release.
