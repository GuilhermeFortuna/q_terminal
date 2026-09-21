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
    if feed.live_only && feed.bar_count == 0 {
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
    if conn == "live" && feed.bar_count > 0 {
        return ("Live".to_string(), "positive".to_string());
    }
    ("Loading".to_string(), "warning".to_string())
}

pub fn source_label(following_name: Option<&str>) -> (String, String) {
    match following_name {
        Some(name) if !name.is_empty() => {
            let label = format!("Following: {name}");
            (label, name.to_string())
        }
        _ => ("Configured".to_string(), String::new()),
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
        type ChartContext = super::ChartContextRust;

        #[qinvokable]
        fn set_configured(self: Pin<&mut ChartContext>, symbol: QString, timeframe: QString);

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
        fn sync(self: Pin<&mut ChartContext>);
    }

    impl cxx_qt::Threading for ChartContext {}
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

    configured_symbol: String,
    configured_timeframe: String,
    following_name: Option<String>,
    pending_symbol: String,
    pending_timeframe: String,
    switching: bool,
    feed_addr: usize,
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
            configured_symbol: String::new(),
            configured_timeframe: String::new(),
            following_name: None,
            pending_symbol: String::new(),
            pending_timeframe: String::new(),
            switching: false,
            feed_addr: 0,
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

    pub fn set_configured(&mut self, symbol: &str, timeframe: &str) {
        self.configured_symbol = symbol.to_string();
        self.configured_timeframe = timeframe.to_string();
        self.bump();
    }

    pub fn on_target(
        &mut self,
        deployment_name: &str,
        symbol: &str,
        timeframe: &str,
        following: bool,
    ) {
        self.following_name = if following && !deployment_name.is_empty() {
            Some(deployment_name.to_string())
        } else {
            None
        };
        self.pending_symbol = symbol.to_string();
        self.pending_timeframe = timeframe.to_string();
        self.switching = true;
        self.is_switching = true;
        self.bump();
    }

    pub fn on_retarget(&mut self, symbol: &str, timeframe: &str) {
        self.pending_symbol = symbol.to_string();
        self.pending_timeframe = timeframe.to_string();
        self.switching = true;
        self.is_switching = true;
        self.bump();
    }

    pub fn refresh_from_feed(&mut self, feed: &mut BarFeedRust) {
        feed.update_data_age();
        let feed_symbol = feed.symbol.to_string();
        let feed_timeframe = feed.timeframe.to_string();
        let switching = self.switching
            && (feed.history_loading || feed.bar_count == 0)
            && !self.pending_symbol.is_empty();
        self.is_switching = switching;

        let (source, tooltip) = source_label(self.following_name.as_deref());
        self.source_label = QString::from(&source);
        self.source_tooltip = QString::from(&tooltip);

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
    pub fn set_configured(mut self: std::pin::Pin<&mut Self>, symbol: QString, timeframe: QString) {
        let revision = {
            self.as_mut()
                .rust_mut()
                .set_configured(&symbol.to_string(), &timeframe.to_string());
            self.rust().revision
        };
        self.as_mut().set_revision(revision);
    }

    pub fn on_target(
        mut self: std::pin::Pin<&mut Self>,
        deployment_name: QString,
        symbol: QString,
        timeframe: QString,
        following: bool,
    ) {
        let (is_switching, revision) = {
            self.as_mut().rust_mut().on_target(
                &deployment_name.to_string(),
                &symbol.to_string(),
                &timeframe.to_string(),
                following,
            );
            let r = self.rust();
            (r.is_switching, r.revision)
        };
        self.as_mut().set_is_switching(is_switching);
        self.as_mut().set_revision(revision);
    }

    pub fn on_retarget(mut self: std::pin::Pin<&mut Self>, symbol: QString, timeframe: QString) {
        let (is_switching, revision) = {
            self.as_mut()
                .rust_mut()
                .on_retarget(&symbol.to_string(), &timeframe.to_string());
            let r = self.rust();
            (r.is_switching, r.revision)
        };
        self.as_mut().set_is_switching(is_switching);
        self.as_mut().set_revision(revision);
    }

    pub fn sync(mut self: std::pin::Pin<&mut Self>) {
        let props = {
            let feed_addr = self.rust().feed_addr;
            if feed_addr == 0 {
                return;
            }
            let feed = feed_addr as *mut crate::bridge::bar_feed::ffi::BarFeed;
            unsafe {
                let mut feed_pin = std::pin::Pin::new_unchecked(&mut *feed);
                let feed_rust = feed_pin.as_mut().rust_mut();
                let feed_inner = std::pin::Pin::get_unchecked_mut(feed_rust);
                self.as_mut().rust_mut().refresh_from_feed(feed_inner);
            }
            let r = self.rust();
            (
                r.symbol_line.clone(),
                r.source_label.clone(),
                r.source_tooltip.clone(),
                r.condition_label.clone(),
                r.condition_role.clone(),
                r.last_bar_label.clone(),
                r.is_switching,
                r.revision,
            )
        };
        self.as_mut().set_symbol_line(props.0);
        self.as_mut().set_source_label(props.1);
        self.as_mut().set_source_tooltip(props.2);
        self.as_mut().set_condition_label(props.3);
        self.as_mut().set_condition_role(props.4);
        self.as_mut().set_last_bar_label(props.5);
        self.as_mut().set_is_switching(props.6);
        self.as_mut().set_revision(props.7);
    }
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
