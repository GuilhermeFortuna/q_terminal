# Q-048 implementation plan: Execution controls

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Specification:** [`../specs/Q-048-execution-controls-spec.md`](../specs/Q-048-execution-controls-spec.md)  
**Depends on:** Q-044, Q-047

## Current-system context

After Q-047 the terminal has `ExecutionModels` and `OpsStatus` (cxx-qt), a
`HealthPoller`, the QML workspace (`OpsWorkspace`, `OpsHeader`,
`DeploymentList`, `DeploymentDetail`, the tables), and `docs/ops-parity.md`.
HTTP goes through `reqwest`, as in `src/history/api_client.rs` and the stream
client. Tests use `src/stream/fake_server.rs` and the headless QML harness in
`tests/ops_views.rs`.

The backend commands (all in `q_backend/src/q_backend/api/routers/execution.py`):

| Command | Route | Body |
| --- | --- | --- |
| create account | `POST /api/v1/execution/accounts` | `PaperAccountCreateRequest` |
| create deployment | `POST /api/v1/execution/deployments` | `{paper_account_id, name, broker_mode: paper\|mt5_live, source_backtest_run_id}` |
| lifecycle, flatten | `POST /api/v1/execution/deployments/{id}/actions` | `{action: start\|pause\|stop\|flatten, confirm, actor}` |
| resolve | `POST /api/v1/execution/orders/{id}/resolve` | `{outcome: filled\|not_filled, actor, reason, price?, quantity?, fee?, filled_at?, external_fill_id?}` |
| kill switch | `PUT /api/v1/execution/kill-switch` | `KillSwitchUpdateRequest` |

After Q-044 each route takes `Idempotency-Key`, answers replays with
`Idempotency-Replayed: true`, and uses `idempotency_key_reused` and
`idempotency_in_progress` (`409`, `Retry-After`). Saved runs for the picker
come from `GET /api/v1/backtests?saved_only=true`. The frontend uses the same
call in `ExecutionWorkspace.tsx`. The actor is the operator: today the
frontend sends a fixed label.

## Interfaces produced

```rust
// src/execution/commands.rs   (new)
pub struct CommandClient { /* reqwest, api_base */ }
pub enum Command { CreateAccount{..}, CreateDeployment{..}, Lifecycle{id, action}, Flatten{id},
                   KillSwitch{enabled, reason}, Resolve{order_id, outcome, fill: Option<ManualFill>, reason} }
pub struct CommandOutcome { pub key: Uuid, pub status: u16, pub body: serde_json::Value, pub replayed: bool }
impl CommandClient {
    pub async fn send(&self, action_key: Uuid, cmd: &Command) -> Result<CommandOutcome, CommandError>;
    // retries transport errors and 409 idempotency_in_progress with the same key, up to 3 times, capped backoff
}
// src/execution/enablement.rs   (new; pure)
pub struct Health { api: bool, postgres: bool, worker: WorkerStatus, edge_reachable: bool, stream: bool }
pub fn enabled(cmd: CommandKind, health: &Health) -> Enablement;   // Enabled | Disabled(reason)
// src/execution/controls.rs   (new; cxx-qt)
#[qobject] ExecutionControls { #[qinvokable] request(kind, args_json) -> action id; in-flight/pending/refused state per action }
```

```
qml/ConfirmDialog.qml        new: names the action, the deployment, and LIVE when broker_mode is mt5_live
qml/DeployDialog.qml         new: account, name, saved-run picker, broker mode
qml/ResolveDialog.qml        new: outcome with no default; fill fields required for "filled"
qml/AccountDialog.qml        new
qml/DeploymentDetail.qml     action bar added; qml/OpsHeader.qml kill switch control added
tests/ops_controls.rs        new: criteria 1–3, 5–7
tests/enablement_matrix.rs   new: criterion 4 (table-driven from the spec's §8.1 list)
BOUNDARY.md                  commands listed
README.md                    controls, confirmations, operator name setting
src/config.rs                operator (actor) name: config key `operator`, env Q_TERMINAL_OPERATOR, default $USER
```

## Implementation decisions

- **Key per operator action, minted in the controls object.** `request()` mints a
  UUID, and every send and retry for that action uses it. A second click while
  the action is in flight is ignored, not re-sent, which gives criterion 1's
  one-request-per-action. The client retries only transport errors and
  `idempotency_in_progress`, always with the same key. That is exactly the case
  Q-044's replay exists for.

- **Settle on the stream, with a bound.** A successful response moves the action
  to "awaiting confirmation". It settles when the store's revision brings the
  expected entity state: the deployment's lifecycle, the order's status, the
  control state. After 5 s without it, the control shows "applied; waiting for
  the stream" and keeps waiting. The response body is never written into the
  store. That preserves "the stream is the only source of state", and makes a
  stalled relay visible.

- **Enablement is a pure function with a table test.** `enablement.rs` encodes
  §8.1 row by row. `tests/enablement_matrix.rs` is a literal table copied from
  the spec, so that a change to either shows up as a diff. QML binds
  `enabled` and the tooltip reason to it and decides nothing itself.

- **"Postgres unavailable" keeps flatten and the kill switch enabled when the
  edge is reachable,** as §8.1 says. The backend decides whether it can act on
  them. Flatten is a pending action the worker picks up, and the kill switch is a
  Postgres flag, so with Postgres down both are refused by the API with `503`.
  The terminal shows that refusal verbatim. The terminal must not pre-empt the
  backend's answer on the controls whose availability matters most.

- **Confirmations are dialogs with the consequence in words,** for example "Flatten
  WIN$N M1 · MACrossover — sends a market order through the edge to close
  2 contracts". The confirm button is not the default button. `mt5_live` adds a
  distinct banner.

- **The resolve form has no default outcome,** and "filled" requires price,
  quantity and time, as the backend requires. The actor is the configured
  operator name, and the reason is required. This mirrors the backend's
  "no assume-it's-fine default" rule in `reconciliation.py`.

- **The saved-run picker shows three fields per run** (strategy, symbol,
  timeframe) and the run's saved date. No metrics, charts or results are shown,
  which keeps it inside `BOUNDARY.md`'s "no result browser".

## Ordered implementation

- [x] 1. Work on the branch `Q-048-execution-controls` in `q_terminal`, created from
   `development` by `./work start`. Confirm Q-044 and Q-047 are merged, and that
   the pinned contracts include the Q-044 recapture. Move `CONTRACTS_REV` if
   needed. Commit.
- [x] 2. Write `enablement.rs` and `tests/enablement_matrix.rs` from the spec's
   table. Commit.
- [x] 3. Extend `fake_server.rs` with the five command routes, the idempotency
   semantics, a request log, and scripted refusals and delays. Write
   `CommandClient` with tests for keys, retries and error surfacing. Commit.
- [x] 4. Write `ExecutionControls` with the action lifecycle (in-flight, awaiting
   stream, settled, refused, pending past the bound), and headless tests for
   criterion 2. Commit.
- [x] 5. Write the dialogs and wire the action bar and the kill switch. Write
   `tests/ops_controls.rs` for criteria 1, 3, 5, 6 and 7. Keep `qmllint`
   clean. Commit per dialog.
- [x] 6. Add the operator setting to `config.rs`, with tests. Commit.
- [x] 7. Update `BOUNDARY.md`, `README.md` and the command rows of
   `docs/ops-parity.md`. Commit.
- [x] 8. Run `env -u WAYLAND_DISPLAY -u DISPLAY make check`. Fix, re-run, commit.
- [ ] 9. **Human:** human-verifiable criteria 1–3.

## Validation

- **Unit:** enablement matrix; key reuse on retry; action lifecycle.
- **Integration (headless):** each command against the fake server; refusals;
  cancelled confirmations; resolve form rules; picker.
- **Regression:** Q-046 and Q-047 tests; `qmllint`; `make contracts-check`.
- **Manual:** full lifecycle on paper through the demo edge; kill switch;
  degraded enablement.

```bash
cd /home/gui/projects/q/q_terminal
env -u WAYLAND_DISPLAY -u DISPLAY make check
RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo test --test ops_controls --test enablement_matrix

# human (step 9)
cd /home/gui/projects/q && ./research
systemctl --user start mt5-terminal mt5-gateway mt5-edge q-execution-worker
cd /home/gui/projects/q/q_terminal && make run
```

## Handoff

Give the enablement matrix as tested. Report each command's request log from
the tests, one request per action with the key. Report the settle bound and
where it shows. From the human steps, give the audit-log entries for the
lifecycle sequence, the kill switch's risk event, and the degraded enablement
observed.
