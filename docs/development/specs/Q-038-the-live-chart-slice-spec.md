# Q-038: The live chart slice

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Project direction:** [`q_contracts/docs/system-architecture.md` §4.6, §5.1, §8.1, §10 Phase 3](https://github.com/GuilhermeFortuna/q_contracts/blob/a9724767015905f1e9cd5d98ba492080910db4f2/docs/system-architecture.md#10-roadmap)  
**Depends on:** Q-035, Q-036, Q-037  
**Implementation plan:** [`../plans/Q-038-the-live-chart-slice-plan.md`](../plans/Q-038-the-live-chart-slice-plan.md)

## Purpose

Phase 3 is one live symbol, forming bars fed only by the stream, drawn by a
scene-graph node from a `q_core` buffer, with a catalog-driven historical load
and no controls. The four tasks before this one build those pieces and each
proves its own part. Nothing yet assembles them into the window a person opens,
decides what the chart shows as bars arrive, or says what the surface does when
the stream, the API or the lake is unavailable. This task is the slice itself:
the scene, the viewport policy, the degraded states §8.1 requires, and the
end-to-end proof that the data path and the FFI work from the lake and the
socket to the screen.

## Requirements

### The scene

- The window shows one chart for the configured symbol and timeframe, with the
  symbol, the timeframe, the last price and the last bar's time visible.
- The chart is the Q-037 item. Around it the scene shows a price axis, a time
  axis and a grid aligned to the same viewport the chart draws.
- Nothing in the scene issues a command, submits an order, edits a strategy, or
  opens any surface beyond this one. The window is read-only.
- The scene uses no polling timer to obtain data. Every value it shows comes from
  a property that changed because a bar, a connection state or a load did.

### Viewport policy

- The chart follows the live edge: the viewport ends at the newest bar, forming
  or completed, and holds a configured number of bars.
- The price range is taken from the visible bars' extents with a margin, and
  changes only when the visible extents leave the range, so the chart does not
  rescale on every forming tick.
- When a completed bar arrives, the viewport advances by one bar without
  redrawing the whole series.
- With no bars at all, the scene says so rather than drawing an empty frame.

### Degraded states

- The stream's connection state is always visible, and the age of the newest
  applied bar is shown once it exceeds the timeframe's own interval.
- When the stream is unavailable, the chart keeps its last bars, marks them
  stale with their age, and shows the reason. It does not clear, and it does not
  block the window.
- When the history load falls back to the API, or produces nothing, the scene
  says which source the history came from and what the shortfall was.
- When the API is unreachable at startup, the window opens, says so, and keeps
  retrying. It never fails to start because a service is down.
- Recovery is automatic and visible: when the stream returns, the state, the age
  and the counters return to normal without a restart.

### End to end

- Starting the backend stack and the terminal on a machine with the live
  publisher running produces, with no further action, a chart of that symbol's
  history from the lake with live bars extending it.
- Every bar drawn after startup arrived over the stream. No market-data REST
  route is polled, and the only REST calls on the live path are the snapshot and
  gap-closing calls the protocol defines.

### Documentation

- The repository's README describes the slice, how to configure the symbol,
  timeframe and API address, and how to run it against the stack.
- The boundary document is re-stated as still true, or amended by the same
  review that finds it is not.

### Preserved behaviour

- The headless report keeps its four lines and its exit code.
- The repository still launches, supervises and stops nothing.
- The standing checks pass headless, and the contracts check passes at the
  pinned revision.

## Constraints and non-goals

- **No controls of any kind.** No start, stop, flatten, kill switch, deploy or
  order entry. Phase 4 brings the execution and operations workspace; the
  roadmap puts none of it in this slice.
- **No interaction with the chart.** No pan, zoom, crosshair, hover readout or
  selection. The viewport follows the live edge and is not driven by a mouse.
- **No second symbol, timeframe switcher, or watchlist.**
- **No indicators, overlays, drawings or volume pane.**
- **No research surface of any kind**, per `BOUNDARY.md`: no strategy editor, no
  optimizer, no backtest or result browser, no dataset explorer.
- **No theming system, design tokens, or component library.** The scene is one
  window; a visual system is a decision taken when there are surfaces to share
  it.
- **No changes to `q_core`, the contracts, or the backend.** If the slice needs
  one, it stops and reports rather than reaching across the boundary.

## Acceptance criteria

### Agent-verifiable

1. Headless tests state the viewport policy as cases: the viewport ends at the
   newest bar and holds the configured count; a completed bar advances it by
   one; a forming update inside the current price range does not change the
   range; one outside it does; an empty series shows the empty state.
2. Headless tests state the degraded states as cases: stream unavailable keeps
   the bars, shows staleness with an age and a reason, and leaves the window
   responsive; recovery clears them; a history fallback shows the API source and
   the shortfall; no history at all shows the live-only state; an API down at
   startup opens the window with a retrying state.
3. A test asserts the scene has no timer-driven data fetch, and that after
   startup the only REST calls on the live path are the protocol's snapshot and
   history calls.
4. A test drives the whole path headless — fixture lake, fake stream, item probe
   — and asserts the drawn vertices correspond to history followed by live bars,
   with a strictly ascending, duplicate-free series across the seam.
5. `qmllint` passes on every QML file with no warnings, and the headless report
   still prints its four lines.
6. The full validation suite passes: `env -u WAYLAND_DISPLAY -u DISPLAY make check`.

### Human-verifiable

1. With the research stack and the live publisher running, the terminal is
   started and, with no further action, shows the symbol's history from the lake
   extended by live bars. The history source, bar count, first and last bar
   times, and the time from launch to first frame are reported.
   Command: `./research` in the workspace, then `cd q_terminal && make run`
2. The terminal runs for one hour against the live publisher. The frame-time
   95th percentile and maximum, the applied, dropped, gap and re-snapshot
   counters, and any visible artefact at the history-live seam are reported.
3. Redis, then the API, then the MT5 gateway are each stopped and restarted
   under the running terminal. For each, the state shown, the staleness
   behaviour and the recovery time are reported, and checked against §8.1's row
   for `q_terminal`.
4. A screenshot of the running slice, and one of it in the stale state, are
   attached to the handoff.
5. The phase 3 acceptance is recorded: every human-verifiable criterion of
   Q-033 through Q-038 is listed with its measured result, so the roadmap's
   phase 3 can be closed on evidence.
