//! Live chart studies backed by `q-indicators` streaming state (Q-075).

use q_buffers::frame::{BarColumns, TimeLabel};
use q_indicators::{
    AtrInput, AtrState, BollingerState, EmaState, RsiState, SessionVwapInput, SessionVwapState,
    SmaState,
};

use q_indicators::volume::AggressorSide;

use crate::execution::markers::{self, Marker, MarkerKind};
use crate::execution::overlays::{OverlaySeries, Pane};
use crate::trades::analysis::{Analysis, BarAggregate, VolumeParams};
use crate::trades::time::ExchangeClock;

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
    /// Tape-derived studies (Q-082); their numbers come from the Q-081 volume kernel.
    Delta,
    CumulativeDelta,
    TradeRate,
    LargePrints,
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
            "delta" => Some(Self::Delta),
            "cumulative_delta" => Some(Self::CumulativeDelta),
            "trade_rate" => Some(Self::TradeRate),
            "large_prints" => Some(Self::LargePrints),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sma => "sma",
            Self::Ema => "ema",
            Self::Bollinger => "bollinger",
            Self::SessionVwap => "vwap",
            Self::Rsi => "rsi",
            Self::Atr => "atr",
            Self::Delta => "delta",
            Self::CumulativeDelta => "cumulative_delta",
            Self::TradeRate => "trade_rate",
            Self::LargePrints => "large_prints",
        }
    }

    /// Studies computed from the trade tape rather than from bars.
    pub fn is_volume(self) -> bool {
        matches!(
            self,
            Self::Delta | Self::CumulativeDelta | Self::TradeRate | Self::LargePrints
        )
    }
}

/// The kernel output of every chart bar for one parameter set, by bar label time.
type VolumeRows = ((i64, u64), Vec<(i64, BarAggregate)>);

/// What the legend, picker and readout say about the tape behind a volume study.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TradeLegend {
    pub unit: String,
    pub field: String,
    /// `complete`, `partial`, `loading`, `stale`, `unavailable` or `idle`.
    pub coverage: String,
    pub source_coverage: String,
    pub classified_share: Option<f64>,
}

/// The tape analyses a rebuild draws volume studies from.
pub struct TradeInputs<'a> {
    pub analyses: &'a [Analysis],
    pub clock: ExchangeClock,
    pub legend: TradeLegend,
}

impl TradeInputs<'_> {
    fn analysis(&self, params: VolumeParams) -> Option<&Analysis> {
        self.analyses
            .iter()
            .find(|a| a.params().key() == params.key())
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
    pub visible: bool,
    /// Typed parameters of the tape studies; the price studies leave them at their defaults.
    pub volume: VolumeParams,
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
    legend: TradeLegend,
    /// Kernel output of each chart bar for each distinct parameter set, by bar label time.
    volume_rows: Vec<VolumeRows>,
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
            legend: TradeLegend::default(),
            volume_rows: Vec::new(),
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

    pub fn restore_json(&mut self, json: &str) -> Result<(), String> {
        let entries: Vec<serde_json::Value> =
            serde_json::from_str(json).map_err(|e| e.to_string())?;
        self.clear();
        self.next_id = 1;
        for entry in entries.into_iter().take(MAX_STUDIES) {
            let Some(kind) = entry["kind"].as_str().and_then(StudyKind::parse) else {
                eprintln!("workspace: dropped study with unknown kind");
                continue;
            };
            if kind.is_volume() {
                let defaults = VolumeParams::default();
                let params = VolumeParams {
                    window_ms: entry["window_ms"].as_i64().unwrap_or(defaults.window_ms),
                    large_print_threshold: entry["large_print_threshold"]
                        .as_f64()
                        .unwrap_or(defaults.large_print_threshold),
                };
                if self.add_volume_study(kind, params).is_err() {
                    eprintln!("workspace: dropped study with invalid parameters");
                    continue;
                }
                if let Some(spec) = self.specs.last_mut() {
                    spec.palette_index = entry["palette_index"]
                        .as_u64()
                        .map_or(spec.palette_index, |p| p.min(7) as u8);
                    spec.visible = entry["visible"].as_bool().unwrap_or(true);
                }
                continue;
            }
            let Some(period) = entry["period"].as_i64() else {
                eprintln!("workspace: dropped study with invalid period");
                continue;
            };
            let Some(source) = entry["source"].as_str().and_then(PriceSource::parse) else {
                eprintln!("workspace: dropped study with unknown price source");
                continue;
            };
            let Some(num_std) = entry["num_std"].as_f64() else {
                eprintln!("workspace: dropped study with invalid deviation");
                continue;
            };
            let palette = entry["palette_index"]
                .as_u64()
                .unwrap_or((self.specs.len() % 8) as u64) as u8;
            if self.add_study(kind, period, source, num_std).is_err() {
                eprintln!("workspace: dropped study with invalid parameters");
                continue;
            }
            if let Some(spec) = self.specs.last_mut() {
                spec.palette_index = palette.min(7);
                spec.visible = entry["visible"].as_bool().unwrap_or(true);
            }
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        self.specs.clear();
        self.reset_values();
    }

    pub fn reset_values(&mut self) {
        self.overlays.clear();
        self.volume_rows.clear();
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
            visible: true,
            volume: VolumeParams::default(),
        });
        Ok(id)
    }

    /// Adds a tape study. The window applies to the trade rate and the threshold to large
    /// prints; the kernel validates both, so nothing here second-guesses its ranges.
    pub fn add_volume_study(
        &mut self,
        kind: StudyKind,
        params: VolumeParams,
    ) -> Result<u64, String> {
        if !kind.is_volume() {
            return Err("not a volume study".into());
        }
        if self.specs.len() >= MAX_STUDIES {
            return Err("at most eight studies per chart".into());
        }
        params.validate().map_err(|e| e.to_string())?;
        let id = self.next_id;
        self.next_id += 1;
        let palette_index = (self.specs.len() % 8) as u8;
        self.specs.push(StudySpec {
            id,
            kind,
            period: 1,
            source: PriceSource::Close,
            num_std: 2.0,
            palette_index,
            visible: true,
            volume: params,
        });
        Ok(id)
    }

    pub fn update_volume_study(&mut self, id: u64, params: VolumeParams) -> Result<(), String> {
        params.validate().map_err(|e| e.to_string())?;
        match self
            .specs
            .iter_mut()
            .find(|s| s.id == id && s.kind.is_volume())
        {
            Some(spec) => {
                spec.volume = params;
                Ok(())
            }
            None => Err(format!("volume study id {id} not found")),
        }
    }

    /// The kernel parameters of the tape studies, for the feed to analyse under.
    pub fn volume_params(&self) -> Vec<VolumeParams> {
        self.specs
            .iter()
            .filter(|s| s.kind.is_volume())
            .map(|s| s.volume)
            .collect()
    }

    /// The threshold of the first large-print study, which decides what the tape flags.
    pub fn large_print_threshold(&self) -> Option<f64> {
        self.specs
            .iter()
            .find(|s| s.kind == StudyKind::LargePrints)
            .map(|s| s.volume.large_print_threshold)
    }

    pub fn has_volume_studies(&self) -> bool {
        self.specs.iter().any(|s| s.kind.is_volume())
    }

    pub fn legend(&self) -> &TradeLegend {
        &self.legend
    }

    pub fn remove_study(&mut self, id: u64) -> bool {
        if let Some(i) = self.specs.iter().position(|s| s.id == id) {
            self.specs.remove(i);
            true
        } else {
            false
        }
    }

    pub fn set_visible(&mut self, id: u64, visible: bool) -> bool {
        if let Some(spec) = self.specs.iter_mut().find(|spec| spec.id == id) {
            spec.visible = visible;
            true
        } else {
            false
        }
    }

    pub fn update_study(
        &mut self,
        id: u64,
        period: i64,
        source: PriceSource,
        num_std: f64,
    ) -> Result<(), String> {
        if let Some(spec) = self.specs.iter_mut().find(|s| s.id == id) {
            validate_study_params(spec.kind, period, num_std)?;
            spec.period = period;
            spec.source = source;
            spec.num_std = num_std;
            Ok(())
        } else {
            Err(format!("study id {id} not found"))
        }
    }

    pub fn study_list_json(&self) -> String {
        let mut items: Vec<serde_json::Value> = Vec::new();
        for s in &self.specs {
            let source = match s.source {
                PriceSource::Close => "close",
                PriceSource::Open => "open",
                PriceSource::High => "high",
                PriceSource::Low => "low",
                PriceSource::Hlc3 => "hlc3",
            };
            let name = match s.kind {
                StudyKind::Sma => format!("SMA ({})", s.period),
                StudyKind::Ema => format!("EMA ({})", s.period),
                StudyKind::Bollinger => format!("Bollinger ({}, {}σ)", s.period, s.num_std),
                StudyKind::SessionVwap => format!("Session VWAP ({}σ)", s.num_std),
                StudyKind::Rsi => format!("RSI ({})", s.period),
                StudyKind::Atr => format!("ATR ({})", s.period),
                _ => self.volume_name(s),
            };
            let mut item = serde_json::json!({
                "id": s.id,
                "kind": s.kind.as_str(),
                "name": name,
                "period": s.period,
                "source": source,
                "num_std": s.num_std,
                "palette_index": s.palette_index,
                "visible": s.visible,
            });
            if s.kind.is_volume() {
                item["volume"] = true.into();
                item["window_ms"] = s.volume.window_ms.into();
                item["large_print_threshold"] = s.volume.large_print_threshold.into();
                item["unit"] = self.legend.unit.clone().into();
                item["coverage"] = self.legend.coverage.clone().into();
            }
            items.push(item);
        }
        serde_json::json!({
            "studies": items,
            "vwap_unavailable": self.vwap_unavailable_reason,
            "tape": {
                "unit": self.legend.unit,
                "field": self.legend.field,
                "coverage": self.legend.coverage,
                "source_coverage": self.legend.source_coverage,
                "classified_share": self.legend.classified_share,
            },
        })
        .to_string()
    }

    fn unit(&self) -> &str {
        if self.legend.unit.is_empty() {
            "volume"
        } else {
            &self.legend.unit
        }
    }

    fn volume_name(&self, s: &StudySpec) -> String {
        let unit = self.unit();
        match s.kind {
            StudyKind::Delta => format!("Delta ({unit})"),
            StudyKind::CumulativeDelta => format!("Cumulative delta ({unit})"),
            StudyKind::TradeRate => {
                format!("Trade rate (trades/s, {}s)", s.volume.window_ms / 1000)
            }
            _ => format!(
                "Large prints (≥ {} {unit})",
                quantity(s.volume.large_print_threshold)
            ),
        }
    }

    /// The aggregate of the chart bar labelled `time_ms`, for the study's parameters.
    fn bar_for(&self, spec: &StudySpec, time_ms: i64) -> Option<&BarAggregate> {
        let rows = &self
            .volume_rows
            .iter()
            .find(|(key, _)| *key == spec.volume.key())?
            .1;
        rows.binary_search_by_key(&time_ms, |(t, _)| *t)
            .ok()
            .map(|i| &rows[i].1)
    }

    /// What the readout and legend say for one tape study at one bar, units and coverage
    /// included. A bar the tape never reached says so instead of showing zero.
    fn volume_text(&self, spec: &StudySpec, bar: Option<&BarAggregate>) -> String {
        let unit = self.unit();
        let Some(bar) = bar else {
            return match self.legend.coverage.as_str() {
                "complete" | "partial" | "stale" => "no tape for this bar".to_string(),
                other => format!("tape {other}"),
            };
        };
        let classified = bar
            .classified_share
            .filter(|share| *share > 0.0)
            .map(|share| format!("{:.0}% classified", share * 100.0));
        let mut text = match spec.kind {
            StudyKind::Delta if bar.classified_share.is_some_and(|share| share <= 0.0) => {
                "no aggressor side reported".to_string()
            }
            StudyKind::Delta => format!("{} {unit}", signed_quantity(bar.delta)),
            StudyKind::CumulativeDelta => {
                format!("{} {unit} session", signed_quantity(bar.cumulative_delta))
            }
            StudyKind::TradeRate => match bar.trade_rate {
                Some(rate) => format!("{rate:.1} trades/s"),
                None => "warming up".to_string(),
            },
            _ => format!("{} large", bar.large_prints),
        };
        if let (Some(c), StudyKind::Delta | StudyKind::CumulativeDelta) = (classified, spec.kind) {
            text.push_str(&format!(" · {c}"));
        }
        if self.legend.coverage != "complete" {
            text.push_str(&format!(" · {}", self.legend.coverage));
        }
        text
    }

    /// Values are looked up in the cached plotted series so the legend and chart cannot
    /// diverge through a second indicator calculation. Composite studies expose centre.
    pub fn values_json(&self, time_ms: i64) -> String {
        let values = self
            .specs
            .iter()
            .map(|spec| {
                if spec.kind.is_volume() {
                    let bar = self.bar_for(spec, time_ms);
                    let value = match spec.kind {
                        StudyKind::LargePrints => bar.map(|b| b.large_prints as f64),
                        _ => self.plotted(spec, time_ms),
                    };
                    return serde_json::json!({
                        "id": spec.id,
                        "value": value,
                        "text": self.volume_text(spec, bar),
                    });
                }
                serde_json::json!({"id": spec.id, "value": self.plotted(spec, time_ms)})
            })
            .collect::<Vec<_>>();
        serde_json::Value::Array(values).to_string()
    }

    fn plotted(&self, spec: &StudySpec, time_ms: i64) -> Option<f64> {
        let suffix = match spec.kind {
            StudyKind::Sma => "sma",
            StudyKind::Ema => "ema",
            StudyKind::Bollinger => "bb-mid",
            StudyKind::SessionVwap => "vwap",
            StudyKind::Rsi => "rsi",
            StudyKind::Atr => "atr",
            StudyKind::Delta => "delta",
            StudyKind::CumulativeDelta => "cvd",
            StudyKind::TradeRate => "rate",
            StudyKind::LargePrints => return None,
        };
        let key = format!("study-{}-{suffix}", spec.id);
        self.overlays
            .iter()
            .find(|line| line.key == key)
            .and_then(|line| line.points.iter().find(|(time, _)| *time == time_ms))
            .and_then(|(_, value)| *value)
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
        self.rebuild_with(label, completed, forming, None);
    }

    /// Like [`rebuild`](Self::rebuild), with the tape the volume studies are drawn from.
    pub fn rebuild_with(
        &mut self,
        label: TimeLabel,
        completed: &[BarColumns],
        forming: Option<&BarColumns>,
        trades: Option<&TradeInputs>,
    ) {
        self.label = label;
        self.legend = trades.map(|t| t.legend.clone()).unwrap_or_default();
        if self.specs.is_empty() {
            self.overlays.clear();
            self.volume_rows.clear();
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

        self.build_volume_rows(&all_times, forming_preview.map(|f| f.0), trades);
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

    /// The UTC open of the chart bar labelled `label_ms`.
    fn utc_of(&self, label_ms: i64, trades: Option<&TradeInputs>) -> i64 {
        match (self.label, trades) {
            (TimeLabel::BrasiliaWallclock, Some(t)) => t.clock.local_to_utc_ms(label_ms),
            _ => label_ms,
        }
    }

    /// Looks up the kernel's output for each chart bar once, per distinct parameter set.
    fn build_volume_rows(
        &mut self,
        times: &[i64],
        forming: Option<i64>,
        trades: Option<&TradeInputs>,
    ) {
        self.volume_rows.clear();
        let Some(inputs) = trades else {
            return;
        };
        let labels: Vec<i64> = times
            .iter()
            .copied()
            .chain(forming)
            .map(markers::normalize_ms)
            .collect();
        let mut seen: Vec<(i64, u64)> = Vec::new();
        for spec in self.specs.iter().filter(|s| s.kind.is_volume()) {
            let key = spec.volume.key();
            if seen.contains(&key) {
                continue;
            }
            seen.push(key);
            let Some(analysis) = inputs.analysis(spec.volume) else {
                continue;
            };
            // Bars before the session's first one have no tape: skip them without a lookup.
            let first_open = analysis.bars().first().map_or(i64::MAX, |b| b.open_ms);
            let start = labels.partition_point(|&label| self.utc_of(label, trades) < first_open);
            let rows = labels[start..]
                .iter()
                .filter_map(|&label| {
                    analysis
                        .bar(self.utc_of(label, trades))
                        .map(|bar| (label, bar.clone()))
                })
                .collect();
            self.volume_rows.push((key, rows));
        }
    }

    /// Markers for the large prints of every large-print study: at most the latest 1000 per
    /// study. `bar_opens` are the chart bars' label times in milliseconds (ascending) with the
    /// forming bar last, and each marker's `bar_index` indexes it.
    pub fn large_print_markers(&self, bar_opens: &[i64], trades: &TradeInputs) -> Vec<Marker> {
        let unit = self.unit();
        let mut out = Vec::new();
        for spec in self
            .specs
            .iter()
            .filter(|s| s.visible && s.kind == StudyKind::LargePrints)
        {
            let Some(analysis) = trades.analysis(spec.volume) else {
                continue;
            };
            for print in analysis.prints() {
                let label = match self.label {
                    TimeLabel::BrasiliaWallclock => trades.clock.utc_to_local_ms(print.bar_open_ms),
                    _ => print.bar_open_ms,
                };
                let Ok(index) = bar_opens.binary_search(&label) else {
                    continue;
                };
                let (kind, side) = match print.side {
                    AggressorSide::Buy => (MarkerKind::LargeBuy, "BUY"),
                    AggressorSide::Sell => (MarkerKind::LargeSell, "SELL"),
                    AggressorSide::Unknown => (MarkerKind::LargeUnknown, "UNKNOWN SIDE"),
                };
                let bar = analysis.bar(print.bar_open_ms);
                let mut lines = vec![
                    format!("{side} print"),
                    format!("{} {unit} @ {}", quantity(print.volume), print.price),
                    format!("at {}", clock_text(print.time_msc)),
                ];
                if let Some(bar) = bar {
                    lines.push(format!(
                        "bar delta {} {unit}{}",
                        signed_quantity(bar.delta),
                        bar.classified_share
                            .map(|c| format!(" · {:.0}% classified", c * 100.0))
                            .unwrap_or_default()
                    ));
                }
                out.push(Marker {
                    id: format!("print-{}-{}", spec.id, print.time_msc),
                    bar_index: index,
                    bar_open_ms: label,
                    price: Some(print.price),
                    kind,
                    label: format!("LARGE {side}"),
                    detail: lines.join("\n"),
                });
            }
        }
        out
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
            if !spec.visible {
                continue;
            }
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
                StudyKind::Delta | StudyKind::CumulativeDelta | StudyKind::TradeRate => {
                    let rows = self
                        .volume_rows
                        .iter()
                        .find(|(key, _)| *key == spec.volume.key())
                        .map(|(_, rows)| rows.as_slice())
                        .unwrap_or(&[]);
                    let value = |bar: &BarAggregate| -> Option<f64> {
                        match spec.kind {
                            // A bar of unknown-side volume has no direction to report.
                            StudyKind::Delta => bar
                                .classified_share
                                .filter(|share| *share > 0.0)
                                .map(|_| bar.delta),
                            StudyKind::CumulativeDelta => Some(bar.cumulative_delta),
                            _ => bar.trade_rate,
                        }
                    };
                    // Only the session's bars have points; earlier chart bars stay gaps.
                    let pts: Vec<(i64, Option<f64>)> = rows
                        .iter()
                        .map(|(label, bar)| (*label, value(bar).and_then(finite_opt)))
                        .collect();
                    let (suffix, label) = match spec.kind {
                        StudyKind::Delta => ("delta", "Delta"),
                        StudyKind::CumulativeDelta => ("cvd", "CVD"),
                        _ => ("rate", "Trades/s"),
                    };
                    push_line(
                        &format!("study-{}-{suffix}", spec.id),
                        label,
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

/// A quantity without noise digits: whole numbers print whole.
fn quantity(value: f64) -> String {
    if (value - value.round()).abs() < 1e-9 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

fn signed_quantity(value: f64) -> String {
    let text = quantity(value.abs());
    if value < 0.0 {
        format!("−{text}")
    } else {
        format!("+{text}")
    }
}

fn clock_text(time_msc: i64) -> String {
    let ms = time_msc.rem_euclid(86_400_000);
    format!(
        "{:02}:{:02}:{:02}.{:03} UTC",
        ms / 3_600_000,
        ms % 3_600_000 / 60_000,
        ms % 60_000 / 1000,
        ms % 1000
    )
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
        StudyKind::Delta
        | StudyKind::CumulativeDelta
        | StudyKind::TradeRate
        | StudyKind::LargePrints => return Err("use add_volume_study for tape studies".into()),
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
        // Tape studies read the feed's analyses, not a streaming state of their own.
        _ => StudyState::SessionVwap(SessionVwapState::new()),
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
        let values: serde_json::Value =
            serde_json::from_str(&set.values_json(180_000_000_000)).unwrap();
        assert_eq!(values[0]["value"].as_f64(), Some(batch[2]));
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

    #[test]
    fn study_set_update_study_modifies_spec() {
        let mut set = StudySet::new();
        let id = set
            .add_study(StudyKind::Ema, 20, PriceSource::Close, 2.0)
            .unwrap();
        assert!(set.update_study(id, 50, PriceSource::Open, 2.0).is_ok());
        assert_eq!(set.specs()[0].period, 50);
        assert_eq!(set.specs()[0].source, PriceSource::Open);
        let json = set.study_list_json();
        assert!(json.contains("\"name\":\"EMA (50)\""));
        assert!(json.contains("\"period\":50"));
    }

    #[test]
    fn restore_drops_invalid_studies_and_keeps_valid_entries() {
        let mut set = StudySet::new();
        set.restore_json(r#"[
            {"kind":"ema","period":21,"source":"close","num_std":2.0,"visible":false,"palette_index":3},
            {"kind":"ema","period":0,"source":"close","num_std":2.0,"visible":true,"palette_index":0}
        ]"#).unwrap();
        assert_eq!(set.specs().len(), 1);
        assert_eq!(set.specs()[0].palette_index, 3);
        assert!(!set.specs()[0].visible);
    }

    #[test]
    fn target_reset_keeps_study_configuration_but_drops_old_values() {
        let mut set = StudySet::new();
        set.add_study(StudyKind::Ema, 2, PriceSource::Close, 2.0)
            .unwrap();
        let bars = bars_from_close(&[10.0, 20.0], None);
        set.rebuild(TimeLabel::Utc, std::slice::from_ref(&bars), None);
        let before: serde_json::Value =
            serde_json::from_str(&set.values_json(120_000_000_000)).unwrap();
        assert!(before[0]["value"].as_f64().is_some());
        set.reset_values();
        assert_eq!(set.specs().len(), 1);
        assert!(set.values_json(120_000_000_000).contains("null"));
    }
}
