#![allow(clippy::await_holding_lock)]

#[rustfmt::skip]
#[path = "../contracts/stream.rs"]
pub mod contracts_stream;

#[path = "../src/bridge.rs"]
pub mod bridge;
#[path = "../src/chart_bridge.rs"]
pub mod chart_bridge;
#[path = "../src/config.rs"]
pub mod config;
#[path = "../src/execution/mod.rs"]
pub mod execution;
#[path = "../src/history/mod.rs"]
pub mod history;
#[path = "../src/startup.rs"]
pub mod startup;
#[path = "../src/stream/mod.rs"]
pub mod stream;

use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use arrow::array::{ArrayRef, Float64Array, Int64Array, TimestampMicrosecondArray};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;
use config::Config;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;
use q_io::DigestAlgorithm;
use stream::client::ConnectionState;
use stream::fake_server::FakeServer;
use stream::topic_state::BarColumns;

fn write_test_parquet(path: &Path, times: &[i64], prices: &[f64]) {
    let schema = Arc::new(Schema::new(vec![
        Field::new(
            "time",
            DataType::Timestamp(TimeUnit::Microsecond, None),
            false,
        ),
        Field::new("open", DataType::Float64, false),
        Field::new("high", DataType::Float64, false),
        Field::new("low", DataType::Float64, false),
        Field::new("close", DataType::Float64, false),
        Field::new("tick_volume", DataType::Int64, false),
        Field::new("spread", DataType::Int64, false),
        Field::new("real_volume", DataType::Int64, false),
    ]));
    let rows = times.len();
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(TimestampMicrosecondArray::from(times.to_vec())) as ArrayRef,
            Arc::new(Float64Array::from(prices.to_vec())),
            Arc::new(Float64Array::from(
                prices.iter().map(|p| p + 1.0).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                prices.iter().map(|p| p - 1.0).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(prices.to_vec())),
            Arc::new(Int64Array::from(vec![1i64; rows])),
            Arc::new(Int64Array::from(vec![1i64; rows])),
            Arc::new(Int64Array::from(vec![1i64; rows])),
        ],
    )
    .unwrap();
    let file = File::create(path).unwrap();
    let props = WriterProperties::builder().build();
    let mut writer = ArrowWriter::try_new(file, schema, Some(props)).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
}

fn dummy_bar(t: i64, close: f64) -> BarColumns {
    BarColumns::single(t, close - 0.5, close + 0.5, close - 1.0, close, 100.0)
}

#[tokio::test(flavor = "current_thread")]
async fn test_slice_end_to_end_lake_to_live_vertices_and_no_polling() {
    let _guard = chart_bridge::QT_TEST_MUTEX.lock().unwrap();
    chart_bridge::ensure_application();
    chart_bridge::register_chart_types();

    // 1. Prepare fixture lake
    let temp = tempfile::tempdir().unwrap();
    let rel = "bars/part-0.parquet";
    let abs = temp.path().join(rel);
    std::fs::create_dir_all(abs.parent().unwrap()).unwrap();

    // 5 history bars: 1_000_000, 1_060_000, 1_120_000, 1_180_000, 1_240_000
    let history_times = vec![1_000_000, 1_060_000, 1_120_000, 1_180_000, 1_240_000];
    let history_prices = vec![10.0, 10.5, 11.0, 11.5, 12.0];
    write_test_parquet(&abs, &history_times, &history_prices);

    let digest = q_io::digest_file(&abs, DigestAlgorithm::Sha256).unwrap();
    let size = abs.metadata().unwrap().len();

    let manifest = format!(
        "{{\"dataset_id\":\"ds-1\",\"version\":1,\"published_at\":\"2026-01-01T00:00:00Z\",\"state\":\"published\",\"checksum_algorithm\":\"sha256\",\"row_count\":5,\"files\":[{{\"path\":\"{rel}\",\"size_bytes\":{size},\"checksum\":\"{digest}\"}}],\"subject\":{{}},\"arrow_schema\":{{}},\"time_range\":{{}}}}"
    );
    let catalog = format!(
        "{{\"root\":\"{}\",\"datasets\":[{}]}}",
        temp.path().to_str().unwrap(),
        manifest
    );

    // 2. Prepare fake server with catalog and initial stream snapshots
    let server = FakeServer::start().await;
    server.set_catalog(&catalog).await;

    // Snapshot completed bar at 1_240_000 (overlapping last history bar to test seam trimming)
    server
        .set_snapshot("bars.completed", 100, dummy_bar(1_240_000, 12.0))
        .await;
    server
        .set_snapshot("bars.forming", 100, dummy_bar(1_300_000, 12.2))
        .await;

    let config = Config {
        api_base: server.api_base(),
        symbol: "PETR4".to_string(),
        timeframe: "1m".to_string(),
    };

    // 3. Start slice
    let ctx = startup::setup_slice(&Ok(config));
    assert!(!ctx.feed_ptr.is_null());
    let feed = ctx.feed_ptr;

    // Wait for history load to complete and client to become Live
    let mut ready = false;
    for _i in 0..100 {
        chart_bridge::process_events();
        tokio::time::sleep(Duration::from_millis(50)).await;
        chart_bridge::process_events();
        unsafe {
            let src = chart_bridge::feed_history_source(feed);
            let count = chart_bridge::feed_bar_count(feed);
            let loading = chart_bridge::feed_history_loading(feed);
            let err = chart_bridge::feed_history_error(feed);
            if _i % 10 == 0 || _i > 90 {
                eprintln!(
                    "poll {} src: {:?}, count: {}, loading: {}, err: {:?}",
                    _i, src, count, loading, err
                );
            }
            if src == "lake" && count >= 5 {
                ready = true;
                break;
            }
        }
    }
    unsafe {
        let err = chart_bridge::feed_history_error(feed);
        let src = chart_bridge::feed_history_source(feed);
        assert!(
            ready,
            "history load should succeed from lake, but src={src}, err={err}"
        );
    }

    // Check rest calls once live
    let client = ctx.stream_client.as_ref().expect("stream client active");
    assert_eq!(client.connection_state(), ConnectionState::Live);

    let rest_calls_at_live = unsafe { chart_bridge::feed_rest_calls(feed) };
    assert!(
        rest_calls_at_live > 0,
        "must have made catalog and snapshot REST calls"
    );
    let server_rest_calls_at_live = server.rest_calls();

    // 4. Send live completed bar and forming bar over the stream
    server
        .send_bar(
            "bars.completed",
            101,
            "PETR4",
            "1m",
            dummy_bar(1_300_000, 12.5),
        )
        .await;
    server
        .send_bar(
            "bars.forming",
            101,
            "PETR4",
            "1m",
            dummy_bar(1_360_000, 12.8),
        )
        .await;

    // Wait for deliveries to be processed
    for _ in 0..60 {
        chart_bridge::process_events();
        tokio::time::sleep(Duration::from_millis(50)).await;
        chart_bridge::process_events();
        unsafe {
            if chart_bridge::feed_bar_count(feed) >= 6 {
                break;
            }
        }
    }
    chart_bridge::process_events();

    // 5. Assertions:
    // a) History source is lake
    unsafe {
        assert_eq!(chart_bridge::feed_history_source(feed), "lake");
        assert_eq!(chart_bridge::feed_bar_count(feed), 6);

        // b) Timestamps are strictly ascending with no duplicate time across the seam
        let len = chart_bridge::feed_bar_times_len(feed);
        let mut times = Vec::new();
        for i in 0..len {
            times.push(chart_bridge::feed_bar_time_at(feed, i));
        }
        assert_eq!(
            len, 6,
            "expected 4 history + 1 snapshot + 1 live completed = 6 completed bars"
        );

        // Must be [1_000_000, 1_060_000, 1_120_000, 1_180_000, 1_240_000, 1_300_000]
        assert_eq!(
            times,
            vec![1_000_000, 1_060_000, 1_120_000, 1_180_000, 1_240_000, 1_300_000]
        );

        // Assert strictly ascending (no duplicates)
        for pair in times.windows(2) {
            assert!(
                pair[0] < pair[1],
                "times must be strictly ascending, got {} >= {}",
                pair[0],
                pair[1]
            );
        }

        // c) Assert drawn vertices are history followed by live bars
        // Viewport: 0 to 7 bars, low 5.0, high 20.0 on a 400x200 surface
        chart_bridge::feed_rebuild_geometry(feed, 0, 7, 5.0, 20.0, 400.0, 200.0);
        let vlen = chart_bridge::feed_vertex_len(feed);
        // 6 completed buckets + 1 forming bucket = 7 buckets * 12 vertices = 84 vertices
        assert_eq!(
            vlen,
            7 * 12,
            "expected 84 vertices for 6 completed + 1 forming bucket"
        );

        // The first 6 * 12 = 72 vertices must be completed bars (forming == 0.0)
        for i in 0..(6 * 12) {
            let v = chart_bridge::feed_vertex_at(feed, i);
            assert!(
                v.forming < 0.1,
                "vertex {} should belong to completed bar",
                i
            );
        }

        // The last 12 vertices must be the live forming bar (forming >= 0.5)
        for i in (6 * 12)..(7 * 12) {
            let v = chart_bridge::feed_vertex_at(feed, i);
            assert!(
                v.forming >= 0.5,
                "vertex {} should belong to forming bar",
                i
            );
        }

        // Vertex X positions must strictly advance from history to live bars
        let v_first = chart_bridge::feed_vertex_at(feed, 0);
        let v_mid = chart_bridge::feed_vertex_at(feed, 4 * 12);
        let v_last = chart_bridge::feed_vertex_at(feed, 6 * 12);
        assert!(
            v_first.x < v_mid.x && v_mid.x < v_last.x,
            "vertices must advance from history (x={}) to live (x={}) to forming (x={})",
            v_first.x,
            v_mid.x,
            v_last.x
        );

        // d) Assert rest_calls stopped growing once live
        let rest_calls_after = chart_bridge::feed_rest_calls(feed);
        assert_eq!(
            rest_calls_after, rest_calls_at_live,
            "feed rest_calls must not grow once live (no polling)"
        );
        assert_eq!(
            server.rest_calls(),
            server_rest_calls_at_live,
            "server received REST calls must not grow once live (no polling)"
        );
    }

    server.shutdown().await;
}
