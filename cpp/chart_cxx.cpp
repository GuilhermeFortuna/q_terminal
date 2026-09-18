#include "chart_cxx.h"

#include "bar_chart_item.h"
#include "bar_chart_node.h"
#include "bar_chart_probe.h"
#include "q-qt/src/bar_series.cxxqt.h"
#include "q_terminal/src/bar_feed.cxxqt.h"

#include <mutex>

#include <QtCore/QCoreApplication>
#include <QtCore/QDebug>
#include <QtCore/QFileInfo>
#include <QtCore/QUrl>
#include <QtGui/QGuiApplication>
#include <QtQml/QQmlComponent>
#include <QtQml/QQmlEngine>

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
            qputenv("QT_QPA_PLATFORM", QByteArrayLiteral("offscreen"));
            static int argc = 3;
            static char arg0[] = "q_terminal_test";
            static char arg1[] = "-platform";
            static char arg2[] = "offscreen";
            static char* argv[] = {arg0, arg1, arg2, nullptr};
            new QGuiApplication(argc, argv);
        }
        register_bar_chart_types();
    });
}

void reset_chart_probe_state() {
    reset_probe_node();
}

struct ViewportProbe::Impl {
    QQmlEngine engine;
    QObject* viewport{nullptr};
};

ViewportProbe::ViewportProbe() : m_impl(std::make_unique<Impl>()) {
    ensure_test_app();
    m_impl->engine.addImportPath(QStringLiteral("target/cxxqt/qml_modules"));

    QQmlComponent component(&m_impl->engine);
    QFileInfo fileInfo(QStringLiteral("qml/Viewport.qml"));
    if (fileInfo.exists()) {
        component.loadUrl(QUrl::fromLocalFile(fileInfo.absoluteFilePath()));
    } else {
        component.loadUrl(QUrl(QStringLiteral("qrc:/qt/qml/qml/Viewport.qml")));
    }
    if (component.isError()) {
        qWarning() << "ViewportProbe component error:" << component.errorString();
    }
    m_impl->viewport = component.create();
    if (!m_impl->viewport && component.isError()) {
        qWarning() << "ViewportProbe create error:" << component.errorString();
    }
}

ViewportProbe::~ViewportProbe() {
    if (m_impl->viewport) {
        delete m_impl->viewport;
    }
}

void ViewportProbe::set_bars_visible(int count) {
    if (m_impl->viewport) {
        m_impl->viewport->setProperty("barsVisible", count);
    }
}

void ViewportProbe::set_price_margin(double margin) {
    if (m_impl->viewport) {
        m_impl->viewport->setProperty("priceMargin", margin);
    }
}

void ViewportProbe::update(int bar_count, double low, double high, int revision) {
    if (m_impl->viewport) {
        m_impl->viewport->setProperty("barCount", bar_count);
        m_impl->viewport->setProperty("low", low);
        m_impl->viewport->setProperty("high", high);
        m_impl->viewport->setProperty("revision", revision);
    }
}

ViewportProbeResult ViewportProbe::result() const {
    ViewportProbeResult out{};
    if (m_impl->viewport) {
        out.first_bar = m_impl->viewport->property("firstBar").toInt();
        out.last_bar = m_impl->viewport->property("lastBar").toInt();
        out.low_price = m_impl->viewport->property("lowPrice").toDouble();
        out.high_price = m_impl->viewport->property("highPrice").toDouble();
        out.empty = m_impl->viewport->property("empty").toBool();
    }
    return out;
}

std::unique_ptr<ViewportProbe> make_viewport_probe() {
    return std::make_unique<ViewportProbe>();
}

struct ChartPaneProbe::Impl {
    QQmlEngine engine;
    QObject* pane{nullptr};
    BarChartNode* node{nullptr};
};

ChartPaneProbe::ChartPaneProbe() : m_impl(std::make_unique<Impl>()) {
    ensure_test_app();
    m_impl->engine.addImportPath(QStringLiteral("target/cxxqt/qml_modules"));

    QQmlComponent component(&m_impl->engine);
    QFileInfo fileInfo(QStringLiteral("qml/ChartPane.qml"));
    if (fileInfo.exists()) {
        component.loadUrl(QUrl::fromLocalFile(fileInfo.absoluteFilePath()));
    } else {
        component.loadUrl(QUrl(QStringLiteral("qrc:/qt/qml/qml/ChartPane.qml")));
    }
    if (component.isError()) {
        qWarning() << "ChartPaneProbe component error:" << component.errorString();
    }
    m_impl->pane = component.create();
    if (!m_impl->pane && component.isError()) {
        qWarning() << "ChartPaneProbe create error:" << component.errorString();
    }
}

ChartPaneProbe::~ChartPaneProbe() {
    if (m_impl->node) {
        delete m_impl->node;
    }
    if (m_impl->pane) {
        delete m_impl->pane;
    }
}

void ChartPaneProbe::set_series(BarSeries* series) {
    if (m_impl->pane) {
        m_impl->pane->setProperty("feed", QVariant::fromValue(static_cast<QObject*>(series)));
    }
}

void ChartPaneProbe::set_feed(BarFeed* feed) {
    if (m_impl->pane) {
        m_impl->pane->setProperty("feed", QVariant::fromValue(static_cast<QObject*>(feed)));
    }
}

void ChartPaneProbe::set_size(float width, float height) {
    if (m_impl->pane) {
        m_impl->pane->setProperty("width", width);
        m_impl->pane->setProperty("height", height);
    }
}

void ChartPaneProbe::set_bars_visible(int count) {
    if (m_impl->pane) {
        m_impl->pane->setProperty("barsVisible", count);
    }
}

void ChartPaneProbe::set_price_margin(double margin) {
    if (m_impl->pane) {
        m_impl->pane->setProperty("priceMargin", margin);
    }
}

ChartPaneProbeResult ChartPaneProbe::result() const {
    ChartPaneProbeResult out{};
    if (m_impl->pane) {
        out.top_price_label = rust::String(m_impl->pane->property("topPriceLabel").toString().toStdString());
        out.bottom_price_label = rust::String(m_impl->pane->property("bottomPriceLabel").toString().toStdString());
        out.empty = m_impl->pane->property("empty").toBool();

        auto* chartItem = m_impl->pane->findChild<BarChartItem*>();
        if (chartItem) {
            chartItem->setSize(QSizeF(m_impl->pane->property("width").toFloat(),
                                      m_impl->pane->property("height").toFloat()));
            auto* node = static_cast<BarChartNode*>(chartItem->testUpdatePaintNode(m_impl->node));
            m_impl->node = node;
            if (node) {
                out.vertex_count = node->risingVertexCount() + node->fallingVertexCount() + node->formingVertexCount();
            }
        }
    }
    return out;
}

std::unique_ptr<ChartPaneProbe> make_chart_pane_probe() {
    return std::make_unique<ChartPaneProbe>();
}


