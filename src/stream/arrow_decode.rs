use crate::stream::topic_state::BarColumns;
use arrow::array::{Array, Float64Array, Int64Array, TimestampMicrosecondArray};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::ipc::reader::StreamReader;
use arrow::ipc::writer::StreamWriter;
use arrow::record_batch::RecordBatch;
use std::io::Cursor;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrowDecodeError {
    ReaderError(String),
    MissingColumn(String),
    TypeMismatch {
        column: String,
        expected: String,
        actual: String,
    },
    EmptyBatch,
}

impl std::fmt::Display for ArrowDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReaderError(e) => write!(f, "arrow reader error: {e}"),
            Self::MissingColumn(c) => write!(f, "missing required column: {c}"),
            Self::TypeMismatch {
                column,
                expected,
                actual,
            } => write!(
                f,
                "column {column} type mismatch: expected {expected}, got {actual}"
            ),
            Self::EmptyBatch => write!(f, "empty record batch"),
        }
    }
}

impl std::error::Error for ArrowDecodeError {}

pub fn decode_arrow_bars(bytes: &[u8]) -> Result<BarColumns, ArrowDecodeError> {
    let reader = StreamReader::try_new(Cursor::new(bytes), None)
        .map_err(|e| ArrowDecodeError::ReaderError(e.to_string()))?;

    let mut time_vec = Vec::new();
    let mut open_vec = Vec::new();
    let mut high_vec = Vec::new();
    let mut low_vec = Vec::new();
    let mut close_vec = Vec::new();
    let mut volume_vec = Vec::new();

    for batch_res in reader {
        let batch = batch_res.map_err(|e| ArrowDecodeError::ReaderError(e.to_string()))?;
        if batch.num_rows() == 0 {
            continue;
        }

        // Time column
        let time_col = batch
            .column_by_name("time")
            .ok_or_else(|| ArrowDecodeError::MissingColumn("time".to_string()))?;
        if let Some(ts_arr) = time_col
            .as_any()
            .downcast_ref::<TimestampMicrosecondArray>()
        {
            time_vec.extend(ts_arr.values().iter().copied());
        } else if let Some(i64_arr) = time_col.as_any().downcast_ref::<Int64Array>() {
            time_vec.extend(i64_arr.values().iter().copied());
        } else {
            return Err(ArrowDecodeError::TypeMismatch {
                column: "time".to_string(),
                expected: "timestamp[us] or int64".to_string(),
                actual: format!("{:?}", time_col.data_type()),
            });
        }

        // Open
        let open_col = batch
            .column_by_name("open")
            .ok_or_else(|| ArrowDecodeError::MissingColumn("open".to_string()))?;
        let open_arr = open_col
            .as_any()
            .downcast_ref::<Float64Array>()
            .ok_or_else(|| ArrowDecodeError::TypeMismatch {
                column: "open".to_string(),
                expected: "float64".to_string(),
                actual: format!("{:?}", open_col.data_type()),
            })?;
        open_vec.extend(open_arr.values().iter().copied());

        // High
        let high_col = batch
            .column_by_name("high")
            .ok_or_else(|| ArrowDecodeError::MissingColumn("high".to_string()))?;
        let high_arr = high_col
            .as_any()
            .downcast_ref::<Float64Array>()
            .ok_or_else(|| ArrowDecodeError::TypeMismatch {
                column: "high".to_string(),
                expected: "float64".to_string(),
                actual: format!("{:?}", high_col.data_type()),
            })?;
        high_vec.extend(high_arr.values().iter().copied());

        // Low
        let low_col = batch
            .column_by_name("low")
            .ok_or_else(|| ArrowDecodeError::MissingColumn("low".to_string()))?;
        let low_arr = low_col
            .as_any()
            .downcast_ref::<Float64Array>()
            .ok_or_else(|| ArrowDecodeError::TypeMismatch {
                column: "low".to_string(),
                expected: "float64".to_string(),
                actual: format!("{:?}", low_col.data_type()),
            })?;
        low_vec.extend(low_arr.values().iter().copied());

        // Close
        let close_col = batch
            .column_by_name("close")
            .ok_or_else(|| ArrowDecodeError::MissingColumn("close".to_string()))?;
        let close_arr = close_col
            .as_any()
            .downcast_ref::<Float64Array>()
            .ok_or_else(|| ArrowDecodeError::TypeMismatch {
                column: "close".to_string(),
                expected: "float64".to_string(),
                actual: format!("{:?}", close_col.data_type()),
            })?;
        close_vec.extend(close_arr.values().iter().copied());

        // Volume: real_volume, tick_volume, or volume
        if let Some(rv_col) = batch.column_by_name("real_volume") {
            if let Some(rv_arr) = rv_col.as_any().downcast_ref::<Int64Array>() {
                volume_vec.extend(rv_arr.values().iter().map(|v| *v as f64));
            } else if let Some(rv_arr) = rv_col.as_any().downcast_ref::<Float64Array>() {
                volume_vec.extend(rv_arr.values().iter().copied());
            } else {
                volume_vec.extend(vec![0.0; batch.num_rows()]);
            }
        } else if let Some(tv_col) = batch.column_by_name("tick_volume") {
            if let Some(tv_arr) = tv_col.as_any().downcast_ref::<Int64Array>() {
                volume_vec.extend(tv_arr.values().iter().map(|v| *v as f64));
            } else if let Some(tv_arr) = tv_col.as_any().downcast_ref::<Float64Array>() {
                volume_vec.extend(tv_arr.values().iter().copied());
            } else {
                volume_vec.extend(vec![0.0; batch.num_rows()]);
            }
        } else if let Some(v_col) = batch.column_by_name("volume") {
            if let Some(v_arr) = v_col.as_any().downcast_ref::<Float64Array>() {
                volume_vec.extend(v_arr.values().iter().copied());
            } else if let Some(v_arr) = v_col.as_any().downcast_ref::<Int64Array>() {
                volume_vec.extend(v_arr.values().iter().map(|v| *v as f64));
            } else {
                volume_vec.extend(vec![0.0; batch.num_rows()]);
            }
        } else {
            volume_vec.extend(vec![0.0; batch.num_rows()]);
        }
    }

    Ok(BarColumns {
        time: time_vec,
        open: open_vec,
        high: high_vec,
        low: low_vec,
        close: close_vec,
        volume: volume_vec,
    })
}

pub fn encode_arrow_bars(bars: &BarColumns) -> Result<Vec<u8>, ArrowDecodeError> {
    let schema = Arc::new(Schema::new(vec![
        Field::new(
            "time",
            DataType::Timestamp(
                TimeUnit::Microsecond,
                Some("naive-wallclock-America/Sao_Paulo".into()),
            ),
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

    let time_arr = Arc::new(
        TimestampMicrosecondArray::from_iter_values(bars.time.iter().copied())
            .with_timezone("naive-wallclock-America/Sao_Paulo"),
    );
    let open_arr = Arc::new(Float64Array::from_iter_values(bars.open.iter().copied()));
    let high_arr = Arc::new(Float64Array::from_iter_values(bars.high.iter().copied()));
    let low_arr = Arc::new(Float64Array::from_iter_values(bars.low.iter().copied()));
    let close_arr = Arc::new(Float64Array::from_iter_values(bars.close.iter().copied()));
    let tick_arr = Arc::new(Int64Array::from(vec![0i64; bars.time.len()]));
    let spread_arr = Arc::new(Int64Array::from(vec![0i64; bars.time.len()]));
    let real_arr = Arc::new(Int64Array::from_iter_values(
        bars.volume.iter().map(|v| *v as i64),
    ));

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            time_arr, open_arr, high_arr, low_arr, close_arr, tick_arr, spread_arr, real_arr,
        ],
    )
    .map_err(|e| ArrowDecodeError::ReaderError(e.to_string()))?;

    let mut buf = Vec::new();
    {
        let mut writer = StreamWriter::try_new(&mut buf, &schema)
            .map_err(|e| ArrowDecodeError::ReaderError(e.to_string()))?;
        writer
            .write(&batch)
            .map_err(|e| ArrowDecodeError::ReaderError(e.to_string()))?;
        writer
            .finish()
            .map_err(|e| ArrowDecodeError::ReaderError(e.to_string()))?;
    }

    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arrow_roundtrip() {
        let bars = BarColumns {
            time: vec![1000, 2000],
            open: vec![10.0, 11.0],
            high: vec![12.0, 13.0],
            low: vec![9.0, 10.0],
            close: vec![11.5, 12.5],
            volume: vec![100.0, 200.0],
        };

        let bytes = encode_arrow_bars(&bars).expect("encode succeeds");
        let decoded = decode_arrow_bars(&bytes).expect("decode succeeds");

        assert_eq!(decoded.time, bars.time);
        assert_eq!(decoded.open, bars.open);
        assert_eq!(decoded.high, bars.high);
        assert_eq!(decoded.low, bars.low);
        assert_eq!(decoded.close, bars.close);
        assert_eq!(decoded.volume, bars.volume);
    }
}
