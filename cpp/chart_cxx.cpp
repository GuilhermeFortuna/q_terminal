#include "chart_cxx.h"

#include "bar_chart_item.h"
#include "bar_chart_probe.h"
#include "q-qt/src/bar_series.cxxqt.h"

#include <mutex>

#include <QtCore/QCoreApplication>

#include "q_terminal/src/chart_bridge.cxx.h"

namespace {

ProbeResult toProbeResult(const InternalProbeResult& result) {
    ProbeResult out;
    out.revision = result.revision;
    out.uploads = result.uploads;
    out.geometry_allocations = result.geometryAllocations;
    out.rising_vertex_count = result.risingVertexCount;
    out.falling_vertex_count = result.fallingVertexCount;
    out.forming_vertex_count = result.formingVertexCount;
    out.paint_node_call_count = result.paintNodeCallCount;
    out.update_request_count = result.updateRequestCount;
    out.vertices.reserve(result.vertices.size());
    for (const BarVertex& vertex : result.vertices) {
        out.vertices.push_back(
            ProbeVertex{vertex.x, vertex.y, vertex.direction, vertex.forming});
    }
    return out;
}

} // namespace

BarSeries* make_test_series(int bar_count) {
    return probe_make_test_series(bar_count);
}

ProbeResult chart_probe_sync(BarSeries* series, int first_bar, int last_bar, double low,
                             double high, float width_px, float height_px, ProbeState& state) {
    InternalProbeState probeState;
    probeState.uploadedRevision = state.uploaded_revision;
    probeState.uploadCount = state.upload_count;
    probeState.geometryAllocations = state.geometry_allocations;
    InternalProbeResult result =
        probe_sync(series, first_bar, last_bar, low, high, width_px, height_px, &probeState);
    state.uploaded_revision = probeState.uploadedRevision;
    state.upload_count = probeState.uploadCount;
    state.geometry_allocations = probeState.geometryAllocations;
    return toProbeResult(result);
}

ProbeResult chart_probe_item_paint(BarChartItem* item, int frames) {
    return toProbeResult(probe_item_paint(item, frames));
}

BarChartItem* make_test_chart_item(BarSeries* series, int first_bar, int last_bar, double low,
                                   double high, float width_px, float height_px) {
    auto* item = new BarChartItem();
    item->setSeries(series);
    item->setFirstBar(first_bar);
    item->setLastBar(last_bar);
    item->setLowPrice(low);
    item->setHighPrice(high);
    chart_item_set_size(item, width_px, height_px);
    return item;
}

void chart_item_set_size(BarChartItem* item, float width_px, float height_px) {
    item->setSize(QSizeF(width_px, height_px));
}

void chart_item_apply_view(BarChartItem* item, int first_bar, int last_bar, double low,
                           double high, float width_px, float height_px) {
    item->setFirstBar(first_bar);
    item->setLastBar(last_bar);
    item->setLowPrice(low);
    item->setHighPrice(high);
    chart_item_set_size(item, width_px, height_px);
}

void chart_series_ingest_completed(BarSeries* series, std::int64_t time, double open, double high,
                                   double low, double close, double volume) {
    series->ingest_completed_bar(time, open, high, low, close, volume);
}

double chart_series_low(BarSeries* series) {
    return series->getLow();
}

double chart_series_high(BarSeries* series) {
    return series->getHigh();
}

void chart_series_ingest_forming(BarSeries* series, std::int64_t time, double open, double high,
                                 double low, double close, double volume) {
    series->ingest_forming_bar(time, open, high, low, close, volume);
}

void chart_series_rebuild(BarSeries* series) {
    series->rebuild_geometry();
}

std::int64_t chart_series_geometry_revision(BarSeries* series) {
    return series->geometry_revision();
}

int chart_series_vertex_len(BarSeries* series) {
    return static_cast<int>(series->vertex_len());
}

ProbeVertex chart_series_vertex_at(BarSeries* series, std::size_t index) {
    const auto* source = reinterpret_cast<const BarVertex*>(series->vertex_ptr());
    const BarVertex& vertex = source[index];
    return ProbeVertex{vertex.x, vertex.y, vertex.direction, vertex.forming};
}

void ensure_test_app() {
    static std::once_flag once;
    std::call_once(once, []() {
        if (QCoreApplication::instance() == nullptr) {
            static int argc = 1;
            static char arg0[] = "q_terminal_test";
            static char* argv[] = {arg0, nullptr};
            new QCoreApplication(argc, argv);
        }
    });
}

void reset_chart_probe_state() {
    reset_probe_node();
}
