use std::path::{Path, PathBuf};

use q_buffers::frame::BarColumns;
use q_io::{bar_file_rows, read_bar_files, RowRange};

use crate::config::Config;
use crate::history::api_client::{ohlcv_to_columns, ApiClient};
use crate::history::manifest::{resolve, select_current, Dataset, ManifestError};
use crate::history::verify::{verify, Verdict, VerifiedSet};

pub const API_BAR_CAP: usize = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Lake,
    Api,
    None,
}

#[derive(Debug, Clone)]
pub struct Loaded {
    pub bars: BarColumns,
    pub source: Source,
    pub dataset: Option<Dataset>,
    pub shortfall: usize,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    LakeRead(String),
    ApiParse(String),
    ApiTransport(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::LakeRead(msg) => write!(f, "lake read failed: {msg}"),
            LoadError::ApiParse(msg) => write!(f, "api parse failed: {msg}"),
            LoadError::ApiTransport(msg) => write!(f, "api transport failed: {msg}"),
        }
    }
}

impl std::error::Error for LoadError {}

pub fn resolve_dataset_paths(
    dataset: &Dataset,
    root: &Path,
) -> Result<Vec<PathBuf>, ManifestError> {
    dataset
        .files
        .iter()
        .map(|file| resolve(root, &file.path))
        .collect()
}

/// Reads the last `bars` rows of the dataset positionally, via `bar_file_rows`
/// and one `read_bar_files` call with a row range.
pub async fn load_from_lake(
    dataset: &Dataset,
    root: &Path,
    bars: usize,
) -> Result<BarColumns, LoadError> {
    let paths =
        resolve_dataset_paths(dataset, root).map_err(|err| LoadError::LakeRead(err.to_string()))?;
    let path_refs: Vec<&Path> = paths.iter().map(|path| path.as_path()).collect();

    let total_rows = path_refs
        .iter()
        .map(|path| bar_file_rows(path).map_err(|err| LoadError::LakeRead(err.to_string())))
        .try_fold(0usize, |acc, count| count.map(|value| acc + value))?;

    if total_rows == 0 {
        return Ok(empty_bars());
    }

    let start = total_rows.saturating_sub(bars);
    let frame = read_bar_files(
        &path_refs,
        Some(RowRange {
            start,
            end: total_rows,
        }),
    )
    .map_err(|err| LoadError::LakeRead(err.to_string()))?;

    Ok(frame.bars().clone())
}

/// The JSON bars endpoint, capped by the API at 5,000.
pub async fn load_from_api(
    api: &ApiClient,
    symbol: &str,
    timeframe: &str,
    bars: usize,
) -> Result<(BarColumns, usize), LoadError> {
    let requested = bars.min(API_BAR_CAP);
    let response = api
        .fetch_ohlcv(symbol, timeframe, requested)
        .await
        .map_err(|err| LoadError::ApiTransport(err.to_string()))?;
    let columns = ohlcv_to_columns(&response).map_err(LoadError::ApiParse)?;
    let shortfall = bars.saturating_sub(columns.time.len());
    Ok((columns, shortfall))
}

fn empty_bars() -> BarColumns {
    use q_buffers::frame::TimeLabel;

    BarColumns {
        time: Vec::new(),
        open: Vec::new(),
        high: Vec::new(),
        low: Vec::new(),
        close: Vec::new(),
        tick_volume: None,
        spread: None,
        real_volume: None,
        label: TimeLabel::BrasiliaWallclock,
    }
}

fn lake_fallback_reason(verdict: &Verdict) -> String {
    match verdict {
        Verdict::Verified => "lake read failed".to_string(),
        Verdict::SizeMismatch { path } => format!("verification size mismatch for `{path}`"),
        Verdict::DigestMismatch { path } => format!("verification digest mismatch for `{path}`"),
        Verdict::Missing { path } => format!("verification missing file `{path}`"),
    }
}

/// Selection, verification, read, and the fallback policy in one place.
pub async fn load(
    api: &ApiClient,
    config: &Config,
    bars: usize,
    cache: &std::sync::Mutex<VerifiedSet>,
) -> Loaded {
    let (lake_loaded, lake_reason) = try_load_from_lake(api, config, bars, cache).await;
    if let Some(loaded) = lake_loaded {
        return loaded;
    }

    match load_from_api(api, &config.symbol, &config.timeframe, bars).await {
        Ok((bars, shortfall)) if !bars.time.is_empty() => Loaded {
            bars,
            source: Source::Api,
            dataset: None,
            shortfall,
            reason: lake_reason,
        },
        Ok(_) => Loaded {
            bars: empty_bars(),
            source: Source::None,
            dataset: None,
            shortfall: bars,
            reason: Some(
                lake_reason
                    .map(|reason| format!("{reason}; api returned no bars"))
                    .unwrap_or_else(|| "no history available from lake or api".to_string()),
            ),
        },
        Err(err) => Loaded {
            bars: empty_bars(),
            source: Source::None,
            dataset: None,
            shortfall: bars,
            reason: Some(
                lake_reason
                    .map(|reason| format!("{reason}; {}", err))
                    .unwrap_or_else(|| err.to_string()),
            ),
        },
    }
}

async fn try_load_from_lake(
    api: &ApiClient,
    config: &Config,
    bars: usize,
    cache: &std::sync::Mutex<VerifiedSet>,
) -> (Option<Loaded>, Option<String>) {
    let catalog = match api.fetch_catalog(&config.symbol, &config.timeframe).await {
        Ok(response) => response,
        Err(_) => return (None, Some("catalog unreachable".to_string())),
    };

    let dataset = match select_current(&catalog.datasets) {
        Ok(Some(dataset)) => dataset,
        Ok(None) => return (None, Some("no published dataset".to_string())),
        Err(ManifestError::UnsupportedDigest { name }) => {
            return (None, Some(format!("unsupported digest `{name}`")));
        }
        Err(err) => return (None, Some(err.to_string())),
    };

    let root = Path::new(&catalog.root);
    let verdict = {
        let mut cache_guard = cache.lock().unwrap();
        verify(&dataset, root, &mut cache_guard)
    };
    if verdict != Verdict::Verified {
        return (None, Some(lake_fallback_reason(&verdict)));
    }

    match load_from_lake(&dataset, root, bars).await {
        Ok(bars) if !bars.time.is_empty() => (
            Some(Loaded {
                bars,
                source: Source::Lake,
                dataset: Some(dataset),
                shortfall: 0,
                reason: None,
            }),
            None,
        ),
        Ok(_) => (None, Some("lake read returned no bars".to_string())),
        Err(err) => (None, Some(err.to_string())),
    }
}

pub fn source_label(source: Source) -> &'static str {
    match source {
        Source::Lake => "lake",
        Source::Api => "api",
        Source::None => "none",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::manifest::ManifestFile;
    use q_io::DigestAlgorithm;
    use std::fs::File;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use arrow::array::{ArrayRef, Float64Array, Int64Array, TimestampMicrosecondArray};
    use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
    use arrow::record_batch::RecordBatch;
    use parquet::arrow::ArrowWriter;
    use parquet::file::properties::WriterProperties;
    use std::sync::Arc as ArrowArc;

    static NEXT_PORT: AtomicUsize = AtomicUsize::new(19_000);

    fn write_parquet(path: &Path, rows: usize, start_time: i64) {
        let schema = ArrowArc::new(Schema::new(vec![
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
                ArrowArc::new(TimestampMicrosecondArray::from(times)) as ArrayRef,
                ArrowArc::new(Float64Array::from(prices.clone())),
                ArrowArc::new(Float64Array::from(prices.clone())),
                ArrowArc::new(Float64Array::from(prices.clone())),
                ArrowArc::new(Float64Array::from(prices)),
                ArrowArc::new(Int64Array::from(vec![1i64; rows])),
                ArrowArc::new(Int64Array::from(vec![1i64; rows])),
                ArrowArc::new(Int64Array::from(vec![1i64; rows])),
            ],
        )
        .unwrap();
        let file = File::create(path).unwrap();
        let props = WriterProperties::builder().build();
        let mut writer = ArrowWriter::try_new(file, schema, Some(props)).unwrap();
        writer.write(&batch).unwrap();
        writer.close().unwrap();
    }

    fn dataset_with_files(root: &Path, files: Vec<(String, usize, i64)>) -> Dataset {
        let mut manifest_files = Vec::new();
        let mut row_count = 0usize;
        for (rel, rows, start) in files {
            row_count += rows;
            let abs = root.join(&rel);
            std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
            write_parquet(&abs, rows, start);
            let digest = q_io::digest_file(&abs, DigestAlgorithm::Sha256).unwrap();
            manifest_files.push(ManifestFile {
                path: rel,
                size_bytes: abs.metadata().unwrap().len(),
                checksum: digest,
            });
        }
        Dataset {
            id: "ds-test".to_string(),
            version: 1,
            published_at: "2026-01-01T00:00:00Z".to_string(),
            algorithm: DigestAlgorithm::Sha256,
            files: manifest_files,
            row_count,
        }
    }

    #[tokio::test]
    async fn load_from_lake_returns_last_rows_in_order() {
        let temp = tempfile::tempdir().unwrap();
        let dataset = dataset_with_files(
            temp.path(),
            vec![
                ("part-0.parquet".to_string(), 400, 1_000_000),
                (
                    "part-1.parquet".to_string(),
                    400,
                    1_000_000 + 400 * 60_000_000,
                ),
                (
                    "part-2.parquet".to_string(),
                    400,
                    1_000_000 + 800 * 60_000_000,
                ),
            ],
        );

        let bars = load_from_lake(&dataset, temp.path(), 1_000).await.unwrap();
        assert_eq!(bars.time.len(), 1_000);
        assert_eq!(
            bars.time.first().copied(),
            Some(1_000_000 + 200 * 60_000_000)
        );
        assert_eq!(
            bars.time.last().copied(),
            Some(1_000_000 + 1_199 * 60_000_000)
        );
        for window in bars.time.windows(2) {
            assert!(window[0] < window[1]);
        }
    }

    #[tokio::test]
    async fn load_from_lake_larger_than_dataset_returns_all_rows() {
        let temp = tempfile::tempdir().unwrap();
        let dataset =
            dataset_with_files(temp.path(), vec![("part-0.parquet".to_string(), 50, 1_000)]);
        let bars = load_from_lake(&dataset, temp.path(), 1_000).await.unwrap();
        assert_eq!(bars.time.len(), 50);
    }

    #[tokio::test]
    async fn load_from_lake_empty_dataset_returns_nothing() {
        let temp = tempfile::tempdir().unwrap();
        let dataset = Dataset {
            id: "empty".to_string(),
            version: 1,
            published_at: "2026-01-01T00:00:00Z".to_string(),
            algorithm: DigestAlgorithm::Sha256,
            files: Vec::new(),
            row_count: 0,
        };
        let bars = load_from_lake(&dataset, temp.path(), 100).await.unwrap();
        assert!(bars.time.is_empty());
    }

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

    #[tokio::test]
    async fn load_from_api_parses_ascending_columns() {
        let base = spawn_fake_api(FakeApi {
            catalog_status: 503,
            catalog_body: "{}".to_string(),
            ohlcv_body: ohlcv_json(500),
            ohlcv_calls: Arc::new(AtomicUsize::new(0)),
        })
        .await;
        let api = ApiClient::new(&base);
        let (bars, shortfall) = load_from_api(&api, "PETR4", "1m", 500).await.unwrap();
        assert_eq!(bars.time.len(), 500);
        assert_eq!(shortfall, 0);
        for window in bars.time.windows(2) {
            assert!(window[0] <= window[1]);
        }
    }

    #[tokio::test]
    async fn load_from_api_reports_shortfall_above_cap() {
        let base = spawn_fake_api(FakeApi {
            catalog_status: 503,
            catalog_body: "{}".to_string(),
            ohlcv_body: ohlcv_json(5_000),
            ohlcv_calls: Arc::new(AtomicUsize::new(0)),
        })
        .await;
        let api = ApiClient::new(&base);
        let (bars, shortfall) = load_from_api(&api, "PETR4", "1m", 10_000).await.unwrap();
        assert_eq!(bars.time.len(), 5_000);
        assert_eq!(shortfall, 5_000);
    }

    #[tokio::test]
    async fn load_from_api_malformed_bar_reports_field() {
        let base = spawn_fake_api(FakeApi {
            catalog_status: 503,
            catalog_body: "{}".to_string(),
            ohlcv_body: "[{\"timestamp\":\"not-a-date\",\"open\":1.0,\"high\":2.0,\"low\":0.5,\"close\":1.5,\"volume\":1}]"
                .to_string(),
            ohlcv_calls: Arc::new(AtomicUsize::new(0)),
        })
        .await;
        let api = ApiClient::new(&base);
        let err = load_from_api(&api, "PETR4", "1m", 10).await.unwrap_err();
        assert!(err.to_string().contains("timestamp"));
    }
}
