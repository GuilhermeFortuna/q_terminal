use q_buffers::frame::BarColumns;

/// Drops history bars at or after `first_streamed_time`, so history and live
/// never duplicate or reorder a bar.
pub fn trim_to_seam(bars: BarColumns, first_streamed_time: Option<i64>) -> BarColumns {
    let cutoff = match first_streamed_time {
        Some(time) => time,
        None => return bars,
    };

    let keep = bars
        .time
        .iter()
        .position(|&time| time >= cutoff)
        .unwrap_or(bars.time.len());

    if keep == bars.time.len() {
        return bars;
    }

    BarColumns {
        time: bars.time[..keep].to_vec(),
        open: bars.open[..keep].to_vec(),
        high: bars.high[..keep].to_vec(),
        low: bars.low[..keep].to_vec(),
        close: bars.close[..keep].to_vec(),
        tick_volume: bars.tick_volume.map(|values| values[..keep].to_vec()),
        spread: bars.spread.map(|values| values[..keep].to_vec()),
        real_volume: bars.real_volume.map(|values| values[..keep].to_vec()),
        label: bars.label,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use q_buffers::frame::TimeLabel;

    fn bars(times: &[i64]) -> BarColumns {
        let len = times.len();
        BarColumns {
            time: times.to_vec(),
            open: vec![1.0; len],
            high: vec![2.0; len],
            low: vec![0.5; len],
            close: vec![1.5; len],
            tick_volume: None,
            spread: None,
            real_volume: None,
            label: TimeLabel::Utc,
        }
    }

    #[test]
    fn drops_history_at_or_after_first_streamed_time() {
        let trimmed = trim_to_seam(bars(&[10, 20, 30, 40]), Some(30));
        assert_eq!(trimmed.time, vec![10, 20]);
    }

    #[test]
    fn passes_through_without_streamed_time() {
        let input = bars(&[10, 20, 30]);
        let output = trim_to_seam(input.clone(), None);
        assert_eq!(output.time, input.time);
    }

    #[test]
    fn result_is_strictly_ascending_without_duplicates() {
        let trimmed = trim_to_seam(bars(&[10, 20, 30, 40, 50]), Some(40));
        for window in trimmed.time.windows(2) {
            assert!(window[0] < window[1]);
        }
        let unique: std::collections::HashSet<_> = trimmed.time.iter().collect();
        assert_eq!(unique.len(), trimmed.time.len());
    }
}
