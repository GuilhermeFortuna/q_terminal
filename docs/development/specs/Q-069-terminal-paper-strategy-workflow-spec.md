# Q-069: Terminal paper strategy workflow

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Project direction:** [`BOUNDARY.md`](../../../BOUNDARY.md), [`docs/design/ui-direction.md`](../../design/ui-direction.md)  
**Depends on:** Q-068  
**Implementation plan:** [`../plans/Q-069-terminal-paper-strategy-workflow-plan.md`](../plans/Q-069-terminal-paper-strategy-workflow-plan.md)

## Purpose

The terminal currently creates deployments from a saved-backtest picker and
cannot set strategy parameters, symbol or paper costs. This task makes it the
operator's paper deployment configuration and audit surface. Research remains
the place for strategy creation, optimization and backtest analysis; the
terminal edits only a selected registry strategy's execution configuration.

## Requirements

### Create a paper deployment

- Replace the saved-run-only create flow with a catalog-first form populated
  by Q-067 `GET /api/v1/execution/strategy-catalog`. Show built-in candle
  strategies and saved custom wrappers, with description, provenance and
  typed entry/exit parameters. Preserve saved-run deployment as an optional
  source path without opening a backtest result browser.
- Let the operator choose an exact symbol using the existing market symbol
  search, an M15-or-slower timeframe, a paper account, sizing and risk limits,
  and per-deployment point value, slippage and commission. Show defaults and
  units. A review step displays strategy, parameters, symbol/timeframe, costs,
  account and `PAPER` before submission.
- The terminal sends the catalog request to the backend with one UUID
  `Idempotency-Key` per action; retries reuse the key. It never builds a
  compiled config, computes a hash, evaluates a strategy or sends an order.
  Backend validation messages appear beside the relevant field when possible.
- The create form offers only paper mode in the dev workflow. No UI action
  sets `live_activation_enabled` or offers an MT5 live deployment. Existing
  live rows, if present, remain visibly locked and non-startable under the
  dev profile.

### Edit a paused paper deployment

- Show the current revision and full server-owned configuration. Permit edits
  to parameters, sizing, risk and paper costs in `draft`, or `paused` when
  flat, with no pending unknown order or flatten action. Strategy identity,
  symbol, timeframe, account and broker mode are read-only during edit.
- Submit the complete replacement plus `expected_revision` and actor through
  Q-067 PATCH with an idempotency key. Show a review summary of changed
  values. A `409` conflict refreshes current state and explains why the edit
  was refused; it never silently overwrites another revision.
- A successful HTTP response is not applied optimistically to the execution
  store. The new configuration appears when the deployment stream event
  arrives. The UI explains that resume evaluates only newly completed bars.

### Audit and results

- The selected deployment shows an ordered decision → intent → dispatch
  attempt → paper fill/rejection trail with timestamps, revision, reason,
  quote bid/ask/time, fill price, slippage and fee. Unknown orders remain
  distinct from rejections; an attempted dispatch is not labeled filled.
- Show Q-068 per-deployment realized/unrealized/net P&L, fees, closed-trade
  count, win rate and completed-bar equity-delta history. Indicate when the
  current mark is unavailable. Keep the existing paper-account balance and
  equity separate from deployment P&L.
- Fetch performance on selection and after completed-bar deployment events,
  with generation-safe cancellation when selection changes. If the mark for
  that event's bar has not committed yet, show it as pending and retry that
  bar until the matching mark or a bounded unavailable result arrives. Do not add a
  per-window poller or perform money arithmetic in QML/Rust. Keep the existing
  one-process, one-execution-store, one-stream shell architecture.

### Terminal boundary and design

- Update `BOUNDARY.md` and README to permit registry strategy selection and
  deployment parameter configuration only. Continue to prohibit strategy
  code editing, custom-strategy authoring, optimizers, result browsers and
  backend process ownership.
- Adapt the existing design references: MuseScore 4's inspector grouping for
  the configuration form, Qt Quick Controls for accessible fields/focus, and
  Wireshark's dense table/detail pairing for the audit trail. Use existing
  theme tokens and semantic roles, and record the adaptation in
  `docs/design/components.md`.

## Acceptance criteria

1. Against a fake API, catalog and custom-wrapper choices render typed defaults
   and valid choices. Tick/genome entries cannot be selected; symbols and
   timeframes are validated before submission and again by the backend.
2. Creating a paper deployment sends the chosen values once with an idempotency
   key and no live activation; the new row appears only after its stream event.
3. Draft and paused-flat edits send complete replacements and the current
   revision. Running/open-position/unknown-order edits are disabled with a
   reason. A stale-revision response leaves the store unchanged and offers
   refresh.
4. Audit and performance views distinguish attempt, fill, rejection, unknown,
   stale mark, account equity and deployment net P&L. Selection changes cannot
   display another deployment's late response.
5. Keyboard/focus, screen-reader labels, narrow window and detached-window
   states work. Gallery captures are inspected and documented against the
   named references; the frame-time budget remains p95 under 16 ms.
6. `BOUNDARY.md` states the narrow exception, `make contracts-check` passes,
   and `env -u WAYLAND_DISPLAY -u DISPLAY make check` passes.
7. In the paper walkthrough, configure a catalog strategy and symbol, inspect
   one completed-bar decision and paper fill, pause while flat, edit a
   parameter, resume, and see the next new-bar decision carry the new revision.
