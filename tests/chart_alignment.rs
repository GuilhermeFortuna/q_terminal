//! Q-049 criterion 5: markers and overlay lines keep their bar alignment across a pan and
//! a zoom. The reference is the bar geometry itself (`q_buffers::pack`, what the bar item
//! uploads): a marker or line vertex must sit on the bar's centre x, and a fill marker on
//! the y of its price.

#![allow(clippy::float_cmp)]

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

use execution::markers::{Marker, MarkerKind};
use execution::overlays::{line_layers, marker_layers, OverlaySeries, Pane, View};
use q_buffers::{pack, Bucket, Surface, Viewport};

const BARS: usize = 300;
const W: f32 = 1000.0;
const H: f32 = 500.0;

fn close_of(i: usize) -> f64 {
    100.0 + ((i * 7) % 13) as f64
}

fn hlc() -> Vec<(f64, f64, f64)> {
    (0..BARS)
        .map(|i| (close_of(i) + 1.0, close_of(i) - 1.0, close_of(i)))
        .collect()
}

fn opens() -> Vec<i64> {
    (0..BARS as i64).map(|i| 1_000_000 + i * 60_000).collect()
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

fn check(first: usize, last: usize) {
    let view = View {
        first,
        last,
        low: 85.0,
        high: 120.0,
        width: W,
        height: H,
    };
    let buckets: Vec<Bucket> = (first..last)
        .map(|i| {
            let (high, low, close) = hlc()[i];
            Bucket {
                start: i,
                end: i + 1,
                open: close,
                high,
                low,
                close,
                forming: false,
            }
        })
        .collect();
    let mut verts = Vec::new();
    pack(
        &buckets,
        Viewport {
            first_bar: first,
            last_bar: last,
            low: view.low,
            high: view.high,
        },
        Surface {
            width_px: W,
            height_px: H,
        },
        &mut verts,
    );
    let per_bar = verts.len() / buckets.len();

    // One fill marker per bar at its close, one price line through the closes.
    let markers: Vec<Marker> = (first..last)
        .map(|i| Marker {
            id: format!("fill-{i}"),
            bar_index: i,
            bar_open_ms: opens()[i],
            price: Some(close_of(i)),
            kind: MarkerKind::Fill,
            label: "FILL buy".into(),
            detail: String::new(),
        })
        .collect();
    let (layers, hits) = marker_layers(&markers, &hlc(), view);
    assert_eq!(hits.len(), markers.len());
    let fill = &layers[0];
    let glyph = fill.xy.len() / markers.len(); // 12 floats: two triangles

    let series = [OverlaySeries {
        key: "close".into(),
        label: "Close".into(),
        pane: Pane::Price,
        rgba: 0xffffffff,
        points: (0..BARS).map(|i| (opens()[i], Some(close_of(i)))).collect(),
    }];
    let lines = line_layers(&series, &opens(), view);
    assert_eq!(lines.len(), 1);

    for (n, i) in (first..last).enumerate() {
        let bar = &verts[n * per_bar..(n + 1) * per_bar];
        let (min_x, max_x) = bar
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(v.x), b.max(v.x)));
        let bar_cx = (min_x + max_x) / 2.0;

        let g = &fill.xy[n * glyph..(n + 1) * glyph];
        let gx: Vec<f32> = g.iter().step_by(2).copied().collect();
        let gy: Vec<f32> = g.iter().skip(1).step_by(2).copied().collect();
        let marker_cx = (gx.iter().cloned().fold(f32::MAX, f32::min)
            + gx.iter().cloned().fold(f32::MIN, f32::max))
            / 2.0;
        assert!(
            near(marker_cx, bar_cx),
            "marker x, bar {i} in {first}..{last}"
        );
        let marker_cy = (gy.iter().cloned().fold(f32::MAX, f32::min)
            + gy.iter().cloned().fold(f32::MIN, f32::max))
            / 2.0;
        assert!(
            bar.iter().any(|v| near(v.y, marker_cy)),
            "fill y is the bar's close, bar {i} in {first}..{last}"
        );

        assert!(near(lines[0].xy[2 * n], bar_cx), "line x, bar {i}");
        assert!(near(lines[0].xy[2 * n + 1], marker_cy), "line y, bar {i}");
    }
}

#[test]
fn aligned_at_the_live_edge() {
    check(200, 300);
}

#[test]
fn aligned_after_a_pan() {
    check(37, 137);
    check(120, 220);
}

#[test]
fn aligned_after_zooming_in_and_out() {
    check(250, 270); // 20 bars
    check(0, 300); // everything
    check(140, 160);
}

#[test]
fn markers_outside_the_view_are_not_drawn_and_holes_break_lines() {
    let view = View {
        first: 10,
        last: 20,
        low: 85.0,
        high: 120.0,
        width: W,
        height: H,
    };
    let m = |i: usize| Marker {
        id: String::new(),
        bar_index: i,
        bar_open_ms: 0,
        price: None,
        kind: MarkerKind::Buy,
        label: String::new(),
        detail: String::new(),
    };
    let (_, hits) = marker_layers(&[m(5), m(15), m(25)], &hlc(), view);
    assert_eq!(hits.len(), 1);

    let mut points: Vec<(i64, Option<f64>)> = opens().iter().map(|t| (*t, Some(100.0))).collect();
    points[14].1 = None;
    let s = OverlaySeries {
        key: "x".into(),
        label: "x".into(),
        pane: Pane::Price,
        rgba: 1,
        points,
    };
    let lines = line_layers(&[s], &opens(), view);
    assert_eq!(lines.len(), 2, "a missing value splits the line");
}
