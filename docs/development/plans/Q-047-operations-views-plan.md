# Q-047 implementation plan: Operations views

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Specification:** [`../specs/Q-047-operations-views-spec.md`](../specs/Q-047-operations-views-spec.md)  
**Depends on:** Q-046, Q-045

## Current-system context

`qml/Main.qml` composes a header, `ChartPane.qml` and `StatusStrip.qml` for one
configured symbol. `src/bar_feed.rs` is the cxx-qt pattern: a `#[cxx_qt::bridge]`
`#[qobject]` with `#[qproperty]` fields (`connection_state`, `last_error`,
`data_age_ms`, the counters), fed from the stream thread through `BarSink`
and a listener that queues a Qt-thread update with `Threading`.
`src/startup.rs::run_slice` opens the window in degraded states when the
configuration or the API is missing. `tests/degraded_states.rs` and
`tests/slice_end_to_end.rs` drive the app headlessly, and `make bench-frames`
runs `--bench-frames` with synthetic bars. `BOUNDARY.md` lists the surfaces
the terminal owns and the ones it never grows. Build: `build.rs` registers the
QML module and the C++ bridges. `qmllint -W 0` runs on `qml/Main.qml`.

After Q-046, `src/execution/store.rs::ExecutionStore` holds the state behind
`ExecutionHandle`, with a `revision()` counter and `view()` accessors. After
Q-045, `GET /api/v1/execution/health` returns `worker_status`,
`worker_heartbeat_age_s`, `edge {reachable, mt5_connected, terminal_build,
checked_at}`, `kill_switch_enabled`, `live_capability_locked` and
`unknown_order_count`. `GET /api/v1/execution/positions` returns
`unrealized_pnl` and `mark_price` per position. The paged routes
`/api/v1/execution/deployments/{id}/{orders,fills,decisions}`,
`/api/v1/execution/accounts/{id}/ledger` and the risk-event list page older rows.

The frontend surface to reach parity with is
`q_frontend/src/workspaces/execution/ExecutionWorkspace.tsx` (1 306 lines) with
`src/types/execution.ts` and `src/api/queries/execution.ts`.

## Interfaces produced

```rust
// src/execution/models.rs   (new; cxx-qt bridge)
#[qobject] ExecutionModels {
    #[qproperty(i64, revision)]                 // bumped at most once per frame from the store's revision
    #[qproperty(QString, selected_deployment_id)]
    // list models (QAbstractListModel subclasses via cxx-qt inheritance):
    deployments, orders, fills, decisions, risk_events, ledger, accounts
    #[qinvokable] fn load_older(&self, table: QString)   // one paged REST request, appended
}
// src/execution/health.rs   (new)
pub struct HealthPoller;                          // GET /api/v1/execution/health + /positions every 2 s
#[qobject] OpsStatus { stream, api, worker_status, worker_heartbeat_age_s, edge_reachable,
                       edge_mt5_connected, terminal_build, kill_switch_enabled, live_locked, unknown_orders,
                       postgres_available }
```

```
qml/OpsWorkspace.qml        new: header + deployments list + chart + detail area (replaces Main.qml's body)
qml/OpsHeader.qml           new: stream / API / worker / edge / kill switch / live lock
qml/DeploymentList.qml      new
qml/DeploymentDetail.qml    new: tab bar over OrdersTable, FillsTable, DecisionsTable, RiskTable, LedgerTable
qml/*Table.qml              new: ListView-based, virtualized
qml/Format.js               new: decimal-string and time formatting only
tests/ops_views.rs          new: criteria 1–5 headless
docs/ops-parity.md          new: frontend field → terminal place (criterion 6)
BOUNDARY.md                 operations surfaces listed
src/main.rs                 --bench-frames gains --execution-rows N (synthetic store)
```

## Implementation decisions

- **One `ExecutionModels` object, with a frame-coalesced revision.** The store
  listener sets a dirty flag. A `QTimer`-free approach ties the refresh to
  `QQuickWindow::beforeSynchronizing`, through the existing C++ helper in
  `cpp/chart_cxx.cpp`, so that models reset at most once per frame. That is the
  same principle as the bar path's one buffer upload per frame (§4.6).

- **List models reset per revision, not per row.** At the snapshot limits
  (50 recent rows per table per deployment, a handful of deployments),
  a model reset is cheap and keeps the Rust side free of row-diff bookkeeping.
  `load_older` rows live in a per-table "older" segment that a reset keeps.
  If the frame benchmark shows reset cost in the budget, that is the measurement
  that would justify diffing. It goes in the handoff, not in speculative code.

- **Health and unrealized P&L are the two polled reads,** on one 2 s timer. Health
  is not a topic (§8.1 describes it as health). Unrealized P&L changes with
  every quote and is computed by the backend's ledger, and recomputing it in QML
  or Rust would be a second implementation of ledger semantics (invariant 1).
  Both requests go through the same poller, so that criterion 5 counts one
  periodic source.

- **Postgres down is inferred from the health response** (`api_status` and
  `503` on persistence routes), and marks every deployment "unknown", as §8.1
  prescribes, without clearing the store.

- **Formatting only, in `Format.js`.** Decimal strings are grouped and padded
  for display, and times are shown in the operator's local zone. Nothing is
  summed, netted or converted. A lint-style test greps `qml/` for arithmetic
  on fields named like money or quantity.

- **Parity is a checked document,** `docs/ops-parity.md`. It is a table of every
  field and panel of the frontend workspace, with its terminal location or a
  reason. Q-050 uses it as the removal gate.

- **`BOUNDARY.md` gains the operations surfaces** (deployments, orders, fills,
  decisions, risk, ledger, health), and keeps its "never grows" list
  unchanged.

## Ordered implementation

- [x] 1. Work on the branch `Q-047-operations-views` in `q_terminal`, created from
   `development` by `./work start`. Confirm Q-046 and Q-045 are merged, and that
   the pinned contracts have the Q-045 health fields. Move `CONTRACTS_REV` if
   needed. Commit.
- [x] 2. Write `docs/ops-parity.md` from `ExecutionWorkspace.tsx` before any UI, so
   that the views are built against the list. Commit.
- [x] 3. Implement `ExecutionModels` with the per-frame revision, and a headless
   test for criterion 3. Commit.
- [x] 4. Implement `HealthPoller` and `OpsStatus`, with fake-server tests for the
   2 s cadence and each health state. Commit.
- [x] 5. Write the QML: `OpsWorkspace`, `OpsHeader`, `DeploymentList`,
   `DeploymentDetail`, the tables and `Format.js`. Move the chart into the
   workspace. Keep `qmllint` clean. Commit per component.
- [ ] 6. Write `tests/ops_views.rs` for criteria 1, 2, 4 and 5, extending the
   degraded-state harness from `tests/degraded_states.rs`. Confirm they pass.
   Commit.
- [ ] 7. Add `--execution-rows` to `--bench-frames`, and record p95 frame time with
   10 000 rows and a live chart. Commit.
- [ ] 8. Tick every row of `docs/ops-parity.md`. Update `BOUNDARY.md` and
   `README.md` (what the workspace shows; health cadence). Commit.
- [ ] 9. Run `env -u WAYLAND_DISPLAY -u DISPLAY make check`. Fix, re-run, commit.
- [ ] 10. **Human:** human-verifiable criteria 1 and 2.

## Validation

- **Unit:** formatting; per-frame coalescing; health state mapping.
- **Integration (headless):** tables from a fake-server store; selection and
  paging; each §8.1 state; request cadence.
- **Performance:** `bench-frames` with a large store and a live chart.
- **Regression:** every Q-035 to Q-038 and Q-046 test; `qmllint`;
  `make contracts-check`.
- **Manual:** thirty-minute live comparison; the §8.1 service-stop sequence;
  two screenshots.

```bash
cd /home/gui/projects/q/q_terminal
env -u WAYLAND_DISPLAY -u DISPLAY make check
RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo test --test ops_views
BENCH_EXECUTION_ROWS=10000 make bench-frames

# human (step 10)
cd /home/gui/projects/q && ./research
systemctl --user start q-execution-worker
cd /home/gui/projects/q/q_terminal && make run
```

## Handoff

Give the parity document's counts: fields mapped, and fields not carried with
their reasons. Give the frame benchmark's p50, p95 and p99 with 10 000 rows. List
each §8.1 state with its test. From the human steps, give the thirty-minute
comparison result, the p95 frame time, what each service stop showed, and the
two screenshots.
