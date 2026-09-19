use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QVariant};
use serde_json::json;

use crate::execution::store::{ExecutionHandle, GLOBAL_KEY};

#[derive(Clone, Copy)]
pub struct RawTableModel(pub *mut ffi::TableModel);
unsafe impl Send for RawTableModel {}
unsafe impl Sync for RawTableModel {}

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;

        include!("cpp/table_bridge.h");
        type TableModel;

        fn make_table_model() -> *mut TableModel;
        unsafe fn delete_table_model(model: *mut TableModel);

        unsafe fn table_model_set_roles_csv(model: *mut TableModel, roles: &str);
        unsafe fn table_model_reset_json(model: *mut TableModel, json_array: &str);
        unsafe fn table_model_append_json(model: *mut TableModel, json_array: &str);
        unsafe fn table_model_count(model: *mut TableModel) -> i32;
        unsafe fn table_model_clear(model: *mut TableModel);
        unsafe fn table_model_to_variant(model: *mut TableModel) -> QVariant;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(i64, revision)]
        #[qproperty(i64, redraw_count)]
        #[qproperty(QString, selected_deployment_id)]
        #[qproperty(QString, selected_account_id)]
        #[qproperty(QVariant, deployments)]
        #[qproperty(QVariant, orders)]
        #[qproperty(QVariant, fills)]
        #[qproperty(QVariant, decisions)]
        #[qproperty(QVariant, risk_events)]
        #[qproperty(QVariant, ledger)]
        #[qproperty(QVariant, accounts)]
        #[qproperty(QString, api_base)]
        type ExecutionModels = super::ExecutionModelsRust;

        #[qinvokable]
        fn sync(self: Pin<&mut ExecutionModels>);

        #[qinvokable]
        fn select_deployment(self: Pin<&mut ExecutionModels>, deployment_id: QString);

        #[qinvokable]
        fn select_account(self: Pin<&mut ExecutionModels>, account_id: QString);

        #[qinvokable]
        fn load_older(self: Pin<&mut ExecutionModels>, table: QString);

        #[qinvokable]
        fn setup(self: Pin<&mut ExecutionModels>, api_base: QString);
    }

    impl cxx_qt::Threading for ExecutionModels {}
}

pub struct ExecutionModelsRust {
    pub revision: i64,
    pub redraw_count: i64,
    pub selected_deployment_id: QString,
    pub selected_account_id: QString,
    pub deployments: QVariant,
    pub orders: QVariant,
    pub fills: QVariant,
    pub decisions: QVariant,
    pub risk_events: QVariant,
    pub ledger: QVariant,
    pub accounts: QVariant,
    pub api_base: QString,

    pub handle: Option<ExecutionHandle>,
    pub dirty: Arc<AtomicBool>,

    pub m_deployments: RawTableModel,
    pub m_orders: RawTableModel,
    pub m_fills: RawTableModel,
    pub m_decisions: RawTableModel,
    pub m_risk: RawTableModel,
    pub m_ledger: RawTableModel,
    pub m_accounts: RawTableModel,

    pub older_orders: HashMap<String, Vec<serde_json::Value>>,
    pub older_fills: HashMap<String, Vec<serde_json::Value>>,
    pub older_decisions: HashMap<String, Vec<serde_json::Value>>,
    pub older_risk: Vec<serde_json::Value>,
    pub older_ledger: HashMap<String, Vec<serde_json::Value>>,
}

impl Default for ExecutionModelsRust {
    fn default() -> Self {
        unsafe {
            let m_deployments = RawTableModel(ffi::make_table_model());
            ffi::table_model_set_roles_csv(
                m_deployments.0,
                "id,name,symbol,timeframe,broker_mode,lifecycle,pending_action,last_bar_close_time,net_position,position_side,avg_entry_price,unknown_orders",
            );

            let m_orders = RawTableModel(ffi::make_table_model());
            ffi::table_model_set_roles_csv(
                m_orders.0,
                "id,created_at,intent_id,side,order_type,quantity,status,reconciliation_state,rejection_reason",
            );

            let m_fills = RawTableModel(ffi::make_table_model());
            ffi::table_model_set_roles_csv(
                m_fills.0,
                "id,order_id,created_at,filled_at,side,price,quantity,fee,slippage",
            );

            let m_decisions = RawTableModel(ffi::make_table_model());
            ffi::table_model_set_roles_csv(
                m_decisions.0,
                "id,created_at,bar_close_time,signal_action,outcome,requested_quantity,reason",
            );

            let m_risk = RawTableModel(ffi::make_table_model());
            ffi::table_model_set_roles_csv(
                m_risk.0,
                "id,created_at,rejection_code,message,context",
            );

            let m_ledger = RawTableModel(ffi::make_table_model());
            ffi::table_model_set_roles_csv(
                m_ledger.0,
                "id,created_at,entry_type,amount,balance_after,description",
            );

            let m_accounts = RawTableModel(ffi::make_table_model());
            ffi::table_model_set_roles_csv(
                m_accounts.0,
                "id,name,currency,initial_balance,cash_balance,session_pnl",
            );

            Self {
                revision: 0,
                redraw_count: 0,
                selected_deployment_id: QString::default(),
                selected_account_id: QString::default(),
                deployments: ffi::table_model_to_variant(m_deployments.0),
                orders: ffi::table_model_to_variant(m_orders.0),
                fills: ffi::table_model_to_variant(m_fills.0),
                decisions: ffi::table_model_to_variant(m_decisions.0),
                risk_events: ffi::table_model_to_variant(m_risk.0),
                ledger: ffi::table_model_to_variant(m_ledger.0),
                accounts: ffi::table_model_to_variant(m_accounts.0),
                api_base: QString::default(),
                handle: None,
                dirty: Arc::new(AtomicBool::new(false)),
                m_deployments,
                m_orders,
                m_fills,
                m_decisions,
                m_risk,
                m_ledger,
                m_accounts,
                older_orders: HashMap::new(),
                older_fills: HashMap::new(),
                older_decisions: HashMap::new(),
                older_risk: Vec::new(),
                older_ledger: HashMap::new(),
            }
        }
    }
}

impl Drop for ExecutionModelsRust {
    fn drop(&mut self) {
        unsafe {
            ffi::delete_table_model(self.m_deployments.0);
            ffi::delete_table_model(self.m_orders.0);
            ffi::delete_table_model(self.m_fills.0);
            ffi::delete_table_model(self.m_decisions.0);
            ffi::delete_table_model(self.m_risk.0);
            ffi::delete_table_model(self.m_ledger.0);
            ffi::delete_table_model(self.m_accounts.0);
        }
    }
}

impl ExecutionModelsRust {
    pub fn bind_handle(&mut self, handle: ExecutionHandle) {
        let dirty = self.dirty.clone();
        handle.set_listener(Arc::new(move || {
            dirty.store(true, Ordering::Release);
        }));
        self.handle = Some(handle);
        self.dirty.store(true, Ordering::Release);
    }

    pub fn rebuild_all(&mut self) {
        let handle = match &self.handle {
            Some(h) => h.clone(),
            None => return,
        };

        handle.read(|store| {
            let data = store.data();

            // 1. Deployments
            let mut dep_rows = Vec::new();
            let mut first_dep_id = String::new();

            for d in data.deployments.values() {
                if first_dep_id.is_empty() {
                    first_dep_id = d.id.clone();
                }
                let pos = data.positions.get(&d.id);
                let (net_pos, side, avg_entry) = if let Some(p) = pos {
                    if p.is_open {
                        (
                            p.quantity.clone(),
                            p.side.clone(),
                            p.average_entry_price.clone().unwrap_or_default(),
                        )
                    } else {
                        ("0".to_string(), "flat".to_string(), String::new())
                    }
                } else {
                    ("0".to_string(), "flat".to_string(), String::new())
                };

                let unk_count = data
                    .orders
                    .get(&d.id)
                    .map(|ring| {
                        ring.iter()
                            .filter(|o| {
                                o.reconciliation_state == "unknown"
                                    || o.reconciliation_state == "failed"
                            })
                            .count()
                    })
                    .unwrap_or(0);

                dep_rows.push(json!({
                    "id": d.id,
                    "name": d.name,
                    "symbol": d.symbol,
                    "timeframe": d.timeframe,
                    "broker_mode": d.broker_mode,
                    "lifecycle": d.lifecycle,
                    "pending_action": d.pending_action.as_ref().and_then(|v| v.as_str()).unwrap_or(""),
                    "last_bar_close_time": d.last_bar_close_time.as_ref().and_then(|v| v.as_str()).unwrap_or(""),
                    "net_position": net_pos,
                    "position_side": side,
                    "avg_entry_price": avg_entry,
                    "unknown_orders": unk_count,
                }));
            }

            let dep_json = serde_json::to_string(&dep_rows).unwrap_or_else(|_| "[]".into());
            unsafe {
                ffi::table_model_reset_json(self.m_deployments.0, &dep_json);
            }

            let sel_dep = self.selected_deployment_id.to_string();
            let active_dep = if sel_dep.is_empty() {
                first_dep_id
            } else {
                sel_dep
            };
            if self.selected_deployment_id.to_string() != active_dep && !active_dep.is_empty() {
                self.selected_deployment_id = QString::from(&active_dep);
            }

            // 2. Orders for active deployment (newest first)
            let mut order_rows = Vec::new();
            if let Some(orders) = data.orders.get(&active_dep) {
                for o in orders.iter().rev() {
                    order_rows.push(json!({
                        "id": o.id,
                        "created_at": o.created_at.as_deref().unwrap_or(""),
                        "intent_id": o.intent_id,
                        "side": o.side,
                        "order_type": o.order_type,
                        "quantity": o.quantity,
                        "status": o.status,
                        "reconciliation_state": o.reconciliation_state,
                        "rejection_reason": o.rejection_reason.as_ref().and_then(|v| v.as_str()).unwrap_or(""),
                    }));
                }
            }
            if let Some(older) = self.older_orders.get(&active_dep) {
                order_rows.extend(older.clone());
            }
            let ord_json = serde_json::to_string(&order_rows).unwrap_or_else(|_| "[]".into());
            unsafe {
                ffi::table_model_reset_json(self.m_orders.0, &ord_json);
            }

            // 3. Fills for active deployment (newest first)
            let mut fill_rows = Vec::new();
            if let Some(fills) = data.fills.get(&active_dep) {
                for f in fills.iter().rev() {
                    fill_rows.push(json!({
                        "id": f.id,
                        "order_id": f.order_id,
                        "created_at": f.created_at.as_deref().unwrap_or(""),
                        "filled_at": f.filled_at,
                        "side": f.side,
                        "price": f.price,
                        "quantity": f.quantity,
                        "fee": f.fee,
                        "slippage": f.slippage,
                    }));
                }
            }
            if let Some(older) = self.older_fills.get(&active_dep) {
                fill_rows.extend(older.clone());
            }
            let fill_json = serde_json::to_string(&fill_rows).unwrap_or_else(|_| "[]".into());
            unsafe {
                ffi::table_model_reset_json(self.m_fills.0, &fill_json);
            }

            // 4. Decisions for active deployment (newest first)
            let mut dec_rows = Vec::new();
            if let Some(decs) = data.decisions.get(&active_dep) {
                for d in decs.iter().rev() {
                    dec_rows.push(json!({
                        "id": d.id,
                        "created_at": d.created_at.as_deref().unwrap_or(""),
                        "bar_close_time": d.bar_close_time,
                        "signal_action": d.signal_action,
                        "outcome": d.outcome,
                        "requested_quantity": d.requested_quantity.as_deref().unwrap_or(""),
                        "reason": d.reason.as_ref().and_then(|v| v.as_str()).unwrap_or(""),
                    }));
                }
            }
            if let Some(older) = self.older_decisions.get(&active_dep) {
                dec_rows.extend(older.clone());
            }
            let dec_json = serde_json::to_string(&dec_rows).unwrap_or_else(|_| "[]".into());
            unsafe {
                ffi::table_model_reset_json(self.m_decisions.0, &dec_json);
            }

            // 5. Risk events (for active deployment + global)
            let mut risk_rows = Vec::new();
            if let Some(risks) = data.risk.get(&active_dep) {
                for r in risks.iter().rev() {
                    risk_rows.push(json!({
                        "id": r.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                        "created_at": r.get("created_at").and_then(|v| v.as_str()).unwrap_or(""),
                        "rejection_code": r.get("rejection_code").and_then(|v| v.as_str()).unwrap_or(""),
                        "message": r.get("message").and_then(|v| v.as_str()).unwrap_or(""),
                        "context": r.get("context").map(|c| c.to_string()).unwrap_or_default(),
                    }));
                }
            }
            if let Some(global_risks) = data.risk.get(GLOBAL_KEY) {
                for r in global_risks.iter().rev() {
                    risk_rows.push(json!({
                        "id": r.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                        "created_at": r.get("created_at").and_then(|v| v.as_str()).unwrap_or(""),
                        "rejection_code": r.get("rejection_code").and_then(|v| v.as_str()).unwrap_or(""),
                        "message": r.get("message").and_then(|v| v.as_str()).unwrap_or(""),
                        "context": r.get("context").map(|c| c.to_string()).unwrap_or_default(),
                    }));
                }
            }
            risk_rows.extend(self.older_risk.clone());
            let risk_json = serde_json::to_string(&risk_rows).unwrap_or_else(|_| "[]".into());
            unsafe {
                ffi::table_model_reset_json(self.m_risk.0, &risk_json);
            }

            // 6. Accounts
            let mut acc_rows = Vec::new();
            let mut first_acc_id = String::new();
            for a in data.accounts.values() {
                if first_acc_id.is_empty() {
                    first_acc_id = a.id.clone();
                }
                acc_rows.push(json!({
                    "id": a.id,
                    "name": a.name,
                    "currency": a.currency,
                    "initial_balance": a.initial_balance,
                    "cash_balance": a.cash_balance,
                    "session_pnl": a.realized_pnl,
                }));
            }
            let acc_json = serde_json::to_string(&acc_rows).unwrap_or_else(|_| "[]".into());
            unsafe {
                ffi::table_model_reset_json(self.m_accounts.0, &acc_json);
            }

            let sel_acc = self.selected_account_id.to_string();
            let active_acc = if sel_acc.is_empty() {
                first_acc_id
            } else {
                sel_acc
            };
            if self.selected_account_id.to_string() != active_acc && !active_acc.is_empty() {
                self.selected_account_id = QString::from(&active_acc);
            }

            // 7. Ledger for active account
            let mut ledger_rows = Vec::new();
            if let Some(older) = self.older_ledger.get(&active_acc) {
                ledger_rows.extend(older.clone());
            }
            let led_json = serde_json::to_string(&ledger_rows).unwrap_or_else(|_| "[]".into());
            unsafe {
                ffi::table_model_reset_json(self.m_ledger.0, &led_json);
            }
        });
    }

    pub fn refresh_deployment_detail(&mut self) {
        let handle = match &self.handle {
            Some(h) => h.clone(),
            None => return,
        };
        let active_dep = self.selected_deployment_id.to_string();

        handle.read(|store| {
            let data = store.data();

            // Orders
            let mut order_rows = Vec::new();
            if let Some(orders) = data.orders.get(&active_dep) {
                for o in orders.iter().rev() {
                    order_rows.push(json!({
                        "id": o.id,
                        "created_at": o.created_at.as_deref().unwrap_or(""),
                        "intent_id": o.intent_id,
                        "side": o.side,
                        "order_type": o.order_type,
                        "quantity": o.quantity,
                        "status": o.status,
                        "reconciliation_state": o.reconciliation_state,
                        "rejection_reason": o.rejection_reason.as_ref().and_then(|v| v.as_str()).unwrap_or(""),
                    }));
                }
            }
            if let Some(older) = self.older_orders.get(&active_dep) {
                order_rows.extend(older.clone());
            }
            let ord_json = serde_json::to_string(&order_rows).unwrap_or_else(|_| "[]".into());
            unsafe {
                ffi::table_model_reset_json(self.m_orders.0, &ord_json);
            }

            // Fills
            let mut fill_rows = Vec::new();
            if let Some(fills) = data.fills.get(&active_dep) {
                for f in fills.iter().rev() {
                    fill_rows.push(json!({
                        "id": f.id,
                        "order_id": f.order_id,
                        "created_at": f.created_at.as_deref().unwrap_or(""),
                        "filled_at": f.filled_at,
                        "side": f.side,
                        "price": f.price,
                        "quantity": f.quantity,
                        "fee": f.fee,
                        "slippage": f.slippage,
                    }));
                }
            }
            if let Some(older) = self.older_fills.get(&active_dep) {
                fill_rows.extend(older.clone());
            }
            let fill_json = serde_json::to_string(&fill_rows).unwrap_or_else(|_| "[]".into());
            unsafe {
                ffi::table_model_reset_json(self.m_fills.0, &fill_json);
            }

            // Decisions
            let mut dec_rows = Vec::new();
            if let Some(decs) = data.decisions.get(&active_dep) {
                for d in decs.iter().rev() {
                    dec_rows.push(json!({
                        "id": d.id,
                        "created_at": d.created_at.as_deref().unwrap_or(""),
                        "bar_close_time": d.bar_close_time,
                        "signal_action": d.signal_action,
                        "outcome": d.outcome,
                        "requested_quantity": d.requested_quantity.as_deref().unwrap_or(""),
                        "reason": d.reason.as_ref().and_then(|v| v.as_str()).unwrap_or(""),
                    }));
                }
            }
            if let Some(older) = self.older_decisions.get(&active_dep) {
                dec_rows.extend(older.clone());
            }
            let dec_json = serde_json::to_string(&dec_rows).unwrap_or_else(|_| "[]".into());
            unsafe {
                ffi::table_model_reset_json(self.m_decisions.0, &dec_json);
            }

            // Risk
            let mut risk_rows = Vec::new();
            if let Some(risks) = data.risk.get(&active_dep) {
                for r in risks.iter().rev() {
                    risk_rows.push(json!({
                        "id": r.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                        "created_at": r.get("created_at").and_then(|v| v.as_str()).unwrap_or(""),
                        "rejection_code": r.get("rejection_code").and_then(|v| v.as_str()).unwrap_or(""),
                        "message": r.get("message").and_then(|v| v.as_str()).unwrap_or(""),
                        "context": r.get("context").map(|c| c.to_string()).unwrap_or_default(),
                    }));
                }
            }
            if let Some(global_risks) = data.risk.get(GLOBAL_KEY) {
                for r in global_risks.iter().rev() {
                    risk_rows.push(json!({
                        "id": r.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                        "created_at": r.get("created_at").and_then(|v| v.as_str()).unwrap_or(""),
                        "rejection_code": r.get("rejection_code").and_then(|v| v.as_str()).unwrap_or(""),
                        "message": r.get("message").and_then(|v| v.as_str()).unwrap_or(""),
                        "context": r.get("context").map(|c| c.to_string()).unwrap_or_default(),
                    }));
                }
            }
            risk_rows.extend(self.older_risk.clone());
            let risk_json = serde_json::to_string(&risk_rows).unwrap_or_else(|_| "[]".into());
            unsafe {
                ffi::table_model_reset_json(self.m_risk.0, &risk_json);
            }
        });
    }

    pub fn refresh_ledger(&mut self) {
        let active_acc = self.selected_account_id.to_string();
        let mut ledger_rows = Vec::new();
        if let Some(older) = self.older_ledger.get(&active_acc) {
            ledger_rows.extend(older.clone());
        }
        let led_json = serde_json::to_string(&ledger_rows).unwrap_or_else(|_| "[]".into());
        unsafe {
            ffi::table_model_reset_json(self.m_ledger.0, &led_json);
        }
    }
}

impl ffi::ExecutionModels {
    pub fn setup(mut self: Pin<&mut Self>, api_base: QString) {
        self.as_mut().set_api_base(api_base);
    }

    pub fn sync(mut self: Pin<&mut Self>) {
        if self.rust().dirty.swap(false, Ordering::AcqRel) {
            self.as_mut().rust_mut().rebuild_all();
            let rev = self.rust().revision + 1;
            let redraws = self.rust().redraw_count + 1;
            self.as_mut().set_revision(rev);
            self.as_mut().set_redraw_count(redraws);
        }
    }

    pub fn select_deployment(mut self: Pin<&mut Self>, deployment_id: QString) {
        self.as_mut().set_selected_deployment_id(deployment_id);
        self.as_mut().rust_mut().refresh_deployment_detail();
    }

    pub fn select_account(mut self: Pin<&mut Self>, account_id: QString) {
        self.as_mut().set_selected_account_id(account_id);
        self.as_mut().rust_mut().refresh_ledger();
    }

    pub fn load_older(self: Pin<&mut Self>, table: QString) {
        let api_base = self.rust().api_base.to_string();
        let table_str = table.to_string();
        let dep_id = self.rust().selected_deployment_id.to_string();
        let acc_id = self.rust().selected_account_id.to_string();

        let model_ptr = match table_str.as_str() {
            "orders" => self.rust().m_orders,
            "fills" => self.rust().m_fills,
            "decisions" => self.rust().m_decisions,
            "risk" | "risk-events" | "risk_events" => self.rust().m_risk,
            "ledger" => self.rust().m_ledger,
            _ => return,
        };

        let current_count = unsafe { ffi::table_model_count(model_ptr.0) };
        let offset = current_count;

        let url = match table_str.as_str() {
            "orders" => {
                if dep_id.is_empty() {
                    return;
                }
                format!("{api_base}/api/v1/execution/deployments/{dep_id}/orders?limit=50&offset={offset}")
            }
            "fills" => {
                if dep_id.is_empty() {
                    return;
                }
                format!("{api_base}/api/v1/execution/deployments/{dep_id}/fills?limit=50&offset={offset}")
            }
            "decisions" => {
                if dep_id.is_empty() {
                    return;
                }
                format!("{api_base}/api/v1/execution/deployments/{dep_id}/decisions?limit=50&offset={offset}")
            }
            "risk" | "risk-events" | "risk_events" => {
                format!("{api_base}/api/v1/execution/risk-events?limit=50&offset={offset}")
            }
            "ledger" => {
                if acc_id.is_empty() {
                    return;
                }
                format!("{api_base}/api/v1/execution/accounts/{acc_id}/ledger?limit=50&offset={offset}")
            }
            _ => return,
        };

        let qt_thread = self.qt_thread();

        let _ = std::thread::Builder::new()
            .name("load-older".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(_) => return,
                };

                rt.block_on(async move {
                    let client = reqwest::Client::new();
                    let resp = match client.get(&url).send().await {
                        Ok(r) => r,
                        Err(_) => return,
                    };
                    if !resp.status().is_success() {
                        return;
                    }
                    let body: serde_json::Value = match resp.json().await {
                        Ok(b) => b,
                        Err(_) => return,
                    };
                    let items = body.get("items").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                    if items.is_empty() {
                        return;
                    }
                    let items_json = serde_json::to_string(&items).unwrap_or_default();

                    let _ = qt_thread.queue(move |mut models| {
                        let table = table_str.as_str();
                        let ptr = match table {
                            "orders" => models.rust().m_orders,
                            "fills" => models.rust().m_fills,
                            "decisions" => models.rust().m_decisions,
                            "risk" | "risk-events" | "risk_events" => models.rust().m_risk,
                            "ledger" => models.rust().m_ledger,
                            _ => return,
                        };
                        unsafe {
                            ffi::table_model_append_json(ptr.0, &items_json);
                        }
                        let mut rust = models.as_mut().rust_mut();
                        match table {
                            "orders" => {
                                rust.older_orders.entry(dep_id).or_default().extend(items);
                            }
                            "fills" => {
                                rust.older_fills.entry(dep_id).or_default().extend(items);
                            }
                            "decisions" => {
                                rust.older_decisions.entry(dep_id).or_default().extend(items);
                            }
                            "risk" | "risk-events" | "risk_events" => {
                                rust.older_risk.extend(items);
                            }
                            "ledger" => {
                                rust.older_ledger.entry(acc_id).or_default().extend(items);
                            }
                            _ => {}
                        }
                    });
                });
            });
    }
}
