# Q-084 implementation plan: Live context, evidence and confirmed changes

> **For implementation agents:** Read the linked spec and repository instructions.
> Use superpowers:executing-plans when that skill is available. Start only through
> `./work start Q-084 --agent <agent> --worktree` after written-plan approval and
> completed dependencies. Implement this task natively; delegation requires separate authorization.

**Goal:** Explain the active chart’s current market context with editable settings and confirmed changes.
**Architecture:** Rust ContextController owns ContextState, per-bar outputs, analysis revision and confirmed events; QML owns section/layout/editing.
**Spec:** [Specification](../specs/Q-084-live-context-evidence-and-confirmed-changes-spec.md)
**Status:** written plan awaiting human review.

## Global constraints

- The linked spec defines the interface, defaults and acceptance criteria; do not widen scope.
- Use the existing repository toolchain and canonical checks without resource-slice wrappers.
- Commit focused changes on the task branch. Never push, merge or change protected branches.
- Report unavailable prerequisites with the documented board workflow; do not substitute shortcuts.

## Ordered implementation

- [ ] 1. Pin a published release containing Q-081 and Q-083 and add tests/context_feed.rs using existing fake history/live data. Compare controller outputs with Q-083 for completed/forming bars and inspect-at-bar requests.
- [ ] 2. Implement src/context state/controller/event modules; connect accepted feed generations and analysis revisions. Add tests/context_events.rs for order, duplicates, 100-event bound, replay suppression, stale continuity and rebuild seeding.
- [ ] 3. Correct src/studies.rs session_day_key using timezone-aware label conversion shared by context. Test UTC midnight versus B3 local midnight, historical daylight-saving offsets and local-labelled input without double conversion.
- [ ] 4. Adapt MarketContextSection and settings popover from the named reference patterns. Use semantic roles/types, keyboard/focus and accessible evidence; inspect gallery captures while implementing.
- [ ] 5. Add v5 analysis settings/collapsed state, validate Apply atomically and use Q-077 autosave. Test old-version migrations, invalid persisted profile and close/reopen with edited independent settings.
- [ ] 6. Update reference/component records, BOUNDARY.md, README and workspaces documentation. Run focused context/migration tests, offscreen make check and the combined frame benchmark; record evidence and commit.

## Review focus

- Removing a visible EMA does not change the independent profile; controller test compares outputs.
- Replayed category changes are not new events; fake gap/correction tests assert an empty seeded list.
- A setting edit changes revision and invalidates old evidence; Apply/Cancel tests assert atomicity.
- UTC day and B3 exchange-local day differ around midnight; shared session-key fixtures prove conversion.
- Stale/disconnected context remains visibly stale and creates no events; gallery and fake-stream tests cover it.

## Validation and handoff

Run focused context-feed/events, session-key and workspace migration suites, then `env -u WAYLAND_DISPLAY -u DISPLAY make check` and `make bench-frames`. Inspect gallery captures at 1920×1080 and 2560×1440. A read-only B3 walkthrough demonstrates provisional-to-confirmed context and restart restoration; no execution action is required.

Record acceptance results, exact dependency pins, any manual evidence and open follow-ups.
Commit the final changes, then run `./work board set Q-084 in-review -m "<changes; checks and results; follow-ups>"`
from the workspace root. The human owns integration and any required release.
