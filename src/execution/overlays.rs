//! Indicator overlays and marker geometry for the deployment chart (Q-049).
//!
//! Indicator values come from the backend's deployment chart route, computed there with
//! `q_core`; nothing here computes an indicator. This module parses the route's response,
//! aligns values to bars by time, and packs markers and overlay lines into vertex buffers in
//! surface coordinates, with the same bar-to-x and price-to-y mapping the bar geometry uses.

use crate::execution::markers::{parse_ms, Marker, MarkerKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Price,
    Oscillator,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OverlaySeries {
    pub key: String,
    pub label: String,
    pub pane: Pane,
    pub rgba: u32,
    /// Bar open (ms) and the backend's value for it.
    pub points: Vec<(i64, Option<f64>)>,
}

const PALETTE: [u32; 4] = [0x2962ffff, 0xff9800ff, 0xab47bcff, 0x26c6daff];

fn parse_color(text: Option<&str>, index: usize) -> u32 {
    if let Some(hex) = text.and_then(|t| t.strip_prefix('#')) {
        if hex.len() == 6 {
            if let Ok(v) = u32::from_str_radix(hex, 16) {
                return (v << 8) | 0xff;
            }
        }
    }
    PALETTE[index % PALETTE.len()]
}

/// Parses a `GET /api/v1/execution/deployments/{id}/chart` response into overlay series.
pub fn parse_chart(json: &str) -> Result<Vec<OverlaySeries>, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let bars: Vec<Option<i64>> = v
        .get("bars")
        .and_then(|b| b.as_array())
        .ok_or("missing bars")?
        .iter()
        .map(|b| {
            b.get("timestamp")
                .and_then(|t| t.as_str())
                .and_then(parse_ms)
        })
        .collect();
    let mut out = Vec::new();
    for (i, ind) in v
        .get("indicators")
        .and_then(|x| x.as_array())
        .ok_or("missing indicators")?
        .iter()
        .enumerate()
    {
        let values = ind
            .get("values")
            .and_then(|x| x.as_array())
            .ok_or("indicator without values")?;
        let points = bars
            .iter()
            .zip(values.iter())
            .filter_map(|(t, val)| t.map(|t| (t, val.as_f64().filter(|f| f.is_finite()))))
            .collect();
        out.push(OverlaySeries {
            key: ind["key"].as_str().unwrap_or_default().to_string(),
            label: ind["label"].as_str().unwrap_or_default().to_string(),
            pane: if ind["pane"].as_str() == Some("oscillator") {
                Pane::Oscillator
            } else {
                Pane::Price
            },
            rgba: parse_color(ind["color"].as_str(), i),
            points,
        });
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy)]
pub struct View {
    pub first: usize,
    pub last: usize,
    pub low: f64,
    pub high: f64,
    pub width: f32,
    pub height: f32,
}

impl View {
    /// Centre of bar `index`, as `pack` in q-buffers places it.
    pub fn bar_x(&self, index: usize) -> f32 {
        let total = (self.last - self.first) as f32;
        (index as f32 + 0.5 - self.first as f32) / total * self.width
    }

    pub fn price_y(&self, price: f64) -> f32 {
        let range = self.high - self.low;
        if !range.is_finite() || range <= 0.0 {
            return self.height * 0.5;
        }
        (((self.high - price) / range) * f64::from(self.height)) as f32
    }

    fn valid(&self) -> bool {
        self.last > self.first && self.width > 0.0 && self.height > 0.0
    }

    fn visible(&self, index: usize) -> bool {
        index >= self.first && index < self.last
    }
}

pub const MODE_TRIANGLES: u8 = 0;
pub const MODE_LINE_STRIP: u8 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub mode: u8,
    pub rgba: u32,
    /// x, y pairs in surface coordinates.
    pub xy: Vec<f32>,
}

impl Layer {
    fn new(mode: u8, rgba: u32) -> Self {
        Self {
            mode,
            rgba,
            xy: Vec::new(),
        }
    }
}

/// Where a marker was drawn, for hover.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub x: f32,
    pub y: f32,
    pub detail: String,
}

pub const MARKER_SIZE: f32 = 6.0;

fn kind_color(kind: MarkerKind) -> u32 {
    match kind {
        MarkerKind::Buy => 0x26a69aff,
        MarkerKind::Sell => 0xef5350ff,
        MarkerKind::Close => 0xffd54fff,
        MarkerKind::Fill => 0xffffffff,
    }
}

fn tri(l: &mut Layer, a: (f32, f32), b: (f32, f32), c: (f32, f32)) {
    l.xy.extend_from_slice(&[a.0, a.1, b.0, b.1, c.0, c.1]);
}

/// Packs one glyph per marker: an up triangle under the bar for a buy, a down triangle over
/// it for a sell, a diamond on the close for a close, a square at the price for a fill.
/// `hlc` is each bar's high, low and close, indexed like the bars.
pub fn marker_layers(
    markers: &[Marker],
    hlc: &[(f64, f64, f64)],
    view: View,
) -> (Vec<Layer>, Vec<Hit>) {
    let mut layers: Vec<Layer> = [
        MarkerKind::Buy,
        MarkerKind::Sell,
        MarkerKind::Close,
        MarkerKind::Fill,
    ]
    .iter()
    .map(|k| Layer::new(MODE_TRIANGLES, kind_color(*k)))
    .collect();
    let mut hits = Vec::new();
    if !view.valid() {
        return (Vec::new(), hits);
    }
    let s = MARKER_SIZE;
    for m in markers {
        if !view.visible(m.bar_index) {
            continue;
        }
        let Some(&(high, low, close)) = hlc.get(m.bar_index) else {
            continue;
        };
        let x = view.bar_x(m.bar_index);
        let (li, cy) = match m.kind {
            MarkerKind::Buy => (0, view.price_y(low) + s * 1.5),
            MarkerKind::Sell => (1, view.price_y(high) - s * 1.5),
            MarkerKind::Close => (2, view.price_y(m.price.unwrap_or(close))),
            MarkerKind::Fill => (3, view.price_y(m.price.unwrap_or(close))),
        };
        let l = &mut layers[li];
        match m.kind {
            MarkerKind::Buy => tri(l, (x, cy - s), (x - s, cy + s), (x + s, cy + s)),
            MarkerKind::Sell => tri(l, (x - s, cy - s), (x + s, cy - s), (x, cy + s)),
            MarkerKind::Close => {
                tri(l, (x, cy - s), (x - s, cy), (x, cy + s));
                tri(l, (x, cy - s), (x, cy + s), (x + s, cy));
            }
            MarkerKind::Fill => {
                let h = s * 0.6;
                tri(l, (x - h, cy - h), (x + h, cy - h), (x - h, cy + h));
                tri(l, (x + h, cy - h), (x + h, cy + h), (x - h, cy + h));
            }
        }
        hits.push(Hit {
            x,
            y: cy,
            detail: m.detail.clone(),
        });
    }
    layers.retain(|l| !l.xy.is_empty());
    (layers, hits)
}

/// Share of the surface height the oscillator pane takes when any oscillator is present.
pub const OSCILLATOR_SHARE: f32 = 0.25;

/// Packs overlay lines. Price-pane series use the price axis; oscillators share a pane
/// below the bars, scaled to their visible range. `bar_opens_ms` indexes like the bars.
pub fn line_layers(series: &[OverlaySeries], bar_opens_ms: &[i64], view: View) -> Vec<Layer> {
    let mut out = Vec::new();
    if !view.valid() {
        return out;
    }
    let last = view.last.min(bar_opens_ms.len());
    let value_at = |s: &OverlaySeries, ms: i64| -> Option<f64> {
        s.points
            .binary_search_by_key(&ms, |p| p.0)
            .ok()
            .and_then(|i| s.points[i].1)
    };

    let osc_top = view.height * (1.0 - OSCILLATOR_SHARE);
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for s in series.iter().filter(|s| s.pane == Pane::Oscillator) {
        for i in view.first..last {
            if let Some(v) = value_at(s, bar_opens_ms[i]) {
                lo = lo.min(v);
                hi = hi.max(v);
            }
        }
    }
    if lo.is_finite() {
        let mut sep = Layer::new(MODE_LINE_STRIP, 0x2a2e39ff);
        sep.xy
            .extend_from_slice(&[0.0, osc_top, view.width, osc_top]);
        out.push(sep);
    }
    let osc_y = |v: f64| -> f32 {
        let range = hi - lo;
        let norm = if range > 0.0 { (hi - v) / range } else { 0.5 };
        osc_top + 4.0 + (norm as f32) * (view.height - osc_top - 8.0)
    };

    for s in series {
        let mut run = Layer::new(MODE_LINE_STRIP, s.rgba);
        let flush = |run: &mut Layer, out: &mut Vec<Layer>| {
            if run.xy.len() >= 4 {
                out.push(run.clone());
            }
            run.xy.clear();
        };
        for i in view.first..last {
            match value_at(s, bar_opens_ms[i]) {
                Some(v) => {
                    let y = match s.pane {
                        Pane::Price => view.price_y(v),
                        Pane::Oscillator => osc_y(v),
                    };
                    run.xy.extend_from_slice(&[view.bar_x(i), y]);
                }
                None => flush(&mut run, &mut out),
            }
        }
        flush(&mut run, &mut out);
    }
    out
}
