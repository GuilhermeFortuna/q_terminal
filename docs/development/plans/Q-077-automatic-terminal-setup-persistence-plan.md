# Q-077 implementation plan: Automatic terminal setup persistence

> **For implementation agents:** Read the linked spec and repository instructions.
> Use superpowers:executing-plans when that skill is available. Start only through
> `./work start Q-077 --agent <agent> --worktree` after written-plan approval and
> completed dependencies. Implement this task natively; delegation requires separate authorization.

**Goal:** Restore the operator’s last terminal setup automatically on every launch.
**Architecture:** Rust workspace and chart-target state remain authoritative; QML signals schedule saves and render errors.
**Spec:** [Specification](../specs/Q-077-automatic-terminal-setup-persistence-spec.md)
**Status:** written plan awaiting human review.

## Global constraints

- The linked spec defines the interface, defaults and acceptance criteria; do not widen scope.
- Use the existing repository toolchain and canonical checks without resource-slice wrappers.
- Commit focused changes on the task branch. Never push, merge or change protected branches.
- Report unavailable prerequisites with the documented board workflow; do not substitute shortcuts.

## Ordered implementation

- [x] 1. Add tests in tests/workspace_autosave.rs for debounce, final-window capture, shutdown flush and write failures; use a temporary config directory and injected clock/writer, not real sleeps or user files.
- [x] 2. Extend src/workspace/schema.rs, store.rs and resolve.rs with WorkspaceChartPreferences and v1/v2 migrations. Implement atomic workspace/state writes and retain dirty revisions on failure. Add serialization, corrupt/future-file and failure-injection assertions.
- [ ] 3. Integrate src/workspace/autosave.rs and WorkspaceController with accepted study/layout/target revisions. Connect Main.qml and shell closing signals; prevent restore and teardown from scheduling destructive saves.
- [ ] 4. Resolve saved target plus explicit config precedence before startup connects BarFeed/ChartTarget. Test session-only overrides, unavailable symbol lookup, delayed followed-deployment resolution and fallback without starting a deployment.
- [ ] 5. Persist visible-bar count and restore live-follow. Exercise the complete close/reopen workflow through the existing offscreen harness, including switching workspaces and one-window versus final-window closure.
- [ ] 6. Update docs/workspaces.md and README.md. Run the validation gate, commit focused changes and hand off the restoration/write-failure evidence.

## Review focus

- Final-window capture runs before shell structure is destroyed; tests assert nonempty saved layout.
- Restore does not trigger autosave of incomplete default state; startup tests inspect write counts.
- Future-version and read-only files are preserved; injected writer tests assert unchanged bytes.
- Deployment-follow restore waits for confirmed state and issues no execution action; fake-server tests record requests.
- Explicit startup overrides do not silently overwrite the last manual target; reopen tests cover both sessions.

## Validation and handoff

Run focused workspace/target tests, then `env -u WAYLAND_DISPLAY -u DISPLAY make check`. Temporary-directory reopen tests provide the persistence evidence; no GPU, Wine or live account is required.

Record acceptance results, exact dependency pins, any manual evidence and open follow-ups.
Commit the final changes, then run `./work board set Q-077 in-review -m "<changes; checks and results; follow-ups>"`
from the workspace root. The human owns integration and any required release.
