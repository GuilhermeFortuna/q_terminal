pub use q_terminal::{
    bridge, chart_bridge, chart_target, config, contracts_stream, execution, execution_controls,
    execution_models, history, ops_session, ops_status, startup, stream,
};

use history::{verify, Dataset, ManifestFile, Verdict, VerifiedSet};
use q_io::{bar_file_rows, read_bar_files, DigestAlgorithm, RowRange};
use std::fs::File;
use std::path::Path;
use std::time::Instant;

use arrow::array::{ArrayRef, Float64Array, Int64Array, TimestampMicrosecondArray};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;
use std::sync::Arc;

fn write_parquet(path: &Path, rows: usize, start_time: i64) {
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

fn peak_rss_kb() -> u64 {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if let Some(value) = line.strip_prefix("VmHWM:") {
                return value
                    .trim()
                    .trim_end_matches("kB")
                    .trim()
                    .parse()
                    .unwrap_or(0);
            }
        }
    }
    0
}

#[test]
#[ignore]
fn bench_history_load_200k() {
    const ROWS: usize = 200_000;
    const REQUESTED: usize = 200_000;

    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let files = [
        ("part-0.parquet", 80_000, 1_000_000_i64),
        ("part-1.parquet", 70_000, 1_000_000 + 80_000 * 60_000_000),
        ("part-2.parquet", 50_000, 1_000_000 + 150_000 * 60_000_000),
    ];

    let mut manifest_files = Vec::new();
    for (name, rows, start) in files {
        let path = root.join(name);
        write_parquet(&path, rows, start);
        manifest_files.push(ManifestFile {
            path: name.to_string(),
            size_bytes: path.metadata().unwrap().len(),
            checksum: q_io::digest_file(&path, DigestAlgorithm::Sha256).unwrap(),
        });
    }

    let dataset = Dataset {
        id: "bench-ds".to_string(),
        version: 1,
        published_at: "2026-01-01T00:00:00Z".to_string(),
        algorithm: DigestAlgorithm::Sha256,
        files: manifest_files,
        row_count: ROWS,
    };

    let rss_before = peak_rss_kb();
    let verify_start = Instant::now();
    let verdict = verify(&dataset, root, &mut VerifiedSet::default());
    let verify_secs = verify_start.elapsed().as_secs_f64();
    assert_eq!(verdict, Verdict::Verified);

    let read_start = Instant::now();
    let paths: Vec<_> = dataset
        .files
        .iter()
        .map(|file| root.join(&file.path))
        .collect();
    let path_refs: Vec<&Path> = paths.iter().map(|path| path.as_path()).collect();
    let total_rows = path_refs
        .iter()
        .map(|path| bar_file_rows(path).unwrap())
        .sum::<usize>();
    let start = total_rows.saturating_sub(REQUESTED);
    let frame = read_bar_files(
        &path_refs,
        Some(RowRange {
            start,
            end: total_rows,
        }),
    )
    .unwrap();
    let read_secs = read_start.elapsed().as_secs_f64();
    let rss_after = peak_rss_kb();

    let total_secs = verify_secs + read_secs;
    println!("\n=== History Load Benchmark (200,000 bars) ===");
    println!("Verify time: {:.3} s", verify_secs);
    println!("Read time: {:.3} s", read_secs);
    println!("Total time: {:.3} s", total_secs);
    println!(
        "Peak RSS delta: {} kB",
        rss_after.saturating_sub(rss_before)
    );
    println!("Loaded bars: {}", frame.len());

    assert_eq!(frame.len(), REQUESTED);
    assert!(
        total_secs < 5.0,
        "verify+read must complete within 5 seconds, took {:.3}s",
        total_secs
    );
}
