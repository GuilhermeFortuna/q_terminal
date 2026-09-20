use std::sync::Arc;

use cxx_qt_lib::{QQmlApplicationEngine, QString, QUrl};

use crate::bridge;
use crate::bridge::bar_feed::parse_timeframe_ms;
use crate::chart_bridge;
use crate::config::{Config, ConfigError};
use crate::ops_session;
use crate::stream::client::{ConnectionState, StreamClient};
use crate::stream::sink::BarSink;

/// Context holding the loaded slice resources.
pub struct SliceContext {
    pub engine: cxx::UniquePtr<cxx_qt_lib::QQmlApplicationEngine>,
    pub feed_ptr: *mut chart_bridge::BarFeed,
    pub stream_client: Option<StreamClient>,
    pub health_poller: Option<std::sync::Arc<crate::execution::health::HealthPoller>>,
}

impl Drop for SliceContext {
    fn drop(&mut self) {
        if let Some(poller) = &self.health_poller {
            poller.stop();
        }
        let _ = self.stream_client.take();
    }
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
                let (_exec_handle, client_opt, poller) =
                    ops_session::wire_ops_session(&mut engine, cfg, sink.clone());
                let client = client_opt.expect("execution stream client");

                let sink_drainer = {
                    let sink = sink.clone();
                    let feed_addr = feed_ptr as usize;
                    Arc::new(move || {
                        let feed = feed_addr as *mut chart_bridge::BarFeed;
                        for delivery in sink.drain() {
                            match delivery {
                                crate::stream::sink::BarDelivery::Completed(cols) => {
                                    for i in 0..cols.time.len() {
                                        unsafe {
                                            chart_bridge::post_feed_completed_bar(
                                                feed,
                                                cols.time[i],
                                                cols.open[i],
                                                cols.high[i],
                                                cols.low[i],
                                                cols.close[i],
                                            );
                                        }
                                    }
                                }
                                crate::stream::sink::BarDelivery::Forming(cols) => {
                                    if let Some(&t) = cols.time.first() {
                                        unsafe {
                                            chart_bridge::post_feed_forming_bar(
                                                feed,
                                                t,
                                                cols.open[0],
                                                cols.high[0],
                                                cols.low[0],
                                                cols.close[0],
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    })
                };
                sink.set_listener(sink_drainer);

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
                health_poller = Some(poller);
            }
        }
    }

    SliceContext {
        engine,
        feed_ptr,
        stream_client,
        health_poller,
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
