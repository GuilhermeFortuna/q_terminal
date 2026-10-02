//! Current-session trade tape and the volume studies derived from it (Q-082).
//!
//! `TradeFeed` owns the join of a frozen session snapshot with live deliveries, the bounded
//! columnar history of the session and the `q-indicators` volume state. It knows nothing of
//! Qt or the network: the stream client drives it and QML only arranges what it reports.

pub mod analysis;
pub mod feed;
pub mod history;
pub mod model;
pub mod notify;
pub mod time;

pub use feed::{TradeFeed, TradeHandle};
