//! QML face of one tape panel (Q-082).
//!
//! Each panel owns a `TapeModel` with its own display filters and its own bounded rows, all
//! read from the process-level trade feed. Filters decide what the list shows; they never
//! reach the feed, so two panels with different filters see the same session totals.

use std::pin::Pin;
use std::time::Duration;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QVariant};

use crate::trades::feed::{ResyncReason, TradeHandle};
use crate::trades::model::{side_name, SideFilter, TapeFilter, TapeView};
use crate::trades::notify::Coalescer;

use crate::execution_models::ffi as tables;
use crate::execution_models::RawTableModel;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;

    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QVariant, rows)]
        #[qproperty(QString, state_json)]
        #[qproperty(f64, min_volume)]
        #[qproperty(QString, side)]
        #[qproperty(i32, row_count)]
        #[qproperty(i64, revision)]
        type TapeModel = super::TapeModelRust;

        /// Follows the shared trade feed. Called once when the panel is created.
        #[qinvokable]
        fn bind(self: Pin<&mut TapeModel>);

        /// Sets the display filter; returns whether it changed. `side` is `all`, `buy`,
        /// `sell` or `unknown`.
        #[qinvokable]
        fn set_filter(self: Pin<&mut TapeModel>, min_volume: f64, side: QString) -> bool;

        /// The filter as the workspace stores it.
        #[qinvokable]
        fn filter_json(self: &TapeModel) -> QString;

        /// Restores a stored filter; an invalid one leaves the defaults.
        #[qinvokable]
        fn restore_filter_json(self: Pin<&mut TapeModel>, json: QString) -> bool;

        /// Follows a deterministic fixture session instead of the process feed. For gallery
        /// captures and tests.
        #[qinvokable]
        fn bind_fixture(self: Pin<&mut TapeModel>, state: QString, minutes: i32);

        /// Reloads the session after its history filled up.
        #[qinvokable]
        fn retry(self: Pin<&mut TapeModel>);

        #[qinvokable]
        fn sync(self: Pin<&mut TapeModel>);
    }

    impl cxx_qt::Threading for TapeModel {}
}

pub struct TapeModelRust {
    pub rows: QVariant,
    pub state_json: QString,
    pub min_volume: f64,
    pub side: QString,
    pub row_count: i32,
    pub revision: i64,
    model: RawTableModel,
    view: TapeView,
    handle: Option<TradeHandle>,
}

impl Default for TapeModelRust {
    fn default() -> Self {
        let filter = TapeFilter::default();
        unsafe {
            let model = RawTableModel(tables::make_table_model());
            tables::table_model_set_roles_csv(model.0, "time,price,volume,side,large");
            Self {
                rows: tables::table_model_to_variant(model.0),
                state_json: QString::from("{}"),
                min_volume: filter.min_volume,
                side: QString::from(filter.side.as_str()),
                row_count: 0,
                revision: 0,
                model,
                view: TapeView::new(filter),
                handle: None,
            }
        }
    }
}

impl Drop for TapeModelRust {
    fn drop(&mut self) {
        unsafe { tables::delete_table_model(self.model.0) };
    }
}

fn clock_text(time_msc: i64, offset_ms: i64) -> String {
    let local = (time_msc + offset_ms).rem_euclid(86_400_000);
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        local / 3_600_000,
        local % 3_600_000 / 60_000,
        local % 60_000 / 1000,
        local % 1000
    )
}

fn number(value: f64) -> String {
    if (value - value.round()).abs() < 1e-9 {
        format!("{value:.0}")
    } else {
        let text = format!("{value:.5}");
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

impl TapeModelRust {
    /// Rows as the table model's JSON, newest first.
    fn rows_json(&self, offset_ms: i64) -> String {
        let rows: Vec<serde_json::Value> = self
            .view
            .rows()
            .iter()
            .map(|row| {
                serde_json::json!({
                    "time": clock_text(row.time_msc, offset_ms),
                    "price": number(row.price),
                    "volume": number(row.volume),
                    "side": side_name(row.side),
                    "large": row.large,
                })
            })
            .collect();
        serde_json::Value::Array(rows).to_string()
    }
}

impl ffi::TapeModel {
    pub fn bind(mut self: Pin<&mut Self>) {
        let handle = TradeHandle::process();
        self.as_mut().rust_mut().handle = Some(handle.clone());
        let qt = self.qt_thread();
        let coalescer = Coalescer::spawn(Duration::from_millis(60), move || {
            qt.queue(|mut model| model.as_mut().sync()).is_ok()
        });
        handle.subscribe(move || coalescer.trigger());
        self.sync();
    }

    pub fn bind_fixture(mut self: Pin<&mut Self>, state: QString, minutes: i32) {
        let feed =
            crate::trades::fixture::preview_feed(&state.to_string(), minutes.max(1) as usize);
        self.as_mut().rust_mut().handle = Some(TradeHandle::from_feed(feed));
        self.sync();
    }

    pub fn set_filter(mut self: Pin<&mut Self>, min_volume: f64, side: QString) -> bool {
        let Some(side) = SideFilter::parse(&side.to_string()) else {
            return false;
        };
        if !min_volume.is_finite() || min_volume < 0.0 {
            return false;
        }
        let filter = TapeFilter { min_volume, side };
        if self.rust().view.filter() == filter {
            return false;
        }
        self.as_mut().rust_mut().view.set_filter(filter);
        self.as_mut().set_min_volume(min_volume);
        self.as_mut().set_side(QString::from(side.as_str()));
        self.sync();
        true
    }

    pub fn filter_json(&self) -> QString {
        QString::from(
            serde_json::to_string(&self.rust().view.filter()).unwrap_or_else(|_| "{}".into()),
        )
    }

    pub fn restore_filter_json(mut self: Pin<&mut Self>, json: QString) -> bool {
        match serde_json::from_str::<TapeFilter>(&json.to_string()) {
            Ok(filter) if filter.min_volume.is_finite() && filter.min_volume >= 0.0 => self
                .as_mut()
                .set_filter(filter.min_volume, QString::from(filter.side.as_str())),
            _ => false,
        }
    }

    pub fn retry(self: Pin<&mut Self>) {
        if let Some(handle) = &self.rust().handle {
            handle.request_restart(ResyncReason::Retry);
        }
    }

    pub fn sync(mut self: Pin<&mut Self>) {
        let Some(handle) = self.rust().handle.clone() else {
            return;
        };
        let (changed, state, offset_ms) = {
            let mut rust = self.as_mut().rust_mut();
            handle.read(|feed| {
                let changed = rust.view.sync(feed, feed.display_threshold());
                let offset = feed.clock().map_or(-3 * 3_600_000, |c| c.offset_ms());
                let mut report = serde_json::to_value(feed.report()).unwrap_or_default();
                report["shown_rows"] = rust.view.rows().len().into();
                report["large_threshold"] = feed.display_threshold().into();
                (changed, report.to_string(), offset)
            })
        };
        if changed {
            let json = self.rust().rows_json(offset_ms);
            unsafe { tables::table_model_reset_json(self.rust().model.0, &json) };
            let count = self.rust().view.rows().len() as i32;
            self.as_mut().set_row_count(count);
            let rev = self.rust().revision + 1;
            self.as_mut().set_revision(rev);
        }
        if self.rust().state_json.to_string() != state {
            self.as_mut().set_state_json(QString::from(state));
        }
    }
}
