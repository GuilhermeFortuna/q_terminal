# Q-062 implementation plan: Live candlestick renderer hot path

> **For implementation agents:** Read the linked spec and the `AGENTS.md`,
> `README.md`, and `BOUNDARY.md` of each repository first. This plan coordinates
> `q_core` and `q_terminal`; start each repository's implementation only through
> its approved board task. Do not start Q-062 while its plan is Blocked.

**Goal:** Keep completed candle geometry persistent across ordinary forming-bar
ticks while retaining the existing Rust, `QQuickItem`, and `QSGGeometryNode`
architecture.  
**Spec:** [`../specs/Q-062-live-candlestick-renderer-hot-path-spec.md`](../specs/Q-062-live-candlestick-renderer-hot-path-spec.md)  
**Pinned baseline:** `q_terminal` uses `q_core` tag `v2026.09.16`, commit
`b097df33979f0c91c7a91a8df5e0514cb68e26bf`.

## Current system and dependency gate

`cpp/bar_chart_item.cpp` rebuilds geometry on every paint and passes one vertex
view to `cpp/bar_chart_node.cpp`. The node copies a probe snapshot, partitions
into three temporary vectors, and syncs all three `QSGGeometryNode` buffers.
The pinned `q_core/crates/q-qt/src/bar_series.rs` reduces completed bars, appends
the forming bucket, packs the entire visible range using
`q_core/crates/q-buffers/src/geometry.rs`, then copies into its CXX vertex type.
`q_core/crates/q-buffers/src/lod.rs` already reuses bucket capacity and caps
completed buckets by width; preserve its exact reduction boundaries.

The `q_core` API change must be implemented, reviewed, and released as a pushed
tag through a separate `q_core` board task before the `q_terminal` pin can move.
The human starts and finishes that task under the workspace workflow. Q-062 can
then consume the tag; no implementation agent creates or pushes a release tag
or changes a board status outside `./work`.

## Ordered required implementation

- [ ] **1. Establish a trustworthy benchmark baseline.** Extend
  `cpp/frame_bench.cpp`, its bridge in `src/bridge.rs`, the CLI in `src/main.rs`,
  and `Makefile` so the existing benchmark can run at the live edge, with the
  forming candle visible and a stable timestamp whose high/low/close change.
  Retain its historical/offscreen mode. Add scripted pan/zoom, completion, and
  price-bound-expansion modes without relying on a running backend. Keep
  `--bars`, `--buckets`, `--markers`, `--overlays`, and `Q_BENCH_WINDOWS` working.
  Print per-window p50/p95/p99, graphics API, scene graph sync counts and
  submitted vertices by completed/forming layer, warmed-up allocation counts,
  and geometry CPU timing. Label Qt submissions separately from proven GPU
  transfers. Capture repeated 500k-bar baseline runs at 500, 2,000, and 8,000
  buckets, with and without the existing overlays/markers, on one machine.
  Verify the live-edge mode actually contains a forming candle.

- [ ] **2. Split packing in `q_core` while preserving output.** In
  `crates/q-buffers/src/geometry.rs`, expose a way to pack one forming bucket
  with the same coordinate and flag rules as a full pack. In
  `crates/q-qt/src/bar_series.rs`, retain separate reusable completed and
  forming vertex storage and expose pointer/length/revision for each through
  CXX-Qt. Rebuild the completed slice only when its data or transform key
  changes; a fixed-transform forming update packs only the one forming bucket.
  Clear the forming slice when absent or outside the viewport. Preserve the
  existing combined API until the terminal consumer moves, so this is a
  working intermediate state. Test byte-for-byte parity of concatenated old
  and new geometry, LOD boundaries, tick reuse, completion, empty/flat bars,
  view changes, and zero-size surfaces. Run `q_core`'s canonical `make check`.

- [ ] **3. Release `q_core` and update the terminal pin.** After the separate
  core task is In Review, the human uses `./work finish` to produce the pushed
  date tag. Update every `q_core` tag in `q_terminal/Cargo.toml`, refresh
  `Cargo.lock`, and verify the resolved commit is the released one. Do not
  copy or hand-edit generated CXX or vendored contract code. Build the terminal
  before replacing its renderer calls.

- [ ] **4. Integrate separate scene graph layers.** Forward the two views from
  `src/bar_feed.rs`. In `cpp/bar_chart_item.*`, compare explicit source identity
  (including feed target generation), viewport range, price range, and surface
  dimensions; do not use the current packed composite revision as the sole
  key. In `cpp/bar_chart_node.*`, sync completed rising/falling geometry only
  on completed invalidation and forming geometry only on forming invalidation.
  Reuse buffers and clear stale forming geometry when hidden. Dirty materials
  independently on color changes. Use a static pattern for completed vertex
  data and choose dynamic or stream for forming data from observed cadence;
  mark changed data and nodes dirty as Qt requires. Keep construction and
  mutation of `QSGNode` objects in the scene graph synchronization path.
  Validate an unchanged tick, a new extreme, completion, retarget, and resize
  with the headless probe before proceeding.

- [ ] **5. Remove avoidable production copies.** Fill completed rising/falling
  `QSGGeometry` buffers in a count-and-fill pass over Rust vertices or a
  retained scratch allocation; do not allocate three size-`view.count`
  vectors per update. Make `m_lastUploadedVertices` capture opt-in for probes
  in `cpp/bar_chart_probe.cpp` and `cpp/chart_cxx.cpp`; maintain the current
  vertex-for-vertex test assertions. Keep the safe Rust `geom_vertices` to FFI
  conversion on structural rebuilds unless profiling and an independently
  verified ABI-safe replacement justify changing it. Confirm warmed-up
  forming ticks allocate neither Rust nor C++ renderer storage.

- [ ] **6. Verify behavior and performance.** Extend
  `tests/test_chart_bridge.rs`, `tests/chart_alignment.rs`, and relevant Rust
  tests for exact geometry, single-layer tick updates, ten ticks coalesced to
  one sync, completed-bar transition, offscreen forming ticks, price expansion,
  pan/zoom/LOD, zero size, flat bars, and retarget identity. Include alignment
  with overlays/markers. Run
  `env -u WAYLAND_DISPLAY -u DISPLAY make check` in `q_terminal` and its normal
  `make bench-frames` variants. Compare repeated runs with the baseline on
  the same graphics API and machine. Require zero completed rebuilds and zero
  new renderer allocations on warmed-up fixed-transform ticks, only forming
  vertices submitted for visible ticks, no geometry submitted for offscreen
  ticks, and no material pan/zoom or rebuild frame-time regression outside
  measured noise. Report p50/p95/p99 and whether the existing p95-under-16-ms
  target was met, without inventing a new hardware-independent threshold.

## Optional work, only if measurements justify it

- If `visible_range_json()` scan/serialization and `JSON.parse()` materially
  affect tick cost, add a cached typed visible-extents interface in
  `src/bar_feed.rs` and consume it in `qml/Viewport.qml`. Key it by visible
  indices and feed revision, include the forming bar, and test sticky bounds
  and a newly expanded extreme. Keep JSON for other readouts.
- If completed partitioning remains a material rebuild cost, evaluate a
  `QSGMaterial` with state vertex attributes and one completed buffer. Require
  parity across the active Qt graphics APIs and a benchmark win before
  replacing the simpler flat-color nodes. `QQuickRhiItem` is outside this
  follow-up absent profiling evidence that scene graph geometry is inadequate.

## Review focus

- A forming tick with stable bounds leaves completed Rust storage and scene
  graph buffers unchanged; an extreme correctly transforms all visible bars.
- Completed-to-forming transitions and retargets never retain stale vertices.
- Probe snapshots remain exact without imposing a production copy.
- Benchmark output identifies what Qt was asked to sync, separately from
  transfers actually observed by a graphics profiler.
