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

/// Binds the execution store to the window's models and makes the chart follow the
/// selected deployment.
fn wire_execution(
    engine: std::pin::Pin<&mut QQmlApplicationEngine>,
    handle: ExecutionHandle,
    targeter: std::sync::Arc<ChartTargeter>,
    feed_ptr: *mut chart_bridge::BarFeed,
    chart_ctx_addr: usize,
    configured: (String, String),
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
        rust.on_target = Some(std::sync::Arc::new(move |dep| {
            if chart_ctx_addr != 0 {
                chart_context::notify_target(
                    chart_ctx_addr as *mut chart_context::ffi::ChartContext,
                    dep.as_ref().map(|(_id, name, sym, tf)| {
                        (name.as_str(), sym.as_str(), tf.as_str(), true)
                    }),
                    (&configured_sym, &configured_tf),
                );
            }
            let retargeted = targeter.select(dep);
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
        rust.on_rows = Some(std::sync::Arc::new(move |dec, fills| {
            chart_bridge::feed_set_execution_rows(
                feed_addr as *mut chart_bridge::BarFeed,
                &dec,
                &fills,
            )
        }));
        rust.dirty.clone()
    };
    let addr = models as usize;
    handle.set_listener(std::sync::Arc::new(move || {
        dirty.store(true, std::sync::atomic::Ordering::Release);
        unsafe {
            chart_bridge::post_execution_models_sync(addr as *mut chart_bridge::ExecutionModels)
        };
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
                let tf_ms = parse_timeframe_ms(&cfg.timeframe);
                unsafe {
                    chart_bridge::feed_set_symbol(feed_ptr, &cfg.symbol);
                    chart_bridge::feed_set_timeframe(feed_ptr, &cfg.timeframe, tf_ms);
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
                    (cfg.symbol.clone(), cfg.timeframe.clone()),
                    client.retargeter(),
                    fetcher.clone(),
                );
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
                    }
                }
                wire_execution(
                    engine.as_mut().unwrap(),
                    exec_handle,
                    targeter.clone(),
                    feed_ptr,
                    chart_ctx_addr,
                    (cfg.symbol.clone(), cfg.timeframe.clone()),
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
                        &cfg.symbol,
                        &cfg.timeframe,
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
    run_slice_opts(config, false, 0, 0, 0, 0)
}

pub fn run_slice_opts(
    config: Result<Config, ConfigError>,
    bench: bool,
    auto_close_ms: u64,
    execution_rows: i32,
    markers: i32,
    overlays: i32,
) -> i32 {
    let mut ctx = setup_slice(&config);
    if bench {
        bridge::ffi::run_frame_bench(
            ctx.engine.as_mut().unwrap(),
            2000,
            500_000,
            auto_close_ms as i32,
            execution_rows,
            markers,
            overlays,
        );
    } else if auto_close_ms > 0 {
        chart_bridge::setup_window_auto_close(ctx.engine.as_mut().unwrap(), auto_close_ms as i32);
    }

    chart_bridge::exec_application()
}
