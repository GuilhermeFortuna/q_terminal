# Ownership Boundary and Architectural Constraints

This document defines the strict ownership scope and architectural boundaries for `q_terminal`. Any proposed change that breaches these boundaries must be rejected.

---

## 1. Scope: Live Trading and Operations Only

`q_terminal` is dedicated strictly to the high-performance live trading and operations surface:
- Real-time order book, market depth, and trade tape visualization.
- Real-time order execution, order management, and position monitoring.
- Real-time account risk, margins, and fill state display.
- Operational telemetry and real-time trading session status.

### Operations surfaces (Q-047)

The terminal's read-only operations workspace (`qml/OpsWorkspace.qml` and its components) displays:
- **Deployments** — lifecycle, broker mode, positions, pending actions, and unknown-order counts (`DeploymentList.qml`).
- **Execution detail tables** — orders, fills, decisions, and risk events for the selected deployment (`DeploymentDetail.qml`, `*Table.qml`).
- **Accounts and ledger** — paper-account balances and ledger entries with paged "load older" (`DeploymentDetail.qml`, `LedgerTable.qml`).
- **Health and marks** — stream/API/worker/edge/kill-switch/live-lock status plus backend-computed unrealized P&L and mark prices, polled every two seconds (`OpsHeader.qml`, `OpsStatus`, `HealthPoller`).

### Execution commands (Q-048)

The operations workspace now issues REST commands (each with an `Idempotency-Key`)
and shows effects only when the execution stream delivers them:

- **Create paper account** — `POST /api/v1/execution/accounts`
- **Create deployment** — `POST /api/v1/execution/deployments` (saved-run picker only; no result browser)
- **Lifecycle** — `POST /api/v1/execution/deployments/{id}/actions` (`start`, `pause`, `stop`)
- **Flatten** — same actions route with `action: flatten` (confirmed)
- **Kill switch** — `PUT /api/v1/execution/kill-switch` (engage and release, both confirmed)
- **Resolve unknown order** — `POST /api/v1/execution/orders/{id}/resolve` (no default outcome; filled requires price, quantity, and time)

Enablement follows architecture §8.1 via `ExecutionControls` + `enablement.rs`.
Confirmations name the consequence; `mt5_live` deployments show a distinct live banner.
The configured operator name (`operator` / `Q_TERMINAL_OPERATOR`) is sent as the actor on lifecycle and resolve commands.

---

## 2. Surfaces This Repository Will Never Grow

`q_terminal` is explicitly **not** a general-purpose research or backtesting UI. To prevent scope creep and architecture degradation, `q_terminal` will **never** grow:
- **No Strategy Editor:** Strategy development, code editing, and script compilation belong in research environments.
- **No Parameter Optimizer:** Grid search, genetic optimization, and curve fitting belong in batch compute services.
- **No Backtest / Result Browser:** Historic backtest visualization, walk-forward result browsers, and tear sheets belong in the research surface (`q_frontend`).
- **No Dataset Catalog Explorer:** Data ingestion workflows and dataset inspection belong in dedicated tooling.

---

## 3. No Backend Process Ownership Rule

The application **owns no backend process**:
- **Launches nothing:** `q_terminal` never launches external backend processes, Docker containers, `docker compose`, or local Python daemons.
- **Supervises nothing:** `q_terminal` does not monitor, restart, heartbeat-check, or manage lifecycles of any backend worker.
- **Stops nothing:** `q_terminal` does not stop or kill backend daemon processes.

`q_terminal` connects to already-running backend services and relays strictly over declared wire contracts and protocols. If a backend service is unavailable, `q_terminal` reports the disconnection; it never attempts to spawn or bootstrap the missing service.

---

## 4. Presentation ownership and enforcement

The terminal design system owns presentation decisions only. `qml/theme/` is the source of
truth for palette, typography, spacing, icons, and semantic role resolution; screens and
components consume those tokens and do not infer trading meaning from raw values. Reusable
controls live in `qml/components/`, while `qml/style/` supplies the Qt Quick Controls skin.

The optional `gallery` Cargo feature builds the component/state catalog and `make gallery-shot`
renders it headlessly for review. The normal operations binary does not include the gallery.
`make check` enforces this boundary with `tools/token_gate.py` and full-module `qmllint`; the
gate rejects screen-level colour literals, raw pixel font sizes, unscaled dimensions, and
conditional colour decisions. These checks must not become a place to add backend state,
protocol transformations, or money arithmetic.
