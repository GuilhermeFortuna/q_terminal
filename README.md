# `q_terminal`

High-performance desktop trading and operations terminal for the Q platform, built on Qt 6 / QML and backed by Rust (`q_core`) via CXX-Qt.

---

## The Live Chart Slice

The live chart slice provides a read-only, reactive candlestick chart window displaying live and historical market bars for a configured symbol and timeframe.

### What It Shows

- **Candlestick Chart (`BarChartItem`)**: GPU-accelerated custom Qt Quick scene-graph render node (`QSGRenderNode`) drawing completed bars and the active forming bar directly into vertex buffers.
- **Viewport (`Viewport.qml`)**: Reactive viewport tracking the newest bar on the right edge, maintaining sticky price bounds with vertical padding margins, and resizing dynamically.
- **Chart Pane (`ChartPane.qml`)**: Integrates the candlestick item with time and price gridlines and axis tick labels mapped directly to viewport coordinates.
- **Status Strip (`StatusStrip.qml`)**: Real-time status displaying:
  - Instrument header: active symbol and timeframe.
  - Connection indicator: `CONNECTED`, `CONNECTING`, `RETRYING`, or `ERROR`.
  - Data freshness / staleness: flips to `STALE` when bar delivery age exceeds the timeframe interval.
  - History source & shortfall: indicates whether history loaded from lake or API, and shortfall bar count if any.
  - Reconnect / error banner: displays actionable diagnostics and retry status.
- **Empty State (`EmptyState.qml`)**: Informative placeholder presented while history is loading or before the first bar arrives.
- **Non-Interactive Boundary**: This slice is strictly read-only visualization. It contains no order entry, kill switch, or strategy modification controls.

---

## Configuration

`q_terminal` is configured through a TOML file or environment variables.

### Configuration File

By default, configuration is read from `~/.config/q/terminal.toml`:

```toml
api_base = "http://127.0.0.1:8000"
symbol = "PETR4"
timeframe = "1m"
```

### Environment Variable Overrides

Any configuration setting can be overridden using environment variables:

| Setting | Config Key | Environment Variable | Default |
|---|---|---|---|
| Control API URL | `api_base` | `Q_TERMINAL_API_BASE` | None (required) |
| Trading Symbol | `symbol` | `Q_TERMINAL_SYMBOL` | `PETR4` |
| Bar Timeframe | `timeframe` | `Q_TERMINAL_TIMEFRAME` | `1m` |

### Degraded Startup

If configuration is missing or invalid, or if the API cannot be reached at startup, `q_terminal` does not exit or crash. It opens the window immediately in a degraded state with clear status diagnostics in the status strip and retries connection automatically.

---

## Running locally

The canonical workflow is the workspace `./dev` launcher (Docker-backed Postgres
and Redis; API, outbox relay, and execution worker as systemd user units;
`q_terminal` runs in the foreground and does not supervise them):

```bash
cd /path/to/q
./dev up terminal
```

This starts Postgres, Redis, the control API, and the outbox relay, then launches
`q_terminal`. Ctrl+C stops only the terminal; backends keep running until
`./dev down`.

Override defaults with environment variables:

```bash
Q_TERMINAL_API_BASE="http://127.0.0.1:8000" \
Q_TERMINAL_SYMBOL="PETR4" \
Q_TERMINAL_TIMEFRAME="1m" \
./dev up terminal
```

Or launch the terminal alone after `./dev up execution` (or after starting the
backend stack manually):

```bash
cd q_terminal && make run
```

### Alternative: `./research`

For Research UI / backtest work, use `./research` from the workspace root instead.
It starts a different stack (including the Dramatiq worker) and tears everything
down on Ctrl+C. It is not required for `q_terminal` development.

---

## Repository Structure

```
q_terminal/
├── Cargo.toml          # Package manifest; links cxx-qt and git-pinned q_core tag
├── rust-toolchain.toml # Exact Rust compiler pin (1.98.0)
├── build.rs            # CXX-Qt build script registering QML module and C++ bridges
├── Makefile            # Standing validation targets: check, fmt, lint, test, build, run
├── CONTRACTS_REV       # Pinned q_contracts commit hash
├── BOUNDARY.md         # Explicit ownership scope and prohibited surfaces
├── contracts/          # Vendored generated Rust wire types from q_contracts
├── cpp/                # C++ scene-graph render nodes and QML probes (buffer movement only)
│   ├── bar_chart_item.h / .cpp   # QQuickItem hosting BarChartNode
│   ├── bar_chart_node.h / .cpp   # QSGRenderNode moving vertices to GPU
│   ├── bar_chart_probe.cpp       # Headless vertex and scene graph inspection
│   └── chart_cxx.h / .cpp        # CXX-Qt bridge helper functions and event pump
├── qml/                # Declarative QML scenes
│   ├── Main.qml        # Main application window composing Header, ChartPane, StatusStrip
│   ├── Viewport.qml    # Viewport tracking newest bar and sticky price bounds
│   ├── ChartPane.qml   # Chart pane with gridlines, price/time axes, and BarChartItem
│   ├── StatusStrip.qml # Live connection, freshness, history source, and error status
│   ├── EmptyState.qml  # Placeholder before initial bars arrive
│   └── qmldir          # QML module definition
├── src/                # Rust application and bridge code
│   ├── main.rs         # Entry point: CLI args and window launch
│   ├── startup.rs      # Slice startup sequence and degraded state setup
│   ├── config.rs       # TOML configuration and environment loading
│   ├── bar_feed.rs     # CXX-Qt BarFeed model binding live and historical bars to QML
│   ├── bridge.rs       # CXX-Qt AppInfo projection over q_core::CoreInfo
│   ├── chart_bridge.rs # CXX-Qt chart probe bindings for headless testing
│   ├── execution/      # Execution store: exact live state keyed by entity id
│   ├── history/        # Catalog-driven parquet load, seam stitching, and verification
│   └── stream/         # WebSocket client, envelope framing, and sequence gap recovery
└── tests/              # End-to-end and headless integration tests
    ├── slice_end_to_end.rs    # Full end-to-end lake history to live WebSocket stream test
    ├── test_startup.rs        # Degraded startup and config error window tests
    ├── test_chart_bridge.rs   # Viewport, geometry, and probe unit tests
    ├── execution_store.rs     # Execution protocol against the fake stream server
    ├── execution_convergence.rs # Seeded fault interleavings converge to the snapshot
    ├── test_execution_report.rs # --headless-report --execution
    └── test_headless_report.rs# CLI headless report verification
```

---

## The Execution Store

`q_terminal` keeps an exact, live, in-memory copy of execution state: deployments, accounts,
open positions, orders, decisions, fills, risk events and the kill switch. It is kept by the
same snapshot-then-delta protocol as the bars (subscribe, snapshot, discard at or below the
watermark, fill gaps from history by sequence, re-snapshot on expiry or epoch change), over
the **same stream connection**. There is no QML for it yet; Q-047 draws it.

- **Six topics, one snapshot.** `decisions`, `orders`, `fills`, `risk`, `ledger` and
  `deployments` each follow the protocol independently. A re-snapshot triggered by any one of
  them re-reads `GET /api/v1/stream/execution/snapshot` and re-applies it to all six, each with
  its own watermark. A restart that announces a new epoch on all six costs one snapshot.
- **Replace by id, no arithmetic.** An event replaces its entity. A fill sets the deployment's
  position from the fill's post-fill position, and a ledger entry sets the account from its
  post-entry balances. Decimals stay strings. Recent decisions, fills, orders and risk events
  are bounded per deployment by the snapshot's declared `limits`; older ones are paged from
  the REST routes on demand.
- **Degraded states.** When the stream is lost the store keeps its last state and records
  when it was last confirmed (`is_confirmed`, `last_confirmed_at`). A `503` on the snapshot
  leaves it unconfirmed and retries with the stream's backoff.
- **Change detection.** `ExecutionStore::revision()` increases on every applied change, so a
  view redraws at most once per frame. `ExecutionHandle` shares the store with a listener.
- **Policy.** The execution topics must be `durable` and non-coalescing in the topic policy,
  or `StreamClient` refuses to start.

`StreamClient::start_with(config, Sinks { bars, execution: Some(handle) })` subscribes to the
execution topics too; `StreamClient::start(config, bar_sink)` subscribes to bars only.

### Headless execution report

```bash
cargo run -- --headless-report --execution
```

Connects using the usual configuration (`api_base` plus `symbol` and `timeframe`), waits for
the store to converge, prints its counts, the kill switch and the `(epoch, seq)` applied on
each topic, and exits 0 (1 if the state could not be confirmed within
`Q_TERMINAL_REPORT_TIMEOUT_MS`, default 20000). Compare with the backend:

```bash
curl -s http://127.0.0.1:8000/api/v1/stream/execution/snapshot | jq '{deployments: (.deployments|length), orders: (.orders|length)}'
```

### Stream convergence tests

`tests/execution_store.rs` scripts the protocol against the fake server. The property test
`tests/execution_convergence.rs` runs 200 seeded interleavings of publishes, gaps, duplicates,
expired history, epoch changes, snapshot outages and disconnects; set `Q_TERMINAL_SEEDS=1000`
for more, or `Q_TERMINAL_SEED_ONLY=<n>` to replay one.

---

## Architectural Boundary

`q_terminal` is dedicated strictly to **live trading and operations**. It never grows research surfaces, strategy editors, backtesters, or parameter optimizers.

Furthermore, `q_terminal` **owns no backend process**: it launches nothing, supervises nothing, and stops nothing. See [`BOUNDARY.md`](BOUNDARY.md) for details.

---

## Prerequisites

Building and running `q_terminal` requires:

1. **Rust Toolchain**:
   - `rustc 1.98.0` (pinned in `rust-toolchain.toml`)
   - Components: `rustfmt`, `clippy`

2. **C++ Compiler & Build Tools**:
   - `g++` with C++17 support
   - `make`, `git`, `curl`
   - Debian/Ubuntu packages: `build-essential`, `lld`

3. **Qt 6 Libraries & Tooling**:
   - On Debian/Ubuntu:
     - `qt6-base-dev`
     - `qt6-declarative-dev`
     - `qt6-declarative-dev-tools` (provides `qmllint`)
     - `libgl1-mesa-dev` (or proprietary NVIDIA OpenGL driver)
   - Runtime dependencies: `libglib2.0-0t64` (or `libglib2.0-0`), `libdouble-conversion3`, `libpcre2-16-0`.
   - If a system Qt 6 SDK is not present, `qt-build-utils` / `cxx-qt-build` automatically downloads a minimal Qt 6 runtime (`qt_minimal`).

4. **Python**:
   - **Not required.** Building and running `q_terminal` does not require Python or Python development headers. `make contracts-check` uses `python3` or `uv` if present, but the core terminal build and runtime are strictly Rust and C++/Qt.

---

## Validation Suite

The standing validation command runs all checks headless without requiring a display.
Locally it enters the host user `ci.slice` when available, caps Cargo at half the
logical CPUs (`CARGO_BUILD_JOBS`), and lowers CPU/IO priority — do not wrap it in
`systemd-run`.

```bash
env -u WAYLAND_DISPLAY -u DISPLAY make check
```

This runs:
1. `fmt-check`: Rust formatting verification (`cargo fmt --all --check`).
2. `lint`: Clippy lints denying warnings (`cargo clippy --all-targets -- -D warnings`).
3. `build`: Compiles the binary and generated Qt artifacts.
4. `qml-lint`: Validates QML syntax and property bindings (`qmllint -W 0`).
5. `test`: Unit and integration test suite (`cargo test`).
6. `contracts-check`: Verifies vendored `contracts/` match clean regeneration against `CONTRACTS_REV`.
7. `headless-report`: Executes `cargo run -- --headless-report`.

Individual test suites can be run with:

```bash
# End-to-end lake-to-live-stream slice test
env -u WAYLAND_DISPLAY -u DISPLAY make test
# or specifically:
cargo test --test slice_end_to_end
```

