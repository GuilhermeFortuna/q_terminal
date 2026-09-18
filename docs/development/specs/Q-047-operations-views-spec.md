# Q-047: Operations views

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Project direction:** [`q_contracts/docs/system-architecture.md` §4.6, §5.1, §8.1, §9 invariants 1 and 9, §10 Phase 4](https://github.com/GuilhermeFortuna/q_contracts/blob/f2273a88e52c5b9a8ad5ac9d7eb27f8069cf643d/docs/system-architecture.md#51-ui-ownership-boundary)  
**Depends on:** Q-046, Q-045  
**Implementation plan:** [`../plans/Q-047-operations-views-plan.md`](../plans/Q-047-operations-views-plan.md)

## Purpose

After Q-046 the terminal holds live execution state, but shows only a chart.
The frontend's execution workspace is still where an operator sees what is
running. Phase 4 moves that surface to the terminal. This task draws the
execution store as the terminal's operations workspace: deployments with their
lifecycle and positions, each deployment's orders, fills, decisions and risk
events, the accounts and their ledger, and the health of every process a trade
depends on. It includes every §8.1 degraded state. It stays read-only. Commands
arrive in Q-048, on top of exactly these views.

## Requirements

### Workspace

- The window becomes an operations workspace: a status header, a deployments
  list, the existing chart, and a detail area for the selected deployment. The
  chart keeps its current symbol until Q-049.
- Deployments show name, symbol and timeframe, broker mode (paper or live, with
  live visually unmistakable), lifecycle, any pending action, last evaluated
  bar, net position with entry price, and unknown-order count.
- The detail area shows the selected deployment's orders (status,
  reconciliation state, intent identifier), fills, decisions (outcome and
  reason) and risk events, newest first. The account shows balances and its
  ledger entries. Older rows load on request from the backend's paged routes.
- The status header shows the stream connection, the API, the worker
  (healthy, stale or offline, with heartbeat age), the edge (reachable,
  terminal connected, terminal build), the kill switch, and whether live
  trading is locked.
- Unrealized profit and loss is shown as the backend reports it. The terminal
  never computes it.
- Every figure the frontend's execution workspace shows today is shown here, or
  is listed in the handoff with the reason it is not.

### Behaviour

- Views redraw at most once per frame, driven by the store's revision, however
  many events arrive.
- Health, and the backend's position marks for unrealized profit and loss, are
  read every two seconds by one poller. They are the only periodic requests the
  workspace makes: health is not a stream topic, and marks change with every
  quote and are computed by the backend's ledger.
- Every §8.1 row for `q_terminal` has a visible state: stream disconnected with
  last state kept and its age; Postgres down with every deployment marked
  unknown; API offline; worker down; edge or terminal down with quotes marked
  stale and their age. No state shows data older than the age it displays.
- Tables with many rows scroll smoothly: the workspace meets the §4.6 frame
  budget with the chart live and a large store.

### Boundary

- The workspace contains no strategy editor, optimizer or result browser, and
  no control that changes state. `BOUNDARY.md` names the operations surfaces
  that now exist.

## Constraints and non-goals

- **No commands, confirmations or command feedback.** Q-048.
- **No per-deployment chart or markers.** Q-049.
- **No arithmetic on money or quantities in the terminal.** Values are formatted,
  never derived.
- **No `q_core` change.** Tables are row views over the store, not columnar
  market data.

## Acceptance criteria

### Agent-verifiable

1. Headless QML tests against a store filled from the fake server show each
   deployment's fields, each detail table's rows in order, and the account and
   ledger, with the decimal strings formatted without loss.
2. Selecting a deployment shows only its rows. "Load older" issues one paged
   REST request and appends its rows below.
3. 1 000 events applied within one frame cause one redraw, measured by the
   view's redraw counter.
4. Each §8.1 degraded state is produced by the fake server or a fake health
   response, and asserted headlessly: the state label, the kept data, and its
   age.
5. The poller issues one health request and one positions request every two
   seconds, and they are the only periodic requests, counted by the fake
   server over sixty seconds.
6. A parity checklist in the repository maps every field of the frontend
   execution workspace to its place in the terminal, or to a stated reason.
7. `qmllint` passes with no warnings, and `BOUNDARY.md` lists the operations
   surfaces.
8. `make check` passes.

### Human-verifiable

1. With `./research`, the worker and two paper deployments running for thirty
   minutes, the workspace matches the backend's snapshot at every check. p95
   frame time with the chart live and the deployment tables open stays under
   16 ms.
   Command: `make run`, then `BENCH_EXECUTION_ROWS=10000 make bench-frames`
2. Stopping Redis, then Postgres, then the API, then the worker, then
   `mt5-edge` in turn, and starting each again, shows each §8.1 state and a
   full recovery. Two screenshots are recorded: normal operation and worker
   down.
   Command: `systemctl --user stop q-redis` (and the others in turn)
