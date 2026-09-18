# Q-048: Execution controls

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Project direction:** [`q_contracts/docs/system-architecture.md` §4.5, §5.1, §6.1, §8.1, §9 invariants 4 and 9, §10 Phase 4](https://github.com/GuilhermeFortuna/q_contracts/blob/f2273a88e52c5b9a8ad5ac9d7eb27f8069cf643d/docs/system-architecture.md#45-control-path)  
**Depends on:** Q-044, Q-047  
**Implementation plan:** [`../plans/Q-048-execution-controls-plan.md`](../plans/Q-048-execution-controls-plan.md)

## Purpose

After Q-047 the terminal shows everything about execution and changes nothing.
The frontend's workspace is still the only place to start a deployment, flatten
a position, throw the kill switch or resolve an unknown order. This task adds
those commands to the terminal. Each goes over REST with an idempotency key
(Q-044). Each is enabled or disabled by the §8.1 table, not by guesswork, and
each asks for confirmation where a mistake costs money. The terminal shows a
command's effect only when the stream delivers it, never optimistically. The
seam with research stays where §5.1 puts it: the terminal deploys a saved
strategy that research produced, and never edits one.

## Requirements

### Commands

- The terminal can create a paper account; create a deployment from a saved
  backtest run, for a chosen account and broker mode; start, pause and stop a
  deployment; flatten a deployment; set and clear the kill switch; and resolve
  an unknown order as filled (with the fill's price, quantity and time) or not
  filled, with a note.
- Every command carries an idempotency key generated when the operator acts.
  The terminal's own retries of that action reuse the key. A new action gets a
  new key.
- A command's effect appears only when the stream delivers it. While a command is
  in flight its control shows that, and a command whose result arrives but whose
  stream event does not within a bound says so.
- A refused command shows the backend's error code and message, verbatim.
- The deployment picker lists saved backtest runs by strategy, symbol and
  timeframe only. It is a picker, not a result browser.

### Safety

- Flatten, stop, the kill switch (in both directions), resolving an order, and
  creating a live deployment each require an explicit confirmation that names
  what will happen. For live deployments the confirmation says so prominently.
- Controls follow §8.1 exactly:
  - Postgres unavailable disables start and deploy, and keeps flatten and the
    kill switch enabled when the edge is reachable.
  - Worker down disables start and deploy, and keeps the kill switch.
  - Edge or terminal down disables flatten, and keeps the kill switch.
  - API unreachable disables every command.
  - Redis down keeps commands enabled, because commands go over REST.
- Resolving an order is offered only for orders that are unknown and pending
  reconciliation, and never offers a default outcome.
- The terminal never sends a command without a key, and never sends two commands
  for one operator action.

## Constraints and non-goals

- **No strategy editing, parameter changes or research browsing.** A deployment's
  identity comes from the saved run.
- **No change to the live gates.** Creating an `mt5_live` deployment is allowed.
  Whether it can trade is still decided by the backend's gates, and the terminal
  shows the lock state from health.
- **No manual order entry.** Orders come from strategies. Flatten is the only
  operator-initiated order.
- **No keyboard shortcuts for commands that require confirmation.**

## Acceptance criteria

### Agent-verifiable

1. Against the fake server, each command sends one request with a UUID
   `Idempotency-Key`, the documented body, and no second request for one action.
   A transport retry reuses the key.
2. For each command, the control shows in-flight, then settles only when the
   matching stream event arrives. A delayed event shows the pending message
   after the bound.
3. A refused command (for example, an illegal lifecycle transition) shows the
   backend's code and message.
4. A test drives the full §8.1 enablement matrix, for every command under every
   degraded state, and it matches the table in the spec.
5. Every confirmation-requiring command sends nothing when the confirmation is
   cancelled.
6. Resolve is offered only for unknown, pending orders. The filled form refuses
   to submit without price, quantity and time.
7. The deployment picker lists saved runs, and a live deployment's confirmation
   carries the live warning.
8. `qmllint` passes with no warnings, and `BOUNDARY.md` lists the commands
   the terminal now owns.
9. `make check` passes.

### Human-verifiable

1. Against `./research`, the worker and the demo edge: create a paper account,
   deploy a saved MACrossover run on `WIN$N` M1, start it, pause it, start it,
   flatten it after an entry, stop it. Each change appears in the workspace
   from the stream, and the audit log shows exactly one command each.
   Command: `make run`, then `curl -s http://127.0.0.1:8000/api/v1/execution/audit-events | jq`
2. Set the kill switch while a deployment runs. The next signal is rejected as
   `kill_switch` and shown in the risk table. Clear it, and trading resumes.
3. With the edge stopped, flatten is disabled and the kill switch is still
   enabled. With the worker stopped, start and deploy are disabled.
