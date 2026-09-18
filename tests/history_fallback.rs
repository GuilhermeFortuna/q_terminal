#[path = "../src/bridge.rs"]
#[allow(dead_code)]
mod bridge;
#[path = "../src/config.rs"]
#[allow(dead_code)]
mod config;
#[path = "../src/history/mod.rs"]
mod history;

use config::Config;
use history::{load, ApiClient, Source, VerifiedSet};
use q_io::DigestAlgorithm;
use std::fs::File;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

static NEXT_PORT: AtomicUsize = AtomicUsize::new(20_000);

struct FakeApi {
    catalog_status: u16,
    catalog_body: String,
    ohlcv_body: String,
    ohlcv_calls: Arc<AtomicUsize>,
}

async fn spawn_fake_api(fake: FakeApi) -> String {
    let listener = tokio::net::TcpListener::bind(format!(
        "127.0.0.1:{}",
        NEXT_PORT.fetch_add(1, Ordering::SeqCst)
    ))
    .await
    .unwrap();
    let addr = listener.local_addr().unwrap();
    let ohlcv_calls = fake.ohlcv_calls.clone();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 16_384];
            let read = socket.read(&mut buf).await.unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..read]);
            let first_line = request.lines().next().unwrap_or("");

            let (status, body) = if first_line.contains("/api/v1/catalog/datasets") {
                (fake.catalog_status, fake.catalog_body.clone())
            } else if first_line.contains("/api/v1/market/ohlcv/") {
                ohlcv_calls.fetch_add(1, Ordering::SeqCst);
                (200u16, fake.ohlcv_body.clone())
            } else {
                (404u16, "{}".to_string())
            };

            let response = format!(
                "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });

    format!("http://{}", addr)
}

fn ohlcv_json(count: usize) -> String {
    let mut bars = Vec::new();
    for index in 0..count {
        bars.push(format!(
            "{{\"timestamp\":\"2026-01-01T12:{:02}:00+00:00\",\"open\":1.0,\"high\":2.0,\"low\":0.5,\"close\":1.5,\"volume\":10}}",
            index % 60
        ));
    }
    format!("[{}]", bars.join(","))
}

fn test_config(base: &str) -> Config {
    Config {
        api_base: base.to_string(),
        symbol: "PETR4".to_string(),
        timeframe: "1m".to_string(),
    }
}

fn catalog_json(root: &str, datasets: &str) -> String {
    format!("{{\"root\":\"{root}\",\"datasets\":{datasets}}}")
}

fn published_manifest(root_rel: &str, digest: &str, size: u64) -> String {
    format!(
        "{{\"dataset_id\":\"ds-1\",\"version\":1,\"published_at\":\"2026-01-01T00:00:00Z\",\"state\":\"published\",\"checksum_algorithm\":\"sha256\",\"row_count\":10,\"files\":[{{\"path\":\"{root_rel}\",\"size_bytes\":{size},\"checksum\":\"{digest}\"}}],\"subject\":{{}},\"arrow_schema\":{{}},\"time_range\":{{}}}}"
    )
}

#[tokio::test]
async fn catalog_503_falls_back_to_api() {
    let calls = Arc::new(AtomicUsize::new(0));
    let base = spawn_fake_api(FakeApi {
        catalog_status: 503,
        catalog_body: "{}".to_string(),
        ohlcv_body: ohlcv_json(100),
        ohlcv_calls: calls.clone(),
    })
    .await;
    let api = ApiClient::new(&base);
    let cache = Mutex::new(VerifiedSet::default());
    let loaded = load(&api, &test_config(&base), 100, &cache).await;
    assert_eq!(loaded.source, Source::Api);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn empty_catalog_falls_back_to_api() {
    let calls = Arc::new(AtomicUsize::new(0));
    let base = spawn_fake_api(FakeApi {
        catalog_status: 200,
        catalog_body: catalog_json("/lake", "[]"),
        ohlcv_body: ohlcv_json(50),
        ohlcv_calls: calls.clone(),
    })
    .await;
    let api = ApiClient::new(&base);
    let loaded = load(
        &api,
        &test_config(&base),
        50,
        &Mutex::new(VerifiedSet::default()),
    )
    .await;
    assert_eq!(loaded.source, Source::Api);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn missing_file_falls_back_to_api() {
    let calls = Arc::new(AtomicUsize::new(0));
    let base = spawn_fake_api(FakeApi {
        catalog_status: 200,
        catalog_body: catalog_json(
            "/lake",
            &format!("[{}]", published_manifest("missing.parquet", "abc", 10)),
        ),
        ohlcv_body: ohlcv_json(25),
        ohlcv_calls: calls.clone(),
    })
    .await;
    let api = ApiClient::new(&base);
    let loaded = load(
        &api,
        &test_config(&base),
        25,
        &Mutex::new(VerifiedSet::default()),
    )
    .await;
    assert_eq!(loaded.source, Source::Api);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn digest_mismatch_falls_back_to_api() {
    let temp = tempfile::tempdir().unwrap();
    let rel = "bars/part-0.parquet";
    let abs = temp.path().join(rel);
    std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
    std::fs::write(&abs, b"bad").unwrap();

    let calls = Arc::new(AtomicUsize::new(0));
    let base = spawn_fake_api(FakeApi {
        catalog_status: 200,
        catalog_body: catalog_json(
            temp.path().to_str().unwrap(),
            &format!(
                "[{}]",
                published_manifest(rel, "deadbeef", abs.metadata().unwrap().len())
            ),
        ),
        ohlcv_body: ohlcv_json(30),
        ohlcv_calls: calls.clone(),
    })
    .await;
    let api = ApiClient::new(&base);
    let loaded = load(
        &api,
        &test_config(&base),
        30,
        &Mutex::new(VerifiedSet::default()),
    )
    .await;
    assert_eq!(loaded.source, Source::Api);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn unsupported_digest_falls_back_to_api() {
    let calls = Arc::new(AtomicUsize::new(0));
    let manifest =
        published_manifest("bars/part-0.parquet", "abc", 10).replace("\"sha256\"", "\"blake3\"");
    let base = spawn_fake_api(FakeApi {
        catalog_status: 200,
        catalog_body: catalog_json("/lake", &format!("[{manifest}]")),
        ohlcv_body: ohlcv_json(20),
        ohlcv_calls: calls.clone(),
    })
    .await;
    let api = ApiClient::new(&base);
    let loaded = load(
        &api,
        &test_config(&base),
        20,
        &Mutex::new(VerifiedSet::default()),
    )
    .await;
    assert_eq!(loaded.source, Source::Api);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn both_paths_failing_returns_none_with_reason() {
    let calls = Arc::new(AtomicUsize::new(0));
    let base = spawn_fake_api(FakeApi {
        catalog_status: 503,
        catalog_body: "{}".to_string(),
        ohlcv_body: "[]".to_string(),
        ohlcv_calls: calls.clone(),
    })
    .await;
    let api = ApiClient::new(&base);
    let cache = Mutex::new(VerifiedSet::default());
    let loaded = load(&api, &test_config(&base), 100, &cache).await;
    assert_eq!(loaded.source, Source::None);
    assert!(loaded.reason.is_some());
    assert!(loaded.bars.time.is_empty());
}

#[tokio::test]
async fn successful_lake_read_never_calls_api() {
    let temp = tempfile::tempdir().unwrap();
    let rel = "bars/part-0.parquet";
    let abs = temp.path().join(rel);
    std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
    std::fs::write(&abs, b"not-parquet").unwrap();

    // Use real parquet via load_from_lake test helper pattern - write minimal valid parquet
    write_test_parquet(&abs, 10, 1_000_000);
    let digest = q_io::digest_file(&abs, DigestAlgorithm::Sha256).unwrap();

    let calls = Arc::new(AtomicUsize::new(0));
    let base = spawn_fake_api(FakeApi {
        catalog_status: 200,
        catalog_body: catalog_json(
            temp.path().to_str().unwrap(),
            &format!(
                "[{}]",
                published_manifest(rel, &digest, abs.metadata().unwrap().len())
            ),
        ),
        ohlcv_body: ohlcv_json(10),
        ohlcv_calls: calls.clone(),
    })
    .await;
    let api = ApiClient::new(&base);
    let loaded = load(
        &api,
        &test_config(&base),
        10,
        &Mutex::new(VerifiedSet::default()),
    )
    .await;
    assert_eq!(loaded.source, Source::Lake);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

fn write_test_parquet(path: &std::path::Path, rows: usize, start_time: i64) {
    use arrow::array::{ArrayRef, Float64Array, Int64Array, TimestampMicrosecondArray};
    use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
    use arrow::record_batch::RecordBatch;
    use parquet::arrow::ArrowWriter;
    use parquet::file::properties::WriterProperties;
    use std::sync::Arc;

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
    let times: Vec<i64> = (0..rows)
        .map(|index| start_time + index as i64 * 60_000_000)
        .collect();
    let prices: Vec<f64> = (0..rows).map(|index| 100.0 + index as f64).collect();
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(TimestampMicrosecondArray::from(times)) as ArrayRef,
            Arc::new(Float64Array::from(prices.clone())),
            Arc::new(Float64Array::from(prices.clone())),
            Arc::new(Float64Array::from(prices.clone())),
            Arc::new(Float64Array::from(prices)),
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
