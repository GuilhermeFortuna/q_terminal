# Q-035 implementation plan: Terminal stream client

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Specification:** [`../specs/Q-035-terminal-stream-client-spec.md`](../specs/Q-035-terminal-stream-client-spec.md)  
**Depends on:** Q-034

## Current-system context

`q_terminal` is the Q-008 skeleton. `src/main.rs` (43 lines) initialises three
cxx-qt crates, takes `--headless-report` (four printed lines: app version, core
version, contracts rev, render backend) or loads `qml/Main.qml` into a
`QQmlApplicationEngine` and runs the event loop. `src/bridge.rs` declares one
`AppInfo` QObject with four `QString` properties and reads `q_qt::CoreInfoRust`
through a `std::mem::transmute` mirror. `src/render_backend.cpp` reports the
scene-graph graphics API. `qml/Main.qml` is a 640×480 window with four `Text`
items. `tests/test_bridge.rs` and `tests/test_headless_report.rs` assert those
strings, including the hard-coded core version `2026.9.12`.

`Cargo.toml` has no HTTP, WebSocket, async or Arrow dependency. `q-qt` is a git
dependency pinned to the tag `v2026.09.12`; `q_core` is at `v2026.09.15.2` and
Q-034 adds a further tag that this task must pin, which also updates the
`2026.9.12` assertions in both tests.

`CONTRACTS_REV` is `998a50570905524bfb9af0465a725b170f2970df`. `contracts/`
holds `api.rs` (1,983 lines), `catalog.rs`, `edge.rs` and `stream.rs` (44
lines: `StreamEnvelope`, `SubscribeFrame`, `SubscribedFrame`,
`CursorExpiredFrame`, `LaggingFrame`, `EpochChangedFrame`). `make contracts-check`
diffs the tree against a clean regeneration at the pin. The generated topic
policy from Q-009 is available in the same generated tree and is the only
source for topic names, classes and coalesce keys.

`schema/stream/framing.md` §2.2 defines the binary frame as
`u32 little-endian header_len | UTF-8 JSON envelope without payload | raw Arrow
IPC stream bytes`, and §2.1 the one classification rule for text frames: an
object with `type` is that control frame, otherwise it is an envelope.

`schema/api/openapi.yaml` gives the REST half: `GET /api/v1/stream/{topic}/latest`
with an optional `key`, answering `LatestResponse {topic, entries}`, and
`GET /api/v1/stream/{topic}/history`, answering `HistoryPageResponse {topic,
epoch, entries, next_seq}` or the expired form. `topics.yaml` declares
`bars.forming` ephemeral, coalescing on `(symbol, timeframe)` with roughly one
hour of retention, and `bars.completed` ephemeral, never coalesced, overflowing
to `lagging`, with roughly a day.

Q-014's server behaviour this client is written against: subscription
acknowledgement per topic with cursor, epoch and last sequence; entries in
stream order at most once per connection; `lagging {topic, from_seq}` on
overflow of a non-coalescing topic; `epoch_changed` before any entry of a new
epoch and on Redis loss; `503 stream_unavailable` when Redis is unreachable at
connect, and a rejection then close if it becomes unreachable mid-connection.

`BOUNDARY.md` forbids launching, supervising or stopping any backend process.
`make check` routes through `scripts/ci.sh` and runs
`fmt-check lint build qml-lint test contracts-check` plus the headless report.

The gap is that nothing in this repository opens a socket.

## Interfaces produced

```rust
// src/config.rs
/// Read once at startup from $XDG_CONFIG_HOME/q_terminal/config.toml, with
/// every field overridable by an environment variable. No default address is
/// compiled into a binary that talks to a machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config { pub api_base: String, pub symbol: String, pub timeframe: String }
impl Config { pub fn load() -> Result<Self, ConfigError>; }

// src/stream/frame.rs
pub struct BinaryFrame<'a> { pub header: EnvelopeHeader, pub arrow: &'a [u8] }
#[derive(Debug, Clone, serde::Deserialize)]
pub struct EnvelopeHeader { /* topic, schema_major, seq, epoch, producer_id,
                              origin_ts, payload_kind, payload_schema, key */ }
pub enum ServerFrame { Control(ControlFrame), Envelope(EnvelopeHeader), Binary(/* offsets */) }
pub enum ControlFrame { Subscribed(..), Rejected(..), CursorExpired(..), Lagging(..), EpochChanged(..) }

/// Splits a binary frame; never base64-decodes. Errors are per frame and never
/// close the connection.
pub fn split_binary(bytes: &[u8]) -> Result<BinaryFrame<'_>, FrameError>;
/// Classifies a text frame by the presence of `type`, and by nothing else.
pub fn classify_text(text: &str) -> Result<ServerFrame, FrameError>;

// src/stream/topic_state.rs
/// Per-topic position in the §4.2 protocol.
pub struct TopicState { /* epoch, last_applied_seq, phase, buffered: Vec<Pending> */ }
pub enum Phase { Subscribing, Buffering, Live }
pub enum Action { Apply(BarColumns), FetchHistory { from_seq: i64 }, ReSnapshot, Drop(DropReason) }

/// The whole protocol as a pure function of (state, event): what the client does
/// with an acknowledgement, an entry, a gap, a lagging notice, an epoch change,
/// a snapshot, or a history page. Holds no socket and no clock.
pub fn on_event(state: &mut TopicState, event: Event) -> Vec<Action>;

// src/stream/client.rs
pub enum ConnectionState { Connecting, Live, Reconnecting { attempt: u32 }, Unavailable }
pub struct Counters { pub applied: u64, pub dropped: u64, pub gaps_closed: u64, pub resnapshots: u64 }

/// Owns the runtime thread, the socket and the REST calls. Delivers columns to
/// the Qt object through a `CxxQtThread`; never touches it directly.
pub struct StreamClient { /* ... */ }
impl StreamClient {
    pub fn start(config: Config, sink: BarSink) -> Self;
    pub fn shutdown(self);
}

/// The only thing the transport can do to the UI: hand it columns, or a state.
pub struct BarSink { /* CxxQtThread<BarFeed>, coalescing slot for forming bars */ }

// src/bridge/bar_feed.rs  (cxx_qt bridge)
// QObject `BarFeed`, QML-exposed properties:
//   connection_state, last_error (QString); data_age_ms (i64);
//   applied, dropped, gaps_closed, resnapshots (i64)
// It owns the Q-034 `BarSeries` and forwards deliveries into it on the UI thread.
```

## Implementation decisions

- **The protocol is a pure state machine tested without a socket, and the
  transport is a thin shell around it.** Every rule in §4.2 and §4.3 —
  buffering, the watermark discard, gap detection, epoch reset — is a decision
  about `(state, event)`, not about IO. Writing it as a function makes the
  interesting cases table-driven unit tests instead of timing-dependent
  integration tests, which is the only way the expired-history and
  epoch-change paths get covered honestly.

- **`tokio` with a dedicated multi-thread runtime on its own thread, plus
  `tokio-tungstenite` and `reqwest` with `rustls`.** Qt owns the main thread's
  event loop and cannot host an async executor; a separate runtime is the only
  arrangement that keeps socket work off it. `rustls` avoids a system OpenSSL
  dependency in a repository whose build already downloads its own Qt.

- **Delivery crosses to the UI thread through cxx-qt's `CxxQtThread` queued
  closure, never through a shared mutable series.** The series' pointer handoff
  (Q-034) is safe only because mutation happens on one thread; a mutex would
  move the decode's cost onto the render path at exactly the wrong moment.

- **Forming-bar coalescing is a single-slot replace on the sink, drained by the
  UI thread.** §4.4 says the client coalesces again on its side to one render
  per frame. A slot gives that for free and bounds memory at one bar; a queue
  would grow under a stalled UI and then deliver a burst of superseded bars.

- **Completed bars are delivered through an unbounded ordered channel and never
  coalesced.** The topic's policy is `on_overflow: lag`, so the server has
  already decided that dropping one is a protocol event, not a rendering
  convenience. If this channel ever grew without bound the client would be
  failing to apply, which the counters make visible.

- **Gap closing is strictly ordered before later entries.** The client holds
  entries above the gap in the topic's buffer while history is fetched, then
  applies history and buffer in sequence order. Applying out of order would put
  the series into the `BeforeLast` refusal Q-034 defines, which is the correct
  failure but a worse one than not creating the gap.

- **Every frame-level failure is a counted drop, never a disconnect.** A
  malformed frame is one producer's defect; tearing down a healthy connection
  over it would turn a cosmetic fault into an outage, and §8.1 asks the terminal
  to keep its last state and show it aging rather than to go blank.

- **Topic names, coalesce keys and classes come from the generated policy, and
  the client asserts at startup that the two topics it wants are declared with
  the classes it assumes.** `contracts-check` keeps the vendored copy honest;
  the startup assertion keeps a future policy change from silently turning a
  never-coalesced topic into a coalescing one.

- **Configuration is read at startup and never re-read.** A terminal that
  changes its API address while connected has no defined state for the applied
  series, and the slice has no need for it.

## Ordered implementation

- [x] 1. Work on the branch `Q-035-terminal-stream-client` in `q_terminal`,
   created from `development` by `./work start`. Confirm Q-034 has merged and
   has a `q_core` release tag; pin `q-qt` to it, update the core-version
   assertions in `tests/test_bridge.rs` and `tests/test_headless_report.rs`, and
   confirm `make check` passes before adding anything. Commit.
- [ ] 2. Add `tokio`, `tokio-tungstenite`, `reqwest` (rustls, json), `serde`,
   `serde_json` and `arrow` to `Cargo.toml`, with default features off where
   they pull in a TLS or time-zone stack. Confirm `make build` and the headless
   report still work. Commit.
- [ ] 3. Write failing tests for `config.rs`: a complete file loads; a missing
   file with the environment variables set loads; a missing address is an error
   naming the field; no address is baked in. Implement. Confirm they pass.
   Commit.
- [ ] 4. Write failing tests in `stream/frame.rs` against bytes built in the
   test: a well-formed binary frame splits into header and Arrow bytes; a
   three-byte frame, a `header_len` past the end, a non-JSON header, and a
   header missing `seq` each give a `FrameError`; a text frame with `type` of
   each of the five control values classifies as that frame; one without `type`
   classifies as an envelope; a frame whose `payload_schema` is not the topic's
   is rejected. Implement `split_binary` and `classify_text`. Confirm they pass.
   Commit.
- [ ] 5. Write failing table-driven tests for `on_event` covering: buffer before
   snapshot; discard at or below the watermark; apply above it in order; a
   duplicate sequence applied once; a gap emitting `FetchHistory` and holding
   later entries; history closing the gap and releasing the buffer; expired
   history emitting `ReSnapshot`; `epoch_changed` resetting one topic only;
   `lagging` and `cursor_expired` emitting `ReSnapshot`; a forming-topic gap
   emitting `ReSnapshot` rather than `FetchHistory`; an entry for another symbol
   emitting `Drop`. Implement `topic_state.rs`. Confirm they pass. Commit.
- [ ] 6. Write failing tests for the sink: 1,000 forming deliveries between two
   drains leave one bar, the newest; 1,000 completed deliveries leave 1,000 in
   order; a completed bar and a superseding forming bar drain in that order.
   Implement `BarSink`. Confirm they pass. Commit.
- [ ] 7. Add the `BarFeed` bridge owning the Q-034 `BarSeries`, with the
   properties and the drain that applies deliveries on the UI thread. Write
   failing tests that drive it headless and assert the series advances and the
   counters move. Implement. Confirm they pass. Commit.
- [ ] 8. Write a fake stream server in the test support module: it speaks the
   framing, serves `latest` and `history`, and can be told to send a gap, an
   epoch change, a lagging notice, an expired history range, a 503, or to close
   mid-stream. Write failing end-to-end tests for connect-snapshot-live, each
   fault, and reconnection with capped backoff. Implement `client.rs`. Confirm
   they pass. Commit.
- [ ] 9. Write failing tests for shutdown: the runtime thread joins, and a
   delivery queued during teardown lands nowhere. Implement. Confirm they pass.
   Commit.
- [ ] 10. Add a measurement test behind `make bench-stream`: a synthetic
   full-rate stream for sixty seconds, reporting process CPU, the 95th-percentile
   socket-to-revision delay, and steady-state allocations per entry. Commit.
- [ ] 11. Add the startup assertion that both topics are declared with the
   expected class and coalesce key, with a test for a policy that disagrees.
   Commit.
- [ ] 12. Run `env -u WAYLAND_DISPLAY -u DISPLAY make check`. Fix, re-run,
   commit.
- [ ] 13. **Human:** run the research stack and the terminal against a real
   symbol for thirty minutes; restart Redis under it; stop and restart the API
   under it. Report each observation.

## Validation

- **Unit:** configuration; framing split and classification, with every
  malformed case; the protocol state machine's full table; sink coalescing and
  ordering; the policy assertion.
- **Integration:** the fake server end to end, per fault: gap, expired history,
  epoch change, lagging, cursor expired, 503, mid-stream close, reconnection
  backoff.
- **Regression:** the headless report's four lines; `make contracts-check`;
  `qml-lint`; the existing bridge tests with the new core version.
- **Threading:** no IO or decode on the UI thread; clean shutdown.
- **Measurement:** CPU, delivery delay, allocations per entry.
- **Manual:** thirty minutes against the live publisher; Redis restart; API
  restart.

```bash
cd /home/gui/projects/q/q_terminal
env -u WAYLAND_DISPLAY -u DISPLAY make check
cargo test --test stream_protocol
cargo test --test stream_faults
make bench-stream

# human (step 13)
cd /home/gui/projects/q && ./research
cd /home/gui/projects/q/q_terminal && make run
```

## Handoff

Report the `q_core` tag pinned and the two test assertions updated with it.
Report the measured CPU, delivery delay and per-entry allocations. Report, from
the thirty-minute run, the counts of applied entries, dropped frames, gaps
closed and re-snapshots, with a cause for every drop. Report what the Redis and
API restarts did, and how long recovery took. State explicitly that no quote,
durable topic, command or catalog code landed, and that the terminal still
launches nothing.
