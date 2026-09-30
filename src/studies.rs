//! Live chart studies backed by `q-indicators` streaming state (Q-075).

use q_buffers::frame::{BarColumns, TimeLabel};
use q_indicators::{
    AtrInput, AtrState, BollingerState, EmaState, RsiState, SessionVwapInput, SessionVwapState,
    SmaState,
};

use crate::execution::markers;
use crate::execution::overlays::{OverlaySeries, Pane};

pub const MAX_STUDIES: usize = 8;

/// Session key for VWAP: exchange-local calendar day `YYYYMMDD` from the bar open time.
///
/// The bar series [`TimeLabel`] names which wall clock the stored microsecond counts use
/// (UTC or naive America/Sao_Paulo per the lake contract). No offset arithmetic is applied;
/// the proleptic Gregorian date parts are taken from `normalize_ms(time)` the same way the
/// chart axis formats bar times.
pub fn session_day_key(time: i64, _label: TimeLabel) -> i64 {
    let ms = markers::normalize_ms(time);
    if ms <= 0 {
        return 0;
    }
    let total_secs = ms / 1000;
    let total_mins = total_secs / 60;
    let total_hours = total_mins / 60;
    let days = total_hours / 24;

    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    y * 10_000 + (m as i64) * 100 + d as i64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceSource {
    Close,
    Open,
    High,
    Low,
    Hlc3,
}

impl PriceSource {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "close" => Some(Self::Close),
            "open" => Some(Self::Open),
            "high" => Some(Self::High),
            "low" => Some(Self::Low),
            "hlc3" => Some(Self::Hlc3),
            _ => None,
        }
    }

    fn value(self, open: f64, high: f64, low: f64, close: f64) -> f64 {
        match self {
            Self::Close => close,
            Self::Open => open,
            Self::High => high,
            Self::Low => low,
            Self::Hlc3 => (high + low + close) / 3.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudyKind {
    Sma,
    Ema,
    Bollinger,
    SessionVwap,
    Rsi,
    Atr,
}

impl StudyKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "sma" => Some(Self::Sma),
            "ema" => Some(Self::Ema),
            "bollinger" => Some(Self::Bollinger),
            "vwap" => Some(Self::SessionVwap),
            "rsi" => Some(Self::Rsi),
            "atr" => Some(Self::Atr),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StudySpec {
    pub id: u64,
    pub kind: StudyKind,
    pub period: i64,
    pub source: PriceSource,
    pub num_std: f64,
    pub palette_index: u8,
}

#[derive(Debug, Clone)]
enum StudyState {
    Sma(SmaState),
    Ema(EmaState),
    Bollinger(BollingerState),
    SessionVwap(SessionVwapState),
    Rsi(RsiState),
    Atr(AtrState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VolumeKind {
    Tick,
    Real,
}

#[derive(Debug, Clone)]
pub struct StudySet {
    specs: Vec<StudySpec>,
    next_id: u64,
    label: TimeLabel,
    volume_kind: Option<VolumeKind>,
    volume_mixed: bool,
    vwap_unavailable_reason: Option<String>,
    /// Cached overlay lines, rebuilt from bars.
    overlays: Vec<OverlaySeries>,
    study_palette: [u32; 8],
}

impl Default for StudySet {
    fn default() -> Self {
        Self::new()
    }
}

impl StudySet {
    pub fn new() -> Self {
        Self {
            specs: Vec::new(),
            next_id: 1,
            label: TimeLabel::Utc,
            volume_kind: None,
            volume_mixed: false,
            vwap_unavailable_reason: None,
            overlays: Vec::new(),
            study_palette: [0; 8],
        }
    }

    pub fn set_study_palette(&mut self, rgba: [u32; 8]) {
        self.study_palette = rgba;
    }

    pub fn overlays(&self) -> &[OverlaySeries] {
        &self.overlays
    }

    pub fn vwap_unavailable_reason(&self) -> Option<&str> {
        self.vwap_unavailable_reason.as_deref()
    }

    pub fn specs(&self) -> &[StudySpec] {
        &self.specs
    }

    pub fn clear(&mut self) {
        self.specs.clear();
        self.overlays.clear();
        self.volume_kind = None;
        self.volume_mixed = false;
        self.vwap_unavailable_reason = None;
    }

    pub fn add_study(
        &mut self,
        kind: StudyKind,
        period: i64,
        source: PriceSource,
        num_std: f64,
    ) -> Result<u64, String> {
        if self.specs.len() >= MAX_STUDIES {
            return Err("at most eight studies per chart".into());
        }
        validate_study_params(kind, period, num_std)?;
        let id = self.next_id;
        self.next_id += 1;
        let palette_index = (self.specs.len() % 8) as u8;
        self.specs.push(StudySpec {
            id,
            kind,
            period,
            source,
            num_std,
            palette_index,
        });
        Ok(id)
    }

    pub fn remove_study(&mut self, id: u64) -> bool {
        if let Some(i) = self.specs.iter().position(|s| s.id == id) {
            self.specs.remove(i);
            true
        } else {
            false
        }
    }

    pub fn study_list_json(&self) -> String {
        let mut items: Vec<serde_json::Value> = Vec::new();
        for s in &self.specs {
            let kind = match s.kind {
                StudyKind::Sma => "sma",
                StudyKind::Ema => "ema",
                StudyKind::Bollinger => "bollinger",
                StudyKind::SessionVwap => "vwap",
                StudyKind::Rsi => "rsi",
                StudyKind::Atr => "atr",
            };
            let source = match s.source {
                PriceSource::Close => "close",
                PriceSource::Open => "open",
                PriceSource::High => "high",
                PriceSource::Low => "low",
                PriceSource::Hlc3 => "hlc3",
            };
            items.push(serde_json::json!({
                "id": s.id,
                "kind": kind,
                "period": s.period,
                "source": source,
                "num_std": s.num_std,
                "palette_index": s.palette_index,
            }));
        }
        serde_json::json!({
            "studies": items,
            "vwap_unavailable": self.vwap_unavailable_reason,
        })
        .to_string()
    }

    fn observe_volume(&mut self, bars: &BarColumns) {
        let kind = if bars.real_volume.is_some() {
            Some(VolumeKind::Real)
        } else if bars.tick_volume.is_some() {
            Some(VolumeKind::Tick)
        } else {
            None
        };
        if let Some(k) = kind {
            match self.volume_kind {
                None => self.volume_kind = Some(k),
                Some(prev) if prev != k => {
                    self.volume_mixed = true;
                }
                _ => {}
            }
        }
    }

    pub fn rebuild(
        &mut self,
        label: TimeLabel,
        completed: &[BarColumns],
        forming: Option<&BarColumns>,
    ) {
        self.label = label;
        if self.specs.is_empty() {
            self.overlays.clear();
            return;
        }
        for bars in completed {
            self.observe_volume(bars);
        }
        if let Some(f) = forming {
            self.observe_volume(f);
        }

        self.vwap_unavailable_reason = if self.volume_mixed {
            Some("mixed tick and real volume".into())
        } else if self.specs.iter().any(|s| s.kind == StudyKind::SessionVwap)
            && self.volume_kind.is_none()
        {
            Some("no volume column".into())
        } else {
            None
        };

        let mut states: Vec<StudyState> = self.specs.iter().map(new_state).collect();

        let mut all_times: Vec<i64> = Vec::new();
        let mut opens = Vec::new();
        let mut highs = Vec::new();
        let mut lows = Vec::new();
        let mut closes = Vec::new();
        let mut tick_vols: Vec<f64> = Vec::new();
        let mut real_vols: Vec<f64> = Vec::new();

        for bars in completed {
            for i in 0..bars.time.len() {
                all_times.push(bars.time[i]);
                opens.push(bars.open[i]);
                highs.push(bars.high[i]);
                lows.push(bars.low[i]);
                closes.push(bars.close[i]);
                tick_vols.push(
                    bars.tick_volume
                        .as_ref()
                        .and_then(|v| v.get(i))
                        .map(|v| *v as f64)
                        .unwrap_or(0.0),
                );
                real_vols.push(
                    bars.real_volume
                        .as_ref()
                        .and_then(|v| v.get(i))
                        .map(|v| *v as f64)
                        .unwrap_or(0.0),
                );
            }
        }

        let forming_preview = forming.map(|f| {
            let i = 0;
            (
                f.time.get(i).copied().unwrap_or(0),
                f.open.get(i).copied().unwrap_or(0.0),
                f.high.get(i).copied().unwrap_or(0.0),
                f.low.get(i).copied().unwrap_or(0.0),
                f.close.get(i).copied().unwrap_or(0.0),
                f.tick_volume
                    .as_ref()
                    .and_then(|v| v.first())
                    .map(|v| *v as f64)
                    .unwrap_or(0.0),
                f.real_volume
                    .as_ref()
                    .and_then(|v| v.first())
                    .map(|v| *v as f64)
                    .unwrap_or(0.0),
            )
        });

        self.overlays = self.pack_overlays(
            &mut states,
            &all_times,
            &opens,
            &highs,
            &lows,
            &closes,
            &tick_vols,
            &real_vols,
            forming_preview,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn pack_overlays(
        &self,
        states: &mut [StudyState],
        times: &[i64],
        opens: &[f64],
        highs: &[f64],
        lows: &[f64],
        closes: &[f64],
        tick_vols: &[f64],
        real_vols: &[f64],
        forming: Option<(i64, f64, f64, f64, f64, f64, f64)>,
    ) -> Vec<OverlaySeries> {
        let mut out = Vec::new();
        let n = times.len();
        let vol_kind = self.volume_kind;
        let vwap_ok = self.vwap_unavailable_reason.is_none();

        for (spec, state) in self.specs.iter().zip(states.iter_mut()) {
            let color = self.study_palette[spec.palette_index as usize % 8];
            let ms_at = |i: usize| markers::normalize_ms(times[i]);

            let mut push_line = |key: &str,
                                 label: &str,
                                 pane: Pane,
                                 osc_slot: Option<u8>,
                                 pts: Vec<(i64, Option<f64>)>| {
                out.push(OverlaySeries {
                    key: key.to_string(),
                    label: label.to_string(),
                    pane,
                    rgba: color,
                    color_role: Some(spec.palette_index),
                    osc_slot,
                    points: pts,
                });
            };

            match spec.kind {
                StudyKind::Sma => {
                    let mut pts = Vec::with_capacity(n + 1);
                    for i in 0..n {
                        let v = spec.source.value(opens[i], highs[i], lows[i], closes[i]);
                        let val = state_as_sma(state).commit(v);
                        pts.push((ms_at(i), finite_opt(val)));
                    }
                    if let Some((t, o, h, l, c, _, _)) = forming {
                        let v = spec.source.value(o, h, l, c);
                        let val = state_as_sma(state).preview(v);
                        pts.push((markers::normalize_ms(t), finite_opt(val)));
                    }
                    push_line(
                        &format!("study-{}-sma", spec.id),
                        &format!("SMA {}", spec.period),
                        Pane::Price,
                        None,
                        pts,
                    );
                }
                StudyKind::Ema => {
                    let mut pts = Vec::with_capacity(n + 1);
                    for i in 0..n {
                        let v = spec.source.value(opens[i], highs[i], lows[i], closes[i]);
                        let val = state_as_ema(state).commit(v);
                        pts.push((ms_at(i), finite_opt(val)));
                    }
                    if let Some((t, o, h, l, c, _, _)) = forming {
                        let v = spec.source.value(o, h, l, c);
                        let val = state_as_ema(state).preview(v);
                        pts.push((markers::normalize_ms(t), finite_opt(val)));
                    }
                    push_line(
                        &format!("study-{}-ema", spec.id),
                        &format!("EMA {}", spec.period),
                        Pane::Price,
                        None,
                        pts,
                    );
                }
                StudyKind::Bollinger => {
                    let mut mid = Vec::with_capacity(n + 1);
                    let mut up = Vec::with_capacity(n + 1);
                    let mut lo = Vec::with_capacity(n + 1);
                    for i in 0..n {
                        let v = spec.source.value(opens[i], highs[i], lows[i], closes[i]);
                        let b = state_as_bollinger(state).commit(v);
                        mid.push((ms_at(i), finite_opt(b.middle)));
                        up.push((ms_at(i), finite_opt(b.upper)));
                        lo.push((ms_at(i), finite_opt(b.lower)));
                    }
                    if let Some((t, o, h, l, c, _, _)) = forming {
                        let v = spec.source.value(o, h, l, c);
                        let b = state_as_bollinger(state).preview(v);
                        let ms = markers::normalize_ms(t);
                        mid.push((ms, finite_opt(b.middle)));
                        up.push((ms, finite_opt(b.upper)));
                        lo.push((ms, finite_opt(b.lower)));
                    }
                    let id = spec.id;
                    push_line(
                        &format!("study-{}-bb-mid", id),
                        &format!("BB {}", spec.period),
                        Pane::Price,
                        None,
                        mid,
                    );
                    push_line(
                        &format!("study-{}-bb-up", id),
                        &format!("BB+{}", spec.num_std),
                        Pane::Price,
                        None,
                        up,
                    );
                    push_line(
                        &format!("study-{}-bb-lo", id),
                        &format!("BB-{}", spec.num_std),
                        Pane::Price,
                        None,
                        lo,
                    );
                }
                StudyKind::SessionVwap if vwap_ok && vol_kind.is_some() => {
                    let vk = vol_kind.unwrap();
                    let mut vwap_pts = Vec::with_capacity(n + 1);
                    let mut up_pts = Vec::with_capacity(n + 1);
                    let mut lo_pts = Vec::with_capacity(n + 1);
                    for i in 0..n {
                        let vol = match vk {
                            VolumeKind::Real => real_vols[i],
                            VolumeKind::Tick => tick_vols[i],
                        };
                        let session = session_day_key(times[i], self.label);
                        let input =
                            SessionVwapInput::new(highs[i], lows[i], closes[i], vol, session);
                        if let Ok(out) = state_as_vwap(state).commit(input) {
                            vwap_pts.push((ms_at(i), finite_opt(out.vwap)));
                            let band = spec.num_std * out.std_dev;
                            up_pts.push((ms_at(i), finite_opt(out.vwap + band)));
                            lo_pts.push((ms_at(i), finite_opt(out.vwap - band)));
                        }
                    }
                    if let Some((t, _, h, l, c, tv, rv)) = forming {
                        let vol = match vk {
                            VolumeKind::Real => rv,
                            VolumeKind::Tick => tv,
                        };
                        let session = session_day_key(t, self.label);
                        let input = SessionVwapInput::new(h, l, c, vol, session);
                        if let Ok(out) = state_as_vwap(state).preview(input) {
                            let ms = markers::normalize_ms(t);
                            vwap_pts.push((ms, finite_opt(out.vwap)));
                            let band = spec.num_std * out.std_dev;
                            up_pts.push((ms, finite_opt(out.vwap + band)));
                            lo_pts.push((ms, finite_opt(out.vwap - band)));
                        }
                    }
                    let id = spec.id;
                    push_line(
                        &format!("study-{}-vwap", id),
                        "VWAP",
                        Pane::Price,
                        None,
                        vwap_pts,
                    );
                    push_line(
                        &format!("study-{}-vwap-up", id),
                        &format!("VWAP+{}", spec.num_std),
                        Pane::Price,
                        None,
                        up_pts,
                    );
                    push_line(
                        &format!("study-{}-vwap-lo", id),
                        &format!("VWAP-{}", spec.num_std),
                        Pane::Price,
                        None,
                        lo_pts,
                    );
                }
                StudyKind::Rsi => {
                    let mut pts = Vec::with_capacity(n + 1);
                    for (i, &close) in closes.iter().enumerate().take(n) {
                        let val = state_as_rsi(state).commit(close);
                        pts.push((ms_at(i), finite_opt(val)));
                    }
                    if let Some((t, _, _, _, c, _, _)) = forming {
                        let val = state_as_rsi(state).preview(c);
                        pts.push((markers::normalize_ms(t), finite_opt(val)));
                    }
                    push_line(
                        &format!("study-{}-rsi", spec.id),
                        &format!("RSI {}", spec.period),
                        Pane::Oscillator,
                        Some(spec.id as u8),
                        pts,
                    );
                }
                StudyKind::Atr => {
                    let mut pts = Vec::with_capacity(n + 1);
                    for i in 0..n {
                        let val =
                            state_as_atr(state).commit(AtrInput::new(highs[i], lows[i], closes[i]));
                        pts.push((ms_at(i), finite_opt(val)));
                    }
                    if let Some((t, _, h, l, c, _, _)) = forming {
                        let val = state_as_atr(state).preview(AtrInput::new(h, l, c));
                        pts.push((markers::normalize_ms(t), finite_opt(val)));
                    }
                    push_line(
                        &format!("study-{}-atr", spec.id),
                        &format!("ATR {}", spec.period),
                        Pane::Oscillator,
                        Some(spec.id as u8),
                        pts,
                    );
                }
                _ => {}
            }
        }
        out
    }
}

fn validate_study_params(kind: StudyKind, period: i64, num_std: f64) -> Result<(), String> {
    match kind {
        StudyKind::Sma => SmaState::new(period)
            .map(|_| ())
            .map_err(|e| e.to_string())?,
        StudyKind::Ema => EmaState::new(period)
            .map(|_| ())
            .map_err(|e| e.to_string())?,
        StudyKind::Bollinger => BollingerState::new(period, num_std)
            .map(|_| ())
            .map_err(|e| e.to_string())?,
        StudyKind::SessionVwap => {}
        StudyKind::Rsi => RsiState::new(period)
            .map(|_| ())
            .map_err(|e| e.to_string())?,
        StudyKind::Atr => AtrState::new(period)
            .map(|_| ())
            .map_err(|e| e.to_string())?,
    }
    Ok(())
}

fn new_state(spec: &StudySpec) -> StudyState {
    match spec.kind {
        StudyKind::Sma => StudyState::Sma(SmaState::new(spec.period).expect("valid sma")),
        StudyKind::Ema => StudyState::Ema(EmaState::new(spec.period).expect("valid ema")),
        StudyKind::Bollinger => StudyState::Bollinger(
            BollingerState::new(spec.period, spec.num_std).expect("valid bollinger"),
        ),
        StudyKind::SessionVwap => StudyState::SessionVwap(SessionVwapState::new()),
        StudyKind::Rsi => StudyState::Rsi(RsiState::new(spec.period).expect("valid rsi")),
        StudyKind::Atr => StudyState::Atr(AtrState::new(spec.period).expect("valid atr")),
    }
}

fn finite_opt(v: f64) -> Option<f64> {
    if v.is_finite() {
        Some(v)
    } else {
        None
    }
}

fn state_as_sma(s: &mut StudyState) -> &mut SmaState {
    match s {
        StudyState::Sma(st) => st,
        _ => panic!("expected sma state"),
    }
}

fn state_as_ema(s: &mut StudyState) -> &mut EmaState {
    match s {
        StudyState::Ema(st) => st,
        _ => panic!("expected ema state"),
    }
}

fn state_as_bollinger(s: &mut StudyState) -> &mut BollingerState {
    match s {
        StudyState::Bollinger(st) => st,
        _ => panic!("expected bollinger state"),
    }
}

fn state_as_vwap(s: &mut StudyState) -> &mut SessionVwapState {
    match s {
        StudyState::SessionVwap(st) => st,
        _ => panic!("expected vwap state"),
    }
}

fn state_as_rsi(s: &mut StudyState) -> &mut RsiState {
    match s {
        StudyState::Rsi(st) => st,
        _ => panic!("expected rsi state"),
    }
}

fn state_as_atr(s: &mut StudyState) -> &mut AtrState {
    match s {
        StudyState::Atr(st) => st,
        _ => panic!("expected atr state"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use q_indicators::{session_vwap, sma};

    fn bars_from_close(close: &[f64], vol: Option<i64>) -> BarColumns {
        let n = close.len();
        BarColumns {
            time: (0..n).map(|i| (i as i64 + 1) * 60_000_000).collect(),
            open: close.to_vec(),
            high: close.iter().map(|c| c + 1.0).collect(),
            low: close.iter().map(|c| c - 1.0).collect(),
            close: close.to_vec(),
            tick_volume: vol.map(|_| vec![100; n]),
            spread: None,
            real_volume: vol.map(|v| vec![v; n]),
            label: TimeLabel::Utc,
        }
    }

    #[test]
    fn sma_matches_batch_kernel() {
        let close = [10.0, 20.0, 30.0, 40.0, 50.0];
        let batch = sma(&close, 3).unwrap();
        let mut set = StudySet::new();
        set.study_palette = [0xff0000ff; 8];
        set.add_study(StudyKind::Sma, 3, PriceSource::Close, 2.0)
            .unwrap();
        let bars = bars_from_close(&close, None);
        set.rebuild(TimeLabel::Utc, &[bars], None);
        let line = set
            .overlays()
            .iter()
            .find(|s| s.key.contains("sma"))
            .unwrap();
        for (i, &exp) in batch.iter().enumerate() {
            let got = line.points[i].1;
            if exp.is_finite() {
                assert_eq!(got.unwrap().to_bits(), exp.to_bits(), "index {i}");
            } else {
                assert!(got.is_none(), "index {i}: expected no point for NaN warmup");
            }
        }
    }

    #[test]
    fn forming_preview_does_not_commit() {
        let close = [10.0, 20.0, 30.0];
        let mut set = StudySet::new();
        set.study_palette = [0xff0000ff; 8];
        set.add_study(StudyKind::Ema, 2, PriceSource::Close, 2.0)
            .unwrap();
        let bars = bars_from_close(&close, None);
        set.rebuild(TimeLabel::Utc, std::slice::from_ref(&bars), None);
        let committed_last = set.overlays()[0].points.last().unwrap().1;
        let forming = bars_from_close(&[99.0], None);
        set.rebuild(TimeLabel::Utc, std::slice::from_ref(&bars), Some(&forming));
        let preview_last = set.overlays()[0].points.last().unwrap().1;
        assert_ne!(preview_last, committed_last);
        set.rebuild(TimeLabel::Utc, &[bars], None);
        assert_eq!(set.overlays()[0].points.len(), 3);
    }

    #[test]
    fn vwap_unavailable_on_mixed_volume() {
        let mut set = StudySet::new();
        set.add_study(StudyKind::SessionVwap, 1, PriceSource::Close, 2.0)
            .unwrap();
        let tick_only = BarColumns {
            time: vec![60_000_000],
            open: vec![1.0],
            high: vec![2.0],
            low: vec![0.5],
            close: vec![1.5],
            tick_volume: Some(vec![10]),
            spread: None,
            real_volume: None,
            label: TimeLabel::Utc,
        };
        let real_only = BarColumns {
            time: vec![120_000_000],
            open: vec![1.0],
            high: vec![2.0],
            low: vec![0.5],
            close: vec![1.5],
            tick_volume: None,
            spread: None,
            real_volume: Some(vec![10]),
            label: TimeLabel::Utc,
        };
        set.rebuild(TimeLabel::Utc, &[tick_only, real_only], None);
        assert_eq!(
            set.vwap_unavailable_reason().unwrap(),
            "mixed tick and real volume"
        );
        assert!(set.overlays().is_empty());
    }

    #[test]
    fn session_key_changes_reset_vwap() {
        let mut set = StudySet::new();
        set.add_study(StudyKind::SessionVwap, 1, PriceSource::Close, 1.0)
            .unwrap();
        let day1 = 1_767_225_600_000_000i64; // 2026-01-01 approx in us - use two distinct keys
        let bars = BarColumns {
            time: vec![day1, day1 + 86_400_000_000],
            open: vec![10.0, 10.0],
            high: vec![12.0, 12.0],
            low: vec![8.0, 8.0],
            close: vec![10.0, 20.0],
            tick_volume: Some(vec![100, 100]),
            spread: None,
            real_volume: None,
            label: TimeLabel::Utc,
        };
        let high = bars.high.clone();
        let low = bars.low.clone();
        let close = bars.close.clone();
        let vol: Vec<f64> = vec![100.0, 100.0];
        let session: Vec<i64> = bars
            .time
            .iter()
            .map(|t| session_day_key(*t, TimeLabel::Utc))
            .collect();
        let batch_vwap = session_vwap(&high, &low, &close, &vol, &session).unwrap();
        set.rebuild(TimeLabel::Utc, &[bars], None);
        let line = set
            .overlays()
            .iter()
            .find(|s| s.key.contains("vwap"))
            .unwrap();
        assert_eq!(
            line.points[1].1.unwrap().to_bits(),
            batch_vwap.vwap[1].to_bits()
        );
    }
}
