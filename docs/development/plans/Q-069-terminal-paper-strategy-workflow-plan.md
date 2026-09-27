# Q-069 implementation plan: Terminal paper strategy workflow

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Specification:** [`../specs/Q-069-terminal-paper-strategy-workflow-spec.md`](../specs/Q-069-terminal-paper-strategy-workflow-spec.md)  
**Depends on:** Q-068

## Current-system context

`qml/DeployDialog.qml` fetches saved runs and sends only their ID plus name,
account and broker mode. `src/execution/commands.rs` owns idempotent REST
commands; `src/execution_controls.rs` parses QML requests. The shell owns one
execution store and shared selection. `DeploymentDetail.qml` already displays
decisions, orders, fills, risk and ledger. The backend computes all account
and position money values. `BOUNDARY.md` currently forbids a strategy editor;
the new permission is limited to deployment configuration.

## Interfaces consumed

| Backend interface | Terminal use |
| --- | --- |
| `GET /api/v1/execution/strategy-catalog` | Typed eligible strategy choices and defaults |
| `GET /api/v1/market/symbols/search` | Exact-symbol discovery |
| `POST /api/v1/execution/deployments` with `catalog` | Create paper draft |
| `GET /api/v1/execution/deployments/{id}` | Full current revision for editing |
| `PATCH /api/v1/execution/deployments/{id}/configuration` | Paused-flat edit with `expected_revision` |
| `GET /api/v1/execution/deployments/{id}/performance[/marks]` | Summary and paged history |
| Existing execution stream | Authoritative deployment, decision, order and fill state |

Extend `Command::CreateDeployment` for the catalog body and add
`Command::EditDeployment { deployment_id, expected_revision, actor,
configuration }`. Both use the existing `CommandClient::send` key/retry
behavior. Add typed Rust decoding for catalog and performance responses;
QML receives view-facing fields and never performs financial calculations.

## Implementation decisions

- Keep one create dialog with catalog as the first source and saved run as a
  secondary source. Build fields from server parameter specs; client checks
  obvious type/range errors, while the backend remains authoritative.
- Offer only `paper` in create. Keep the existing live-row visual warning but
  disable live start in the dev profile. No hidden live activation switch.
- Populate edit from a fresh deployment detail response, then send a complete
  replacement and revision. On HTTP success, wait for the stream to settle
  just as existing execution commands do.
- Load performance only for the selected deployment, at selection and
  completed-bar updates. Use a request generation token to discard late
  responses after selection changes. If a completed-bar event precedes its
  performance mark, retry that bar with a bounded request until the matching
  mark or an unavailable result is returned; do not add another periodic poller.
- Group config controls using MuseScore's inspector pattern; use Qt Quick
  Controls and theme tokens. Pair the dense audit table with detail as in
  Wireshark, and record reference adaptations in `docs/design/components.md`.

## Ordered implementation

- [x] 1. On the Q-069 task branch, pin Q-066 contracts and run
  `make contracts-check`. Update `BOUNDARY.md` and README with the narrow
  deployment-configuration permission and paper-only dev behavior.
- [ ] 2. Add fake-server tests for catalog fetch, symbol search, typed form
  values and paper-only create payload. Extend Rust commands/controls and
  replace the saved-run-only QML create flow. Verify idempotent retry and
  stream-only settlement.
- [ ] 3. Add fake-server tests for full-replacement PATCH, revision conflict,
  disabled lifecycle/position states and late stream events. Implement the
  paused edit UI and command path.
- [ ] 4. Add performance response models and selection-scoped fetches. Extend
  audit details with dispatch/quote/fee fields and build the compact summary
  and equity-delta history. Test stale marks, no closed trades, unknown orders
  and late responses after switching deployments.
- [ ] 5. Record MuseScore/Qt Quick Controls/Wireshark adaptations in
  `docs/design/components.md`. Render gallery shots per component with
  `env -u WAYLAND_DISPLAY -u DISPLAY make gallery-shot`, inspect the images,
  and fix visual/focus issues before moving on.
- [ ] 6. Run `env -u WAYLAND_DISPLAY -u DISPLAY make check` and the frame
  benchmark. During an open market, run the Q-068 paper walkthrough through
  `./dev up full`; record evidence without sending broker orders. Commit
  focused changes on the task branch.

## Review focus

- The terminal must never compute or accept a client-supplied config hash.
- HTTP success alone must not optimistically rewrite stream-owned state.
- The edit form must not allow symbol/timeframe or strategy switching.
- A late performance response must not appear under another selection.
- The UI must not imply a paper fill is an actual MT5 trade.
