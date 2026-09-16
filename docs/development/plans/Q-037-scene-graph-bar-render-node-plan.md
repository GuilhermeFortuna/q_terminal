# Q-037 implementation plan: Scene-graph bar render node

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Specification:** [`../specs/Q-037-scene-graph-bar-render-node-spec.md`](../specs/Q-037-scene-graph-bar-render-node-spec.md)  
**Depends on:** Q-034

## Current-system context

`cpp/` holds only `README.md`, stating the rule this task is built against:
scene-graph render nodes move buffers into GPU memory and compute nothing.
`src/render_backend.h` and `src/render_backend.cpp` are the existing C++: they
walk `QQmlApplicationEngine::rootObjects()`, find the `QQuickWindow`, read
`QSGRendererInterface::graphicsApi()`, print it, and push it onto the `AppInfo`
QObject on `sceneGraphInitialized` and `afterRendering`. That is the repository's
only precedent for reaching the scene graph, and it shows that
`q_terminal` links `QtQuick` in addition to `QtCore` and `QtQml`.

`src/bridge.rs` declares the C++ side through `unsafe extern "C++"` with
`include!("render_backend.h")`, which is the pattern a new header follows.
`build.rs` is the cxx-qt build; `qml/qmldir` declares `module qml` with
`Main 1.0 Main.qml`, and `make qml-lint` runs `qmllint -W 0` against
`target/cxxqt/qml_modules/qml/qmldir` and `qml/Main.qml`.

From Q-034, `q-qt` exposes the `BarSeries` QObject with `set_viewport`,
`set_surface`, `rebuild_geometry`, `vertex_ptr`, `vertex_len` and
`geometry_revision`, producing `#[repr(C)] BarVertex { x, y, direction, forming }`
in surface pixel coordinates, with bodies and wicks already laid out and the
forming bucket flagged and last.

`make check` runs headless through `scripts/ci.sh`
(`fmt-check lint build qml-lint test contracts-check` plus the headless report),
and the documented invocation strips `WAYLAND_DISPLAY` and `DISPLAY`, so
anything this task tests must run without a display.

The gap is that the vertex data Q-034 produces has no consumer, and the slice's
frame budget has never been measured.

## Interfaces produced

```cpp
// cpp/bar_chart_node.h
// A QSGGeometryNode that owns geometry and a flat-colour material per bucket
// class. It computes nothing: it copies BarVertex values into QSGGeometry and
// sets the material colours it is given.
struct BarVertexView { const float* data; size_t count; long long revision; };

class BarChartNode : public QSGGeometryNode {
public:
    BarChartNode();
    // Uploads only when `view.revision` differs from the last upload. Reuses the
    // geometry allocation unless the vertex count changed.
    void sync(const BarVertexView& view, QColor rising, QColor falling, QColor forming);
    long long uploadedRevision() const;
    int uploadCount() const;   // test-only counter
};

// cpp/bar_chart_item.h
// The QQuickItem QML instantiates. Holds no series state: it forwards its size
// to the series, asks for a rebuild in updatePaintNode, and hands the node the
// pointer it gets back.
class BarChartItem : public QQuickItem {
    Q_OBJECT
    Q_PROPERTY(QObject* series READ series WRITE setSeries NOTIFY seriesChanged)
    Q_PROPERTY(int firstBar ... ) Q_PROPERTY(int lastBar ... )
    Q_PROPERTY(double lowPrice ... ) Q_PROPERTY(double highPrice ... )
    Q_PROPERTY(QColor risingColor ... ) Q_PROPERTY(QColor fallingColor ... )
    Q_PROPERTY(QColor formingColor ... )
public:
    QSGNode* updatePaintNode(QSGNode* old, UpdatePaintNodeData*) override;
    void geometryChange(const QRectF& newGeometry, const QRectF& old) override;
};

// cpp/bar_chart_probe.h  (test support, built into the test binary only)
// Drives an item and node without a window: calls the same sync path and
// returns the uploaded vertices and counters.
struct ProbeResult { std::vector<float> vertices; long long revision; int uploads; };
ProbeResult probe_render(QObject* series, int firstBar, int lastBar,
                         double low, double high, float widthPx, float heightPx,
                         int frames);
```

```rust
// src/bridge/chart.rs  (cxx_qt bridge additions)
// unsafe extern "C++" {
//     include!("bar_chart_item.h");
//     fn register_bar_chart_item();              // qmlRegisterType, called at startup
//     include!("bar_chart_probe.h");
//     fn probe_render(...) -> ProbeVertices;     // test-only, feature-gated
// }
```

## Implementation decisions

- **`QSGGeometryNode` with a flat-colour material per bucket class, not
  `QSGRenderNode`.** `QSGRenderNode` means writing against the selected
  graphics API directly — OpenGL, Vulkan and the rest — which is a per-API
  implementation and an explicit backend choice the spec forbids. A geometry
  node with Qt's own material is API-agnostic, is uploaded the same way, and
  keeps the C++ to buffer movement, which is the rule `cpp/README.md` states.
  The architecture's "custom scene-graph node" requirement is about drawing from
  a `q_core` buffer rather than from QML items, and this satisfies it.

- **Three geometry sets — rising, falling, forming — as child nodes of one
  item.** Qt's flat-colour material carries one colour, and colouring per vertex
  would need a custom material and shader for what is a three-way split. The
  `direction` and `forming` flags already partition the vertices, so the split
  is a partition of the upload, not a computation.

- **`rebuild_geometry` is called inside `updatePaintNode`.** That is the one
  place in a Qt Quick frame where the GUI thread is blocked and the render
  thread owns the scene graph, which is exactly the precondition Q-034 states
  for its pointer. Any other call site would be a data race with the sink's
  deliveries.

- **The revision is the only upload trigger.** Comparing bytes would cost the
  upload it is trying to avoid; a dirty flag on the item would miss mutations
  that arrived between frames. Q-034's counter advances on every accepted
  mutation and on nothing else, which is precisely the question the node asks.

- **The item forwards its size to the series in `geometryChange`, not in
  `updatePaintNode`.** Vertex data is built in surface pixels, so a size change
  must reach the series before the rebuild; doing it in the paint path would
  render one frame at the old size.

- **Property changes coalesce through `update()`.** Qt already collapses several
  `update()` calls in one frame into one `updatePaintNode`, so the single-repaint
  requirement is satisfied by using the standard mechanism rather than by adding
  a timer.

- **A probe function, not a window, is what the tests drive.** CI runs with no
  display, and `QQuickWindow::grabWindow` needs one. The probe calls the same
  `sync` path with the same series and returns the uploaded vertices and the
  counters, so the headless tests assert the bytes that would have been
  uploaded. The remaining question — what it looks like — is a human criterion
  with a screenshot, which is the honest split.

- **Degenerate geometry is handled by Q-034's packing, and the node asserts
  rather than repairs.** A zero-height price range or a sub-pixel bucket is a
  layout decision; if the node started widening marks, the same chart would
  differ between the vertex data and the screen, and the vertex-for-vertex test
  would stop meaning anything.

## Ordered implementation

- [ ] 1. Work on the branch `Q-037-scene-graph-bar-render-node` in `q_terminal`,
   created from `development` by `./work start`. Confirm Q-034's tag is the one
   `q-qt` is pinned to, and that `env -u WAYLAND_DISPLAY -u DISPLAY make check`
   passes before changing anything.
- [ ] 2. Add the `QtQuick` link and the `cpp/` sources to `build.rs`, register a
   placeholder `BarChartItem` that draws nothing, instantiate it in
   `qml/Main.qml`, and confirm `make build`, `make qml-lint` and the headless
   report still pass. Commit.
- [ ] 3. Write the probe and a failing headless test: a series of four known
   bars at a known viewport and item size produces, through the probe, exactly
   the vertices `BarSeries` reports through its pointer. Implement
   `BarChartNode::sync` and the item's `updatePaintNode` until it passes.
   Commit.
- [ ] 4. Write failing tests for the upload rules: the first frame uploads; a
   second frame at an unchanged revision does not; a mutation between frames
   causes one upload; ten mutations between two frames cause one upload.
   Implement the revision guard. Confirm they pass. Commit.
- [ ] 5. Write failing tests for allocation: ten frames at a constant bucket
   count allocate geometry once; a changed bucket count reallocates once.
   Implement geometry reuse. Confirm they pass. Commit.
- [ ] 6. Write failing tests for the degenerate cases: an empty series produces
   no vertices and no error; a high equal to a low produces a non-empty mark; a
   viewport of 8,000 buckets into 400 pixels produces vertices for every bucket;
   a zero-sized item produces nothing. Fix whatever fails; if the fix belongs in
   the packing, stop and report rather than compute in C++. Commit.
- [ ] 7. Write a failing test that changing `firstBar`, `lastBar`, `lowPrice`,
   `highPrice` and the size in one QML transaction results in one
   `updatePaintNode`. Implement. Confirm it passes. Commit.
- [ ] 8. Split the upload into rising, falling and forming child nodes driven by
   the vertex flags, with a test asserting each set's vertex count for a known
   series. Commit.
- [ ] 9. Add `make bench-frames`: run the windowed application with a frame-time
   recorder for a fixed duration at a configured bucket count, reporting the
   95th percentile, the maximum, uploads per frame and the selected graphics
   API. Commit.
- [ ] 10. Run `env -u WAYLAND_DISPLAY -u DISPLAY make check`. Fix, re-run,
   commit.
- [ ] 11. **Human:** with the research stack and the live publisher running,
   record five minutes of frames at 2,000 buckets over a 500,000-bar series, then
   repeat at 500 and 8,000; take a screenshot of the running chart.

## Validation

- **Unit (headless, through the probe):** vertex-for-vertex equality with the
  series; upload rules; allocation reuse; degenerate cases; single repaint;
  the three-way flag split.
- **Regression:** the headless report's four lines; `make qml-lint`;
  `make contracts-check`; Q-035 and Q-036 tests unchanged.
- **Inspection:** the added C++ contains no arithmetic over bar values.
- **Measurement:** frame-time percentiles, maximum, uploads per frame and the
  graphics API at 500, 2,000 and 8,000 buckets.
- **Manual:** the screenshot of completed and forming bars.

```bash
cd /home/gui/projects/q/q_terminal
env -u WAYLAND_DISPLAY -u DISPLAY make check
cargo test --test chart_node
make bench-frames

# human (step 11)
cd /home/gui/projects/q && ./research
cd /home/gui/projects/q/q_terminal && make run
```

## Handoff

Report the graphics API the scene graph selected on the target machine, and the
95th-percentile and maximum frame times with uploads per frame at 500, 2,000 and
8,000 buckets, against the 16 ms budget. Report the geometry allocation counts
measured. Attach the screenshot. State explicitly that the C++ computes nothing
over bar values, name every case where a degenerate viewport needed a decision,
and say whether any of them would be better fixed in Q-034's packing.
