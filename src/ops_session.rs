use std::pin::Pin;
use std::sync::Arc;

use cxx_qt_lib::QQmlApplicationEngine;

use crate::config::Config;
use crate::execution::health::HealthPoller;
use crate::execution::store::ExecutionHandle;
use crate::stream::client::{Sinks, StreamClient};
use crate::stream::sink::BarSink;

fn engine_addr(engine: &mut cxx::UniquePtr<QQmlApplicationEngine>) -> usize {
    let pin = engine.as_mut().unwrap();
    unsafe { std::ptr::from_mut(Pin::get_unchecked_mut(pin)) as usize }
}

/// Wires execution stream, health poller, and store bindings into the QML window.
pub fn wire_ops_session(
    engine: &mut cxx::UniquePtr<QQmlApplicationEngine>,
    config: &Config,
    bar_sink: BarSink,
) -> (ExecutionHandle, Option<StreamClient>, Arc<HealthPoller>) {
    let handle = ExecutionHandle::new();

    let client = StreamClient::start_with(
        config.clone(),
        Sinks {
            bars: bar_sink,
            execution: Some(handle.clone()),
        },
    );

    crate::execution_models::stage_execution_handle(handle.clone());
    crate::execution_controls::stage_control_handle(handle.clone());

    unsafe {
        let models = crate::chart_bridge::find_window_execution_models(engine.as_mut().unwrap());
        if !models.is_null() {
            crate::chart_bridge::execution_models_setup(models, &config.api_base);
            crate::chart_bridge::execution_models_bind_handle(models);
        }
        let controls =
            crate::chart_bridge::find_window_execution_controls(engine.as_mut().unwrap());
        if controls != 0 {
            crate::chart_bridge::execution_controls_setup(
                controls,
                &config.api_base,
                &config.operator,
            );
            crate::chart_bridge::execution_controls_bind_handle(controls);
        }
    }

    let poller = Arc::new(HealthPoller::new(&config.api_base));
    let api_base = config.api_base.clone();
    let engine_addr = engine_addr(engine);

    poller.start({
        move |update| unsafe {
            let engine_mut = engine_addr as *mut QQmlApplicationEngine;
            let status =
                crate::chart_bridge::find_window_ops_status(Pin::new_unchecked(&mut *engine_mut));
            if status != 0 {
                if update.api_offline {
                    crate::chart_bridge::ops_status_mark_api_offline(status);
                } else if update.api_degraded {
                    crate::chart_bridge::ops_status_mark_postgres_down(status);
                } else if let Some(json) = update.health_json.as_ref() {
                    crate::chart_bridge::ops_status_apply_health(status, json);
                }
                if let Some(pos) = update.positions_json.as_ref() {
                    crate::chart_bridge::ops_status_apply_positions(status, pos);
                }
            }
            let controls = crate::chart_bridge::find_window_execution_controls(Pin::new_unchecked(
                &mut *engine_mut,
            ));
            if controls != 0 {
                let worker_status = update
                    .health_json
                    .as_ref()
                    .and_then(|j| {
                        serde_json::from_str::<serde_json::Value>(j)
                            .ok()
                            .and_then(|v| {
                                v.get("worker_status")
                                    .and_then(|s| s.as_str())
                                    .map(String::from)
                            })
                    })
                    .unwrap_or_else(|| "unknown".to_string());
                let hb = update
                    .health_json
                    .as_ref()
                    .and_then(|j| {
                        serde_json::from_str::<serde_json::Value>(j)
                            .ok()
                            .and_then(|v| v.get("worker_heartbeat_age_s").and_then(|s| s.as_f64()))
                    })
                    .unwrap_or(0.0);
                let (edge_reachable, edge_mt5) = update
                    .health_json
                    .as_ref()
                    .and_then(|j| {
                        serde_json::from_str::<serde_json::Value>(j).ok().map(|v| {
                            let edge = v.get("edge");
                            (
                                edge.and_then(|e| e.get("reachable"))
                                    .and_then(|r| r.as_bool())
                                    .unwrap_or(false),
                                edge.and_then(|e| e.get("mt5_connected"))
                                    .and_then(|r| r.as_bool())
                                    .unwrap_or(false),
                            )
                        })
                    })
                    .unwrap_or((false, false));
                crate::chart_bridge::execution_controls_update_health(
                    controls,
                    update.api_offline,
                    update.postgres_available,
                    &worker_status,
                    hb,
                    edge_reachable,
                    edge_mt5,
                );
            }
        }
    });

    let _ = api_base;
    (handle, Some(client), poller)
}
