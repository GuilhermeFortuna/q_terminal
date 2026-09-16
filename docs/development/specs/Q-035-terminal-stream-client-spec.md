# Q-035: Terminal stream client

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Project direction:** [`q_contracts/docs/system-architecture.md` §4.2, §4.3, §4.4, §8.1, §10 Phase 3](https://github.com/GuilhermeFortuna/q_contracts/blob/a9724767015905f1e9cd5d98ba492080910db4f2/docs/system-architecture.md#42-snapshot-then-delta-without-the-race)  
**Depends on:** Q-034  
**Implementation plan:** [`../plans/Q-035-terminal-stream-client-plan.md`](../plans/Q-035-terminal-stream-client-plan.md)

## Purpose

The slice's rule is that live bars reach the terminal only over the stream. The
backend has published them since Q-013, served them over a WebSocket since
Q-014, and answered snapshot and history requests since Q-015, but `q_terminal`
has no network code at all: it is a window that prints four version strings.
This task gives it the client half of the §4.2 protocol for one symbol's bars —
subscribe, snapshot, reconcile, then live — with the gap, epoch, lag and
disconnection handling §4.3 and §8.1 require, and delivers decoded bars into the
Q-034 series without ever computing on, or blocking, the UI thread.

## Requirements

### Connect and subscribe

- The client connects to the configured API's stream endpoint and subscribes to
  the forming-bars and completed-bars topics for one configured symbol and
  timeframe. Address, symbol and timeframe come from configuration the terminal
  reads at startup; none of them is compiled in.
- Entries arriving after the subscription acknowledgement are buffered, not
  applied, until the snapshot has been applied.
- A topic the server rejects is reported with the server's reason and leaves the
  other topic's subscription working.

### Snapshot and reconciliation

- After the acknowledgement, the client fetches each subscribed topic's latest
  value over REST, applies it, discards every buffered entry at or below that
  topic's watermark, applies the rest in sequence order, and only then continues
  live.
- The client tracks, per topic, the epoch and the last sequence it applied, and
  it applies an entry at most once for a given epoch and sequence.
- A sequence gap on the completed-bars topic is closed from REST history before
  any later entry is applied. A gap that history cannot close, because the range
  has expired, is treated as a re-snapshot.
- A gap on the forming-bars topic is not a gap: the topic coalesces, and the
  client resynchronises from the latest value.
- An epoch change on a topic discards that topic's applied state and re-runs the
  whole protocol for it, using the same code path as the first connection.
- A lagging notice, an expired cursor, and a stream-unavailable rejection each
  re-run the protocol for the topics they name.

### Decoding

- Binary frames are decoded as the framing specification defines them: a
  little-endian length prefix, a JSON envelope header without the payload, then
  raw Arrow IPC stream bytes, which are never base64-decoded on this path.
- Text frames are classified by the presence of a discriminator: with one, the
  frame is that control frame; without one, it is an envelope. No other key is
  used to tell them apart.
- A frame whose header is malformed, whose declared payload schema is not the
  one the topic's policy names, or whose Arrow payload does not match the bars
  schema, is dropped, counted and reported, and does not disturb the connection
  or the applied state.
- A frame for a topic the client did not subscribe to, or for a symbol or
  timeframe other than the configured one, is dropped and counted.

### Threading and delivery

- All socket, HTTP and decode work happens off the Qt UI thread. The UI thread
  is handed decoded columns, and never bytes, sockets or futures.
- Forming-bar updates are coalesced on the client side to at most one delivery
  per displayed frame; completed bars are never coalesced and never dropped.
- Delivery into the series preserves order: a completed bar is applied before any
  forming bar that supersedes it.
- The client stops cleanly on application exit: no thread outlives the process,
  and no delivery lands after the series is gone.

### Connection state

- The client exposes to QML, as properties that notify on change: a connection
  state drawn from a closed set that at least covers connecting, live,
  reconnecting and unavailable; the age of the most recent applied entry; the
  last error's reason; and counters for applied entries, dropped frames, gaps
  closed from history, and re-snapshots.
- Reconnection is automatic, with exponential backoff capped at thirty seconds,
  and every reconnection re-runs the snapshot protocol.
- While disconnected the last applied state is kept and its age keeps
  increasing, so the surface can show data it no longer trusts as stale rather
  than as current.

### Cost

- One symbol's forming and completed bars at full rate cost less than five
  percent of one core in the terminal process, and no allocation per entry on
  the steady-state path beyond the decoded batch itself.
- Delivery adds less than 10 ms, at the 95th percentile, between a frame
  arriving on the socket and the series revision advancing.

### Boundaries preserved

- The terminal launches, supervises and stops nothing. If the API or the stream
  is unavailable, the client reports it and retries; it never starts a service.
- The vendored contracts are used as the source of topic names, control frames
  and policy. No retention, backpressure or coalesce key is hand-copied.
- The existing headless report keeps its four lines and its exit code, and the
  existing checks pass unchanged.

## Constraints and non-goals

- **No quotes topic.** The slice renders bars. The last price comes from the
  forming bar. A quote consumer lands with the surface that needs one.
- **No durable topics.** Decisions, orders, fills, risk and deployments belong to
  the execution and operations surfaces in phase 4.
- **No commands.** This client only reads. Every command goes over REST in a
  later task.
- **No historical backfill.** Filling the chart from the catalog is Q-036; this
  task's REST use is limited to the snapshot and the gap-closing history the
  protocol requires.
- **No rendering and no QML surface.** The chart is Q-037 and Q-038; this task's
  observable output is the series' state and the exposed properties.
- **No authentication.** The API is local and unauthenticated today; adding a
  credential path is a decision taken when the API grows one.
- **No multi-symbol subscription.** One symbol, one timeframe, per the phase 3
  slice.

## Acceptance criteria

### Agent-verifiable

1. Tests against a fake server state the protocol as cases: entries arriving
   before the snapshot are buffered and then discarded at or below the
   watermark; entries above it apply in order; a duplicate sequence applies
   once; a gap on completed bars is closed from history before later entries
   apply; an expired history range re-snapshots; an epoch change re-snapshots
   that topic only; a lagging notice and a cursor-expired notice each re-run the
   protocol; a rejected topic leaves the other working.
2. Decoder tests state the framing rules as cases: a well-formed binary frame
   yields its header and batch; a truncated length prefix, a header that is not
   JSON, a declared schema that is not the topic's, and an Arrow payload with a
   wrong column each drop the frame, advance the drop counter, and leave the
   connection open; a text frame with a discriminator is the named control
   frame and one without is an envelope.
3. A test asserts no socket, HTTP or Arrow work runs on the UI thread, and that
   1,000 forming updates between two displayed frames produce one delivery while
   1,000 completed bars produce 1,000.
4. Reconnection tests: the server closing mid-stream reconnects with growing
   backoff capped at thirty seconds, re-runs the protocol, and reports the
   states in order; a 503 at connect leaves the client retrying and reporting
   unavailable rather than failing.
5. A shutdown test asserts no thread outlives the process and no delivery lands
   after teardown.
6. A measurement test reports CPU cost and the 95th-percentile socket-to-revision
   delay over a synthetic full-rate stream.
7. `make contracts-check` passes with the vendored contracts regenerated at the
   pinned revision, and the headless report still prints its four lines.
8. The full validation suite passes: `env -u WAYLAND_DISPLAY -u DISPLAY make check`.

### Human-verifiable

1. Against the running research stack with the live publisher on a real symbol,
   the client is run for thirty minutes and reports zero unexplained drops, the
   number of gaps closed and re-snapshots taken, and the observed delivery
   delay.
   Command: `./research` in the workspace, then `cd q_terminal && make run`
2. Redis is restarted under the running client; the client is confirmed to
   re-snapshot both topics, recover without a restart, and report the epoch
   change.
3. The API is stopped under the running client; the client is confirmed to show
   unavailable with a growing data age, and to recover when the API returns.
