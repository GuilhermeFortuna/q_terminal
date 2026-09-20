//! Q-049 criterion 2: marker placement. The vectors are the placement rules of the
//! frontend's `liveChartMarkers.ts` (decision on the bar whose open equals its close time,
//! else `close - timeframe`; fill on the greatest open at or before it; `hold` unmarked).

#[rustfmt::skip]
#[path = "../contracts/stream.rs"]
pub mod contracts_stream;
#[path = "../src/bridge.rs"]
pub mod bridge;
#[path = "../src/config.rs"]
pub mod config;
#[path = "../src/execution/mod.rs"]
pub mod execution;
#[path = "../src/history/mod.rs"]
pub mod history;
#[path = "../src/stream/mod.rs"]
pub mod stream;

use execution::markers;

use contracts_stream::{ExecutionDecisionState, ExecutionFillEvent};
use markers::{decision_markers, fill_markers, normalize_ms, parse_ms, MarkerKind};
use serde_json::json;

const TF: i64 = 60_000;
const DEP: &str = "11111111-1111-1111-1111-111111111111";

fn t0() -> i64 {
    parse_ms("2026-09-18T10:00:00Z").unwrap()
}

/// Ten one-minute bars opening at 10:00:00Z.
fn opens() -> Vec<i64> {
    (0..10).map(|i| t0() + i * TF).collect()
}

const ACCOUNT: &str = "22222222-2222-2222-2222-222222222222";

fn decision(id: &str, action: &str, close: &str) -> ExecutionDecisionState {
    serde_json::from_value(json!({
        "entity": "decision", "id": id, "deployment_id": DEP, "account_id": ACCOUNT,
        "bar_close_time": close, "config_hash": "abc", "outcome": "order_filled",
        "signal_action": action, "requested_quantity": "1.0",
        "strategy_name": "momentum_alpha", "strategy_version": 1, "symbol": "PETR4",
        "timeframe": "M1", "updated_at": close
    }))
    .unwrap()
}

fn fill(id: &str, at: &str, price: &str) -> ExecutionFillEvent {
    serde_json::from_value(json!({
        "entity": "fill", "id": id, "deployment_id": DEP, "account_id": ACCOUNT,
        "order_id": format!("order-{id}"), "broker_mode": "paper",
        "external_fill_id": format!("ext-{id}"), "side": "buy", "quantity": "1.0",
        "price": price, "fee": "0.10", "slippage": "0.01", "filled_at": at,
        "position_after": {"id": "pos", "deployment_id": DEP, "side": "long",
            "quantity": "1.0", "average_entry_price": price, "is_open": true,
            "updated_at": at},
        "updated_at": at
    }))
    .unwrap()
}

#[test]
fn parse_ms_handles_zones_and_fractions() {
    assert_eq!(parse_ms("1970-01-01T00:00:00Z"), Some(0));
    assert_eq!(parse_ms("1970-01-01T00:00:01.5Z"), Some(1_500));
    assert_eq!(parse_ms("1970-01-01T03:00:00+03:00"), Some(0));
    assert_eq!(parse_ms("1970-01-01T00:00:00.123456+00:00"), Some(123));
    assert_eq!(parse_ms("2026-09-18T10:00:00Z"), Some(1_789_725_600_000));
    assert_eq!(parse_ms("not a time"), None);
    assert_eq!(parse_ms(""), None);
}

#[test]
fn normalize_ms_accepts_every_unit() {
    let ms = 1_789_725_600_000;
    assert_eq!(normalize_ms(ms / 1_000), ms);
    assert_eq!(normalize_ms(ms), ms);
    assert_eq!(normalize_ms(ms * 1_000), ms);
    assert_eq!(normalize_ms(ms * 1_000_000), ms);
    assert_eq!(normalize_ms(0), 0);
}

#[test]
fn decision_on_bar_whose_open_equals_its_close_time() {
    let m = decision_markers(
        &[decision("d1", "buy", "2026-09-18T10:03:00Z")],
        &opens(),
        TF,
    );
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].bar_index, 3);
    assert_eq!(m[0].bar_open_ms, t0() + 3 * TF);
    assert_eq!(m[0].kind, MarkerKind::Buy);
    assert_eq!(m[0].price, None);
}

#[test]
fn decision_falls_back_to_close_minus_timeframe() {
    // Close 10:04:00 is not a bar open of a window ending at 10:03; use 10:03 (close - tf).
    let bars: Vec<i64> = (0..4).map(|i| t0() + i * TF).collect();
    let m = decision_markers(&[decision("d1", "sell", "2026-09-18T10:04:00Z")], &bars, TF);
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].bar_index, 3);
    assert_eq!(m[0].kind, MarkerKind::Sell);
}

#[test]
fn exact_match_wins_over_close_minus_timeframe() {
    let m = decision_markers(
        &[decision("d1", "close", "2026-09-18T10:05:00Z")],
        &opens(),
        TF,
    );
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].bar_index, 5);
    assert_eq!(m[0].kind, MarkerKind::Close);
}

#[test]
fn hold_and_unknown_actions_have_no_marker() {
    let ds = [
        decision("h", "hold", "2026-09-18T10:03:00Z"),
        decision("x", "flatten", "2026-09-18T10:03:00Z"),
        decision("b", "buy", "2026-09-18T10:04:00Z"),
    ];
    let m = decision_markers(&ds, &opens(), TF);
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].id, "decision-b");
}

#[test]
fn decisions_outside_the_window_or_malformed_are_dropped() {
    let ds = [
        decision("early", "buy", "2026-09-18T09:00:00Z"),
        decision("late", "buy", "2026-09-18T12:00:00Z"),
        decision("bad", "buy", "yesterday"),
    ];
    assert!(decision_markers(&ds, &opens(), TF).is_empty());
    assert!(decision_markers(&ds, &[], TF).is_empty());
}

#[test]
fn every_actionable_decision_gets_exactly_one_marker() {
    let ds = [
        decision("a", "buy", "2026-09-18T10:01:00Z"),
        decision("b", "sell", "2026-09-18T10:02:00Z"),
        decision("c", "close", "2026-09-18T10:03:00Z"),
        decision("d", "hold", "2026-09-18T10:04:00Z"),
    ];
    let m = decision_markers(&ds, &opens(), TF);
    let ids: Vec<&str> = m.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["decision-a", "decision-b", "decision-c"]);
}

#[test]
fn decision_detail_matches_the_frontend_text() {
    let mut d = decision("d1", "buy", "2026-09-18T10:03:00Z");
    d.reason = Some(json!("ema_cross"));
    d.requested_quantity = Some("50.00".into());
    let m = decision_markers(&[d], &opens(), TF);
    assert_eq!(m[0].label, "BUY");
    assert_eq!(
        m[0].detail,
        "BUY decision\nreason: ema_cross\nqty: 50.00\nbar close: 2026-09-18T10:03:00Z"
    );
    let mut bare = decision("d2", "sell", "2026-09-18T10:03:00Z");
    bare.reason = None;
    bare.requested_quantity = None;
    let m = decision_markers(&[bare], &opens(), TF);
    assert_eq!(
        m[0].detail,
        "SELL decision\nbar close: 2026-09-18T10:03:00Z"
    );
}

#[test]
fn fill_lands_on_the_bar_containing_it_at_its_price() {
    let m = fill_markers(&[fill("f1", "2026-09-18T10:03:41Z", "12.34")], &opens());
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].bar_index, 3);
    assert_eq!(m[0].kind, MarkerKind::Fill);
    assert_eq!(m[0].price, Some(12.34));
}

#[test]
fn fill_on_a_bar_open_belongs_to_that_bar() {
    let m = fill_markers(&[fill("f1", "2026-09-18T10:05:00Z", "1")], &opens());
    assert_eq!(m[0].bar_index, 5);
}

#[test]
fn fill_before_the_first_bar_has_no_marker() {
    assert!(fill_markers(&[fill("f1", "2026-09-18T09:59:59Z", "1")], &opens()).is_empty());
    assert!(fill_markers(&[fill("f1", "2026-09-18T10:00:00Z", "1")], &[]).is_empty());
}

#[test]
fn fill_after_the_last_bar_goes_on_the_last_bar() {
    let m = fill_markers(&[fill("f1", "2026-09-18T11:30:00Z", "1")], &opens());
    assert_eq!(m[0].bar_index, 9);
}

#[test]
fn fill_detail_and_unparseable_price() {
    let m = fill_markers(&[fill("f1", "2026-09-18T10:03:41Z", "n/a")], &opens());
    assert_eq!(m[0].price, None);
    assert_eq!(m[0].label, "FILL buy");
    assert_eq!(
        m[0].detail,
        "FILL buy\nqty: 1.0\nprice: n/a\nat: 2026-09-18T10:03:41Z"
    );
}
