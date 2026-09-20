//! Decision and fill markers for the deployment chart (Q-049).
//!
//! Placement rules are presentation rules ported from the frontend's
//! `liveChartMarkers.ts`, which stays the oracle until Q-050 removes it. A decision is
//! marked on the bar whose open equals its close time, or else `close - timeframe`; a fill
//! goes on the greatest bar open at or before it. `hold` produces no marker. Prices are
//! parsed from decimal strings for placement only; the store keeps the strings.

use crate::contracts_stream::{ExecutionDecisionState, ExecutionFillEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
    Buy,
    Sell,
    Close,
    Fill,
}

impl MarkerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MarkerKind::Buy => "buy",
            MarkerKind::Sell => "sell",
            MarkerKind::Close => "close",
            MarkerKind::Fill => "fill",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Marker {
    pub id: String,
    /// Index into the `bar_opens` the marker was placed against.
    pub bar_index: usize,
    pub bar_open_ms: i64,
    /// Fill price. Decisions carry none and are placed by the bar's range.
    pub price: Option<f64>,
    pub kind: MarkerKind,
    pub label: String,
    pub detail: String,
}

/// Brings a bar or event time in seconds, milliseconds, microseconds or nanoseconds to
/// milliseconds, by magnitude.
pub fn normalize_ms(t: i64) -> i64 {
    if t <= 0 {
        0
    } else if t > 10_000_000_000_000_000 {
        t / 1_000_000
    } else if t > 10_000_000_000_000 {
        t / 1_000
    } else if t < 100_000_000_000 {
        t * 1_000
    } else {
        t
    }
}

/// Parses an RFC 3339 timestamp to epoch milliseconds. Returns `None` when malformed.
pub fn parse_ms(text: &str) -> Option<i64> {
    let t = text.trim();
    if t.len() < 19 || !t.is_char_boundary(19) {
        return None;
    }
    let num = |a: usize, b: usize| t.get(a..b)?.parse::<i64>().ok();
    let (year, month, day) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let (hour, minute, second) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let mut rest = &t[19..];
    let mut millis = 0i64;
    if let Some(frac) = rest.strip_prefix('.') {
        let digits: String = frac.chars().take_while(|c| c.is_ascii_digit()).collect();
        rest = &frac[digits.len()..];
        millis = format!("{:0<3}", &digits[..digits.len().min(3)])
            .parse()
            .ok()?;
    }
    let mut offset_min = 0i64;
    if rest.starts_with('+') || rest.starts_with('-') {
        let sign = if rest.starts_with('-') { -1 } else { 1 };
        let body = rest[1..].replace(':', "");
        let hh = body.get(0..2)?.parse::<i64>().ok()?;
        let mm = body.get(2..4).map_or(Some(0), |m| m.parse::<i64>().ok())?;
        offset_min = sign * (hh * 60 + mm);
    }
    let y = year - i64::from(month <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let secs = days * 86_400 + hour * 3_600 + minute * 60 + second - offset_min * 60;
    Some(secs * 1_000 + millis)
}

fn contains(bar_opens: &[i64], ms: i64) -> Option<usize> {
    bar_opens.binary_search(&ms).ok()
}

fn json_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Markers for the buy, sell and close decisions whose bar is in `bar_opens` (ascending,
/// milliseconds). Everything else, holds included, is dropped.
pub fn decision_markers(
    decisions: &[ExecutionDecisionState],
    bar_opens: &[i64],
    tf_ms: i64,
) -> Vec<Marker> {
    let mut out = Vec::new();
    for d in decisions {
        let kind = match d.signal_action.as_str() {
            "buy" => MarkerKind::Buy,
            "sell" => MarkerKind::Sell,
            "close" => MarkerKind::Close,
            _ => continue,
        };
        let Some(close_ms) = parse_ms(&d.bar_close_time) else {
            continue;
        };
        let Some(index) =
            contains(bar_opens, close_ms).or_else(|| contains(bar_opens, close_ms - tf_ms))
        else {
            continue;
        };
        let action = d.signal_action.to_uppercase();
        let mut lines = vec![format!("{action} decision")];
        if let Some(reason) = d.reason.as_ref().filter(|r| !r.is_null()) {
            lines.push(format!("reason: {}", json_text(reason)));
        }
        if let Some(qty) = &d.requested_quantity {
            lines.push(format!("qty: {qty}"));
        }
        lines.push(format!("bar close: {}", d.bar_close_time));
        out.push(Marker {
            id: format!("decision-{}", d.id),
            bar_index: index,
            bar_open_ms: bar_opens[index],
            price: None,
            kind,
            label: action,
            detail: lines.join("\n"),
        });
    }
    out
}

/// One marker per fill that lands on a bar: the greatest bar open at or before it.
pub fn fill_markers(fills: &[ExecutionFillEvent], bar_opens: &[i64]) -> Vec<Marker> {
    let mut out = Vec::new();
    for f in fills {
        let Some(fill_ms) = parse_ms(&f.filled_at) else {
            continue;
        };
        let after = bar_opens.partition_point(|&open| open <= fill_ms);
        if after == 0 {
            continue;
        }
        let index = after - 1;
        out.push(Marker {
            id: format!("fill-{}", f.id),
            bar_index: index,
            bar_open_ms: bar_opens[index],
            price: f.price.trim().parse::<f64>().ok(),
            kind: MarkerKind::Fill,
            label: format!("FILL {}", f.side),
            detail: format!(
                "FILL {}\nqty: {}\nprice: {}\nat: {}",
                f.side, f.quantity, f.price, f.filled_at
            ),
        });
    }
    out
}
