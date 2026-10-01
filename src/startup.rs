use std::sync::Arc;

use cxx_qt_lib::{QQmlApplicationEngine, QString, QUrl};

use crate::bridge;
use crate::bridge::bar_feed::parse_timeframe_ms;
use crate::chart_bridge;
use crate::chart_context;
use crate::chart_target::{ChartTargeter, OverlayFetcher};
use crate::config::{Config, ConfigError};
use crate::execution::store::ExecutionHandle;
use crate::ops_session;
use crate::stream::client::{ConnectionState, StreamClient};
use crate::stream::sink::BarSink;

/// Context holding the loaded slice resources.
pub struct SliceContext {
    // Fields drop in declaration order. The stream client goes first: its listeners post to
    // the feed and the models, which the engine owns and destroys.
    pub stream_client: Option<StreamClient>,
    pub health_poller: Option<std::sync::Arc<crate::execution::health::HealthPoller>>,
    pub targeter: Option<std::sync::Arc<ChartTargeter>>,
    pub engine: cxx::UniquePtr<cxx_qt_lib::QQmlApplicationEngine>,
    pub feed_ptr: *mut chart_bridge::BarFeed,
}

impl Drop for SliceContext {
    fn drop(&mut self) {
        if let Some(poller) = &self.health_poller {
            poller.stop();
        }
        let _ = self.stream_client.take();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitialTargetResolution {
    pub symbol: String,
    pub timeframe: String,
    pub mode: crate::chart_target::ChartMode,
    pub is_session_override: bool,
    pub followed_deployment_id: Option<String>,
    pub last_manual: (String, String),
    pub visible_bars: i32,
}

pub fn resolve_initial_target(
    cfg_symbol: &str,
    cfg_timeframe: &str,
    env_symbol: Option<&str>,
    env_timeframe: Option<&str>,
    saved_prefs: Option<&crate::workspace::schema::WorkspaceChartPreferences>,
) -> InitialTargetResolution {
    let clean_env_sym = env_symbol.map(|s| s.trim()).filter(|s| !s.is_empty());
    let clean_env_tf = env_timeframe.map(|s| s.trim()).filter(|s| !s.is_empty());
    let is_session_override = clean_env_sym.is_some() || clean_env_tf.is_some();

    let (saved_sym, saved_tf, saved_mode, saved_dep_id, saved_man_sym, saved_man_tf, saved_vb) =
        if let Some(p) = saved_prefs {
            (
                Some(p.symbol.trim()).filter(|s| !s.is_empty()),
                Some(p.timeframe.trim()).filter(|s| !s.is_empty()),
                Some(p.mode.as_str()),
                p.followed_deployment_id
                    .clone()
                    .filter(|s| !s.trim().is_empty()),
                p.last_manual_symbol
                    .clone()
                    .filter(|s| !s.trim().is_empty()),
                p.last_manual_timeframe
                    .clone()
                    .filter(|s| !s.trim().is_empty()),
                p.visible_bars.unwrap_or(120),
            )
        } else {
            (None, None, None, None, None, None, 120)
        };

    let clean_cfg_sym = Some(cfg_symbol.trim()).filter(|s| !s.is_empty());
    let clean_cfg_tf = Some(cfg_timeframe.trim()).filter(|s| !s.is_empty());

    let symbol = clean_env_sym
        .or(saved_sym)
        .or(clean_cfg_sym)
        .unwrap_or("PETR4")
        .to_string();

    let timeframe = clean_env_tf
        .or(saved_tf)
        .or(clean_cfg_tf)
        .unwrap_or("1m")
        .to_string();

    let mode = if is_session_override {
        crate::chart_target::ChartMode::Manual
    } else if let Some(m) = saved_mode {
        if m == "manual" {
            crate::chart_target::ChartMode::Manual
        } else {
            crate::chart_target::ChartMode::Following
        }
    } else {
        crate::chart_target::ChartMode::Following
    };

    let last_manual = (
        saved_man_sym.unwrap_or_else(|| {
            if !is_session_override {
                symbol.clone()
            } else {
                saved_sym.or(clean_cfg_sym).unwrap_or("PETR4").to_string()
            }
        }),
        saved_man_tf.unwrap_or_else(|| {
            if !is_session_override {
                timeframe.clone()
            } else {
                saved_tf.or(clean_cfg_tf).unwrap_or("1m").to_string()
            }
        }),
    );

    let followed_deployment_id = if is_session_override {
        None
    } else {
        saved_dep_id
    };

    InitialTargetResolution {
        symbol,
        timeframe,
        mode,
        is_session_override,
        followed_deployment_id,
        last_manual,
        visible_bars: if saved_vb > 0 { saved_vb } else { 120 },
    }
}

/// Binds the execution store to the window's models and makes the chart follow the
/// selected deployment.
fn wire_execution(
    engine: std::pin::Pin<&mut QQmlApplicationEngine>,
    handle: ExecutionHandle,
    targeter: std::sync::Arc<ChartTargeter>,
    feed_ptr: *mut chart_bridge::BarFeed,
    chart_ctx_addr: usize,
    configured: (String, String),
    initial_target: InitialTargetResolution,
) {
    let feed_addr = feed_ptr as usize;
    let models = chart_bridge::find_window_execution_models(engine);
    if models.is_null() {
        return;
    }
    use cxx_qt::CxxQtType;
    let ffi_models = models as *mut crate::bridge::execution_models::ffi::ExecutionModels;
    let configured_sym = configured.0.clone();
    let configured_tf = configured.1.clone();
    let dirty = unsafe {
        let mut pin = std::pin::Pin::new_unchecked(&mut *ffi_models);
        let mut rust = pin.as_mut().rust_mut();
        rust.bind_handle(handle.clone());
        let targeter_for_target = targeter.clone();
        rust.on_target = Some(std::sync::Arc::new(move |dep| {
            let is_manual = targeter_for_target.is_manual();
            if chart_ctx_addr != 0 {
                chart_context::notify_target(
                    chart_ctx_addr as *mut chart_context::ffi::ChartContext,
                    dep.as_ref().map(|(_id, name, sym, tf)| {
                        (name.as_str(), sym.as_str(), tf.as_str(), !is_manual)
                    }),
                    (&configured_sym, &configured_tf),
                );
            }
            let retargeted = targeter_for_target.select(dep);
            if chart_ctx_addr != 0 {
                if let Some(target) = retargeted {
                    chart_context::notify_retarget(
                        chart_ctx_addr as *mut chart_context::ffi::ChartContext,
                        &target.symbol,
                        &target.timeframe,
                    );
                }
            }
        }));
        let targeter_for_rows = targeter.clone();
        rust.on_rows = Some(std::sync::Arc::new(move |dec, fills| {
            if targeter_for_rows.is_manual() {
                return;
            }
            chart_bridge::feed_set_execution_rows(
                feed_addr as *mut chart_bridge::BarFeed,
                &dec,
                &fills,
            )
        }));
        let handle_for_restore = handle.clone();
        let targeter_for_restore = targeter.clone();
        targeter.set_on_restore_rows(std::sync::Arc::new(move || {
            if let Some((id, _name, _sym, _tf)) = targeter_for_restore.following_target() {
                let (dec, fills) = handle_for_restore.read(|s| {
                    let d = s.data();
                    let dec: Vec<_> = d.decisions.get(&id).into_iter().flatten().collect();
                    let fills: Vec<_> = d.fills.get(&id).into_iter().flatten().collect();
                    (
                        serde_json::to_string(&dec).unwrap_or_else(|_| "[]".into()),
                        serde_json::to_string(&fills).unwrap_or_else(|_| "[]".into()),
                    )
                });
                chart_bridge::feed_set_execution_rows(
                    feed_addr as *mut chart_bridge::BarFeed,
                    &dec,
                    &fills,
                );
            }
        }));
        rust.dirty.clone()
    };
    let addr = models as usize;
    let initial_resolved = initial_target.clone();
    let initial_checked = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let targeter_for_listener = targeter.clone();
    let handle_for_listener = handle.clone();
    handle.set_listener(std::sync::Arc::new(move || {
        dirty.store(true, std::sync::atomic::Ordering::Release);
        unsafe {
            chart_bridge::post_execution_models_sync(addr as *mut chart_bridge::ExecutionModels)
        };
        if !initial_checked.load(std::sync::atomic::Ordering::SeqCst) {
            let is_confirmed = handle_for_listener.read(|s| s.is_confirmed());
            if is_confirmed {
                initial_checked.store(true, std::sync::atomic::Ordering::SeqCst);
                if initial_resolved.mode == crate::chart_target::ChartMode::Following
                    && !initial_resolved.is_session_override
                {
                    if let Some(ref dep_id) = initial_resolved.followed_deployment_id {
                        let (is_active, _dep_name) = handle_for_listener.read(|s| {
                            if let Some(dep) = s.data().deployments.get(dep_id) {
                                let is_archived = dep.archived == Some(true)
                                    || dep.lifecycle.as_str() == "archived";
                                (!is_archived, dep.name.clone())
                            } else {
                                (false, String::new())
                            }
                        });
                        if !is_active {
                            let (sym, tf) = &initial_resolved.last_manual;
                            let _ = targeter_for_listener.request_manual(sym, tf);
                            if chart_ctx_addr != 0 {
                                unsafe {
                                    chart_context::notify_target_error(
                                        chart_ctx_addr as *mut chart_context::ffi::ChartContext,
                                        &format!(
                                            "Followed deployment '{dep_id}' is no longer active; selected manual target {sym} · {tf}."
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }));
}

/// Sets up the window and slice context for the given config.
/// Opens the window whatever the API's state: configuration failures are shown
/// in the scene, not printed to a terminal the user is not reading.
pub fn setup_slice(config: &Result<Config, ConfigError>) -> SliceContext {
    chart_bridge::ensure_application();
    chart_bridge::register_chart_types();

    let mut engine = QQmlApplicationEngine::new();

    let uri = QString::from("target/cxxqt/qml_modules");
    engine.as_mut().unwrap().add_import_path(&uri);

    let qml_file = QString::from("qml/Main.qml");
    let qml_url = QUrl::from_local_file(&qml_file);
    engine.as_mut().unwrap().load(&qml_url);

    bridge::ffi::setup_window(engine.as_mut().unwrap());

    let feed_ptr = unsafe { chart_bridge::find_window_feed(engine.as_mut().unwrap()) };
    let mut stream_client: Option<StreamClient> = None;
    let mut chart_targeter: Option<std::sync::Arc<ChartTargeter>> = None;
    let mut health_poller: Option<std::sync::Arc<crate::execution::health::HealthPoller>> = None;

    if !feed_ptr.is_null() {
        match config {
            Err(err) => unsafe {
                chart_bridge::feed_set_symbol(feed_ptr, "CONFIG_ERROR");
                chart_bridge::feed_set_timeframe(feed_ptr, "", 60_000);
                chart_bridge::feed_set_connection_state(feed_ptr, "error");
                chart_bridge::feed_set_last_error(feed_ptr, &err.to_string());
            },
            Ok(cfg) => {
                let env_symbol = std::env::var("Q_TERMINAL_SYMBOL").ok();
                let env_timeframe = std::env::var("Q_TERMINAL_TIMEFRAME").ok();
                let mut store = crate::workspace::store::WorkspaceStore::open_default();
                let (last_ws, _) = store.load_last_or_default();
                let saved_prefs = last_ws.chart_preferences.get("chart").cloned();

                let initial = resolve_initial_target(
                    &cfg.symbol,
                    &cfg.timeframe,
                    env_symbol.as_deref(),
                    env_timeframe.as_deref(),
                    saved_prefs.as_ref(),
                );

                let tf_ms = parse_timeframe_ms(&initial.timeframe);
                unsafe {
                    chart_bridge::feed_set_symbol(feed_ptr, &initial.symbol);
                    chart_bridge::feed_set_timeframe(feed_ptr, &initial.timeframe, tf_ms);
                    chart_bridge::feed_set_connection_state(feed_ptr, "connecting");
                    chart_bridge::feed_set_last_error(feed_ptr, "");
                }

                let sink = BarSink::new();
                let (exec_handle, client_opt, poller) =
                    ops_session::wire_ops_session(&mut engine, cfg, sink.clone());
                let client = client_opt.expect("execution stream client");
                health_poller = Some(poller);

                let fetcher = OverlayFetcher::new(&cfg.api_base, feed_ptr);
                let targeter = ChartTargeter::new(
                    feed_ptr,
                    initial.last_manual.clone(),
                    client.retargeter(),
                    fetcher.clone(),
                );
                targeter.set_session_override(initial.is_session_override);
                targeter.set_last_manual(&initial.last_manual.0, &initial.last_manual.1);
                if initial.mode == crate::chart_target::ChartMode::Manual {
                    let _ = targeter.request_manual(&initial.symbol, &initial.timeframe);
                    if initial.is_session_override {
                        targeter.set_session_override(true);
                    }
                }
                unsafe {
                    use cxx_qt::CxxQtType;
                    let ffi_feed = feed_ptr as *mut crate::bridge::bar_feed::ffi::BarFeed;
                    let mut pin = std::pin::Pin::new_unchecked(&mut *ffi_feed);
                    pin.as_mut().rust_mut().on_completed =
                        Some(std::sync::Arc::new(move |generation| {
                            fetcher.request(generation)
                        }));
                }
                let chart_ctx =
                    unsafe { chart_bridge::find_window_chart_context(engine.as_mut().unwrap()) };
                let chart_ctx_addr = chart_ctx as usize;
                if chart_ctx_addr != 0 {
                    unsafe {
                        let feed_ffi = feed_ptr as *mut crate::bridge::bar_feed::ffi::BarFeed;
                        chart_context::bind_feed(
                            chart_ctx as *mut chart_context::ffi::ChartContext,
                            feed_ffi,
                        );
                        chart_context::set_configured(
                            chart_ctx as *mut chart_context::ffi::ChartContext,
                            &cfg.symbol,
                            &cfg.timeframe,
                        );
                        chart_context::bind_targeter(
                            chart_ctx as *mut chart_context::ffi::ChartContext,
                            targeter.clone(),
                        );
                        chart_context::set_last_manual(
                            chart_ctx as *mut chart_context::ffi::ChartContext,
                            &initial.last_manual.0,
                            &initial.last_manual.1,
                        );
                        if let Some(ref dep_id) = initial.followed_deployment_id {
                            chart_context::set_followed_deployment(
                                chart_ctx as *mut chart_context::ffi::ChartContext,
                                dep_id,
                            );
                        }
                    }
                }
                wire_execution(
                    engine.as_mut().unwrap(),
                    exec_handle,
                    targeter.clone(),
                    feed_ptr,
                    chart_ctx_addr,
                    (cfg.symbol.clone(), cfg.timeframe.clone()),
                    initial.clone(),
                );
                chart_targeter = Some(targeter);

                crate::chart_target::install_bar_drainer(&sink, feed_ptr);

                let shared = client.shared();
                let feed_addr = feed_ptr as usize;
                let engine_addr = {
                    let pin = engine.as_mut().unwrap();
                    unsafe { std::ptr::from_mut(std::pin::Pin::get_unchecked_mut(pin)) as usize }
                };
                client.set_listener(Arc::new(move || {
                    let feed = feed_addr as *mut chart_bridge::BarFeed;
                    let state = shared.connection_state();
                    let state_str = match state {
                        ConnectionState::Connecting => "connecting",
                        ConnectionState::Live => "live",
                        ConnectionState::Reconnecting { .. } => "retrying",
                        ConnectionState::Unavailable => "unavailable",
                    };
                    let err = shared.last_error();
                    let cnt = shared.counters();
                    unsafe {
                        chart_bridge::post_feed_stream_state(
                            feed,
                            state_str,
                            &err,
                            cnt.applied as i64,
                            cnt.dropped as i64,
                            cnt.gaps_closed as i64,
                            cnt.resnapshots as i64,
                            cnt.rest_calls as i64,
                        );
                        let ops_state = match state {
                            ConnectionState::Connecting => "connecting",
                            ConnectionState::Live => "connected",
                            ConnectionState::Reconnecting { .. } => "reconnecting",
                            ConnectionState::Unavailable => "disconnected",
                        };
                        let age = 0.0;
                        let engine_mut = engine_addr as *mut cxx_qt_lib::QQmlApplicationEngine;
                        let status = chart_bridge::find_window_ops_status(
                            std::pin::Pin::new_unchecked(&mut *engine_mut),
                        );
                        if status != 0 {
                            chart_bridge::ops_status_set_stream(status, ops_state, age);
                        }
                    }
                }));

                unsafe {
                    chart_bridge::feed_setup_and_load(
                        feed_ptr,
                        &cfg.api_base,
                        &initial.symbol,
                        &initial.timeframe,
                    );
                }

                stream_client = Some(client);
            }
        }
    }

    SliceContext {
        stream_client,
        health_poller,
        targeter: chart_targeter,
        engine,
        feed_ptr,
    }
}

/// Opens the window whatever the API's state: configuration failures are shown
/// in the scene, not printed to a terminal the user is not reading.
pub fn run_slice(config: Result<Config, ConfigError>) -> i32 {
    run_slice_opts(config, false, 0, 0, 0, 0, 2000, 500_000, "historical")
}

#[allow(clippy::too_many_arguments)]
pub fn run_slice_opts(
    config: Result<Config, ConfigError>,
    bench: bool,
    auto_close_ms: u64,
    execution_rows: i32,
    markers: i32,
    overlays: i32,
    visible_buckets: i32,
    bar_count: i32,
    scenario: &str,
) -> i32 {
    let mut ctx = setup_slice(&config);
    if bench {
        bridge::ffi::run_frame_bench(
            ctx.engine.as_mut().unwrap(),
            visible_buckets,
            bar_count,
            auto_close_ms as i32,
            execution_rows,
            markers,
            overlays,
            &cxx_qt_lib::QString::from(scenario),
        );
    } else if auto_close_ms > 0 {
        chart_bridge::setup_window_auto_close(ctx.engine.as_mut().unwrap(), auto_close_ms as i32);
    }

    chart_bridge::exec_application()
}
