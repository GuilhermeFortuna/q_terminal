//! Arrow IPC for the `trades` topic and the session history pages (Q-079).
//!
//! Rows are UTC `timestamp[ms]`, price, raw `volume`, optional `volume_real`, raw flags and a
//! zero-based occurrence. The delivery context (provider, symbol, source generation, session,
//! selected volume field and unit) travels in the Arrow schema metadata.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

use arrow::array::{Array, Float64Array, Int32Array, Int64Array, TimestampMillisecondArray};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::ipc::reader::StreamReader;
use arrow::ipc::writer::StreamWriter;
use arrow::record_batch::RecordBatch;

use crate::stream::arrow_decode::ArrowDecodeError;
use crate::trades::feed::SourceContext;
use crate::trades::history::TradeColumns;

/// Rows of a decoded delivery or page, with what the decoder had to refuse.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedTrades {
    pub metadata: HashMap<String, String>,
    pub columns: TradeColumns,
    /// Rows without a finite positive price or selected volume, or an unusable occurrence.
    pub invalid_rows: usize,
}

impl DecodedTrades {
    /// The delivery context, if every field of it is present in the schema metadata.
    pub fn context(&self) -> Option<SourceContext> {
        let get = |name: &str| self.metadata.get(name).cloned();
        Some(SourceContext {
            provider_id: get("provider_id")?,
            symbol: get("symbol")?,
            source_generation: get("source_generation")?,
            exchange_timezone: get("exchange_timezone")?,
            session_key: get("session_key")?,
            volume_field: get("volume_field")?,
            volume_unit: get("volume_unit")?,
        })
    }
}

fn column<'a, T: 'static>(
    batch: &'a RecordBatch,
    name: &str,
    expected: &str,
) -> Result<&'a T, ArrowDecodeError> {
    let col = batch
        .column_by_name(name)
        .ok_or_else(|| ArrowDecodeError::MissingColumn(name.to_string()))?;
    col.as_any()
        .downcast_ref::<T>()
        .ok_or_else(|| ArrowDecodeError::TypeMismatch {
            column: name.to_string(),
            expected: expected.to_string(),
            actual: format!("{:?}", col.data_type()),
        })
}

/// Decodes one trades delivery or history page.
///
/// `volume_field` names the column analysis reads; when `None` the schema metadata decides.
pub fn decode_arrow_trades(
    bytes: &[u8],
    volume_field: Option<&str>,
) -> Result<DecodedTrades, ArrowDecodeError> {
    let reader = StreamReader::try_new(Cursor::new(bytes), None)
        .map_err(|e| ArrowDecodeError::ReaderError(e.to_string()))?;
    let metadata: HashMap<String, String> = reader.schema().metadata().clone().into();
    let field = volume_field
        .map(str::to_string)
        .or_else(|| metadata.get("volume_field").cloned())
        .unwrap_or_else(|| "volume".to_string());
    let mut columns = TradeColumns::default();
    let mut invalid = 0;
    for batch in reader {
        let batch = batch.map_err(|e| ArrowDecodeError::ReaderError(e.to_string()))?;
        let time: &TimestampMillisecondArray = column(&batch, "time_msc", "timestamp[ms]")?;
        let price: &Float64Array = column(&batch, "price", "float64")?;
        let raw: &Float64Array = column(&batch, "volume", "float64")?;
        let real: &Float64Array = column(&batch, "volume_real", "float64")?;
        let flags: &Int32Array = column(&batch, "raw_flags", "int32")?;
        let occurrence: &Int64Array = column(&batch, "occurrence", "int64")?;
        for i in 0..batch.num_rows() {
            let selected = match field.as_str() {
                "volume_real" if real.is_valid(i) => Some(real.value(i)),
                "volume_real" => None,
                _ => raw.is_valid(i).then(|| raw.value(i)),
            };
            let usable = selected.filter(|v| v.is_finite() && *v > 0.0);
            let p = price.value(i);
            let occ = u32::try_from(occurrence.value(i)).ok();
            match (usable, occ) {
                (Some(volume), Some(occ)) if p.is_finite() && p > 0.0 => {
                    columns.push(time.value(i), p, volume, flags.value(i) as u32, occ);
                }
                _ => invalid += 1,
            }
        }
    }
    Ok(DecodedTrades {
        metadata,
        columns,
        invalid_rows: invalid,
    })
}

/// Encodes rows the way the gateway publishes them. The selected volume is written to the
/// column `ctx.volume_field` names; the other column carries a different raw value.
pub fn encode_arrow_trades(
    ctx: &SourceContext,
    rows: &TradeColumns,
) -> Result<Vec<u8>, ArrowDecodeError> {
    let metadata: HashMap<String, String> = [
        ("provider_id", &ctx.provider_id),
        ("symbol", &ctx.symbol),
        ("source_generation", &ctx.source_generation),
        ("exchange_timezone", &ctx.exchange_timezone),
        ("session_key", &ctx.session_key),
        ("volume_field", &ctx.volume_field),
        ("volume_unit", &ctx.volume_unit),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.clone()))
    .collect();
    let schema = Arc::new(
        Schema::new(vec![
            Field::new(
                "time_msc",
                DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into())),
                false,
            ),
            Field::new("price", DataType::Float64, false),
            Field::new("volume", DataType::Float64, false),
            Field::new("volume_real", DataType::Float64, true),
            Field::new("raw_flags", DataType::Int32, false),
            Field::new("occurrence", DataType::Int64, false),
        ])
        .with_metadata(metadata),
    );
    let real = ctx.volume_field == "volume_real";
    let raw_volume: Vec<f64> = if real {
        rows.volume.iter().map(|v| v * 10.0).collect()
    } else {
        rows.volume.clone()
    };
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(TimestampMillisecondArray::from(rows.time_msc.clone()).with_timezone("UTC")),
            Arc::new(Float64Array::from(rows.price.clone())),
            Arc::new(Float64Array::from(raw_volume)),
            Arc::new(Float64Array::from(
                rows.volume
                    .iter()
                    .map(|v| real.then_some(*v))
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int32Array::from(
                rows.raw_flags.iter().map(|f| *f as i32).collect::<Vec<_>>(),
            )),
            Arc::new(Int64Array::from(
                rows.occurrence
                    .iter()
                    .map(|o| i64::from(*o))
                    .collect::<Vec<_>>(),
            )),
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

    fn ctx(field: &str) -> SourceContext {
        SourceContext {
            provider_id: "p".into(),
            symbol: "WINZ26".into(),
            source_generation: "g".into(),
            exchange_timezone: "America/Sao_Paulo".into(),
            session_key: "2026-10-01".into(),
            volume_field: field.into(),
            volume_unit: "contracts".into(),
        }
    }

    fn rows() -> TradeColumns {
        let mut c = TradeColumns::default();
        c.push(1_000, 128_450.0, 2.0, 32, 0);
        c.push(1_000, 128_450.0, 2.0, 32, 1);
        c.push(1_001, 128_455.0, 5.0, 64, 0);
        c
    }

    #[test]
    fn roundtrip_keeps_identical_rows_and_context() {
        for field in ["volume_real", "volume"] {
            let bytes = encode_arrow_trades(&ctx(field), &rows()).unwrap();
            let decoded = decode_arrow_trades(&bytes, None).unwrap();
            assert_eq!(decoded.columns, rows(), "{field}");
            assert_eq!(decoded.context(), Some(ctx(field)));
            assert_eq!(decoded.invalid_rows, 0);
        }
    }

    #[test]
    fn missing_selected_volume_marks_the_row_invalid() {
        // Encoded for `volume`, decoded as if `volume_real` were selected: all nulls.
        let bytes = encode_arrow_trades(&ctx("volume"), &rows()).unwrap();
        let decoded = decode_arrow_trades(&bytes, Some("volume_real")).unwrap();
        assert!(decoded.columns.is_empty());
        assert_eq!(decoded.invalid_rows, 3);
    }
}
