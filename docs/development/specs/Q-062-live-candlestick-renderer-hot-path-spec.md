# Q-062: Live candlestick renderer hot path

**Status:** plan awaiting review; status of record is the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Depends on:** Q-037, Q-038, Q-060  
**Implementation plan:** [`../plans/Q-062-live-candlestick-renderer-hot-path-plan.md`](../plans/Q-062-live-candlestick-renderer-hot-path-plan.md)

## Problem

An accepted forming-bar update schedules `BarChartItem::updatePaintNode()`. With a
fixed viewport and price transform, the completed candles have not changed, yet
the current path reduces, packs, partitions, copies, and submits all visible
candles again. The objective is to keep completed geometry resident across
ordinary forming ticks and update only the forming candle's approximately 12
vertices. This is an optimization of the existing renderer, not a new renderer.

## Current architecture and data path

Rust owns bars, LOD reduction, candle geometry, and price semantics. `BarFeed`
delegates geometry operations to the pinned `q_core` `q-qt::BarSeries` (currently
`v2026.09.16`, lockfile commit `b097df3`). `BarChartItem` is a custom
`QQuickItem`; `BarChartNode` owns three `QSGGeometryNode` children with triangle
geometry and flat-color materials for rising, falling, and forming candles.
`ChartPane.qml` supplies the range and price bounds from `Viewport.qml`; the
overlay item shares that transform. LOD caps completed buckets at the rounded
surface width, rather than at the number of historical bars.

The current work on each paint is:

1. `BarChartItem` passes surface and viewport values to `BarFeed` or `BarSeries`
   and calls `rebuild_geometry()` regardless of why the item was scheduled.
2. `q-qt` calls `q-buffers::lod::reduce_into()` for the visible completed range,
   appends a separate forming bucket when visible, then calls
   `q-buffers::geometry::pack()` for every bucket. That fills reusable
   `geom_vertices`; `q-qt` copies them into a second reusable FFI `vertices`
   vector. Each bucket yields 12 vertices.
3. C++ reads `vertex_ptr`/`vertex_len` at scene graph synchronization. On a
   changed composite revision, `BarChartNode::sync()` copies the entire view
   into `m_lastUploadedVertices`, allocates and fills temporary `risingVerts`,
   `fallingVerts`, and `formingVerts`, then copies their positions into the
   three `QSGGeometry` buffers and marks all three dirty.

The `m_lastUploadedVertices` copy supports the headless probe but currently
runs in the production sync path. The Rust conversion exists because the
`q-buffers` vertex and CXX-generated FFI vertex are distinct types. The
temporary C++ partitions are needed by the present flat-color materials, but
their per-update allocations and copies are not inherently required. The
final CPU-to-scene-graph copy remains necessary with these materials; any
claimed GPU transfer reduction must be measured separately from CPU copies.

`Viewport.qml` also calls `visible_range_json()` and `JSON.parse()` when
checking visible high/low. The Rust query scans the visible bars, serializes
their extents, and includes the forming bar. Its share of tick cost is not yet
known. Sticky bounds normally avoid a transform change; a new extreme widens
the price range and legitimately invalidates every candle's pixel coordinates.

## Target architecture and interfaces

- Preserve `BarChartItem`, Qt Quick Scene Graph, triangle batches, Rust geometry
  ownership, and the existing LOD algorithm. Expose separate completed and
  forming vertex views across the `q-qt`/`BarFeed` bridge, with separate
  revisions or equivalently explicit dirty results. The pointer and length of
  each view must remain valid through the synchronized scene graph read.
- Keep completed rising/falling `QSGGeometryNode` buffers resident between
  structural changes. A visible forming candle occupies one reusable
  `QSGGeometryNode` buffer. A fixed-transform forming tick packs and copies only
  that candle; an offscreen forming tick does no candle geometry work.
- Preserve the exact `q-buffers` coordinate, direction, wick/body, LOD, and
  forming-color rules. C++ may partition vertex flags and move positions, but
  must not calculate bar geometry or market values.
- Retain full vertex comparison in headless tests by enabling snapshots on
  probes explicitly or reconstructing them on probe demand. Normal rendering
  must not copy the full vertex view solely for diagnostics.
- Avoid per-frame temporary color vectors by counting completed rising/falling
  vertices and filling their retained scene graph buffers directly on a
  completed rebuild. A bounded second pass over the source is acceptable.
  Retain the Rust `geom_vertices` to FFI conversion on full rebuilds initially;
  removing it requires a safe, measured FFI design and is not a reason to use
  an unchecked layout cast.
- Give mostly persistent completed geometry a static usage hint and frequently
  changed forming geometry a dynamic or stream hint after measuring its cadence.
  Mark vertex data and nodes dirty only for changed geometry; mark materials
  dirty only for changed colors. These hints are not substitutes for
  invalidation. See [Qt's `QSGGeometry` contract](https://doc.qt.io/qt-6/qsggeometry.html).

## Invalidation model

| Event | Completed geometry | Forming geometry |
|---|---|---|
| Forming tick, fixed view/surface/price bounds | Retain untouched | Repack and sync only if visible |
| Forming tick widens visible price bounds | Rebuild for new Y transform | Rebuild |
| Completed bar, history load, or series reset | Rebuild; LOD boundaries may change | Rebuild or clear |
| Pan, zoom, visible range, or LOD column count change | Rebuild | Rebuild or clear |
| Price transform or surface dimensions change | Rebuild | Rebuild or clear |
| Symbol/timeframe/target generation or series object change | Clear old identity and rebuild | Clear old identity and rebuild |
| Candle color change | Keep geometry; update relevant material | Keep geometry; update material |

Keys must compare the actual viewport bounds, price bounds, surface size, and
series identity explicitly; they must not depend on a lossy packed revision.
The `BarFeed` target generation is part of identity because a retarget can
replace its internal series without changing the `BarFeed` QObject. Updates
coalesced before one frame produce at most one sync per changed layer.

## Requirements and constraints

1. A steady forming tick with unchanged bounds must leave completed Rust
   geometry, C++ completed buffers, and their scene graph dirty state untouched.
   Forming disappearance must clear its node without leaving stale pixels.
2. Completion, pan/zoom, resize, retarget, and visible Y-range expansion must
   produce correct geometry and aligned axes, markers, and overlays. Empty,
   one-bar, flat-price, subpixel-bucket, and history-inspection views remain
   valid.
3. Preserve the current pointer handoff during Qt's synchronized paint phase;
   keep scene graph nodes on the render thread. Preserve headless vertex and
   allocation probes while removing their snapshot copy from normal paints.
4. Make no market-data transport, execution, indicator, QML shell, workspace,
   or chart-design changes. Do not switch to QML items per candle, Canvas,
   software rendering, a web chart, or `QQuickRhiItem`. A custom material that
   carries candle state is optional later work only if profiling shows the
   remaining CPU partition cost justifies its complexity.
5. Make coordinated `q_core` changes first, release them through its normal
   reviewed tag workflow, then update `q_terminal`'s tag pin and lockfile. No
   vendored contracts or wire contracts change.

## Benchmark and acceptance criteria

Extend `make bench-frames` rather than replace it. Its current default loads
500,000 historical bars but displays indices `0..visible_buckets`, so synthetic
forming updates at the live edge are offscreen. Add a visible live-edge workload
with a stable forming timestamp and changing OHLC, plus offscreen ticks,
completed-bar arrival, pan/zoom, and price-bound expansion. Compare 500, 2,000,
and 8,000 visible buckets over 500,000 historical bars, and include the existing
marker/overlay options as a compositing stress case. Warm up before counting
steady-state allocations. Record viewport position, graphics API, window size,
build mode, machine, and baseline and changed runs on the same setup.

Report frame p50/p95/p99, geometry-preparation CPU time where practical, Rust
and C++ allocation counts, completed/forming sync and vertex counts, and Qt
dirty/submitted vertex counts. These application counters indicate data handed
to Qt; call them actual GPU uploads only if a graphics profiler or backend
instrumentation verifies the transfers. Keep the repository's existing p95
under 16 ms target as a reported target, not a new cross-hardware promise.

Acceptance requires: after warm-up, fixed-transform forming ticks rebuild and
submit zero completed vertices and make no new renderer allocations; a visible
forming tick submits only its bounded forming geometry; offscreen ticks submit
no candle geometry; structural changes remain visually exact. Pan/zoom and
full-rebuild p50/p95/p99 must show no material regression against repeated
baseline runs on the same machine. Any apparent regression or improvement
within run-to-run noise is reported rather than assigned an arbitrary pass
threshold. Run the canonical `make check` in both affected repositories and
retain the terminal's headless probe and alignment coverage.

## Risks and follow-up decisions

- Distinct CXX and `q-buffers` vertex types make zero-copy casts unsafe without
  a proven shared ABI. A remaining full-rebuild conversion is acceptable.
- Qt backend choice and driver behavior affect whether a geometry hint improves
  uploads. Compare graphics APIs and measure before retaining a hint.
- `visible_range_json()` can still scan thousands of bars per tick. Profile it
  after geometry isolation; if material, add one cached typed extents result
  keyed by visible range and feed revision, with forming extremes included.
- A custom material with state attributes could remove completed CPU color
  partitioning, but adds shader and backend compatibility work. Consider it
  only after Phase 1 measurements demonstrate a worthwhile remaining cost.
