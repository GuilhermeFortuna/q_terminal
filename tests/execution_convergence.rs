//! Criterion 7: seeded interleavings of publishes, gaps, duplicates, expired history,
//! epoch changes, snapshot outages and disconnects always converge to the final snapshot.
//! 200 seeds by default; set `Q_TERMINAL_SEEDS` for more.

#[rustfmt::skip]
#[path = "../contracts/stream.rs"]
pub mod contracts_stream;

#[path = "../src/bridge.rs"]
pub mod bridge;
#[path = "../src/config.rs"]
pub mod config;
#[path = "exec_common/mod.rs"]
mod exec_common;
#[path = "../src/execution/mod.rs"]
pub mod execution;
#[path = "../src/history/mod.rs"]
pub mod history;
#[path = "../src/stream/mod.rs"]
pub mod stream;

use exec_common::{start, DEP, DEP2};
use std::time::Duration;
use stream::fake_exec as fx;
use stream::policy::EXECUTION_TOPICS;

const DEP3: &str = "44444444-4444-4444-4444-444444444444";

/// xorshift64*: small, seeded, dependency-free.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }
}

fn random_event(rng: &mut Rng, topic: &str) -> serde_json::Value {
    let deps = [DEP, DEP2, DEP3];
    let dep = rng.pick(&deps);
    let id = format!("{}", rng.below(6));
    match topic {
        "deployments" => fx::deployment(dep, rng.pick(&["running", "paused", "stopped"])),
        "decisions" => fx::decision(&format!("d{}", rng.next()), dep),
        // An order never changes deployment, so its id fixes it.
        "orders" => fx::order(
            &format!("o{id}"),
            deps[id.parse::<usize>().unwrap() % deps.len()],
            rng.pick(&["submitted", "filled", "cancelled"]),
        ),
        "fills" => {
            let open = rng.below(3) != 0;
            fx::fill(
                &format!("f{}", rng.next()),
                dep,
                if open { "1.0" } else { "0" },
                open,
            )
        }
        "ledger" => fx::ledger(
            &format!("l{}", rng.next()),
            &format!("{}.00", 900 + rng.below(100)),
        ),
        _ => {
            if rng.below(4) == 0 {
                fx::kill_switch(&format!("k{}", rng.next()), rng.below(2) == 0)
            } else {
                fx::risk_rejection(&format!("r{}", rng.next()), dep)
            }
        }
    }
}

async fn run_seed(seed: u64) {
    let mut rng = Rng::new(seed);
    let h = start().await;
    h.server.set_exec_limit(3 + rng.below(4)).await;
    let mut epoch_n = 1;

    for _ in 0..(15 + rng.below(25)) {
        let topic = rng.pick(&EXECUTION_TOPICS);
        match rng.below(12) {
            0..=4 => {
                let ev = random_event(&mut rng, topic);
                h.server.exec_publish(topic, ev).await;
            }
            5 => {
                let ev = random_event(&mut rng, topic);
                h.server.exec_publish_silent(topic, ev).await;
            }
            6 | 7 => {
                let head = h.server.exec_snapshot().await["watermark"][topic]["seq"]
                    .as_i64()
                    .unwrap();
                if head > 0 {
                    h.server
                        .exec_resend(topic, 1 + rng.below(head as usize) as i64)
                        .await;
                }
            }
            8 => h.server.set_history_expired(rng.below(2) == 0),
            9 => {
                epoch_n += 1;
                h.server
                    .send_epoch_changed(topic, &format!("epoch-{epoch_n}"))
                    .await;
            }
            10 => h.server.close_client().await,
            _ => h.server.set_snapshot_503(rng.below(2) == 0),
        }
        if rng.below(3) == 0 {
            tokio::time::sleep(Duration::from_millis(rng.below(30) as u64)).await;
        }
    }

    // The faults end. A protocol cannot see a lost tail until something follows it,
    // so the session ends with one delivered event per topic, as a live system would.
    h.server.set_history_expired(false);
    h.server.set_snapshot_503(false);
    tokio::time::sleep(Duration::from_millis(150)).await;
    for topic in EXECUTION_TOPICS {
        let ev = random_event(&mut rng, topic);
        h.server.exec_publish(topic, ev).await;
    }

    for _ in 0..400 {
        if h.converged().await {
            h.stop().await;
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let counts = h.handle.read(|s| s.counts());
    let marks = h.handle.read(|s| s.watermarks().clone());
    let snap = h.server.exec_snapshot().await;
    let fresh = exec_common::fresh_from(&snap);
    let data = h.handle.read(|s| s.data().clone());
    let confirmed = h.handle.read(|s| s.is_confirmed());
    eprintln!(
        "client: {:?} last_error {:?} counters {:?} rest_calls {}",
        h.client.connection_state(),
        h.client.last_error(),
        h.client.counters(),
        h.server.rest_calls()
    );
    panic!(
        "seed {seed} did not converge: confirmed {confirmed}, store {counts:?}, watermarks {marks:?}, \
         snapshot watermark {}\nequal parts: deployments {} accounts {} positions {} orders {} \
         decisions {} fills {} risk {} control {}\nstore risk {:?}\nfresh risk {:?}",
        snap["watermark"],
        data.deployments == fresh.deployments,
        data.accounts == fresh.accounts,
        data.positions == fresh.positions,
        data.orders == fresh.orders,
        data.decisions == fresh.decisions,
        data.fills == fresh.fills,
        data.risk == fresh.risk,
        data.control == fresh.control,
        data.risk,
        fresh.risk,
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn seeded_interleavings_converge_to_the_final_snapshot() {
    let seeds: u64 = std::env::var("Q_TERMINAL_SEEDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200);
    if let Some(only) = std::env::var("Q_TERMINAL_SEED_ONLY")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        run_seed(only).await;
        return;
    }
    for seed in 1..=seeds {
        run_seed(seed).await;
    }
}
