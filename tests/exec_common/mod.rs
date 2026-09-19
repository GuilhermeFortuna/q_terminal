//! Shared harness for the execution store tests. Included with `#[path]`.
#![allow(dead_code)]

use super::config::Config;
use super::contracts_stream::ExecutionSnapshot;
use super::execution::store::{ExecutionData, ExecutionHandle, ExecutionStore};
use super::stream::client::{Sinks, StreamClient};
use super::stream::fake_server::FakeServer;
use super::stream::policy::EXECUTION_TOPICS;
use super::stream::sink::BarSink;
use std::time::Duration;

pub const DEP: &str = "11111111-1111-1111-1111-111111111111";
pub const DEP2: &str = "33333333-3333-3333-3333-333333333333";

pub struct Harness {
    pub server: FakeServer,
    pub handle: ExecutionHandle,
    pub client: StreamClient,
}

pub async fn start() -> Harness {
    let server = FakeServer::start().await;
    start_on(server).await
}

pub async fn start_on(server: FakeServer) -> Harness {
    let handle = ExecutionHandle::new();
    let config = Config {
        api_base: server.api_base(),
        symbol: "PETR4".to_string(),
        timeframe: "1m".to_string(),
    };
    let client = StreamClient::start_with(
        config,
        Sinks {
            bars: BarSink::new(),
            execution: Some(handle.clone()),
        },
    );
    Harness {
        server,
        handle,
        client,
    }
}

/// What a fresh, exact read of the server's snapshot puts in a store.
pub fn fresh_from(snapshot: &serde_json::Value) -> ExecutionData {
    let snap: ExecutionSnapshot = serde_json::from_value(snapshot.clone()).unwrap();
    let mut store = ExecutionStore::new();
    store.apply_snapshot(&snap, &EXECUTION_TOPICS);
    store.data().clone()
}

impl Harness {
    /// The store equals a fresh snapshot, with every topic's applied sequence at the head.
    pub async fn converged(&self) -> bool {
        let snap = self.server.exec_snapshot().await;
        let fresh = fresh_from(&snap);
        let head_ok = EXECUTION_TOPICS.iter().all(|t| {
            let want = snap["watermark"][t]["seq"].as_i64().unwrap();
            self.handle
                .read(|s| s.watermarks().get(*t).map(|(_, seq)| *seq) == Some(want))
        });
        head_ok && self.handle.read(|s| s.is_confirmed() && *s.data() == fresh)
    }

    pub async fn wait_converged(&self) {
        for _ in 0..200 {
            if self.converged().await {
                return;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        let snap = self.server.exec_snapshot().await;
        let fresh = fresh_from(&snap);
        let got = self.handle.read(|s| s.data().clone());
        panic!(
            "store did not converge\nwatermarks: {:?}\nexpected counts: {}\nstore counts: {:?}\nequal: {}",
            self.handle.read(|s| s.watermarks().clone()),
            snap["watermark"],
            self.handle.read(|s| s.counts()),
            got == fresh
        );
    }

    pub async fn wait_for(&self, what: &str, f: impl Fn(&Harness) -> bool) {
        for _ in 0..200 {
            if f(self) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("timed out waiting for {what}");
    }

    pub async fn stop(self) {
        self.client.shutdown();
        self.server.shutdown().await;
    }
}
