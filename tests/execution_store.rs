pub use q_terminal::{
    bridge, chart_bridge, chart_target, config, contracts_stream, execution, execution_controls,
    execution_models, history, ops_session, ops_status, startup, stream,
};

mod exec_common;

use exec_common::{start, start_on, DEP, DEP2};
use std::time::Duration;
use stream::fake_exec as fx;
use stream::fake_server::FakeServer;
use stream::policy::EXECUTION_TOPICS;

/// A first batch of history across all six topics, so snapshots are not empty.
async fn preload(server: &FakeServer) {
    server
        .exec_publish_silent("deployments", fx::deployment(DEP, "running"))
        .await;
    server
        .exec_publish_silent("deployments", fx::deployment(DEP2, "stopped"))
        .await;
    server
        .exec_publish_silent("decisions", fx::decision("d1", DEP))
        .await;
    server
        .exec_publish_silent("orders", fx::order("o1", DEP, "submitted"))
        .await;
    server
        .exec_publish_silent("orders", fx::order("o1", DEP, "filled"))
        .await;
    server
        .exec_publish_silent("fills", fx::fill("f1", DEP, "1.0", true))
        .await;
    server
        .exec_publish_silent("ledger", fx::ledger("l1", "999.90"))
        .await;
    server
        .exec_publish_silent("risk", fx::risk_rejection("r1", DEP))
        .await;
    server
        .exec_publish_silent("risk", fx::kill_switch("k1", true))
        .await;
}

/// One new event per topic, distinct from anything already logged under `tag`.
fn round(tag: &str) -> Vec<(&'static str, serde_json::Value)> {
    vec![
        ("deployments", fx::deployment(DEP, "paused")),
        ("decisions", fx::decision(&format!("d-{tag}"), DEP)),
        ("orders", fx::order(&format!("o-{tag}"), DEP, "submitted")),
        ("fills", fx::fill(&format!("f-{tag}"), DEP, "2.0", true)),
        ("ledger", fx::ledger(&format!("l-{tag}"), "998.00")),
        ("risk", fx::risk_rejection(&format!("r-{tag}"), DEP)),
    ]
}

#[tokio::test]
async fn subscribe_snapshot_buffered_and_live_deltas_converge() {
    let server = FakeServer::start().await;
    preload(&server).await;
    // The snapshot body is fixed, then held back, so what follows is buffered.
    server.set_snapshot_delay_ms(400);
    let h = start_on(server).await;
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Below and at the watermark: redeliveries of what the snapshot already holds.
    for topic in EXECUTION_TOPICS {
        let head = h.server.exec_snapshot().await["watermark"][topic]["seq"]
            .as_i64()
            .unwrap();
        for seq in 1..=head {
            h.server.exec_resend(topic, seq).await;
        }
    }
    // Above the watermark: new events, buffered until the snapshot lands.
    for (topic, payload) in round("buffered") {
        h.server.exec_publish(topic, payload).await;
    }
    h.wait_converged().await;

    // Live deltas after the snapshot.
    h.server.set_snapshot_delay_ms(0);
    for (topic, payload) in round("live") {
        h.server.exec_publish(topic, payload).await;
    }
    h.wait_converged().await;
    assert!(
        h.client.counters().dropped > 0,
        "below-watermark entries are discarded"
    );
    assert!(h
        .handle
        .read(|s| s.data().control.as_ref().unwrap().kill_switch_enabled));
    h.stop().await;
}

#[tokio::test]
async fn gap_on_each_topic_is_filled_from_history() {
    let h = start().await;
    preload(&h.server).await;
    h.wait_for("live", |h| h.handle.read(|s| s.is_confirmed()))
        .await;
    h.wait_converged().await;
    let resnapshots = h.client.counters().resnapshots;

    for topic in EXECUTION_TOPICS {
        let mut r = round(&format!("gap-{topic}"));
        let payload = r
            .iter()
            .position(|(t, _)| *t == topic)
            .map(|i| r.remove(i).1)
            .unwrap();
        let payload2 = match topic {
            "deployments" => fx::deployment(DEP2, "running"),
            _ => payload.clone(),
        };
        // The first is logged but never delivered: the next delivery shows a gap.
        h.server.exec_publish_silent(topic, payload).await;
        let rest_before = h.server.rest_calls();
        h.server.exec_publish(topic, payload2).await;
        h.wait_converged().await;
        assert!(
            h.server.rest_calls() > rest_before,
            "{topic}: history was fetched"
        );
    }
    assert_eq!(
        h.client.counters().resnapshots,
        resnapshots,
        "gaps never re-snapshot"
    );
    assert!(h.client.counters().gaps_closed >= 6);
    h.stop().await;
}

#[tokio::test]
async fn expired_history_re_snapshots_all_six_topics_once() {
    let h = start().await;
    preload(&h.server).await;
    h.wait_converged().await;
    let before = h.client.counters().resnapshots;

    // Other topics move too, silently: only a snapshot can bring the store level.
    for (topic, payload) in round("silent") {
        h.server.exec_publish_silent(topic, payload).await;
    }
    h.server.set_history_expired(true);
    h.server
        .exec_publish("fills", fx::fill("f-next", DEP, "3.0", true))
        .await;
    h.wait_converged().await;
    assert_eq!(h.client.counters().resnapshots, before + 1);
    h.stop().await;
}

#[tokio::test]
async fn epoch_change_re_snapshots_and_equals_a_fresh_snapshot() {
    let h = start().await;
    preload(&h.server).await;
    h.wait_converged().await;
    let before = h.client.counters().resnapshots;

    for (topic, payload) in round("during-restart") {
        h.server.exec_publish_silent(topic, payload).await;
    }
    // A Redis restart announces the new epoch on all six topics.
    for topic in EXECUTION_TOPICS {
        h.server.send_epoch_changed(topic, "epoch-2").await;
    }
    h.wait_converged().await;
    assert_eq!(
        h.client.counters().resnapshots,
        before + 1,
        "one snapshot, not six"
    );
    assert!(h
        .handle
        .read(|s| s.watermarks().values().all(|(e, _)| e == "epoch-2")));

    // One topic alone changing epoch also re-snapshots everything.
    for (topic, payload) in round("one-topic") {
        h.server.exec_publish_silent(topic, payload).await;
    }
    h.server.send_epoch_changed("orders", "epoch-3").await;
    h.wait_for("epoch-3", |h| {
        h.handle
            .read(|s| s.watermarks().get("orders").map(|(e, _)| e.as_str()) == Some("epoch-3"))
    })
    .await;
    h.wait_converged().await;
    h.stop().await;
}

#[tokio::test]
async fn duplicated_delivery_of_every_event_changes_nothing() {
    let h = start().await;
    preload(&h.server).await;
    h.wait_converged().await;
    for n in 0..4 {
        for (topic, payload) in round(&format!("dup{n}")) {
            let seq = h.server.exec_publish(topic, payload).await;
            h.server.exec_resend(topic, seq).await;
            h.server.exec_resend(topic, seq).await;
        }
    }
    h.wait_converged().await;
    let revision = h.handle.read(|s| s.revision());
    // Redeliver everything once more; nothing may move.
    for topic in EXECUTION_TOPICS {
        let head = h.server.exec_snapshot().await["watermark"][topic]["seq"]
            .as_i64()
            .unwrap();
        for seq in 1..=head {
            h.server.exec_resend(topic, seq).await;
        }
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
    h.wait_converged().await;
    assert_eq!(h.handle.read(|s| s.revision()), revision);
    h.stop().await;
}

#[tokio::test]
async fn disconnect_and_reconnect_converges_and_marks_last_confirmed() {
    let h = start().await;
    preload(&h.server).await;
    h.wait_converged().await;

    // Hold the snapshot back so the disconnected state is observable.
    h.server.set_snapshot_503(true);
    h.server.close_client().await;
    h.wait_for("unconfirmed", |h| !h.handle.read(|s| s.is_confirmed()))
        .await;
    assert!(h.handle.read(|s| s.last_confirmed_at()).is_some());
    let kept = h.handle.read(|s| s.counts());
    assert_eq!(
        kept.deployments, 2,
        "the last state is kept while disconnected"
    );

    for (topic, payload) in round("while-away") {
        h.server.exec_publish(topic, payload).await;
    }
    h.server.set_snapshot_503(false);
    h.wait_converged().await;
    h.stop().await;
}

#[tokio::test]
async fn snapshot_503_keeps_prior_state_unconfirmed_and_recovers() {
    let server = FakeServer::start().await;
    preload(&server).await;
    server.set_snapshot_503(true);
    let h = start_on(server).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!h.handle.read(|s| s.is_confirmed()));
    assert_eq!(h.handle.read(|s| s.counts().deployments), 0);
    assert!(h.server.rest_calls() >= 2, "retries with backoff");

    h.server.set_snapshot_503(false);
    h.wait_converged().await;
    let good = h.handle.read(|s| s.data().clone());

    // Lose the stream again, this time with a 503 on reconnect: state is kept.
    h.server.set_snapshot_503(true);
    h.server.close_client().await;
    h.wait_for("unconfirmed", |h| !h.handle.read(|s| s.is_confirmed()))
        .await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(h.handle.read(|s| *s.data() == good));
    assert!(!h.handle.read(|s| s.is_confirmed()));
    h.server.set_snapshot_503(false);
    h.wait_converged().await;
    h.stop().await;
}

#[tokio::test]
async fn events_during_a_snapshot_outage_do_not_inflate_the_retry_backoff() {
    let h = start().await;
    preload(&h.server).await;
    h.wait_converged().await;

    // Found by the convergence property test (seed 591): each event that asked for a
    // snapshot during the outage used to count as a failed attempt, so the backoff
    // outgrew the outage by orders of magnitude.
    h.server.set_snapshot_503(true);
    for n in 2..14 {
        h.server
            .send_epoch_changed("orders", &format!("epoch-{n}"))
            .await;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    h.server.set_snapshot_503(false);
    h.wait_converged().await;
    h.stop().await;
}
