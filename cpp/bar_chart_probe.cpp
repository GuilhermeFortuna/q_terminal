#include "bar_chart_probe.h"

#include "bar_chart_item.h"
#include "bar_chart_node.h"

#include "q-qt/src/bar_series.cxxqt.h"

namespace {

long long compositeRevision(BarSeries* series, int firstBar, int lastBar) {
    return (series->geometry_revision() << 32) ^ (static_cast<long long>(firstBar) << 16) ^
           static_cast<long long>(lastBar) ^ static_cast<long long>(series->vertex_len());
}

BarChartNode& probeNode() {
    thread_local BarChartNode node;
    return node;
}

InternalProbeResult makeProbeResult(BarChartNode* node, BarSeries* series) {
    InternalProbeResult result;
    result.revision = series->geometry_revision();
    result.uploads = node->uploadCount();
    result.geometryAllocations = node->geometryAllocationCount();
    result.risingVertexCount = node->risingVertexCount();
    result.fallingVertexCount = node->fallingVertexCount();
    result.formingVertexCount = node->formingVertexCount();

    const std::vector<BarVertex>& uploaded = node->lastUploadedVertices();
    result.vertices = uploaded;
    return result;
}

} // namespace

BarSeries* probe_make_test_series(int barCount) {
    auto* series = new BarSeries();
    if (barCount > 0) {
        series->load_history_sample(barCount);
    }
    return series;
}

InternalProbeResult probe_sync(BarSeries* series, int firstBar, int lastBar, double low,
                               double high, float widthPx, float heightPx, InternalProbeState* state) {
    series->set_viewport(firstBar, lastBar, low, high);
    series->set_surface(widthPx, heightPx);
    series->rebuild_geometry();

    BarChartNode& node = probeNode();
    const int uploadsBefore = node.uploadCount();
    const int allocationsBefore = node.geometryAllocationCount();

    const BarVertex* source = series->vertex_ptr();
    BarVertexView view{source, series->vertex_len(),
                       compositeRevision(series, firstBar, lastBar)};
    node.sync(view, QColor(0, 180, 0), QColor(220, 0, 0), QColor(255, 165, 0));

    InternalProbeResult result = makeProbeResult(&node, series);
    result.uploads = node.uploadCount() - uploadsBefore;
    result.geometryAllocations = node.geometryAllocationCount() - allocationsBefore;
    if (state != nullptr) {
        state->uploadedRevision = node.uploadedRevision();
        state->uploadCount = node.uploadCount();
        state->geometryAllocations = node.geometryAllocationCount();
    }
    return result;
}

InternalProbeResult probe_item_paint(BarChartItem* item, int frames) {
    QSGNode* node = nullptr;
    for (int frame = 0; frame < frames; ++frame) {
        node = item->testUpdatePaintNode(node);
    }

    InternalProbeResult result;
    result.paintNodeCallCount = item->paintNodeCallCount();
    result.updateRequestCount = item->updateRequestCount();

    auto* barSeries = qobject_cast<BarSeries*>(item->series());
    if (barSeries == nullptr) {
        return result;
    }

    auto* chartNode = static_cast<BarChartNode*>(node);
    result.revision = barSeries->geometry_revision();
    result.uploads = chartNode->uploadCount();
    result.geometryAllocations = chartNode->geometryAllocationCount();
    result.risingVertexCount = chartNode->risingVertexCount();
    result.fallingVertexCount = chartNode->fallingVertexCount();
    result.formingVertexCount = chartNode->formingVertexCount();

    result.vertices = chartNode->lastUploadedVertices();
    return result;
}

void reset_probe_node() {
    probeNode().resetForTests();
}
