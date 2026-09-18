# Q-049: Deployment chart with markers

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Project direction:** [`q_contracts/docs/system-architecture.md` §4.6, §4.7, §5.1, §9 invariants 1 and 5, §10 Phase 4](https://github.com/GuilhermeFortuna/q_contracts/blob/f2273a88e52c5b9a8ad5ac9d7eb27f8069cf643d/docs/system-architecture.md#46-visualization-data-path)  
**Depends on:** Q-047  
**Implementation plan:** [`../plans/Q-049-deployment-chart-with-markers-plan.md`](../plans/Q-049-deployment-chart-with-markers-plan.md)

## Purpose

The frontend's execution workspace draws each deployment on its own chart: its
symbol and timeframe, the indicators its strategy reads, a marker on every bar
where it decided to buy, sell or close, and a marker at every fill. That view
is how an operator checks that a live strategy does what its backtest did. The
terminal's chart, from phase 3, still shows the one symbol in its
configuration file. This task makes the chart follow the selected deployment,
and draws the deployment's indicators, decision markers and fill markers on
it. It uses the same stream-and-catalog data path and the same frame budget.
With this, the terminal reaches the parity Q-050 requires before the frontend
workspace is removed.

## Requirements

### Following the deployment

- Selecting a deployment retargets the chart to its symbol and timeframe:
  history loads through the catalog, live bars come from the stream, and the
  seam between them follows the Q-036 and Q-038 rules. With no deployment
  selected, the chart shows the configured symbol, as today.
- Retargeting cancels the previous target's history load and bar subscription
  filter. No bar of the previous symbol is ever drawn on the new chart.

### Markers and overlays

- Every decision whose outcome is a buy, sell or close signal is marked on the
  bar it evaluated. Every fill is marked on the bar whose interval contains it,
  at its fill price. Buy, sell, close and fill markers are distinct, and a
  marker shows the decision reason or fill details on hover.
- Markers come from the execution store, so a new decision or fill appears when
  the stream delivers it, with no request.
- The deployment's indicators are drawn as overlays in their declared panes,
  with values as the backend's deployment chart route computes them. They are
  refreshed when a bar completes, not on a timer. The terminal computes no
  indicator.
- Markers and overlays pan and zoom with the bars, and stay aligned at every
  zoom level.

### Budget

- With a deployment selected, its markers and overlays drawn, and the chart
  live, p95 frame time stays under 16 ms on the target machine (§4.6).

## Constraints and non-goals

- **No indicator computation in the terminal or in QML** (invariant 1). Values
  come from the backend, which computes them with `q_core`.
- **No drawing tools, no annotations, no strategy parameters** on the chart.
- **No multi-chart layout.** One chart, following the selection.
- **No new stream topic.** Indicator overlays refresh by REST on bar completion.
  A streamed indicator topic is a later decision, justified by a measurement.

## Acceptance criteria

### Agent-verifiable

1. Selecting a deployment with a different symbol and timeframe loads that
   symbol's history and live bars. A headless probe shows no bar of the previous
   symbol after the switch, including when the switch happens mid-load.
2. For a scripted store, every buy, sell and close decision has exactly one
   marker on the bar it evaluated, and holds have none. Every fill has one marker
   on the bar containing its time, at its price. The frontend's marker placement
   rules, taken from `liveChartMarkers.ts`, are reproduced as test vectors and
   pass.
3. A decision or fill delivered by the stream appears on the chart in the next
   frame, with no REST request.
4. Overlay values equal the chart route's response, and a completed bar
   triggers exactly one chart-route request for the selected deployment.
5. Markers and overlays keep their bar alignment across a pan and a zoom,
   checked with the render probe.
6. `bench-frames` with a selected deployment, 2 000 markers and two overlays
   reports p95 under 16 ms in the headless benchmark.
7. `qmllint` passes with no warnings, and the chart rows of
   `docs/ops-parity.md` are ticked.
8. `make check` passes.

### Human-verifiable

1. With two paper deployments on different symbols, switching between them shows
   each one's bars, indicators and markers. The markers match the decisions and
   fills tables, and p95 frame time stays under 16 ms over ten minutes.
   Command: `make run`
2. A screenshot of a deployment with at least one entry and one exit, beside the
   frontend's chart of the same deployment, shows the same markers on the same
   bars.
