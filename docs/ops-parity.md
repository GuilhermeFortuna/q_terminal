# Operations Parity: Frontend (`ExecutionWorkspace.tsx`) → Terminal (`q_terminal`)

This document establishes the 1:1 mapping between each visual field, signal, panel, and control from `q_frontend/src/workspaces/execution/ExecutionWorkspace.tsx` and its corresponding location in `q_terminal` (or the explicit architectural reason for omission under the Q-047 scope).

## Summary Counts

- **Fields and signals mapped to terminal:** 42
- **Controls deferred or omitted with stated reasons:** 12 (Command mutations deferred to Q-048; web/Tauri multi-window controls omitted by architecture; create dialogs deferred to Q-048)

---

## 1. Environment & Global Signals (Header)

| Frontend Element / Field | Frontend Source / Component | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| Workspace Title ("Execution") | `ExecutionWorkspace` header | `qml/OpsHeader.qml` title | Mapped |
| Paper Environment Badge | `EnvironmentBadges` (`Paper`) | `qml/OpsHeader.qml` / `qml/DeploymentList.qml` | Mapped (`Paper` badge) |
| Live Locked Badge | `EnvironmentBadges` (`Live locked`) | `qml/OpsHeader.qml` `live_locked` badge | Mapped (`OpsStatus.live_locked`) |
| API Status | `HealthSignals` / `StatTile` ("API") | `qml/OpsHeader.qml` API badge | Mapped (`OpsStatus.api_status`) |
| Worker Status | `HealthSignals` / `StatTile` ("Worker") | `qml/OpsHeader.qml` Worker badge | Mapped (`OpsStatus.worker_status`, heartbeat age) |
| Market Data / Edge Status | `HealthSignals` / `StatTile` ("Market data") | `qml/OpsHeader.qml` Edge/Stream status | Mapped (`OpsStatus.edge_*`, `stream_state`) |
| Unknown Orders Count | `HealthSignals` / `StatTile` ("Unknown orders") | `qml/OpsHeader.qml` Unknown orders chip | Mapped (`OpsStatus.unknown_orders`) |
| Worker Lease Offline Banner | `execution-worker-down-banner` | `qml/OpsHeader.qml` Warning banner | Mapped (shown when API ok but worker offline/stale) |
| Reconciliation Required Banner | `execution-unknown-orders-banner` | `qml/OpsHeader.qml` Reconciliation banner | Mapped (shown when `unknown_orders > 0`) |
| Global Kill Switch State | `execution-kill-switch-state` (`Engaged`/`Off`) | `qml/OpsHeader.qml` Kill switch indicator | Mapped (`OpsStatus.kill_switch_enabled`) |
| Global Kill Switch Slide / Release | `PowerOffSlide`, `execution-kill-switch-release` | N/A (read-only in Q-047) | Deferred to Q-048 (Commands) |
| Manual Refresh Button | `Button` (calls `refetch()`) | N/A | Automated via live WebSocket stream & 2s health/positions poller |

---

## 2. Paper Accounts & Balances

| Frontend Element / Field | Frontend Source / Component | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| Account Selector Tabs | `activeAccountId` tab chips | `qml/DeploymentDetail.qml` (Account tab) | Mapped |
| Create Account Button (`+ Account`) | `Button` (`setShowCreateAccount`) | N/A (read-only in Q-047) | Deferred to Q-048 (Commands) |
| Cash Balance | `StatTile` ("Cash balance") | `qml/DeploymentDetail.qml` Account summary | Mapped (`account.cash_balance`, formatted via `Format.js`) |
| Equity (cash) | `StatTile` ("Equity (cash)") | `qml/DeploymentDetail.qml` Account summary | Mapped (`account.cash_balance`, formatted via `Format.js`) |
| Session P&L vs Initial | `StatTile` ("Session P&L vs initial") | `qml/DeploymentDetail.qml` Account summary | Mapped (`cash_balance - initial_balance` display string) |

---

## 3. Deployments List

| Frontend Element / Field | Frontend Source / Component | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| New Deployment Button (`+ New`) | `Button` (`setShowCreateDeployment`) | N/A (read-only in Q-047) | Deferred to Q-048 (Commands) |
| Deployment Name | `item.name` | `qml/DeploymentList.qml` name | Mapped |
| Deployment Lifecycle | `item.lifecycle` (`running`, `paused`, `stopped`) | `qml/DeploymentList.qml` lifecycle badge | Mapped |
| Deployment Status Color Bar | `statusBorder` (green, amber, rose) | `qml/DeploymentList.qml` border indicator | Mapped |
| Symbol & Timeframe | `item.symbol`, `item.timeframe` | `qml/DeploymentList.qml` symbol & timeframe | Mapped |
| Broker Mode | `item.broker_mode` (`paper` vs `live`) | `qml/DeploymentList.qml` broker mode badge | Mapped (visual distinction per spec) |
| Pending Action Indicator | `item.pending_action` | `qml/DeploymentList.qml` pending action badge | Mapped |
| Unknown Orders on Deployment | (Q-047 spec requirement) | `qml/DeploymentList.qml` unknown-order badge | Mapped |
| Net Position & Entry Price | `deployment.open_position` | `qml/DeploymentList.qml` position snippet | Mapped |
| External Monitor Link | `openExecutionMonitor` | N/A | Omitted: `q_terminal` is the dedicated native operations window |

---

## 4. Deployment Detail & Control

| Frontend Element / Field | Frontend Source / Component | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| Current Status (Lifecycle) | `deployment.lifecycle` | `qml/DeploymentDetail.qml` header | Mapped |
| Pending Action (Desired) | `deployment.pending_action` | `qml/DeploymentDetail.qml` header | Mapped |
| Last Bar Close Time | `deployment.last_bar_close_time` | `qml/DeploymentDetail.qml` header | Mapped (formatted local time) |
| Lifecycle Actions (Start, Pause, Stop) | `Button` controls | N/A (read-only in Q-047) | Deferred to Q-048 (Commands) |
| Monitor Live Action | `Button` (`openExecutionMonitor`) | N/A | Omitted: Native terminal view |
| Flatten Action | `Button` (`setConfirmKind('flatten')`) | N/A (read-only in Q-047) | Deferred to Q-048 (Commands) |
| Open Position Side | `deployment.open_position.side` (`long`/`short`/flat) | `qml/DeploymentDetail.qml` Position card | Mapped |
| Open Position Quantity | `deployment.open_position.quantity` | `qml/DeploymentDetail.qml` Position card | Mapped |
| Open Position Avg Entry Price | `deployment.open_position.average_entry_price` | `qml/DeploymentDetail.qml` Position card | Mapped (formatted string) |
| Unrealized P&L | Backend `/positions` poller | `qml/DeploymentDetail.qml` Position card | Mapped (from poller, never computed locally) |
| Mark Price | Backend `/positions` poller | `qml/DeploymentDetail.qml` Position card | Mapped (from poller) |
| Live Candlestick Chart | `ExecutionLiveChartPanel` | `qml/OpsWorkspace.qml` (`ChartPane.qml`) | Mapped (integrated into workspace) |

---

## 5. Detail History Tables

### 5.1 Decisions Table

| Frontend Column / Field | Frontend Type (`Decision`) | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| Time | `created_at` / `bar_close_time` | `qml/DecisionsTable.qml` column 1 | Mapped |
| Action | `signal_action` (`buy`, `sell`, `hold`) | `qml/DecisionsTable.qml` column 2 | Mapped |
| Outcome | `outcome` (`executed`, `submitted`, etc.) | `qml/DecisionsTable.qml` column 3 | Mapped |
| Requested Qty | `requested_quantity` | `qml/DecisionsTable.qml` column 4 | Mapped |
| Reason | `reason` | `qml/DecisionsTable.qml` column 5 | Mapped |

### 5.2 Orders Table

| Frontend Column / Field | Frontend Type (`ExecutionOrder`) | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| Time | `created_at` | `qml/OrdersTable.qml` column 1 | Mapped |
| Order ID | `id` (truncated) | `qml/OrdersTable.qml` column 2 | Mapped |
| Intent Identifier | `intent_id` / client id (spec requirement) | `qml/OrdersTable.qml` column 3 | Mapped |
| Side | `side` (`buy`, `sell`) | `qml/OrdersTable.qml` column 4 | Mapped |
| Type | `order_type` | `qml/OrdersTable.qml` column 5 | Mapped |
| Qty | `quantity` | `qml/OrdersTable.qml` column 6 | Mapped |
| Status | `status` (`pending`, `filled`, etc.) | `qml/OrdersTable.qml` column 7 | Mapped |
| Reconciliation | `reconciliation_state` | `qml/OrdersTable.qml` column 8 | Mapped |

### 5.3 Fills Table

| Frontend Column / Field | Frontend Type (`Fill`) | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| Time | `filled_at` / `created_at` | `qml/FillsTable.qml` column 1 | Mapped |
| Fill ID | `id` (truncated) | `qml/FillsTable.qml` column 2 | Mapped |
| Order ID | `order_id` (truncated) | `qml/FillsTable.qml` column 3 | Mapped |
| Side | `side` (`buy`, `sell`) | `qml/FillsTable.qml` column 4 | Mapped |
| Price | `price` | `qml/FillsTable.qml` column 5 | Mapped (formatted decimal string) |
| Qty | `quantity` | `qml/FillsTable.qml` column 6 | Mapped |
| Fee | `fee` | `qml/FillsTable.qml` column 7 | Mapped (formatted decimal string) |
| Slippage | `slippage` | `qml/FillsTable.qml` column 8 | Mapped (formatted decimal string) |

### 5.4 Ledger Table

| Frontend Column / Field | Frontend Type (`LedgerEntry`) | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| Time | `created_at` | `qml/LedgerTable.qml` column 1 | Mapped |
| Type | `entry_type` | `qml/LedgerTable.qml` column 2 | Mapped |
| Amount | `amount` | `qml/LedgerTable.qml` column 3 | Mapped (formatted decimal string) |
| Balance After | `balance_after` | `qml/LedgerTable.qml` column 4 | Mapped (formatted decimal string) |
| Description | `description` | `qml/LedgerTable.qml` column 5 | Mapped |

### 5.5 Risk Events Table

| Frontend Column / Field | Frontend Type (`RiskEvent`) | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| Time | `created_at` | `qml/RiskTable.qml` column 1 | Mapped |
| Rejection Code | `rejection_code` | `qml/RiskTable.qml` column 2 | Mapped |
| Message | `message` | `qml/RiskTable.qml` column 3 | Mapped |
| Context | `context` (JSON) | `qml/RiskTable.qml` column 4 | Mapped |

### 5.6 Table Paging

| Frontend Control | Frontend Behavior | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| Previous / Next Page | Offset-based page buttons | "Load older" button (`load_older`) | Mapped to append model pattern per spec |

---

## 6. Dialogs & Creation Overlays

| Frontend Dialog | Frontend Purpose | Terminal Status / Rationale |
|---|---|---|
| Create Paper Account Modal | POST `/api/v1/execution/paper-accounts` | Deferred to Q-048 (read-only boundary in Q-047) |
| Create Deployment Modal | POST `/api/v1/execution/deployments` | Deferred to Q-048 (read-only boundary in Q-047) |
| Confirm Flatten Dialog | POST `/api/v1/execution/deployments/{id}/actions` | Deferred to Q-048 (read-only boundary in Q-047) |
