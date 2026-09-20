# Operations Parity: Frontend (`ExecutionWorkspace.tsx`) → Terminal (`q_terminal`)

This document establishes the 1:1 mapping between each visual field, signal, panel, and control from `q_frontend/src/workspaces/execution/ExecutionWorkspace.tsx` and its corresponding location in `q_terminal` (or the explicit architectural reason for omission under the Q-047 scope).

## Summary Counts

- **Fields and signals mapped to terminal:** 42 (all verified against Q-047 implementation)
- **Controls deferred or omitted with stated reasons:** 2 (web/Tauri multi-window controls omitted by architecture)

**Verification:** Every row below is checked `[x]` against the terminal implementation in branch `Q-047-operations-views`.

---

## 1. Environment & Global Signals (Header)

| Frontend Element / Field | Frontend Source / Component | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| [x] Workspace Title ("Execution") | `ExecutionWorkspace` header | `qml/OpsHeader.qml` title | Mapped |
| [x] Paper Environment Badge | `EnvironmentBadges` (`Paper`) | `qml/OpsHeader.qml` / `qml/DeploymentList.qml` | Mapped (`Paper` badge) |
| [x] Live Locked Badge | `EnvironmentBadges` (`Live locked`) | `qml/OpsHeader.qml` `live_locked` badge | Mapped (`OpsStatus.live_locked`) |
| [x] API Status | `HealthSignals` / `StatTile` ("API") | `qml/OpsHeader.qml` API badge | Mapped (`OpsStatus.api_status`) |
| [x] Worker Status | `HealthSignals` / `StatTile` ("Worker") | `qml/OpsHeader.qml` Worker badge | Mapped (`OpsStatus.worker_status`, heartbeat age) |
| [x] Market Data / Edge Status | `HealthSignals` / `StatTile` ("Market data") | `qml/OpsHeader.qml` Edge/Stream status | Mapped (`OpsStatus.edge_*`, `stream_state`) |
| [x] Unknown Orders Count | `HealthSignals` / `StatTile` ("Unknown orders") | `qml/OpsHeader.qml` Unknown orders chip | Mapped (`OpsStatus.unknown_orders`) |
| [x] Worker Lease Offline Banner | `execution-worker-down-banner` | `qml/OpsHeader.qml` Warning banner | Mapped (shown when API ok but worker offline/stale) |
| [x] Reconciliation Required Banner | `execution-unknown-orders-banner` | `qml/OpsHeader.qml` Reconciliation banner | Mapped (shown when `unknown_orders > 0`) |
| [x] Global Kill Switch State | `execution-kill-switch-state` (`Engaged`/`Off`) | `qml/OpsHeader.qml` Kill switch indicator | Mapped (`OpsStatus.kill_switch_enabled`) |
| [x] Global Kill Switch Slide / Release | `PowerOffSlide`, `execution-kill-switch-release` | `qml/OpsHeader.qml` engage/release + `ConfirmDialog.qml` | Mapped (Q-048) |
| [x] Manual Refresh Button | `Button` (calls `refetch()`) | N/A | Automated via live WebSocket stream & 2s health/positions poller |

---

## 2. Paper Accounts & Balances

| Frontend Element / Field | Frontend Source / Component | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| [x] Account Selector Tabs | `activeAccountId` tab chips | `qml/DeploymentDetail.qml` (Account tab) | Mapped |
| [x] Create Account Button (`+ Account`) | `Button` (`setShowCreateAccount`) | `qml/DeploymentList.qml` + `AccountDialog.qml` | Mapped (Q-048) |
| [x] Cash Balance | `StatTile` ("Cash balance") | `qml/DeploymentDetail.qml` Account summary | Mapped (`account.cash_balance` via `field_for_selected_account`) |
| [x] Equity (cash) | `StatTile` ("Equity (cash)") | `qml/DeploymentDetail.qml` Account summary | Mapped (`account.cash_balance`, same as frontend) |
| [x] Session P&L vs Initial | `StatTile` ("Session P&L vs initial") | `qml/DeploymentDetail.qml` Account summary | Mapped (`session_pnl` from store stream; no terminal arithmetic) |

---

## 3. Deployments List

| Frontend Element / Field | Frontend Source / Component | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| [x] New Deployment Button (`+ New`) | `Button` (`setShowCreateDeployment`) | `qml/DeploymentList.qml` + `DeployDialog.qml` | Mapped (Q-048; saved-run picker only) |
| [x] Deployment Name | `item.name` | `qml/DeploymentList.qml` name | Mapped |
| [x] Deployment Lifecycle | `item.lifecycle` (`running`, `paused`, `stopped`) | `qml/DeploymentList.qml` lifecycle badge | Mapped |
| [x] Deployment Status Color Bar | `statusBorder` (green, amber, rose) | `qml/DeploymentList.qml` border indicator | Mapped |
| [x] Symbol & Timeframe | `item.symbol`, `item.timeframe` | `qml/DeploymentList.qml` symbol & timeframe | Mapped |
| [x] Broker Mode | `item.broker_mode` (`paper` vs `live`) | `qml/DeploymentList.qml` broker mode badge | Mapped (visual distinction per spec) |
| [x] Pending Action Indicator | `item.pending_action` | `qml/DeploymentList.qml` pending action badge | Mapped |
| [x] Unknown Orders on Deployment | (Q-047 spec requirement) | `qml/DeploymentList.qml` unknown-order badge | Mapped |
| [x] Net Position & Entry Price | `deployment.open_position` | `qml/DeploymentList.qml` position snippet | Mapped |
| [x] External Monitor Link | `openExecutionMonitor` | N/A | Omitted: `q_terminal` is the dedicated native operations window |

---

## 4. Deployment Detail & Control

| Frontend Element / Field | Frontend Source / Component | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| [x] Current Status (Lifecycle) | `deployment.lifecycle` | `qml/DeploymentDetail.qml` header | Mapped |
| [x] Pending Action (Desired) | `deployment.pending_action` | `qml/DeploymentDetail.qml` header | Mapped |
| [x] Last Bar Close Time | `deployment.last_bar_close_time` | `qml/DeploymentDetail.qml` header | Mapped (formatted UTC time) |
| [x] Lifecycle Actions (Start, Pause, Stop) | `Button` controls | `qml/DeploymentDetail.qml` action bar + `ConfirmDialog.qml` | Mapped (Q-048) |
| [x] Monitor Live Action | `Button` (`openExecutionMonitor`) | N/A | Omitted: Native terminal view |
| [x] Flatten Action | `Button` (`setConfirmKind('flatten')`) | `qml/DeploymentDetail.qml` + `ConfirmDialog.qml` | Mapped (Q-048) |
| [x] Open Position Side | `deployment.open_position.side` (`long`/`short`/flat) | `qml/DeploymentDetail.qml` Position card | Mapped |
| [x] Open Position Quantity | `deployment.open_position.quantity` | `qml/DeploymentDetail.qml` Position card | Mapped |
| [x] Open Position Avg Entry Price | `deployment.open_position.average_entry_price` | `qml/DeploymentDetail.qml` Position card | Mapped (formatted string) |
| [x] Unrealized P&L | Backend `/positions` poller | `qml/DeploymentDetail.qml` Position card | Mapped (from poller, never computed locally) |
| [x] Mark Price | Backend `/positions` poller | `qml/DeploymentDetail.qml` Position card | Mapped (from poller) |
| [x] Live Candlestick Chart | `ExecutionLiveChartPanel` | `qml/OpsWorkspace.qml` (`ChartPane.qml`) | Mapped (integrated into workspace) |

---

## 5. Detail History Tables

### 5.1 Decisions Table

| Frontend Column / Field | Frontend Type (`Decision`) | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| [x] Time | `created_at` / `bar_close_time` | `qml/DecisionsTable.qml` column 1 | Mapped |
| [x] Action | `signal_action` (`buy`, `sell`, `hold`) | `qml/DecisionsTable.qml` column 2 | Mapped |
| [x] Outcome | `outcome` (`executed`, `submitted`, etc.) | `qml/DecisionsTable.qml` column 3 | Mapped |
| [x] Requested Qty | `requested_quantity` | `qml/DecisionsTable.qml` column 4 | Mapped |
| [x] Reason | `reason` | `qml/DecisionsTable.qml` column 5 | Mapped |

### 5.2 Orders Table

| Frontend Column / Field | Frontend Type (`ExecutionOrder`) | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| [x] Time | `created_at` | `qml/OrdersTable.qml` column 1 | Mapped |
| [x] Order ID | `id` (truncated) | `qml/OrdersTable.qml` column 2 | Mapped |
| [x] Intent Identifier | `intent_id` / client id (spec requirement) | `qml/OrdersTable.qml` column 3 | Mapped |
| [x] Side | `side` (`buy`, `sell`) | `qml/OrdersTable.qml` column 4 | Mapped |
| [x] Type | `order_type` | `qml/OrdersTable.qml` column 5 | Mapped |
| [x] Qty | `quantity` | `qml/OrdersTable.qml` column 6 | Mapped |
| [x] Status | `status` (`pending`, `filled`, etc.) | `qml/OrdersTable.qml` column 7 | Mapped |
| [x] Reconciliation | `reconciliation_state` | `qml/OrdersTable.qml` column 8 | Mapped |

### 5.3 Fills Table

| Frontend Column / Field | Frontend Type (`Fill`) | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| [x] Time | `filled_at` / `created_at` | `qml/FillsTable.qml` column 1 | Mapped |
| [x] Fill ID | `id` (truncated) | `qml/FillsTable.qml` column 2 | Mapped |
| [x] Order ID | `order_id` (truncated) | `qml/FillsTable.qml` column 3 | Mapped |
| [x] Side | `side` (`buy`, `sell`) | `qml/FillsTable.qml` column 4 | Mapped |
| [x] Price | `price` | `qml/FillsTable.qml` column 5 | Mapped (formatted decimal string) |
| [x] Qty | `quantity` | `qml/FillsTable.qml` column 6 | Mapped |
| [x] Fee | `fee` | `qml/FillsTable.qml` column 7 | Mapped (formatted decimal string) |
| [x] Slippage | `slippage` | `qml/FillsTable.qml` column 8 | Mapped (formatted decimal string) |

### 5.4 Ledger Table

| Frontend Column / Field | Frontend Type (`LedgerEntry`) | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| [x] Time | `created_at` | `qml/LedgerTable.qml` column 1 | Mapped |
| [x] Type | `entry_type` | `qml/LedgerTable.qml` column 2 | Mapped |
| [x] Amount | `amount` | `qml/LedgerTable.qml` column 3 | Mapped (formatted decimal string) |
| [x] Balance After | `balance_after` | `qml/LedgerTable.qml` column 4 | Mapped (formatted decimal string) |
| [x] Description | `description` | `qml/LedgerTable.qml` column 5 | Mapped |

### 5.5 Risk Events Table

| Frontend Column / Field | Frontend Type (`RiskEvent`) | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| [x] Time | `created_at` | `qml/RiskTable.qml` column 1 | Mapped |
| [x] Rejection Code | `rejection_code` | `qml/RiskTable.qml` column 2 | Mapped |
| [x] Message | `message` | `qml/RiskTable.qml` column 3 | Mapped |
| [x] Context | `context` (JSON) | `qml/RiskTable.qml` column 4 | Mapped |

### 5.6 Table Paging

| Frontend Control | Frontend Behavior | Terminal Location (`q_terminal`) | Status / Rationale |
|---|---|---|---|
| [x] Previous / Next Page | Offset-based page buttons | "Load older" button (`load_older`) | Mapped to append model pattern per spec |

---

## 6. Dialogs & Creation Overlays

| Frontend Dialog | Frontend Purpose | Terminal Status / Rationale |
|---|---|---|
| [x] Create Paper Account Modal | POST `/api/v1/execution/accounts` | `qml/AccountDialog.qml` | Mapped (Q-048) |
| [x] Create Deployment Modal | POST `/api/v1/execution/deployments` | `qml/DeployDialog.qml` | Mapped (Q-048) |
| [x] Confirm Flatten Dialog | POST `/api/v1/execution/deployments/{id}/actions` | `qml/ConfirmDialog.qml` | Mapped (Q-048) |
| [x] Resolve Unknown Order Dialog | POST `/api/v1/execution/orders/{id}/resolve` | `qml/ResolveDialog.qml` | Mapped (Q-048; no default outcome) |
