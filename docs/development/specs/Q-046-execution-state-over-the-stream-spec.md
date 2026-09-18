# Q-046: Execution state over the stream

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Project direction:** [`q_contracts/docs/system-architecture.md` §4.2, §4.3, §4.4, §5.1, §8.1, §10 Phase 4](https://github.com/GuilhermeFortuna/q_contracts/blob/f2273a88e52c5b9a8ad5ac9d7eb27f8069cf643d/docs/system-architecture.md#42-snapshot-then-delta-without-the-race)  
**Depends on:** Q-043  
**Implementation plan:** [`../plans/Q-046-execution-state-over-the-stream-plan.md`](../plans/Q-046-execution-state-over-the-stream-plan.md)

## Purpose

The terminal already holds live bars correctly: it subscribes first, snapshots,
discards at or below the watermark, fills gaps from history and re-snapshots on
an epoch change (Q-035). Everything it knows about execution it will learn the
same way, because §4.4 lets it assume every durable event arrives exactly once
in order, and the frontend's timers are exactly what phase 4 removes. After
Q-043 the backend publishes every execution change and serves a watermarked
execution snapshot. This task gives the terminal an execution store: an exact,
live, in-memory copy of deployments, accounts, positions, orders, decisions,
fills, risk events and the kill switch. It is kept by the same protocol as the
bars, over the same connection, and is testable without a window. Q-047 then
draws it.

## Requirements

### One connection, many topics

- The terminal keeps one stream connection. It subscribes to the bar topics it
  already uses and to the six execution topics, and each topic follows §4.2
  independently.
- The snapshot for the six execution topics is the single execution snapshot.
  A re-snapshot triggered by any one of them re-reads it and re-applies it to
  all six, each with its own watermark.
- Topic policy for the execution topics (class, no coalescing) is checked at
  startup against the vendored contract, as it is for the bar topics.

### The store

- The store holds each entity by kind and identifier, and applies an event by
  replacing the entity with the event's state. An event at or below its topic's
  applied sequence changes nothing.
- A fill updates the deployment's position from the fill's post-fill
  position, and a ledger entry updates the account from its post-entry
  balances. The store does no position or balance arithmetic of its own.
- Recent decisions, fills, orders and risk events are kept up to the contract's
  snapshot limits per deployment. Older ones are not kept, and are fetched on
  request from the paged REST routes.
- The store exposes a revision counter that increases on every applied change,
  so a view can redraw at most once per frame.
- A gap on a durable topic is filled from history by sequence. Expired history,
  a cursor older than retention, or an epoch change re-snapshots.

### Degraded states

- A disconnected stream keeps the last state and marks it with the time it was
  last confirmed, as §8.1 requires. On reconnect the store re-runs §4.2 and ends
  identical to a fresh snapshot.
- An API that answers `503` for the snapshot leaves the store in its previous
  state, marked unconfirmed, and retries with the stream's backoff.

### Parity with the bar path

- The bar path's behaviour is unchanged: every Q-035 to Q-038 test passes
  without edits to its expectations.

## Constraints and non-goals

- **No QML and no views.** The store is projected to QML in Q-047.
- **No commands.** Q-048.
- **No coalescing, and no dropping, of execution events** on the client. The
  per-frame redraw coalescing is the view's concern.
- **No use of routing keys** in client logic (Q-039).
- **No `q_core` change.** Execution entities are small, row-shaped and
  exact-decimal, not columnar market data. Invariant 5 governs market data
  paths.

## Acceptance criteria

### Agent-verifiable

1. Against the fake stream server, a scripted session of subscribe, snapshot,
   buffered deltas and live deltas across all six topics produces a store equal
   to the final snapshot. The buffered deltas include entries at, below and
   above each watermark.
2. A gap on each execution topic triggers a history fetch by sequence. Expired
   history triggers one execution re-snapshot that restores all six topics.
3. An epoch change on one execution topic re-snapshots, and the result equals a
   fresh snapshot.
4. A duplicated delivery of every event (relay at-least-once) leaves the same
   store as single delivery.
5. A disconnect and reconnect, with events published in between, converges to
   the fresh snapshot. The last-confirmed time is set while disconnected.
6. A `503` on the snapshot keeps the prior state, marks it unconfirmed, and
   recovers when the snapshot succeeds.
7. A property test over at least 200 seeded interleavings of publishes, gaps,
   duplicates and reconnects always converges to the final snapshot.
8. Startup fails with a policy error if the vendored policy marks an execution
   topic as coalescing.
9. The Q-035 to Q-038 tests pass unchanged, and the stream benchmark's bar
   figures stay within 10 % of their recorded values.
10. `make check` passes.

### Human-verifiable

1. With `./research` and a paper deployment running, the headless report
   prints the execution store's counts, and they match the backend's snapshot.
   After the API is restarted mid-run, the counts match again within one
   reconnect.
   Command: `cargo run -- --headless-report --execution` and
   `curl -s http://127.0.0.1:8000/api/v1/stream/execution/snapshot | jq '{deployments: (.deployments|length), orders: (.orders|length)}'`
