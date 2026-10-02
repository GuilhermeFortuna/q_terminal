//! Normalize contracted bar timestamps before joining history and live deliveries.

use cxx_qt_lib::{QByteArray, QDateTime, QTimeZone};
use q_buffers::frame::{BarColumns, TimeLabel};

/// Bar timestamps are microseconds. MT5 and lake timestamps encode the exchange wall
/// clock; REST timestamps encode UTC instants. Keep chart storage entirely in UTC.
pub fn bars_to_utc(mut bars: BarColumns) -> BarColumns {
    if bars.label == TimeLabel::Utc {
        return bars;
    }
    let utc = QTimeZone::utc();
    let exchange = QTimeZone::from_iana(&QByteArray::from("America/Sao_Paulo"));
    assert!(
        exchange.is_valid(),
        "America/Sao_Paulo time zone is required"
    );
    for time in &mut bars.time {
        let wall = QDateTime::from_msecs_since_epoch(time.div_euclid(1_000), &utc);
        let instant =
            QDateTime::from_date_and_time_time_zone(&wall.date(), &wall.time(), &exchange);
        *time = instant.to_msecs_since_epoch() * 1_000 + time.rem_euclid(1_000);
    }
    bars.label = TimeLabel::Utc;
    bars
}
