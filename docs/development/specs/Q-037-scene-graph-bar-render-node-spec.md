# Q-037: Scene-graph bar render node

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Project direction:** [`q_contracts/docs/system-architecture.md` §4.6, §5, §5.1, §10 Phase 3](https://github.com/GuilhermeFortuna/q_contracts/blob/a9724767015905f1e9cd5d98ba492080910db4f2/docs/system-architecture.md#46-visualization-data-path)  
**Depends on:** Q-034  
**Implementation plan:** [`../plans/Q-037-scene-graph-bar-render-node-plan.md`](../plans/Q-037-scene-graph-bar-render-node-plan.md)

## Purpose

The slice's rendering claim is specific: bars are drawn by a custom scene-graph
node from a `q_core` buffer, with stream updates coalesced to one buffer upload
per frame and a 95th-percentile frame under 16 ms. Q-034 produces the vertex
data and hands it across the boundary as a pointer; nothing consumes it. The
terminal's C++ directory holds a README and a rule — scene-graph nodes move
buffers and compute nothing — and no node. This task adds the QML item and the
node behind it, and proves the budget on the target machine.

## Requirements

### The item

- A QML item draws one bar series. It takes the series object, the bar range and
  the price range to display, and the colours for rising bars, falling bars and
  the forming bar.
- The item tells the series its own pixel size whenever that changes, so the
  vertex data is built for the surface it will be drawn on.
- Changing the bar range, the price range or the size schedules exactly one
  repaint, however many of them change together.
- The item draws nothing and reports no error when the series is empty or has no
  visible bars.

### The node

- Rendering reads the series' vertex data through the pointer handoff and
  uploads it into scene-graph geometry. It performs no arithmetic over bars: no
  scaling, no aggregation, no candle layout, no min or max.
- Geometry is re-uploaded only when the series' geometry revision has changed
  since the last upload. A repaint at an unchanged revision reuses what is
  already there.
- Rebuilding the vertex data and reading the pointer both happen at the one
  point in the frame where Qt has synchronised the GUI and render threads. The
  pointer is never read while the series can mutate.
- Geometry allocation is reused across frames; a frame whose bucket count is
  unchanged allocates nothing, and a change in bucket count reallocates once.
- Rising, falling and forming buckets are distinguished using the flags the
  vertex data carries, not by re-deriving them from prices.

### Correctness of what is drawn

- For a known series, viewport and item size, the drawn geometry matches the
  vertex data the series produced, vertex for vertex.
- A bar whose high equals its low still draws a visible mark, and a bucket
  narrower than a pixel still draws.
- The forming bucket is visually distinct from completed ones and is always the
  last bucket.

### Budget

- With one symbol's forming bars arriving at full rate and a chart of 2,000
  visible buckets over a series of 500,000 bars, the 95th-percentile frame time
  on the target machine stays under 16 ms, and no frame exceeds 33 ms.
- At most one geometry upload happens per frame however many updates arrived
  during it.
- The measurement states which graphics API the scene graph selected, because
  the number means nothing without it.

### Headless verification

- The node's geometry can be verified without a display, so the repository's
  standing checks keep running headless and in CI.
- The existing headless report keeps its four lines and its exit code.

## Constraints and non-goals

- **No computation in C++.** Layout, scaling, aggregation and flags are
  Q-034's, in Rust. If the node needs a number it does not have, the number is
  added to the vertex data, not computed at the node.
- **No axes, grids, crosshairs, labels, legends or tooltips.** The chart's
  furniture is Q-038's, in QML.
- **No interaction.** Panning, zooming, selection and hit-testing are not in the
  slice. The item is driven by properties.
- **No indicators, overlays, volume pane or second series.**
- **No custom shader pipeline beyond what drawing coloured geometry needs**, and
  no graphics API selected explicitly: the node works with whichever Qt picks,
  and the measurement names it.
- **No change to `q_core`.** If the vertex format is wrong, that is a Q-034
  change and its own branch.

## Acceptance criteria

### Agent-verifiable

1. Headless tests state the node's upload rules as cases: a first paint uploads;
   a repaint at an unchanged revision does not; an advanced revision does; ten
   series mutations between two frames produce one upload.
2. A headless test compares the node's geometry against the series' vertex data
   vertex for vertex, for a known series, viewport and item size.
3. Tests state the degenerate cases: an empty series draws nothing without
   error; a high equal to a low draws a visible mark; a sub-pixel bucket draws;
   a zero-sized item draws nothing.
4. A test asserts that a frame with an unchanged bucket count allocates no
   geometry, and that a changed count reallocates once.
5. A test asserts the item schedules one repaint when bar range, price range and
   size change together.
6. The C++ added by this task contains no arithmetic over bar values; the flags
   it reads are the ones the vertex data carries.
7. The headless report still prints its four lines, and the full validation
   suite passes: `env -u WAYLAND_DISPLAY -u DISPLAY make check`.

### Human-verifiable

1. On the target machine, with the live publisher running, frame times are
   recorded for five minutes at 2,000 visible buckets over a 500,000-bar series:
   the 95th percentile, the maximum, the uploads per frame, and the graphics API
   the scene graph selected are reported.
   Command: `./research` in the workspace, then `cd q_terminal && make run`
2. The same measurement is repeated at 500 and 8,000 visible buckets and
   reported, so the budget's headroom is known rather than assumed.
3. A screenshot of the running chart is attached to the handoff, showing
   completed and forming bars.
