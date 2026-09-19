#![allow(clippy::float_cmp)]

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, stream_state)]
        #[qproperty(f64, stream_age_s)]
        #[qproperty(QString, api_status)]
        #[qproperty(QString, worker_status)]
        #[qproperty(f64, worker_heartbeat_age_s)]
        #[qproperty(bool, edge_reachable)]
        #[qproperty(bool, edge_mt5_connected)]
        #[qproperty(i64, terminal_build)]
        #[qproperty(bool, kill_switch_enabled)]
        #[qproperty(bool, live_locked)]
        #[qproperty(i64, unknown_orders)]
        #[qproperty(bool, postgres_available)]
        #[qproperty(QString, market_data_status)]
        #[qproperty(QString, positions_pnl_json)]
        type OpsStatus = super::OpsStatusRust;

        #[qinvokable]
        fn set_stream_info(self: Pin<&mut OpsStatus>, state: QString, age_s: f64);

        #[qinvokable]
        fn apply_health_json(self: Pin<&mut OpsStatus>, health_json: QString);

        #[qinvokable]
        fn apply_positions_json(self: Pin<&mut OpsStatus>, positions_json: QString);

        #[qinvokable]
        fn mark_api_offline(self: Pin<&mut OpsStatus>);

        #[qinvokable]
        fn mark_postgres_down(self: Pin<&mut OpsStatus>);
    }

    impl cxx_qt::Threading for OpsStatus {}
}

pub struct OpsStatusRust {
    pub stream_state: QString,
    pub stream_age_s: f64,
    pub api_status: QString,
    pub worker_status: QString,
    pub worker_heartbeat_age_s: f64,
    pub edge_reachable: bool,
    pub edge_mt5_connected: bool,
    pub terminal_build: i64,
    pub kill_switch_enabled: bool,
    pub live_locked: bool,
    pub unknown_orders: i64,
    pub postgres_available: bool,
    pub market_data_status: QString,
    pub positions_pnl_json: QString,
}

impl Default for OpsStatusRust {
    fn default() -> Self {
        Self {
            stream_state: QString::from("DISCONNECTED"),
            stream_age_s: 0.0,
            api_status: QString::from("unknown"),
            worker_status: QString::from("unknown"),
            worker_heartbeat_age_s: 0.0,
            edge_reachable: false,
            edge_mt5_connected: false,
            terminal_build: 0,
            kill_switch_enabled: false,
            live_locked: true,
            unknown_orders: 0,
            postgres_available: true,
            market_data_status: QString::from("unknown"),
            positions_pnl_json: QString::from("{}"),
        }
    }
}

impl OpsStatusRust {
    pub fn set_stream_info(&mut self, state: &str, age_s: f64) {
        self.stream_state = QString::from(state);
        self.stream_age_s = age_s;
    }

    pub fn mark_api_offline(&mut self) {
        self.api_status = QString::from("offline");
        self.worker_status = QString::from("unknown");
        self.postgres_available = false;
    }

    pub fn mark_postgres_down(&mut self) {
        self.api_status = QString::from("degraded");
        self.postgres_available = false;
    }

    pub fn apply_health_str(&mut self, json_str: &str) {
        let val: serde_json::Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(_) => return,
        };

        let api_st = val
            .get("api_status")
            .and_then(|v| v.as_str())
            .unwrap_or("ok");
        let worker_st = val
            .get("worker_status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let hb_age = val
            .get("worker_heartbeat_age_s")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let kill_switch = val
            .get("kill_switch_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let live_lock = val
            .get("live_capability_locked")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let unk_orders = val
            .get("unknown_order_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let mkt_st = val
            .get("market_data_status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let edge = val.get("edge");
        let reachable = edge
            .and_then(|e| e.get("reachable"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let mt5_conn = edge
            .and_then(|e| e.get("mt5_connected"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let build = edge
            .and_then(|e| e.get("terminal_build"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        self.api_status = QString::from(api_st);
        self.worker_status = QString::from(worker_st);
        self.worker_heartbeat_age_s = hb_age;
        self.edge_reachable = reachable;
        self.edge_mt5_connected = mt5_conn;
        self.terminal_build = build;
        self.kill_switch_enabled = kill_switch;
        self.live_locked = live_lock;
        self.unknown_orders = unk_orders;
        self.postgres_available = true;
        self.market_data_status = QString::from(mkt_st);
    }

    pub fn apply_positions_str(&mut self, json_str: &str) {
        self.positions_pnl_json = QString::from(json_str);
    }
}

impl ffi::OpsStatus {
    pub fn set_stream_info(mut self: std::pin::Pin<&mut Self>, state: QString, age_s: f64) {
        let state_str = state.to_string();
        self.as_mut().rust_mut().set_stream_info(&state_str, age_s);
        self.as_mut().set_stream_state(state);
        self.as_mut().set_stream_age_s(age_s);
    }

    pub fn mark_api_offline(mut self: std::pin::Pin<&mut Self>) {
        self.as_mut().rust_mut().mark_api_offline();
        self.as_mut().set_api_status(QString::from("offline"));
        self.as_mut().set_worker_status(QString::from("unknown"));
        self.as_mut().set_postgres_available(false);
    }

    pub fn mark_postgres_down(mut self: std::pin::Pin<&mut Self>) {
        self.as_mut().rust_mut().mark_postgres_down();
        self.as_mut().set_api_status(QString::from("degraded"));
        self.as_mut().set_postgres_available(false);
    }

    pub fn apply_health_json(mut self: std::pin::Pin<&mut Self>, health_json: QString) {
        let json_str = health_json.to_string();
        self.as_mut().rust_mut().apply_health_str(&json_str);

        let r = self.rust();
        let api_status = r.api_status.clone();
        let worker_status = r.worker_status.clone();
        let worker_heartbeat_age_s = r.worker_heartbeat_age_s;
        let edge_reachable = r.edge_reachable;
        let edge_mt5_connected = r.edge_mt5_connected;
        let terminal_build = r.terminal_build;
        let kill_switch_enabled = r.kill_switch_enabled;
        let live_locked = r.live_locked;
        let unknown_orders = r.unknown_orders;
        let postgres_available = r.postgres_available;
        let market_data_status = r.market_data_status.clone();

        self.as_mut().set_api_status(api_status);
        self.as_mut().set_worker_status(worker_status);
        self.as_mut()
            .set_worker_heartbeat_age_s(worker_heartbeat_age_s);
        self.as_mut().set_edge_reachable(edge_reachable);
        self.as_mut().set_edge_mt5_connected(edge_mt5_connected);
        self.as_mut().set_terminal_build(terminal_build);
        self.as_mut().set_kill_switch_enabled(kill_switch_enabled);
        self.as_mut().set_live_locked(live_locked);
        self.as_mut().set_unknown_orders(unknown_orders);
        self.as_mut().set_postgres_available(postgres_available);
        self.as_mut().set_market_data_status(market_data_status);
    }

    pub fn apply_positions_json(mut self: std::pin::Pin<&mut Self>, positions_json: QString) {
        let p_str = positions_json.to_string();
        self.as_mut().rust_mut().apply_positions_str(&p_str);
        self.as_mut().set_positions_pnl_json(positions_json);
    }
}
