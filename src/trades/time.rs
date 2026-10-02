//! Exchange-local time and the chart bar intervals trades are grouped into.
//!
//! Trades carry UTC milliseconds. Chart bars carry naive exchange-local labels (the lake
//! contract's `naive-wallclock-America/Sao_Paulo`), so every comparison between the two goes
//! through [`ExchangeClock`] explicitly.

const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 24 * HOUR_MS;
const WEEK_MS: i64 = 7 * DAY_MS;
/// 1970-01-01 was a Thursday; weekly bars open on Monday.
const WEEK_ORIGIN_MS: i64 = 4 * DAY_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExchangeClock {
    /// Exchange-local wall clock minus UTC, in milliseconds.
    offset_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedZone(pub String);

impl std::fmt::Display for UnsupportedZone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unsupported exchange timezone {}", self.0)
    }
}

impl ExchangeClock {
    /// America/Sao_Paulo has had no daylight saving since 2019-02-17, so the current session
    /// is always UTC-3. Any other zone is refused rather than guessed.
    pub fn for_zone(name: &str) -> Result<Self, UnsupportedZone> {
        match name {
            "America/Sao_Paulo" => Ok(Self {
                offset_ms: -3 * HOUR_MS,
            }),
            "UTC" | "Etc/UTC" => Ok(Self { offset_ms: 0 }),
            other => Err(UnsupportedZone(other.to_string())),
        }
    }

    pub fn offset_ms(&self) -> i64 {
        self.offset_ms
    }

    /// Converts a naive exchange-local label to UTC milliseconds.
    pub fn local_to_utc_ms(&self, local_ms: i64) -> i64 {
        local_ms - self.offset_ms
    }

    pub fn utc_to_local_ms(&self, utc_ms: i64) -> i64 {
        utc_ms + self.offset_ms
    }
}

/// Chart bar intervals: fixed-width buckets anchored on the exchange-local clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarGrid {
    interval_ms: i64,
    clock: ExchangeClock,
}

impl BarGrid {
    pub fn new(interval_ms: i64, clock: ExchangeClock) -> Option<Self> {
        (interval_ms > 0).then_some(Self { interval_ms, clock })
    }

    pub fn interval_ms(&self) -> i64 {
        self.interval_ms
    }

    pub fn clock(&self) -> ExchangeClock {
        self.clock
    }

    /// The `[open, close)` UTC interval of the bar containing `time_utc_ms`.
    pub fn bounds(&self, time_utc_ms: i64) -> (i64, i64) {
        let local = self.clock.utc_to_local_ms(time_utc_ms);
        let origin = if self.interval_ms == WEEK_MS {
            WEEK_ORIGIN_MS
        } else {
            0
        };
        let open_local = (local - origin).div_euclid(self.interval_ms) * self.interval_ms + origin;
        let open = self.clock.local_to_utc_ms(open_local);
        (open, open + self.interval_ms)
    }

    /// The UTC open of the chart bar labelled `label_ms` in exchange-local time.
    pub fn open_of_label(&self, label_ms: i64) -> i64 {
        self.clock.local_to_utc_ms(label_ms)
    }
}

/// `YYYYMMDD` session keys become the kernel's numeric session key; anything else hashes.
pub fn numeric_session_key(session_key: &str) -> u64 {
    let digits: String = session_key.chars().filter(char::is_ascii_digit).collect();
    if digits.len() == 8 {
        if let Ok(n) = digits.parse() {
            return n;
        }
    }
    session_key.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SP: &str = "America/Sao_Paulo";

    #[test]
    fn local_labels_convert_to_utc_explicitly() {
        let clock = ExchangeClock::for_zone(SP).unwrap();
        // 2026-10-01 09:00 local is 12:00 UTC.
        let local = 1_790_845_200_000;
        assert_eq!(clock.local_to_utc_ms(local), local + 3 * HOUR_MS);
        assert_eq!(clock.utc_to_local_ms(clock.local_to_utc_ms(local)), local);
    }

    #[test]
    fn unknown_zones_are_refused() {
        assert!(ExchangeClock::for_zone("Europe/Paris").is_err());
    }

    #[test]
    fn bars_align_on_the_local_clock_not_on_utc() {
        let clock = ExchangeClock::for_zone(SP).unwrap();
        let grid = BarGrid::new(5 * 60_000, clock).unwrap();
        // 12:03:20 UTC is 09:03:20 local, inside the 09:00-09:05 local bar.
        let t = 12 * HOUR_MS + 3 * 60_000 + 20_000;
        let (open, close) = grid.bounds(t);
        assert_eq!(open, 12 * HOUR_MS);
        assert_eq!(close, 12 * HOUR_MS + 5 * 60_000);
        assert_eq!(grid.open_of_label(clock.utc_to_local_ms(open)), open);
    }

    #[test]
    fn daily_bars_span_the_local_day() {
        let clock = ExchangeClock::for_zone(SP).unwrap();
        let grid = BarGrid::new(DAY_MS, clock).unwrap();
        // 01:00 UTC is still the previous local day (22:00).
        let (open, close) = grid.bounds(DAY_MS + HOUR_MS);
        assert_eq!(open, 3 * HOUR_MS);
        assert_eq!(close - open, DAY_MS);
        assert!(open <= DAY_MS + HOUR_MS && DAY_MS + HOUR_MS < close);
    }

    #[test]
    fn session_keys_are_stable() {
        assert_eq!(numeric_session_key("2026-10-01"), 20_261_001);
        assert_eq!(numeric_session_key("a"), numeric_session_key("a"));
        assert_ne!(numeric_session_key("a"), numeric_session_key("b"));
    }
}
