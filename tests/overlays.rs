//! Q-049 criteria 3 and 4: markers appear from the stream's store with no request, and
//! overlay values are the chart route's, fetched once per completed bar.
#![allow(clippy::await_holding_lock)]

#[rustfmt::skip]
#[path = "../contracts/stream.rs"]
pub mod contracts_stream;

#[path = "../src/bridge.rs"]
pub mod bridge;
#[path = "../src/chart_bridge.rs"]
pub mod chart_bridge;
#[path = "../src/chart_target.rs"]
pub mod chart_target;
#[path = "../src/config.rs"]
pub mod config;
#[path = "../src/execution/mod.rs"]
pub mod execution;
#[path = "../src/history/mod.rs"]
pub mod history;
#[path = "../src/ops_session.rs"]
pub mod ops_session;
#[allow(unused_imports)]
pub use bridge::execution_controls;
#[allow(unused_imports)]
pub use bridge::execution_models;
#[path = "../src/startup.rs"]
pub mod startup;
#[path = "../src/stream/mod.rs"]
pub mod stream;

use std::time::Duration;

use bridge::bar_feed::{make_bar_columns, BarDelivery, BarFeedRust};
use config::Config;
use cxx_qt::CxxQtType;
use execution::overlays::{parse_chart, Pane};
use execution::store::ExecutionHandle;
use serde_json::json;
use stream::fake_exec as fx;
use stream::fake_server::FakeServer;
use stream::topic_state::BarColumns;

const DEP: &str = "11111111-1111-1111-1111-111111111111";

fn chart_body() -> String {
    json!({
        "symbol": "PETR4", "timeframe": "M1", "window_bound_bars": 20,
        "bars": [
            {"timestamp": "2026-09-18T10:00:00Z", "open": 1.0, "high": 2.0, "low": 0.5, "close": 1.5, "volume": 1},
            {"timestamp": "2026-09-18T10:01:00Z", "open": 1.5, "high": 2.5, "low": 1.0, "close": 2.0, "volume": 1},
            {"timestamp": "2026-09-18T10:02:00Z", "open": 2.0, "high": 3.0, "low": 1.5, "close": 2.5, "volume": 1}
        ],
        "indicators": [
            {"key": "ema_20", "label": "EMA 20", "pane": "price", "color": "#2962ff",
             "values": [null, 1.75, 2.25]},
            {"key": "rsi_14", "label": "RSI 14", "pane": "oscillator", "color": null,
             "values": [null, 40.0, 60.0]}
        ]
    })
    .to_string()
}

#[test]
fn overlay_values_equal_the_route_response() {
    let series = parse_chart(&chart_body()).unwrap();
    assert_eq!(series.len(), 2);
    assert_eq!(series[0].key, "ema_20");
    assert_eq!(series[0].pane, Pane::Price);
    assert_eq!(series[0].rgba, 0x2962ffff);
    let values: Vec<Option<f64>> = series[0].points.iter().map(|p| p.1).collect();
    assert_eq!(values, vec![None, Some(1.75), Some(2.25)]);
    assert_eq!(series[1].pane, Pane::Oscillator);
    let times: Vec<i64> = series[1].points.iter().map(|p| p.0).collect();
    assert_eq!(
        times,
        vec![1_789_725_600_000, 1_789_725_660_000, 1_789_725_720_000]
    );
    assert!(parse_chart("{}").is_err());
}

#[test]
fn overlays_for_a_previous_target_are_dropped() {
    let mut feed = BarFeedRust::new("PETR4", "M1");
    feed.reset_for_target("VALE3", "M1", 2);
    let series = parse_chart(&chart_body()).unwrap();
    assert!(!feed.set_overlays(1, series.clone()));
    assert!(feed.overlay_series().is_empty());
    assert!(feed.set_overlays(2, series));
    assert_eq!(feed.overlay_series().len(), 2);
}

async fn until(what: &str, mut cond: impl FnMut() -> bool) {
    for _ in 0..200 {
        chart_bridge::process_events();
        if cond() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("timed out waiting for {what}");
}

/// A completed bar triggers exactly one chart-route request for the selected deployment,
/// and the overlays are the route's values.
#[tokio::test(flavor = "current_thread")]
async fn a_completed_bar_triggers_exactly_one_chart_request() {
    let _guard = chart_bridge::QT_TEST_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    chart_bridge::ensure_application();
    chart_bridge::register_chart_types();

    let server = FakeServer::start().await;
    server.set_chart_json(&chart_body()).await;
    server
        .send_bar(
            "bars.completed",
            100,
            "PETR4",
            "1m",
            BarColumns::single(1_000, 1.0, 2.0, 0.5, 1.5, 1.0),
        )
        .await;
    let config = Config {
        api_base: server.api_base(),
        symbol: "PETR4".into(),
        timeframe: "1m".into(),
        operator: "operator".into(),
    };
    let ctx = startup::setup_slice(&Ok(config));
    let feed = ctx.feed_ptr;
    let targeter = ctx.targeter.as_ref().unwrap().clone();

    until(
        "first bar",
        || unsafe { chart_bridge::feed_bar_count(feed) } > 0,
    )
    .await;
    // Selecting the deployment (same symbol and timeframe) fetches its overlays once.
    targeter.select(Some((DEP.into(), "PETR4".into(), "1m".into())));
    until("overlay fetch", || server.chart_calls() == 1).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(server.chart_calls(), 1, "selection costs one request");

    // Wait for the feed to hold the route's overlays.
    let ffi_feed = feed as *mut bridge::bar_feed::ffi::BarFeed;
    until("overlays applied", || unsafe {
        (*ffi_feed).rust().overlay_series().len() == 2
    })
    .await;
    let held = unsafe { (*ffi_feed).rust().overlay_series().to_vec() };
    assert_eq!(held, parse_chart(&chart_body()).unwrap());

    // One completed bar, one request; a forming bar and a duplicate snapshot none.
    let before = server.chart_calls();
    server
        .send_bar(
            "bars.completed",
            101,
            "PETR4",
            "1m",
            BarColumns::single(1_060, 1.5, 2.5, 1.0, 2.0, 1.0),
        )
        .await;
    until("request for completed bar", || {
        server.chart_calls() > before
    })
    .await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(server.chart_calls(), before + 1);
}

/// A decision and a fill delivered through the store reach the chart with no REST request.
#[test]
fn store_deliveries_appear_as_markers_without_a_request() {
    let _guard = chart_bridge::QT_TEST_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    chart_bridge::ensure_application();
    chart_bridge::register_chart_types();

    let feed = unsafe { chart_bridge::make_test_feed() };
    let ffi_feed = feed as *mut bridge::bar_feed::ffi::BarFeed;
    unsafe {
        let mut pin = std::pin::Pin::new_unchecked(&mut *ffi_feed);
        pin.as_mut().rust_mut().reset_for_target("PETR4", "M1", 0);
        pin.as_ref().rust().history_controller().open_gate();
    }
    // Ten bars from 10:00, in microseconds like the stream's.
    let t0: i64 = 1_789_725_600_000_000;
    for i in 0..10 {
        let t = t0 + i * 60_000_000;
        let c = make_bar_columns(t, 10.0, 11.0, 9.0, 10.5);
        unsafe {
            std::pin::Pin::new_unchecked(&mut *ffi_feed)
                .rust_mut()
                .apply_delivery(BarDelivery::Completed(c))
        };
    }

    let models = chart_bridge::make_test_execution_models();
    let handle = ExecutionHandle::new();
    unsafe {
        let mut pin = std::pin::Pin::new_unchecked(
            &mut *(models as *mut bridge::execution_models::ffi::ExecutionModels),
        );
        let mut rust = pin.as_mut().rust_mut();
        rust.bind_handle(handle.clone());
        let addr = feed as usize;
        rust.on_rows = Some(std::sync::Arc::new(move |d, f| {
            chart_bridge::feed_set_execution_rows(addr as *mut chart_bridge::BarFeed, &d, &f)
        }));
    }

    let count = |first, last| unsafe {
        chart_bridge::feed_rebuild_overlays(feed, first, last, 8.0, 12.0, 800.0, 400.0);
        chart_bridge::feed_marker_count(feed)
    };
    handle.mutate(|s| {
        s.apply(
            stream::execution::decode_execution_entry(
                "deployments",
                &fx::deployment(DEP, "running"),
            )
            .unwrap(),
        );
    });
    unsafe { chart_bridge::execution_models_sync(models) };
    assert_eq!(count(0, 10), 0);

    let mut d = fx::decision("d1", DEP);
    d["bar_close_time"] = json!("2026-09-18T10:03:00Z");
    let mut fl = fx::fill("f1", DEP, "1.0", true);
    fl["filled_at"] = json!("2026-09-18T10:05:30Z");
    handle.mutate(|s| {
        s.apply(stream::execution::decode_execution_entry("decisions", &d).unwrap());
        s.apply(stream::execution::decode_execution_entry("fills", &fl).unwrap());
    });
    unsafe { chart_bridge::execution_models_select_deployment(models, DEP) };
    unsafe { chart_bridge::execution_models_sync(models) };
    assert_eq!(count(0, 10), 2, "one decision marker and one fill marker");
    unsafe { chart_bridge::delete_test_execution_models(models) };
}
