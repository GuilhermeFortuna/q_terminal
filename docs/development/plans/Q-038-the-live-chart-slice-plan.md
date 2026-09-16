# Q-038 implementation plan: The live chart slice

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Specification:** [`../specs/Q-038-the-live-chart-slice-spec.md`](../specs/Q-038-the-live-chart-slice-spec.md)  
**Depends on:** Q-035, Q-036, Q-037

## Current-system context

After Q-035 through Q-037 this repository holds: `Config` (API address, symbol,
timeframe) read once at startup; a `StreamClient` on its own tokio runtime
thread speaking the §4.2 protocol for `bars.forming` and `bars.completed`; a
`BarFeed` QObject owning the Q-034 `BarSeries` and exposing connection state,
data age, the four counters, and the history properties (loading, progress,
source, dataset id, published time, bars, shortfall, error); a history loader
that selects, verifies and reads a catalog dataset with a JSON API fallback; and
a `BarChartItem` QML item whose node uploads the series' vertex data once per
changed revision.

`qml/Main.qml` is still the Q-008 scene: a 640×480 `Window` with an `AppInfo`
object and four `Text` items in a centred `Column`. `qml/qmldir` declares
`module qml` with `Main 1.0 Main.qml`; `make qml-lint` runs `qmllint -W 0`
against the built module and `Main.qml`, and every QML file added here is linted
the same way.

`README.md` documents the skeleton: structure, the separation of concerns, the
prerequisites, the two run modes and the validation suite. It describes no
configuration, because there was none. `BOUNDARY.md` §1 scopes the repository to
live trading and operations, §2 lists the surfaces it will never grow, and §3
states that it owns no backend process.

`q_contracts/docs/system-architecture.md` §8.1's `q_terminal` column is the
source for the degraded states: a Redis outage shows a disconnected stream and
keeps the last state; an API outage shows offline with direct-Parquet views
still working from cached manifests; a stale MT5 edge shows quotes stale with
the age; an outbox-relay lag is detected by `seq`, closed from REST history and
shown as stream lag. Every client reconnects with exponential backoff capped at
30 s and re-runs the snapshot protocol, and no client trusts a cached view older
than the age it displays.

The gap is that no scene assembles any of this, no policy decides what the chart
shows, and phase 3 has never been run end to end.

## Interfaces produced

```qml
// qml/Main.qml — the window: header, chart, footer. No controls.
// qml/ChartPane.qml
//   properties: feed (BarFeed), barsVisible (int), priceMargin (real)
//   owns the BarChartItem, the price axis, the time axis and the grid, all
//   driven by the same viewport it passes to the item.
// qml/Viewport.qml  (a QtObject, not a visual item)
//   in:  barCount, lastBarTime, low, high, revision, barsVisible, priceMargin
//   out: firstBar, lastBar, lowPrice, highPrice
//   Implements the follow-the-edge and sticky-price-range policy, and nothing
//   else, so the policy is unit-testable on its own.
// qml/StatusStrip.qml
//   connection state, data age, history source, shortfall, counters, last error.
// qml/EmptyState.qml
//   "no bars" and "live only" states, with the reason.
```

```rust
// src/bridge/bar_feed.rs  (extended)
// Added QML-exposed, notifying properties:
//   timeframe_ms (i64)            // how old is "stale" for this timeframe
//   stale (bool)                  // data_age_ms > timeframe_ms
//   live_only (bool)              // history source is None but bars are arriving
// Added read-only property:
//   rest_calls (i64)              // protocol REST calls made since startup; the
//                                 // no-polling test asserts this stops growing

// src/startup.rs
/// Opens the window whatever the API's state: configuration failures are shown
/// in the scene, not printed to a terminal the user is not reading.
pub fn run_slice(config: Result<Config, ConfigError>) -> i32;
```

## Implementation decisions

- **The viewport policy lives in one QML object with no visual role.** It is the
  part of this task most likely to be wrong and most likely to change, and
  keeping it out of the chart pane makes it drivable by a test that asserts
  four numbers rather than by inspecting a rendered scene.

- **The price range is sticky: it changes only when the visible extents leave
  it, and then it is recomputed with the margin.** A range recomputed on every
  forming update makes the whole chart breathe at tick rate, which reads as
  noise and defeats the eye's use of the axis. The rule is cheap and its two
  cases are directly testable.

- **The viewport is driven by the series' revision, not by a timer.** The
  series advances its revision on exactly the mutations that can move the
  viewport, so binding to it is both correct and free; a timer would poll
  something that already notifies, which is what §4 removed from `q_frontend`.

- **Staleness is defined against the timeframe's own interval, not a fixed
  number of seconds.** An M1 chart with a two-minute-old bar is stale; a D1
  chart with one is not. The interval is derived once from the configured
  timeframe.

- **The window opens before the API is reachable, and a configuration error is
  a state in the scene.** §8.1 requires the terminal to keep running and show
  what it cannot reach; a process that exits because a service is down is the
  opposite of that, and would also make the operator's first diagnostic a log
  file.

- **The no-polling requirement is enforced by a counter, not by review.** A REST
  call count exposed by the feed, asserted to stop growing once the protocol has
  settled, is a test that keeps holding after someone adds a surface. Reading
  the code proves it once.

- **The axes and grid are QML items bound to the same viewport the chart
  draws.** They are a handful of text labels and lines at a low update rate;
  putting them in the scene graph node would add computation to the C++ that the
  boundary rule forbids, to save a cost that has not been measured to matter.

- **The end-to-end headless test drives the fixture lake, the fake stream and
  the render probe in one process.** Each of the three tasks before this proves
  its own half of a seam; only a test that spans them can catch a history bar
  and a streamed bar disagreeing, which is the defect this slice exists to rule
  out.

- **`BOUNDARY.md` is re-reviewed rather than assumed.** The slice is the first
  real surface in this repository, and the question of whether it has grown
  toward anything the boundary forbids is worth asking once, with a written
  answer, while the surface is small.

## Ordered implementation

- [ ] 1. Work on the branch `Q-038-the-live-chart-slice` in `q_terminal`, created
   from `development` by `./work start`. Confirm Q-035, Q-036 and Q-037 have all
   merged and that `env -u WAYLAND_DISPLAY -u DISPLAY make check` passes before
   changing anything.
- [ ] 2. Add `timeframe_ms`, `stale`, `live_only` and `rest_calls` to `BarFeed`
   with failing tests: staleness flips at the timeframe interval; `live_only` is
   true only with no history and arriving bars; `rest_calls` counts the
   protocol's calls and nothing else. Implement. Confirm they pass. Commit.
- [ ] 3. Write failing tests for `Viewport.qml` driven headless: the viewport
   ends at the newest bar and holds `barsVisible`; a completed bar advances it by
   one; a forming update inside the range leaves `lowPrice` and `highPrice`
   untouched; one outside recomputes them with the margin; an empty series
   yields the empty state. Implement `Viewport.qml`. Confirm they pass. Commit.
- [ ] 4. Build `ChartPane.qml` around the Q-037 item with the price axis, time
   axis and grid bound to the same viewport. Write failing tests that the axis
   labels match the viewport's bounds and that the pane draws nothing when the
   series is empty. Implement. Confirm `make qml-lint` is clean. Commit.
- [ ] 5. Build `StatusStrip.qml` and `EmptyState.qml`. Write failing tests for
   each §8.1 state: stream unavailable keeps the bars and shows age and reason;
   recovery clears them; a history fallback shows the API source and shortfall;
   no history shows live-only; an API down at startup shows retrying. Implement.
   Confirm they pass. Commit.
- [ ] 6. Replace `qml/Main.qml` with the window that composes the header, the
   chart pane and the status strip, keeping the version strings available but no
   longer the scene's subject. Update `qml/qmldir`. Confirm `make qml-lint` and
   the headless report pass. Commit.
- [ ] 7. Implement `run_slice` so the window opens on a configuration error and
   on an unreachable API, with a test for each. Confirm they pass. Commit.
- [ ] 8. Write the end-to-end headless test: fixture lake, fake stream, render
   probe; assert the drawn vertices are history followed by live bars, that the
   series is strictly ascending with no duplicate time across the seam, and that
   `rest_calls` stops growing once live. Fix whatever it finds in this
   repository; if the defect is in `q_core`, stop and report. Commit.
- [ ] 9. Rewrite `README.md` for the slice: what it shows, the configuration
   file and its environment overrides, how to run it against `./research`, and
   the validation suite. Re-review `BOUNDARY.md` against what now exists and
   either re-state it or amend it with the reason. Commit.
- [ ] 10. Run `env -u WAYLAND_DISPLAY -u DISPLAY make check`. Fix, re-run,
   commit.
- [ ] 11. **Human:** run the stack and the terminal; record the first-frame
   timing; run for one hour; stop and restart Redis, the API and the MT5
   gateway in turn; take the two screenshots.
- [ ] 12. **Human:** collect every human-verifiable result from Q-033 through
   Q-038 into the phase 3 acceptance record, so the roadmap phase closes on
   measured evidence rather than on merged branches.

## Validation

- **Unit:** the feed's staleness, live-only and REST-call counters; the viewport
  policy's five cases; the axis bindings.
- **Integration (headless):** each §8.1 degraded state and its recovery; the
  window opening without an API; the full lake-to-vertices path with the seam
  assertions.
- **Regression:** the headless report; `make contracts-check`; `make qml-lint`
  with no warnings; every Q-035, Q-036 and Q-037 test unchanged.
- **Manual:** a one-hour live run with frame-time percentiles and counters;
  Redis, API and gateway restarts checked against §8.1; two screenshots.
- **Acceptance:** the phase 3 record over Q-033 to Q-038.

```bash
cd /home/gui/projects/q/q_terminal
env -u WAYLAND_DISPLAY -u DISPLAY make check
cargo test --test slice_end_to_end
cargo test --test degraded_states

# human (steps 11 and 12)
cd /home/gui/projects/q && ./research
cd /home/gui/projects/q/q_terminal && make run
```

## Handoff

Report the launch-to-first-frame time, the history source, bar count and bar
time range of the real run, and the one-hour run's frame-time percentiles and
counters. Report, per service stopped, what the surface showed and how long
recovery took, next to §8.1's row for `q_terminal`. Attach both screenshots.
State the result of the `BOUNDARY.md` review. List every phase 3 human criterion
with its measured value, and name anything phase 3 promised that this slice does
not do.
