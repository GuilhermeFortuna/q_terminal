pub use q_terminal::{
    bridge, chart_bridge, chart_target, config, contracts_stream, execution, execution_controls,
    execution_models, history, ops_session, ops_status, startup, stream,
};

use config::{Config, ConfigError};
use startup::setup_slice;

#[test]
fn test_startup_config_error_opens_window_showing_error() {
    let _guard = chart_bridge::QT_TEST_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let err = ConfigError::MissingField("api_base".into());
    let ctx = setup_slice(&Err(err));
    assert!(!ctx.feed_ptr.is_null());
    let mut probe = chart_bridge::make_status_strip_probe();
    unsafe {
        probe.pin_mut().set_feed(ctx.feed_ptr);
    }
    let res = probe.result();
    assert_eq!(res.connection_state, "ERROR");
    assert!(res.last_error.contains("api_base"));
}

#[test]
fn test_startup_unreachable_api_opens_window_retrying() {
    let _guard = chart_bridge::QT_TEST_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let config = Config {
        api_base: "http://127.0.0.1:39999".into(),
        symbol: "PETR4".into(),
        timeframe: "1m".into(),
        operator: "operator".into(),
    };
    let ctx = setup_slice(&Ok(config));
    assert!(!ctx.feed_ptr.is_null());
    let mut probe = chart_bridge::make_status_strip_probe();
    unsafe {
        probe.pin_mut().set_feed(ctx.feed_ptr);
    }
    let res = probe.result();
    assert!(res.connection_state == "CONNECTING" || res.connection_state == "RETRYING");
}
