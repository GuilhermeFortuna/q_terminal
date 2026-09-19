# Q-046 implementation plan: Execution state over the stream

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Specification:** [`../specs/Q-046-execution-state-over-the-stream-spec.md`](../specs/Q-046-execution-state-over-the-stream-spec.md)  
**Depends on:** Q-043

## Current-system context

`src/stream/` is the Q-035 client. `topic_state.rs` holds a pure state machine
(`TopicState`, `Phase`, `Event`, `Action`, `on_event`) whose payload type is
`BarColumns`, and whose `Entry` event carries `symbol` and `timeframe` for
filtering. `client.rs::run_client_loop` opens `/api/v1/stream`, creates exactly
two states (`bars.forming`, `bars.completed`), sends
`{"topics": ["bars.forming", "bars.completed"]}`, dispatches control frames
by `if topic == …` chains, and fetches snapshots through
`fetch_and_apply_snapshot` (`/api/v1/stream/{topic}/latest`) and history
through `handle_actions`
(`/api/v1/stream/{topic}/history?epoch=&from_seq=&limit=500`). Backoff is
`50 ms · 2^(attempt-1)`, capped at 30 s. `ClientShared` carries connection
state and counters (`applied`, `dropped`, `gaps_closed`, `resnapshots`,
`rest_calls`) with a change listener. `policy.rs::KNOWN_TOPICS` checks the
two bar topics against the vendored policy at startup. `fake_server.rs` is a
scriptable in-process stream and REST server used by `tests/stream_protocol.rs`,
`tests/stream_faults.rs` and `tests/bench_stream.rs`. `main.rs` has
`--headless-report`.

`CONTRACTS_REV` is `998a5057`. `contracts/stream.rs` has no execution payloads
until the pin moves to a commit containing Q-039. After Q-043 the backend serves
`GET /api/v1/stream/execution/snapshot`, and every execution topic carries
Q-039 payloads in `payload_kind: "control"` envelopes.

## Interfaces produced

```rust
// src/stream/topic_state.rs   (generalized)
pub enum Payload { Bars(BarColumns), Execution(ExecutionEvent) }
pub struct TopicState { pub topic: String, pub filter: TopicFilter, /* phase, epoch, last_applied_seq, buffered */ }
pub enum TopicFilter { Bars { symbol: String, timeframe: String }, None }
// Event::Snapshot / Entry / HistoryPage carry Payload; Action::Apply(Payload); logic unchanged

// src/stream/execution.rs   (new)
pub enum ExecutionEvent { Deployment(..), Decision(..), Order(..), Fill(..), Risk(..), Ledger(..) }  // contract types
pub fn decode_execution_entry(topic: &str, payload: &serde_json::Value) -> Result<ExecutionEvent, DecodeError>;

// src/execution/store.rs   (new)
pub struct ExecutionStore { /* maps by id; per-deployment recent rings; control; revision; confirmed_at */ }
impl ExecutionStore {
    pub fn apply_snapshot(&mut self, snap: ExecutionSnapshot, topics: &[&str]);   // replaces the listed topics' entities
    pub fn apply(&mut self, event: ExecutionEvent);                                 // replace-by-id
    pub fn revision(&self) -> u64;
    pub fn mark_unconfirmed(&mut self, since: SystemTime);
    pub fn view(&self) -> ExecutionView<'_>;                                        // read accessors for Q-047
}
pub struct ExecutionHandle(Arc<Mutex<ExecutionStore>>);                             // shared with the Qt side, listener like BarSink

// src/stream/client.rs   (changed)
pub struct StreamClient { /* ... */ }
impl StreamClient { pub fn start(config: Config, sinks: Sinks) -> Self; }          // Sinks { bars: BarSink, execution: ExecutionHandle }
// one TopicState per subscribed topic in a HashMap<String, TopicState>; execution snapshot shared by the six

// src/stream/policy.rs   (changed)
KNOWN_TOPICS += decisions, orders, fills, risk, ledger, deployments (durable, no coalesce key)

// src/main.rs   (changed)
--headless-report --execution     // connects, converges, prints store counts and the six applied seqs, exits
```

```
tests/execution_store.rs          new: criteria 1–6, 8 against fake_server
tests/execution_convergence.rs    new: criterion 7 (seeded; 200 in CI, more with Q_TERMINAL_SEEDS)
src/stream/fake_server.rs         extended: execution snapshot route, execution envelopes, history by seq for durable topics
CONTRACTS_REV                     → the Q-043 recapture commit in q_contracts (includes Q-039)
```

## Implementation decisions

- **Generalize `TopicState` over a payload enum; do not fork it.** The phase
  machine (buffer until snapshot, discard at or below watermark, gap to
  history, expiry and epoch to re-snapshot) is exactly §4.2. It is already
  proven by the Q-035 tests, and it is the part that must not diverge between
  bars and execution. The payload and the filter become enums, and every existing
  test keeps its expectations, because bar behaviour is unchanged.

- **A topic map replaces the `if topic == …` chains.** `run_client_loop` holds
  `HashMap<String, TopicState>` and dispatches control frames by name. That is
  the change that lets a topic set grow without adding branches. It happens in
  one commit with no behaviour change, verified by the existing stream tests,
  before any execution code lands.

- **One execution snapshot fans out to six topic states.** Each state's
  watermark comes from the snapshot's watermark for its topic. A re-snapshot
  action from any execution topic is deduplicated within one event-loop turn,
  so that a Redis restart (six `epoch_changed`) costs one REST call.
  `apply_snapshot` replaces only the entities owned by the topics being
  re-snapshotted. Position and account come from the snapshot's collections.

- **Positions are owned by `fills` and accounts by `ledger`** in the store's
  replacement rules. That matches Q-039: a `fills` re-snapshot replaces
  positions, and a `ledger` re-snapshot replaces accounts.

- **Recent rings are bounded by the snapshot's declared `limits`,** read from the
  snapshot, not hard-coded. Q-047 pages older rows from the existing REST
  routes on demand.

- **Decimals stay strings in the store.** The store never computes with them.
  Q-047 formats them for display. Parsing to `f64` would lose exactness for no
  benefit.

- **The convergence property test reuses `fake_server`,** scripting a random
  publish log with injected gaps, duplicate deliveries, expired history,
  epoch changes and disconnects. Its oracle is the fake server's own final
  snapshot.

## Ordered implementation

- [x] 1. Work on the branch `Q-046-execution-state-over-the-stream` in `q_terminal`,
   created from `development` by `./work start`. Confirm Q-043 and its
   `q_contracts` recapture are merged. Move `CONTRACTS_REV`, run
   `make contracts` and `make contracts-check`. Confirm the execution types are
   in `contracts/stream.rs`. Commit.
- [x] 2. Refactor `run_client_loop` to a topic map, with no behaviour change.
   Confirm `tests/stream_protocol.rs`, `tests/stream_faults.rs` and
   `slice_end_to_end.rs` pass unchanged. Commit.
- [x] 3. Generalize `TopicState` to `Payload` and `TopicFilter`. Confirm every
   `topic_state.rs` unit test and the stream tests pass unchanged. Commit.
- [x] 4. Add the execution topics to `policy.rs`, with a test for criterion 8.
   Commit.
- [x] 5. Write `src/stream/execution.rs` (decode) and `src/execution/store.rs`, with
   unit tests: replace-by-id; at-or-below-watermark no-op; fill updates
   position; ledger updates account; rings bounded by limits; revision
   increments on change only. Commit.
- [x] 6. Extend `fake_server.rs` with the execution snapshot and durable-topic
   history. Write `tests/execution_store.rs` for criteria 1–6. Wire the
   execution sink and the fan-out snapshot into the client. Confirm they
   pass. Commit.
- [x] 7. Write `tests/execution_convergence.rs` (criterion 7) and fix anything it
   finds. Commit.
- [x] 8. Add `--headless-report --execution`. Commit.
- [x] 9. Run `make bench-stream` and compare the bar figures with the Q-035
   record (criterion 9). Commit the numbers in the handoff, not in the repo.
- [x] 10. Update `README.md` (execution store, headless report flag). Run
   `env -u WAYLAND_DISPLAY -u DISPLAY make check`. Fix, re-run, commit.
- [ ] 11. **Human:** human-verifiable criterion 1.

## Validation

- **Unit:** decode per topic; store replacement and bounds; generalized state
  machine.
- **Integration:** full protocol against the fake server across the six topics,
  with gaps, expiry, epoch change, duplicates, reconnect and `503`.
- **Property:** seeded convergence.
- **Regression:** every Q-035 to Q-038 test unchanged; the bar benchmark within
  10 %; `make contracts-check`.
- **Manual:** headless counts against the live backend, across an API restart.

```bash
cd /home/gui/projects/q/q_terminal
env -u WAYLAND_DISPLAY -u DISPLAY make check
RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo test --test execution_store --test execution_convergence
make bench-stream

# human (step 11)
cd /home/gui/projects/q && ./research
cd /home/gui/projects/q/q_terminal && cargo run -- --headless-report --execution
```

## Handoff

Report the refactor commits and confirm the existing tests were unchanged at
each. Give the convergence test's seed count and anything it found. Give the
bar benchmark before and after. From the human step, give the headless counts
and the snapshot counts, before and after the API restart.
