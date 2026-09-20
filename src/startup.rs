use std::sync::Arc;

use cxx_qt_lib::{QQmlApplicationEngine, QString, QUrl};

use crate::bridge;
use crate::bridge::bar_feed::parse_timeframe_ms;
use crate::chart_bridge;
use crate::chart_target::ChartTargeter;
use crate::config::{Config, ConfigError};
use crate::execution::store::ExecutionHandle;
use crate::stream::client::{ConnectionState, Sinks, StreamClient};
use crate::stream::sink::BarSink;

/// Context holding the loaded slice resources.
pub struct SliceContext {
    pub engine: cxx::UniquePtr<cxx_qt_lib::QQmlApplicationEngine>,
    pub feed_ptr: *mut chart_bridge::BarFeed,
    pub stream_client: Option<StreamClient>,
    pub targeter: Option<std::sync::Arc<ChartTargeter>>,
}

/// Binds the execution store to the window's models and makes the chart follow the
/// selected deployment.
fn wire_execution(
    engine: std::pin::Pin<&mut QQmlApplicationEngine>,
    handle: ExecutionHandle,
    targeter: std::sync::Arc<ChartTargeter>,
) {
    let models = unsafe { chart_bridge::find_window_execution_models(engine) };
    if models.is_null() {
        return;
    }
    use cxx_qt::CxxQtType;
    let ffi_models = models as *mut crate::bridge::execution_models::ffi::ExecutionModels;
    let dirty = unsafe {
        let mut pin = std::pin::Pin::new_unchecked(&mut *ffi_models);
        let mut rust = pin.as_mut().rust_mut();
        rust.bind_handle(handle.clone());
        rust.on_target = Some(std::sync::Arc::new(move |dep| {
            targeter.select(dep);
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
                let exec_handle = ExecutionHandle::new();
                let client = StreamClient::start_with(
                    cfg.clone(),
                    Sinks {
                        bars: sink.clone(),
                        execution: Some(exec_handle.clone()),
                    },
                );
                let targeter = ChartTargeter::new(
                    feed_ptr,
                    (cfg.symbol.clone(), cfg.timeframe.clone()),
                    client.retargeter(),
                );
                wire_execution(engine.as_mut().unwrap(), exec_handle, targeter.clone());
                chart_targeter = Some(targeter);

                crate::chart_target::install_bar_drainer(&sink, feed_ptr);

                // Set up reactive client state listener without timer polling
                let client_listener = {
                    let shared = client.shared();
                    let feed_addr = feed_ptr as usize;
                    Arc::new(move || {
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
                        }
                    })
                };
                client.set_listener(client_listener);

                // Start historical catalog load
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
        engine,
        feed_ptr,
        stream_client,
        targeter: chart_targeter,
    }
}

/// Opens the window whatever the API's state: configuration failures are shown
/// in the scene, not printed to a terminal the user is not reading.
pub fn run_slice(config: Result<Config, ConfigError>) -> i32 {
    run_slice_opts(config, false, 0, 0)
}

pub fn run_slice_opts(
    config: Result<Config, ConfigError>,
    bench: bool,
    auto_close_ms: u64,
    execution_rows: i32,
) -> i32 {
    let mut ctx = setup_slice(&config);
    if bench {
        bridge::ffi::run_frame_bench(
            ctx.engine.as_mut().unwrap(),
            2000,
            500_000,
            auto_close_ms as i32,
            execution_rows,
        );
    } else if auto_close_ms > 0 {
        chart_bridge::setup_window_auto_close(ctx.engine.as_mut().unwrap(), auto_close_ms as i32);
    }

    chart_bridge::exec_application()
}
