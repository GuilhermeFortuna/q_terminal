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

These surfaces are **read-only** in Q-047; command controls arrive in Q-048.

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
