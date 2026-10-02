//! Chart identity and market-data freshness projection for Q-055.
//!
//! Keeps target source, pending retarget, and feed condition distinct from the raw
//! `BarFeed` counters so QML only arranges and styles the labels.

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

use crate::bridge::bar_feed::BarFeedRust;

/// Formats a Unix-epoch bar time for the identity strip.
pub fn format_last_bar_time(epoch_secs: i64) -> String {
    if epoch_secs <= 0 {
        return "Last bar unavailable".to_string();
    }
    let secs = normalize_epoch_secs(epoch_secs);
    let (y, mo, d, hh, mm, ss) = utc_from_unix(secs);
    format!("{y:04}-{mo:02}-{d:02} {hh:02}:{mm:02}:{ss:02} UTC")
}

fn normalize_epoch_secs(t: i64) -> i64 {
    if t > 1_000_000_000_000_000 {
        t / 1_000_000
    } else if t > 1_000_000_000_000 {
        t / 1_000
    } else {
        t
    }
}

fn utc_from_unix(secs: i64) -> (i32, i32, i32, i32, i32, i32) {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let hh = (rem / 3_600) as i32;
    let mm = ((rem % 3_600) / 60) as i32;
    let ss = (rem % 60) as i32;
    civil_from_days(days as i32)
        .map(|(y, mo, d)| (y, mo, d, hh, mm, ss))
        .unwrap_or((1970, 1, 1, hh, mm, ss))
}

fn civil_from_days(days: i32) -> Option<(i32, i32, i32)> {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    Some((year, m, d))
}

/// Formats a data-age in milliseconds for stale labels.
pub fn format_age_ms(ms: i64) -> String {
    if ms < 0 {
        return String::new();
    }
    let sec = ms / 1_000;
    if sec < 60 {
        return format!("{sec}s");
    }
    let min = sec / 60;
    let rem = sec % 60;
    if min < 60 {
        return format!("{min}m {rem}s");
    }
    let hour = min / 60;
    let rem_min = min % 60;
    format!("{hour}h {rem_min}m")
}

/// Market-data condition label and semantic role from feed state.
pub fn compute_condition(feed: &BarFeedRust) -> (String, String) {
    let conn = feed.connection_state.to_string().to_lowercase();
    let history_error = feed.history_error.to_string();
    if !history_error.is_empty() {
        return (history_error, "critical".to_string());
    }
    if conn == "retrying" || conn == "unavailable" || conn == "error" {
        let label = if conn == "error" {
            "Error"
        } else {
            "Disconnected"
        };
        return (label.to_string(), "critical".to_string());
    }
    if conn == "connecting" {
        return ("Loading".to_string(), "warning".to_string());
    }
    if feed.history_loading {
        return ("Loading".to_string(), "warning".to_string());
    }
    let has_bars = feed.bar_count > 0 || feed.has_forming;
    if feed.live_only && !has_bars {
        return ("Loading".to_string(), "warning".to_string());
    }
    if feed.data_age_ms < 0 || feed.last_time <= 0 {
        if conn == "live" {
            return ("Loading".to_string(), "warning".to_string());
        }
        return ("Last bar unavailable".to_string(), "warning".to_string());
    }
    if feed.stale || feed.data_age_ms > feed.timeframe_ms {
        let age = format_age_ms(feed.data_age_ms);
        let label = if age.is_empty() {
            "Stale".to_string()
        } else {
            format!("Stale {age}")
        };
        return (label, "stale".to_string());
    }
    if conn == "live" && has_bars {
        return ("Live".to_string(), "positive".to_string());
    }
    ("Loading".to_string(), "warning".to_string())
}

pub fn source_label(following_name: Option<&str>) -> (String, String) {
    let (label, tooltip, _) = source_label_mode(following_name, false);
    (label, tooltip)
}

pub fn source_label_mode(following_name: Option<&str>, is_manual: bool) -> (String, String, bool) {
    if is_manual {
        match following_name {
            Some(name) if !name.is_empty() => (
                "Manual (diverged)".to_string(),
                format!("Manual · Diverged from selected deployment: {name}"),
                true,
            ),
            _ => (
                "Manual".to_string(),
                "Manual chart target".to_string(),
                false,
            ),
        }
    } else {
        match following_name {
            Some(name) if !name.is_empty() => {
                let label = format!("Following: {name}");
                (label, name.to_string(), false)
            }
            _ => ("Configured".to_string(), String::new(), false),
        }
    }
}

pub fn symbol_line(
    feed_symbol: &str,
    feed_timeframe: &str,
    switching: bool,
    pending_symbol: &str,
    pending_timeframe: &str,
) -> String {
    if switching && !pending_symbol.is_empty() && !pending_timeframe.is_empty() {
        return format!("Switching to {pending_symbol} · {pending_timeframe}");
    }
    format!("{feed_symbol} · {feed_timeframe} · Candles")
}

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, symbol_line)]
        #[qproperty(QString, source_label)]
        #[qproperty(QString, source_tooltip)]
        #[qproperty(QString, condition_label)]
        #[qproperty(QString, condition_role)]
        #[qproperty(QString, last_bar_label)]
        #[qproperty(bool, is_switching)]
        #[qproperty(i64, revision)]
        #[qproperty(QString, chart_mode)]
        #[qproperty(bool, is_manual)]
        #[qproperty(bool, is_diverged)]
        #[qproperty(QString, target_error)]
        #[qproperty(QString, active_symbol)]
        #[qproperty(QString, active_timeframe)]
        #[qproperty(QString, configured_symbol)]
        #[qproperty(QString, configured_timeframe)]
        #[qproperty(QString, recent_symbols_json)]
        #[qproperty(QString, last_manual_symbol)]
        #[qproperty(QString, last_manual_timeframe)]
        #[qproperty(QString, followed_deployment_id)]
        #[qproperty(bool, is_session_override)]
        type ChartContext = super::ChartContextRust;

        #[qinvokable]
        fn set_configured(self: Pin<&mut ChartContext>, symbol: QString, timeframe: QString);

        #[qinvokable]
        fn set_last_manual(self: Pin<&mut ChartContext>, symbol: QString, timeframe: QString);

        #[qinvokable]
        fn set_followed_deployment(self: Pin<&mut ChartContext>, id: QString);

        #[qinvokable]
        fn on_target(
            self: Pin<&mut ChartContext>,
            deployment_name: QString,
            symbol: QString,
            timeframe: QString,
            following: bool,
        );

        #[qinvokable]
        fn on_retarget(self: Pin<&mut ChartContext>, symbol: QString, timeframe: QString);

        #[qinvokable]
        fn request_target(
            self: Pin<&mut ChartContext>,
            symbol: QString,
            timeframe: QString,
        ) -> bool;

        #[qinvokable]
        fn follow_deployment(self: Pin<&mut ChartContext>);

        #[qinvokable]
        fn clear_error(self: Pin<&mut ChartContext>);

        #[qinvokable]
        fn sync(self: Pin<&mut ChartContext>);

        #[qinvokable]
        fn parse_target_draft(self: Pin<&mut ChartContext>, draft: QString) -> QString;
    }

    impl cxx_qt::Threading for ChartContext {}
}

#[derive(Clone, Default)]
struct PublishedProperties {
    symbol_line: QString,
    source_label: QString,
    source_tooltip: QString,
    condition_label: QString,
    condition_role: QString,
    last_bar_label: QString,
    is_switching: bool,
    revision: i64,
    chart_mode: QString,
    is_manual: bool,
    is_diverged: bool,
    target_error: QString,
    active_symbol: QString,
    active_timeframe: QString,
    configured_symbol: QString,
    configured_timeframe: QString,
    recent_symbols_json: QString,
    last_manual_symbol: QString,
    last_manual_timeframe: QString,
    followed_deployment_id: QString,
    is_session_override: bool,
}

pub struct ChartContextRust {
    pub symbol_line: QString,
    pub source_label: QString,
    pub source_tooltip: QString,
    pub condition_label: QString,
    pub condition_role: QString,
    pub last_bar_label: QString,
    pub is_switching: bool,
    pub revision: i64,
    pub chart_mode: QString,
    pub is_manual: bool,
    pub is_diverged: bool,
    pub target_error: QString,
    pub active_symbol: QString,
    pub active_timeframe: QString,
    pub configured_symbol: QString,
    pub configured_timeframe: QString,
    pub recent_symbols_json: QString,
    pub last_manual_symbol: QString,
    pub last_manual_timeframe: QString,
    pub followed_deployment_id: QString,
    pub is_session_override: bool,

    published: PublishedProperties,
    targeter: Option<std::sync::Arc<crate::chart_target::ChartTargeter>>,
    following_name: Option<String>,
    pending_symbol: String,
    pending_timeframe: String,
    switching: bool,
    feed_addr: usize,
    recents: Vec<String>,
}

impl Default for ChartContextRust {
    fn default() -> Self {
        Self {
            symbol_line: QString::from("· Candles"),
            source_label: QString::from("Configured"),
            source_tooltip: QString::from(""),
            condition_label: QString::from("Loading"),
            condition_role: QString::from("warning"),
            last_bar_label: QString::from("Last bar unavailable"),
            is_switching: false,
            revision: 0,
            chart_mode: QString::from("following"),
            is_manual: false,
            is_diverged: false,
            target_error: QString::from(""),
            active_symbol: QString::from(""),
            active_timeframe: QString::from(""),
            configured_symbol: QString::from(""),
            configured_timeframe: QString::from(""),
            recent_symbols_json: QString::from("[]"),
            last_manual_symbol: QString::from(""),
            last_manual_timeframe: QString::from(""),
            followed_deployment_id: QString::from(""),
            is_session_override: false,
            published: PublishedProperties::default(),
            targeter: None,
            following_name: None,
            pending_symbol: String::new(),
            pending_timeframe: String::new(),
            switching: false,
            feed_addr: 0,
            recents: Vec::new(),
        }
    }
}

impl ChartContextRust {
    pub fn following_name(&self) -> Option<&str> {
        self.following_name.as_deref()
    }

    pub fn bind_feed(&mut self, feed: *mut crate::bridge::bar_feed::ffi::BarFeed) {
        self.feed_addr = feed as usize;
    }

    pub fn bind_targeter(&mut self, targeter: std::sync::Arc<crate::chart_target::ChartTargeter>) {
        let is_manual = targeter.is_manual();
        self.is_manual = is_manual;
        self.chart_mode = QString::from(if is_manual { "manual" } else { "following" });
        let (man_sym, man_tf) = targeter.last_manual();
        self.last_manual_symbol = QString::from(&man_sym);
        self.last_manual_timeframe = QString::from(&man_tf);
        for s in targeter.recent_symbols() {
            if !self.recents.contains(&s) {
                self.recents.push(s);
            }
        }
        self.recent_symbols_json =
            QString::from(&serde_json::to_string(&self.recents).unwrap_or_else(|_| "[]".into()));
        self.targeter = Some(targeter);
        self.bump();
    }

    pub fn set_configured(&mut self, symbol: &str, timeframe: &str) {
        self.configured_symbol = QString::from(symbol);
        self.configured_timeframe = QString::from(timeframe);
        if self.active_symbol.to_string().is_empty() {
            self.active_symbol = QString::from(symbol);
        }
        if self.active_timeframe.to_string().is_empty() {
            self.active_timeframe = QString::from(timeframe);
        }
        if !symbol.is_empty() && !self.recents.iter().any(|s| s == symbol) {
            self.recents.push(symbol.to_string());
            self.recent_symbols_json = QString::from(
                &serde_json::to_string(&self.recents).unwrap_or_else(|_| "[]".into()),
            );
        }
        self.bump();
    }

    pub fn set_last_manual(&mut self, symbol: &str, timeframe: &str) {
        if let Ok((valid_sym, valid_tf)) = crate::chart_target::validate_target(symbol, timeframe) {
            self.last_manual_symbol = QString::from(&valid_sym);
            self.last_manual_timeframe = QString::from(&valid_tf);
            if let Some(targeter) = &self.targeter {
                targeter.set_last_manual(&valid_sym, &valid_tf);
            }
            self.bump();
        }
    }

    pub fn set_followed_deployment_id(&mut self, id: &str) {
        self.followed_deployment_id = QString::from(id);
        self.bump();
    }

    pub fn on_target(
        &mut self,
        deployment_name: &str,
        symbol: &str,
        timeframe: &str,
        following: bool,
    ) {
        let name_clean = if !deployment_name.is_empty() {
            Some(deployment_name.to_string())
        } else {
            None
        };
        self.following_name = name_clean;
        let is_manual = self.is_manual;
        if !is_manual && following {
            self.pending_symbol = symbol.to_string();
            self.pending_timeframe = timeframe.to_string();
            self.switching = true;
            self.is_switching = true;
        }
        let (source, tooltip, diverged) =
            source_label_mode(self.following_name.as_deref(), is_manual);
        self.source_label = QString::from(&source);
        self.source_tooltip = QString::from(&tooltip);
        self.is_diverged = diverged;
        self.bump();
    }

    pub fn on_retarget(&mut self, symbol: &str, timeframe: &str) {
        self.pending_symbol = symbol.to_string();
        self.pending_timeframe = timeframe.to_string();
        self.active_symbol = QString::from(symbol);
        self.active_timeframe = QString::from(timeframe);
        self.switching = true;
        self.is_switching = true;
        self.bump();
    }

    pub fn request_target(&mut self, symbol: &str, timeframe: &str) -> bool {
        let (valid_sym, valid_tf) = match crate::chart_target::validate_target(symbol, timeframe) {
            Ok(pair) => pair,
            Err(err) => {
                self.target_error = QString::from(&err);
                self.bump();
                return false;
            }
        };
        self.target_error = QString::from("");
        self.chart_mode = QString::from("manual");
        self.is_manual = true;
        self.is_session_override = false;
        self.last_manual_symbol = QString::from(&valid_sym);
        self.last_manual_timeframe = QString::from(&valid_tf);

        if let Some(targeter) = &self.targeter {
            targeter.add_recent_symbol(&valid_sym);
            self.recents = targeter.recent_symbols();
            self.recent_symbols_json = QString::from(
                &serde_json::to_string(&self.recents).unwrap_or_else(|_| "[]".into()),
            );
            match targeter.request_manual(&valid_sym, &valid_tf) {
                Ok(Some(target)) => {
                    self.on_retarget(&target.symbol, &target.timeframe);
                }
                Ok(None) => {
                    self.active_symbol = QString::from(&valid_sym);
                    self.active_timeframe = QString::from(&valid_tf);
                }
                Err(err) => {
                    self.target_error = QString::from(&err);
                    self.bump();
                    return false;
                }
            }
        } else {
            self.recents.retain(|s| s != &valid_sym);
            self.recents.insert(0, valid_sym.clone());
            if self.recents.len() > 10 {
                self.recents.truncate(10);
            }
            self.recent_symbols_json = QString::from(
                &serde_json::to_string(&self.recents).unwrap_or_else(|_| "[]".into()),
            );
            self.active_symbol = QString::from(&valid_sym);
            self.active_timeframe = QString::from(&valid_tf);
            self.on_retarget(&valid_sym, &valid_tf);
        }
        let (source, tooltip, diverged) = source_label_mode(self.following_name.as_deref(), true);
        self.source_label = QString::from(&source);
        self.source_tooltip = QString::from(&tooltip);
        self.is_diverged = diverged;
        self.bump();
        true
    }

    pub fn follow_deployment(&mut self) {
        self.target_error = QString::from("");
        self.chart_mode = QString::from("following");
        self.is_manual = false;
        self.is_diverged = false;

        if let Some(targeter) = &self.targeter {
            let retargeted = targeter.follow_deployment();
            if let Some(target) = retargeted {
                self.on_retarget(&target.symbol, &target.timeframe);
            } else if let Some((_id, _name, sym, tf)) = targeter.following_target() {
                self.active_symbol = QString::from(&sym);
                self.active_timeframe = QString::from(&tf);
            } else {
                self.active_symbol = self.configured_symbol.clone();
                self.active_timeframe = self.configured_timeframe.clone();
            }
        } else {
            self.active_symbol = self.configured_symbol.clone();
            self.active_timeframe = self.configured_timeframe.clone();
            self.on_retarget(
                &self.configured_symbol.to_string(),
                &self.configured_timeframe.to_string(),
            );
        }
        let (source, tooltip, diverged) = source_label_mode(self.following_name.as_deref(), false);
        self.source_label = QString::from(&source);
        self.source_tooltip = QString::from(&tooltip);
        self.is_diverged = diverged;
        self.bump();
    }

    pub fn clear_error(&mut self) {
        self.target_error = QString::from("");
        self.bump();
    }

    pub fn refresh_from_feed(&mut self, feed: &mut BarFeedRust) {
        feed.update_data_age();
        let feed_symbol = feed.symbol.to_string();
        let feed_timeframe = feed.timeframe.to_string();
        if !feed_symbol.is_empty() {
            self.active_symbol = QString::from(&feed_symbol);
        }
        if !feed_timeframe.is_empty() {
            self.active_timeframe = QString::from(&feed_timeframe);
        }

        let switching = self.switching
            && (feed.history_loading || feed.bar_count == 0)
            && !self.pending_symbol.is_empty();
        self.is_switching = switching;

        let is_manual = self.is_manual;
        let (source, tooltip, diverged) =
            source_label_mode(self.following_name.as_deref(), is_manual);
        self.source_label = QString::from(&source);
        self.source_tooltip = QString::from(&tooltip);
        self.is_diverged = diverged;

        let sym_line = symbol_line(
            &feed_symbol,
            &feed_timeframe,
            switching,
            &self.pending_symbol,
            &self.pending_timeframe,
        );
        self.symbol_line = QString::from(&sym_line);

        let (condition, role) = compute_condition(feed);
        self.condition_label = QString::from(&condition);
        self.condition_role = QString::from(&role);
        self.last_bar_label = QString::from(&format_last_bar_time(feed.last_time));

        if !feed.history_loading && feed.bar_count > 0 {
            self.switching = false;
        }
        self.bump();
    }

    fn bump(&mut self) {
        self.revision += 1;
    }
}

impl ffi::ChartContext {
    /// Rust projections assign fields directly. Emit only the properties that changed
    /// since the previous publication; generated setters cannot detect these assignments.
    fn sync_props(mut self: std::pin::Pin<&mut Self>) {
        let (dep_id, is_override) = {
            let r = self.rust();
            let dep_id = r
                .targeter
                .as_ref()
                .and_then(|t| t.following_target())
                .map(|(id, ..)| QString::from(&id))
                .unwrap_or_else(|| r.followed_deployment_id.clone());
            let is_override = r
                .targeter
                .as_ref()
                .map(|t| t.is_session_override())
                .unwrap_or(r.is_session_override);
            (dep_id, is_override)
        };
        {
            let mut r = self.as_mut().rust_mut();
            r.followed_deployment_id = dep_id;
            r.is_session_override = is_override;
        }
        let after = {
            let r = self.rust();
            PublishedProperties {
                symbol_line: r.symbol_line.clone(),
                source_label: r.source_label.clone(),
                source_tooltip: r.source_tooltip.clone(),
                condition_label: r.condition_label.clone(),
                condition_role: r.condition_role.clone(),
                last_bar_label: r.last_bar_label.clone(),
                is_switching: r.is_switching,
                revision: r.revision,
                chart_mode: r.chart_mode.clone(),
                is_manual: r.is_manual,
                is_diverged: r.is_diverged,
                target_error: r.target_error.clone(),
                active_symbol: r.active_symbol.clone(),
                active_timeframe: r.active_timeframe.clone(),
                configured_symbol: r.configured_symbol.clone(),
                configured_timeframe: r.configured_timeframe.clone(),
                recent_symbols_json: r.recent_symbols_json.clone(),
                last_manual_symbol: r.last_manual_symbol.clone(),
                last_manual_timeframe: r.last_manual_timeframe.clone(),
                followed_deployment_id: r.followed_deployment_id.clone(),
                is_session_override: r.is_session_override,
            }
        };
        let before = std::mem::replace(&mut self.as_mut().rust_mut().published, after.clone());
        if before.symbol_line != after.symbol_line {
            self.as_mut().symbol_line_changed();
        }
        if before.source_label != after.source_label {
            self.as_mut().source_label_changed();
        }
        if before.source_tooltip != after.source_tooltip {
            self.as_mut().source_tooltip_changed();
        }
        if before.condition_label != after.condition_label {
            self.as_mut().condition_label_changed();
        }
        if before.condition_role != after.condition_role {
            self.as_mut().condition_role_changed();
        }
        if before.last_bar_label != after.last_bar_label {
            self.as_mut().last_bar_label_changed();
        }
        if before.is_switching != after.is_switching {
            self.as_mut().is_switching_changed();
        }
        if before.chart_mode != after.chart_mode {
            self.as_mut().chart_mode_changed();
        }
        if before.is_manual != after.is_manual {
            self.as_mut().is_manual_changed();
        }
        if before.is_diverged != after.is_diverged {
            self.as_mut().is_diverged_changed();
        }
        if before.target_error != after.target_error {
            self.as_mut().target_error_changed();
        }
        if before.active_symbol != after.active_symbol {
            self.as_mut().active_symbol_changed();
        }
        if before.active_timeframe != after.active_timeframe {
            self.as_mut().active_timeframe_changed();
        }
        if before.configured_symbol != after.configured_symbol {
            self.as_mut().configured_symbol_changed();
        }
        if before.configured_timeframe != after.configured_timeframe {
            self.as_mut().configured_timeframe_changed();
        }
        if before.recent_symbols_json != after.recent_symbols_json {
            self.as_mut().recent_symbols_json_changed();
        }
        if before.last_manual_symbol != after.last_manual_symbol {
            self.as_mut().last_manual_symbol_changed();
        }
        if before.last_manual_timeframe != after.last_manual_timeframe {
            self.as_mut().last_manual_timeframe_changed();
        }
        if before.followed_deployment_id != after.followed_deployment_id {
            self.as_mut().followed_deployment_id_changed();
        }
        if before.is_session_override != after.is_session_override {
            self.as_mut().is_session_override_changed();
        }
        if before.revision != after.revision {
            self.as_mut().revision_changed();
        }
    }

    pub fn set_configured(mut self: std::pin::Pin<&mut Self>, symbol: QString, timeframe: QString) {
        self.as_mut()
            .rust_mut()
            .set_configured(&symbol.to_string(), &timeframe.to_string());
        self.sync_props();
    }

    pub fn set_last_manual(
        mut self: std::pin::Pin<&mut Self>,
        symbol: QString,
        timeframe: QString,
    ) {
        self.as_mut()
            .rust_mut()
            .set_last_manual(&symbol.to_string(), &timeframe.to_string());
        self.sync_props();
    }

    pub fn set_followed_deployment(mut self: std::pin::Pin<&mut Self>, id: QString) {
        self.as_mut()
            .rust_mut()
            .set_followed_deployment_id(&id.to_string());
        self.sync_props();
    }

    pub fn on_target(
        mut self: std::pin::Pin<&mut Self>,
        deployment_name: QString,
        symbol: QString,
        timeframe: QString,
        following: bool,
    ) {
        self.as_mut().rust_mut().on_target(
            &deployment_name.to_string(),
            &symbol.to_string(),
            &timeframe.to_string(),
            following,
        );
        self.sync_props();
    }

    pub fn on_retarget(mut self: std::pin::Pin<&mut Self>, symbol: QString, timeframe: QString) {
        self.as_mut()
            .rust_mut()
            .on_retarget(&symbol.to_string(), &timeframe.to_string());
        self.sync_props();
    }

    pub fn request_target(
        mut self: std::pin::Pin<&mut Self>,
        symbol: QString,
        timeframe: QString,
    ) -> bool {
        let ok = self
            .as_mut()
            .rust_mut()
            .request_target(&symbol.to_string(), &timeframe.to_string());
        self.sync_props();
        ok
    }

    pub fn follow_deployment(mut self: std::pin::Pin<&mut Self>) {
        self.as_mut().rust_mut().follow_deployment();
        self.sync_props();
    }

    pub fn clear_error(mut self: std::pin::Pin<&mut Self>) {
        self.as_mut().rust_mut().clear_error();
        self.sync_props();
    }

    pub fn parse_target_draft(self: std::pin::Pin<&mut Self>, draft: QString) -> QString {
        let rust = self.rust();
        QString::from(&crate::chart_target_input::parse_target_draft_json(
            &draft.to_string(),
            &rust.active_symbol.to_string(),
            &rust.active_timeframe.to_string(),
        ))
    }

    pub fn sync(mut self: std::pin::Pin<&mut Self>) {
        let feed_addr = self.rust().feed_addr;
        if feed_addr != 0 {
            let feed = feed_addr as *mut crate::bridge::bar_feed::ffi::BarFeed;
            unsafe {
                let mut feed_pin = std::pin::Pin::new_unchecked(&mut *feed);
                let feed_rust = feed_pin.as_mut().rust_mut();
                let feed_inner = std::pin::Pin::get_unchecked_mut(feed_rust);
                self.as_mut().rust_mut().refresh_from_feed(feed_inner);
            }
        }
        self.sync_props();
    }
}

/// # Safety
/// `ctx` must be a valid `ChartContext` pointer.
pub unsafe fn active_symbol(ctx: *mut ffi::ChartContext) -> String {
    if ctx.is_null() {
        return String::new();
    }
    std::pin::Pin::new_unchecked(&*ctx)
        .active_symbol()
        .to_string()
}

/// # Safety
/// `ctx` must be a valid `ChartContext` pointer.
pub unsafe fn active_timeframe(ctx: *mut ffi::ChartContext) -> String {
    if ctx.is_null() {
        return String::new();
    }
    std::pin::Pin::new_unchecked(&*ctx)
        .active_timeframe()
        .to_string()
}

/// # Safety
/// `ctx` must be a valid `ChartContext` pointer.
pub unsafe fn target_error(ctx: *mut ffi::ChartContext) -> String {
    if ctx.is_null() {
        return String::new();
    }
    std::pin::Pin::new_unchecked(&*ctx)
        .target_error()
        .to_string()
}

/// # Safety
/// `ctx` must be a valid `ChartContext` pointer.
pub unsafe fn sync_ptr(ctx: *mut ffi::ChartContext) {
    if ctx.is_null() {
        return;
    }
    let mut pin = std::pin::Pin::new_unchecked(&mut *ctx);
    pin.as_mut().sync();
}

/// # Safety
/// `ctx` must be a valid `ChartContext` pointer.
pub unsafe fn set_configured(ctx: *mut ffi::ChartContext, symbol: &str, timeframe: &str) {
    if ctx.is_null() {
        return;
    }
    let mut pin = std::pin::Pin::new_unchecked(&mut *ctx);
    pin.as_mut()
        .set_configured(QString::from(symbol), QString::from(timeframe));
}

/// # Safety
/// `ctx` must be a valid `ChartContext` pointer.
pub unsafe fn set_last_manual(ctx: *mut ffi::ChartContext, symbol: &str, timeframe: &str) {
    if ctx.is_null() {
        return;
    }
    let mut pin = std::pin::Pin::new_unchecked(&mut *ctx);
    pin.as_mut()
        .set_last_manual(QString::from(symbol), QString::from(timeframe));
}

/// # Safety
/// `ctx` must be a valid `ChartContext` pointer.
pub unsafe fn set_followed_deployment(ctx: *mut ffi::ChartContext, id: &str) {
    if ctx.is_null() {
        return;
    }
    let mut pin = std::pin::Pin::new_unchecked(&mut *ctx);
    pin.as_mut().set_followed_deployment(QString::from(id));
}

/// # Safety
/// `ctx` must be a valid `ChartContext` pointer.
pub unsafe fn notify_target(
    ctx: *mut ffi::ChartContext,
    following: Option<(&str, &str, &str, bool)>,
    configured: (&str, &str),
) {
    if ctx.is_null() {
        return;
    }
    let mut pin = std::pin::Pin::new_unchecked(&mut *ctx);
    match following {
        Some((name, sym, tf, is_following)) => {
            pin.as_mut().on_target(
                QString::from(name),
                QString::from(sym),
                QString::from(tf),
                is_following,
            );
        }
        None => {
            pin.as_mut().on_target(
                QString::from(""),
                QString::from(configured.0),
                QString::from(configured.1),
                false,
            );
        }
    }
}

/// # Safety
/// `ctx` must be a valid `ChartContext` pointer.
pub unsafe fn notify_retarget(ctx: *mut ffi::ChartContext, symbol: &str, timeframe: &str) {
    if ctx.is_null() {
        return;
    }
    let mut pin = std::pin::Pin::new_unchecked(&mut *ctx);
    pin.as_mut()
        .on_retarget(QString::from(symbol), QString::from(timeframe));
}

/// # Safety
/// `ctx` must be a valid `ChartContext` pointer.
pub unsafe fn notify_target_error(ctx: *mut ffi::ChartContext, error: &str) {
    if ctx.is_null() {
        return;
    }
    let mut pin = std::pin::Pin::new_unchecked(&mut *ctx);
    pin.as_mut().rust_mut().target_error = QString::from(error);
    pin.as_mut().rust_mut().bump();
    pin.as_mut().sync();
}

/// Binds the shared bar feed used to refresh identity labels.
///
/// # Safety
/// `ctx` and `feed` must be valid pointers to objects that outlive the binding.
pub unsafe fn bind_feed(
    ctx: *mut ffi::ChartContext,
    feed: *mut crate::bridge::bar_feed::ffi::BarFeed,
) {
    if ctx.is_null() || feed.is_null() {
        return;
    }
    let mut pin = std::pin::Pin::new_unchecked(&mut *ctx);
    pin.as_mut().rust_mut().bind_feed(feed);
}

/// Binds the chart targeter for manual retarget requests and deployment following.
///
/// # Safety
/// `ctx` must be a valid `ChartContext` pointer.
pub unsafe fn bind_targeter(
    ctx: *mut ffi::ChartContext,
    targeter: std::sync::Arc<crate::chart_target::ChartTargeter>,
) {
    if ctx.is_null() {
        return;
    }
    let mut pin = std::pin::Pin::new_unchecked(&mut *ctx);
    pin.as_mut().rust_mut().bind_targeter(targeter);
    pin.as_mut().sync();
}
