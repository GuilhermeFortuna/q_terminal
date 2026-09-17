#[path = "../contracts/stream.rs"]
pub mod contracts_stream;

#[path = "../src/bridge.rs"]
pub mod bridge;
#[path = "../src/config.rs"]
pub mod config;
#[path = "../src/history/mod.rs"]
pub mod history;
#[path = "../src/stream/mod.rs"]
pub mod stream;

use config::Config;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use stream::client::{ConnectionState, StreamClient};
use stream::fake_server::FakeServer;
use stream::sink::BarSink;
use stream::topic_state::BarColumns;

struct CountingAlloc;
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static COUNTING_ENABLED: AtomicBool = AtomicBool::new(false);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING_ENABLED.load(Ordering::Relaxed) {
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static A: CountingAlloc = CountingAlloc;

fn get_cpu_ticks() -> u64 {
    if let Ok(stat) = std::fs::read_to_string("/proc/self/stat") {
        let parts: Vec<&str> = stat.split_whitespace().collect();
        if parts.len() > 14 {
            let utime: u64 = parts[13].parse().unwrap_or(0);
            let stime: u64 = parts[14].parse().unwrap_or(0);
            return utime + stime;
        }
    }
    0
}

fn dummy_bar(t: i64, close: f64) -> BarColumns {
    BarColumns::single(t, 10.0, 11.0, 9.0, close, 100.0)
}

#[tokio::test]
#[ignore]
async fn test_stream_benchmark() {
    let duration_secs: u64 = std::env::var("BENCH_DURATION_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    println!("\n=== Starting Stream Benchmark ({duration_secs}s synthetic full-rate stream) ===");

    let server = FakeServer::start().await;
    server
        .set_snapshot("bars.completed", 1, dummy_bar(60, 10.0))
        .await;
    server
        .set_snapshot("bars.forming", 1, dummy_bar(120, 10.5))
        .await;

    let config = Config {
        api_base: server.api_base(),
        symbol: "PETR4".to_string(),
        timeframe: "1m".to_string(),
    };

    let sink = BarSink::new();
    let client = StreamClient::start(config, sink.clone());

    // Wait until Live
    for _ in 0..50 {
        if client.connection_state() == ConnectionState::Live {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(client.connection_state(), ConnectionState::Live);

    let mut feed = bridge::bar_feed::BarFeedRust::new("PETR4", "1m");
    feed.history_controller().open_gate();
    sink.drain_into(&mut feed);

    let initial_ticks = get_cpu_ticks();
    let start_instant = Instant::now();

    let mut delays = Vec::with_capacity(duration_secs as usize * 100);
    let mut seq = 2i64;

    // Send loop at full rate (e.g. 50-100 entries/sec, simulating full-rate forming & completed bars)
    let end_time = start_instant + Duration::from_secs(duration_secs);

    // Warm up for 1 second before counting allocations
    let warmup_end = start_instant + Duration::from_secs(1.min(duration_secs));

    let mut counted_entries = 0usize;

    while Instant::now() < end_time {
        let loop_start = Instant::now();
        if loop_start >= warmup_end && !COUNTING_ENABLED.load(Ordering::Relaxed) {
            ALLOC_COUNT.store(0, Ordering::Relaxed);
            COUNTING_ENABLED.store(true, Ordering::Relaxed);
        }

        let send_time = Instant::now();
        server
            .send_bar(
                "bars.forming",
                seq,
                "PETR4",
                "1m",
                dummy_bar(120, ((seq % 100) as f64).mul_add(0.01, 10.0)),
            )
            .await;
        seq += 1;

        if COUNTING_ENABLED.load(Ordering::Relaxed) {
            counted_entries += 1;
        }

        // Wait for bar to arrive at sink and drain into feed
        let mut applied = false;
        let drain_start = Instant::now();
        while drain_start.elapsed() < Duration::from_millis(100) {
            let deliveries = sink.drain();
            if !deliveries.is_empty() {
                for d in deliveries {
                    feed.apply_delivery(d.into());
                }
                let delay = send_time.elapsed();
                delays.push(delay);
                applied = true;
                break;
            }
            tokio::task::yield_now().await;
        }
        if !applied {
            delays.push(send_time.elapsed());
        }

        // Pace to ~50-100 Hz synthetic live stream rate
        let elapsed_loop = loop_start.elapsed();
        if elapsed_loop < Duration::from_millis(10) {
            tokio::time::sleep(Duration::from_millis(10) - elapsed_loop).await;
        }
    }

    COUNTING_ENABLED.store(false, Ordering::Relaxed);
    let total_allocs = ALLOC_COUNT.load(Ordering::Relaxed);
    let total_wall_secs = start_instant.elapsed().as_secs_f64();
    let final_ticks = get_cpu_ticks();

    let ticks_used = final_ticks.saturating_sub(initial_ticks);
    // Typical linux clock ticks: 100 per sec
    let cpu_secs_used = (ticks_used as f64) / 100.0;
    let cpu_pct = (cpu_secs_used / total_wall_secs) * 100.0;

    delays.sort();
    let p95_idx = (delays.len() * 95) / 100;
    let p95_delay = if !delays.is_empty() {
        delays[p95_idx]
    } else {
        Duration::ZERO
    };

    let allocs_per_entry = if counted_entries > 0 {
        (total_allocs as f64) / (counted_entries as f64)
    } else {
        0.0
    };

    println!("--------------------------------------------------");
    println!("Processed total entries: {}", delays.len());
    println!("Elapsed wall-clock time: {:.2} s", total_wall_secs);
    println!(
        "Process CPU cost: {:.2}% of one core (target: < 5.0%)",
        cpu_pct
    );
    println!(
        "95th-percentile socket-to-revision delay: {:.2} ms (target: < 10.0 ms)",
        p95_delay.as_secs_f64() * 1000.0
    );
    println!(
        "Steady-state allocations per entry: {:.1} (target: decoded batch only)",
        allocs_per_entry
    );
    println!("--------------------------------------------------");

    assert!(
        p95_delay < Duration::from_millis(10),
        "95th percentile socket-to-revision delay must be < 10 ms, was {:?}",
        p95_delay
    );
    assert!(
        cpu_pct < 5.0,
        "Process CPU must be < 5% of one core, was {:.2}%",
        cpu_pct
    );

    client.shutdown();
    server.shutdown().await;
}
