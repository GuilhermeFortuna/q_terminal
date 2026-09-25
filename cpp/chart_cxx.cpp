#include "chart_cxx.h"

#include "bar_chart_item.h"
#include "bar_chart_node.h"
#include "bar_chart_probe.h"
#include "q-qt/src/bar_series.cxxqt.h"
#include "q_terminal/src/bar_feed.cxxqt.h"
#include "q_terminal/src/chart_context.cxxqt.h"
#include "q_terminal/src/execution_models.cxxqt.h"

#include <mutex>

#include <QtCore/QAbstractEventDispatcher>
#include <QtCore/QCoreApplication>
#include <QtCore/QDebug>
#include <QtCore/QFileInfo>
#include <QtCore/QThread>
#include <QtCore/QTimer>
#include <QtCore/QUrl>
#include <QtCore/QVariant>
#include <QtGui/QKeyEvent>
#include <QtGui/QWheelEvent>
#include <QtGui/QGuiApplication>
#include <QtQml/QQmlApplicationEngine>
#include <QtQml/QQmlComponent>
#include <QtQml/QQmlEngine>
#include <QtQml/QJSEngine>
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>

#include "q_terminal/src/chart_bridge.cxx.h"

namespace {

ProbeResult toProbeResult(const InternalProbeResult& result) {
    ProbeResult out;
    out.revision = result.revision;
    out.uploads = result.uploads;
    out.geometry_allocations = result.geometryAllocations;
    out.completed_layer_updates = result.completedLayerUpdates;
    out.forming_layer_updates = result.formingLayerUpdates;
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
    item->enableProbeCapture();
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

void ensure_application() {
    if (QCoreApplication::instance() == nullptr) {
        if (qEnvironmentVariableIsEmpty("DISPLAY") &&
            qEnvironmentVariableIsEmpty("WAYLAND_DISPLAY") &&
            qEnvironmentVariableIsEmpty("QT_QPA_PLATFORM")) {
            qputenv("QT_QPA_PLATFORM", QByteArrayLiteral("offscreen"));
        }
        static int argc = 1;
        static char arg0[] = "q_terminal";
        static char* argv[] = {arg0, nullptr};
        new QGuiApplication(argc, argv);
    }
    register_bar_chart_types();
}

int exec_application() {
    if (auto* app = QCoreApplication::instance()) {
        return app->exec();
    }
    return 0;
}

void process_events() {
    if (auto* app = QCoreApplication::instance()) {
        app->processEvents();
        QCoreApplication::sendPostedEvents(nullptr, 0);
    }
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
        // Prices first, as a feed publishes them with the bar that moved them.
        m_impl->viewport->setProperty("low", low);
        m_impl->viewport->setProperty("high", high);
        m_impl->viewport->setProperty("barCount", bar_count);
        m_impl->viewport->setProperty("revision", revision);
    }
}

void ViewportProbe::pan_bars(int delta) {
    if (m_impl->viewport) {
        QMetaObject::invokeMethod(m_impl->viewport, "panBars", Q_ARG(QVariant, QVariant(delta)));
    }
}

void ViewportProbe::zoom_at(double anchor, int direction) {
    if (m_impl->viewport) {
        QMetaObject::invokeMethod(m_impl->viewport, "zoomAt", Q_ARG(QVariant, QVariant(anchor)), Q_ARG(QVariant, QVariant(direction)));
    }
}

void ViewportProbe::return_to_live() {
    if (m_impl->viewport) {
        QMetaObject::invokeMethod(m_impl->viewport, "returnToLive");
    }
}

void ViewportProbe::reset_for_target() {
    if (m_impl->viewport) {
        m_impl->viewport->setProperty("barCount", 0);
        QMetaObject::invokeMethod(m_impl->viewport, "resetForTarget");
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
        out.mode = rust::String(m_impl->viewport->property("mode").toString().toStdString());
    }
    return out;
}

std::unique_ptr<ViewportProbe> make_viewport_probe() {
    return std::make_unique<ViewportProbe>();
}

struct ChartPaneProbe::Impl {
    QQmlEngine engine;
    // The pane lives in a window so focus and key events take the real delivery path.
    std::unique_ptr<QQuickWindow> window;
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
    m_impl->window = std::make_unique<QQuickWindow>();
    if (auto* item = qobject_cast<QQuickItem*>(m_impl->pane)) {
        item->setParentItem(m_impl->window->contentItem());
    }
}

ChartPaneProbe::~ChartPaneProbe() {
    if (m_impl->node) {
        delete m_impl->node;
    }
    if (m_impl->pane) {
        delete m_impl->pane;
    }
    m_impl->window.reset();
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

void ChartPaneProbe::set_context(ChartContext* context) {
    if (m_impl->pane) {
        m_impl->pane->setProperty("context", QVariant::fromValue(static_cast<QObject*>(context)));
    }
}

static bool invokePaneBool(QObject* pane, const char* method) {
    if (!pane) {
        return false;
    }
    QVariant result;
    const bool ok =
        QMetaObject::invokeMethod(pane, method, Qt::DirectConnection, Q_RETURN_ARG(QVariant, result));
    if (!ok) {
        return false;
    }
    return result.toBool();
}

static bool invokePaneBoolArg(QObject* pane, const char* method, const QString& arg) {
    if (!pane) {
        return false;
    }
    QVariant result;
    const bool ok = QMetaObject::invokeMethod(pane, method, Qt::DirectConnection,
                                              Q_RETURN_ARG(QVariant, result), Q_ARG(QVariant, arg));
    if (!ok) {
        return false;
    }
    return result.toBool();
}

static QString invokePaneString(QObject* pane, const char* method) {
    if (!pane) {
        return {};
    }
    QVariant result;
    const bool ok =
        QMetaObject::invokeMethod(pane, method, Qt::DirectConnection, Q_RETURN_ARG(QVariant, result));
    if (!ok) {
        return {};
    }
    return result.toString();
}

static void invokePaneVoid(QObject* pane, const char* method) {
    if (!pane) {
        return;
    }
    QMetaObject::invokeMethod(pane, method, Qt::DirectConnection);
}

static int invokePaneInt(QObject* pane, const char* method) {
    if (!pane) {
        return 0;
    }
    QVariant result;
    const bool ok =
        QMetaObject::invokeMethod(pane, method, Qt::DirectConnection, Q_RETURN_ARG(QVariant, result));
    if (!ok) {
        return 0;
    }
    return result.toInt();
}

static float invokePaneFloat(QObject* pane, const char* method) {
    if (!pane) {
        return 0.0f;
    }
    QVariant result;
    const bool ok =
        QMetaObject::invokeMethod(pane, method, Qt::DirectConnection, Q_RETURN_ARG(QVariant, result));
    if (!ok) {
        return 0.0f;
    }
    return result.toFloat();
}

static void invokePaneVoidArg(QObject* pane, const char* method, const QString& arg) {
    if (!pane) {
        return;
    }
    QMetaObject::invokeMethod(pane, method, Qt::DirectConnection, Q_ARG(QVariant, arg));
}

void ChartPaneProbe::focus_canvas() {
    invokePaneVoid(m_impl->pane, "testFocusCanvas");
    process_events();
}

bool ChartPaneProbe::canvas_type_key(rust::Str text) {
    const QString qtext = QString::fromUtf8(text.data(), static_cast<int>(text.size()));
    const bool opened = invokePaneBoolArg(m_impl->pane, "testCanvasTypeKey", qtext);
    process_events();
    process_events();
    return opened;
}

bool ChartPaneProbe::target_prompt_open() const {
    return invokePaneBool(m_impl->pane, "testTargetPromptOpen");
}

void ChartPaneProbe::set_target_prompt_draft(rust::Str text) {
    const QString qtext = QString::fromUtf8(text.data(), static_cast<int>(text.size()));
    invokePaneVoidArg(m_impl->pane, "testSetTargetPromptDraft", qtext);
    process_events();
    process_events();
}

rust::String ChartPaneProbe::target_prompt_preview() const {
    return rust::String(invokePaneString(m_impl->pane, "testTargetPromptPreview").toStdString());
}

void ChartPaneProbe::submit_target_prompt() {
    invokePaneVoid(m_impl->pane, "testSubmitTargetPrompt");
    process_events();
}

void ChartPaneProbe::cancel_target_prompt() {
    invokePaneVoid(m_impl->pane, "testCancelTargetPrompt");
    process_events();
}

void ChartPaneProbe::hover_canvas(float x, float y) {
    if (m_impl->pane) {
        QMetaObject::invokeMethod(m_impl->pane, "testSetHover", Qt::DirectConnection,
                                  Q_ARG(QVariant, x), Q_ARG(QVariant, y));
        process_events();
    }
}

void ChartPaneProbe::clear_hover() {
    invokePaneVoid(m_impl->pane, "testClearHover");
    process_events();
}

void ChartPaneProbe::select_adjacent_bar(int delta) {
    if (m_impl->pane) {
        QMetaObject::invokeMethod(m_impl->pane, "testSelectAdjacentBar", Qt::DirectConnection,
                                  Q_ARG(QVariant, delta));
        process_events();
    }
}

void ChartPaneProbe::clear_selection() {
    invokePaneVoid(m_impl->pane, "testClearSelection");
    process_events();
}

bool ChartPaneProbe::crosshair_visible() const {
    return invokePaneBool(m_impl->pane, "testCrosshairVisible");
}

int ChartPaneProbe::active_bar_index() const {
    return invokePaneInt(m_impl->pane, "testActiveBarIndex");
}

rust::String ChartPaneProbe::readout_time() const {
    return rust::String(invokePaneString(m_impl->pane, "testReadoutTime").toStdString());
}

rust::String ChartPaneProbe::readout_open() const {
    return rust::String(invokePaneString(m_impl->pane, "testReadoutOpen").toStdString());
}

rust::String ChartPaneProbe::readout_high() const {
    return rust::String(invokePaneString(m_impl->pane, "testReadoutHigh").toStdString());
}

rust::String ChartPaneProbe::readout_low() const {
    return rust::String(invokePaneString(m_impl->pane, "testReadoutLow").toStdString());
}

rust::String ChartPaneProbe::readout_close() const {
    return rust::String(invokePaneString(m_impl->pane, "testReadoutClose").toStdString());
}

bool ChartPaneProbe::readout_forming() const {
    return invokePaneBool(m_impl->pane, "testReadoutForming");
}

rust::String ChartPaneProbe::readout_pointer_price() const {
    return rust::String(invokePaneString(m_impl->pane, "testReadoutPointerPrice").toStdString());
}

float ChartPaneProbe::crosshair_x() const {
    return invokePaneFloat(m_impl->pane, "testCrosshairX");
}

float ChartPaneProbe::crosshair_y() const {
    return invokePaneFloat(m_impl->pane, "testCrosshairY");
}

rust::String ChartPaneProbe::accessible_text() const {
    return rust::String(invokePaneString(m_impl->pane, "testAccessibleText").toStdString());
}

rust::String ChartPaneProbe::marker_tooltip_text() const {
    return rust::String(invokePaneString(m_impl->pane, "testMarkerTooltipText").toStdString());
}

bool ChartPaneProbe::marker_tooltip_visible() const {
    return invokePaneBool(m_impl->pane, "testMarkerTooltipVisible");
}

void ChartPaneProbe::pan_bars(int delta) {
    if (m_impl->pane) {
        QVariant vpVar = m_impl->pane->property("viewport");
        QObject* vp = vpVar.value<QObject*>();
        if (vp) {
            QMetaObject::invokeMethod(vp, "panBars", Qt::DirectConnection, Q_ARG(QVariant, delta));
            process_events();
        }
    }
}

void ChartPaneProbe::zoom_at(double anchor, int direction) {
    if (m_impl->pane) {
        QVariant vpVar = m_impl->pane->property("viewport");
        QObject* vp = vpVar.value<QObject*>();
        if (vp) {
            QMetaObject::invokeMethod(vp, "zoomAt", Qt::DirectConnection, Q_ARG(QVariant, anchor), Q_ARG(QVariant, direction));
            process_events();
        }
    }
}

void ChartPaneProbe::set_size(float width, float height) {
    if (m_impl->pane) {
        m_impl->pane->setProperty("width", width);
        m_impl->pane->setProperty("height", height);
    }
    if (m_impl->window) {
        m_impl->window->resize(static_cast<int>(width), static_cast<int>(height));
    }
}

bool ChartPaneProbe::send_canvas_key(int key, int modifiers, rust::Str text) {
    if (!m_impl->window) {
        return false;
    }
    const QString qtext = QString::fromUtf8(text.data(), static_cast<int>(text.size()));
    const auto mods = Qt::KeyboardModifiers(modifiers);
    QKeyEvent press(QEvent::KeyPress, key, mods, qtext);
    QCoreApplication::sendEvent(m_impl->window.get(), &press);
    QKeyEvent release(QEvent::KeyRelease, key, mods, qtext);
    QCoreApplication::sendEvent(m_impl->window.get(), &release);
    process_events();
    process_events();
    return invokePaneBool(m_impl->pane, "testTargetPromptOpen");
}

void ChartPaneProbe::send_wheel(float x, float y, int angle_delta_y) {
    if (!m_impl->window) {
        return;
    }
    const QPointF local(x, y);
    const QPointF global = m_impl->window->mapToGlobal(local);
    QWheelEvent wheel(local, global, QPoint(), QPoint(0, angle_delta_y), Qt::NoButton,
                      Qt::NoModifier, Qt::NoScrollPhase, false);
    QCoreApplication::sendEvent(m_impl->window.get(), &wheel);
    process_events();
}

int ChartPaneProbe::first_bar() const {
    auto* vp = m_impl->pane ? m_impl->pane->property("viewport").value<QObject*>() : nullptr;
    return vp ? vp->property("firstBar").toInt() : 0;
}

int ChartPaneProbe::last_bar() const {
    auto* vp = m_impl->pane ? m_impl->pane->property("viewport").value<QObject*>() : nullptr;
    return vp ? vp->property("lastBar").toInt() : 0;
}

rust::String ChartPaneProbe::readout_item_pointer_price() const {
    if (!m_impl->pane) {
        return rust::String();
    }
    auto* item = m_impl->pane->findChild<QObject*>(QStringLiteral("readoutPointerPrice"));
    if (!item) {
        return rust::String();
    }
    return rust::String(item->property("text").toString().toStdString());
}

rust::String ChartPaneProbe::navigation_mode() const {
    if (!m_impl->pane) {
        return rust::String();
    }
    auto* vp = m_impl->pane->property("viewport").value<QObject*>();
    return vp ? rust::String(vp->property("mode").toString().toStdString()) : rust::String();
}

double ChartPaneProbe::high_price() const {
    auto* vp = m_impl->pane ? m_impl->pane->property("viewport").value<QObject*>() : nullptr;
    return vp ? vp->property("highPrice").toDouble() : 0.0;
}

double ChartPaneProbe::low_price() const {
    auto* vp = m_impl->pane ? m_impl->pane->property("viewport").value<QObject*>() : nullptr;
    return vp ? vp->property("lowPrice").toDouble() : 0.0;
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

struct StatusStripProbe::Impl {
    QQmlEngine engine;
    QObject* strip{nullptr};
};

StatusStripProbe::StatusStripProbe() : m_impl(std::make_unique<Impl>()) {
    ensure_test_app();
    m_impl->engine.addImportPath(QStringLiteral("target/cxxqt/qml_modules"));

    QQmlComponent component(&m_impl->engine);
    QFileInfo fileInfo(QStringLiteral("qml/StatusStrip.qml"));
    if (fileInfo.exists()) {
        component.loadUrl(QUrl::fromLocalFile(fileInfo.absoluteFilePath()));
    } else {
        component.loadUrl(QUrl(QStringLiteral("qrc:/qt/qml/qml/StatusStrip.qml")));
    }
    m_impl->strip = component.create();
}

StatusStripProbe::~StatusStripProbe() {
    if (m_impl->strip) {
        delete m_impl->strip;
    }
}

void StatusStripProbe::set_feed(BarFeed* feed) {
    if (m_impl->strip) {
        m_impl->strip->setProperty("feed", QVariant::fromValue(static_cast<QObject*>(feed)));
    }
}

StatusStripProbeResult StatusStripProbe::result() const {
    StatusStripProbeResult out{};
    if (m_impl->strip) {
        auto* stateObj = m_impl->strip->findChild<QObject*>("connectionStateText");
        if (stateObj) {
            out.connection_state = rust::String(stateObj->property("text").toString().toStdString());
        }
        auto* errorObj = m_impl->strip->findChild<QObject*>("errorText");
        if (errorObj) {
            out.last_error = rust::String(errorObj->property("text").toString().toStdString());
        }
        auto* histObj = m_impl->strip->findChild<QObject*>("historyText");
        if (histObj) {
            out.history_text = rust::String(histObj->property("text").toString().toStdString());
        }
        auto* staleBadge = m_impl->strip->findChild<QObject*>("staleBadge");
        if (staleBadge) {
            out.stale_visible = staleBadge->property("visible").toBool();
        }
        auto* staleText = m_impl->strip->findChild<QObject*>("staleText");
        if (staleText) {
            out.stale_text = rust::String(staleText->property("text").toString().toStdString());
        }
        auto* liveBadge = m_impl->strip->findChild<QObject*>("liveOnlyBadge");
        if (liveBadge) {
            out.live_only_visible = liveBadge->property("visible").toBool();
        }
    }
    return out;
}

std::unique_ptr<StatusStripProbe> make_status_strip_probe() {
    return std::make_unique<StatusStripProbe>();
}

struct EmptyStateProbe::Impl {
    QQmlEngine engine;
    QObject* emptyState{nullptr};
};

EmptyStateProbe::EmptyStateProbe() : m_impl(std::make_unique<Impl>()) {
    ensure_test_app();
    m_impl->engine.addImportPath(QStringLiteral("target/cxxqt/qml_modules"));

    QQmlComponent component(&m_impl->engine);
    QFileInfo fileInfo(QStringLiteral("qml/ChartEmptyState.qml"));
    if (fileInfo.exists()) {
        component.loadUrl(QUrl::fromLocalFile(fileInfo.absoluteFilePath()));
    } else {
        component.loadUrl(QUrl(QStringLiteral("qrc:/qt/qml/qml/ChartEmptyState.qml")));
    }
    m_impl->emptyState = component.create();
}

EmptyStateProbe::~EmptyStateProbe() {
    if (m_impl->emptyState) {
        delete m_impl->emptyState;
    }
}

void EmptyStateProbe::set_feed(BarFeed* feed) {
    if (m_impl->emptyState) {
        m_impl->emptyState->setProperty("feed", QVariant::fromValue(static_cast<QObject*>(feed)));
    }
}

EmptyStateProbeResult EmptyStateProbe::result() const {
    EmptyStateProbeResult out{};
    if (m_impl->emptyState) {
        auto* msgObj = m_impl->emptyState->findChild<QObject*>("emptyMessageText");
        if (msgObj) {
            out.message = rust::String(msgObj->property("text").toString().toStdString());
        }
        auto* reasonObj = m_impl->emptyState->findChild<QObject*>("emptyReasonText");
        if (reasonObj) {
            out.reason = rust::String(reasonObj->property("text").toString().toStdString());
        }
    }
    return out;
}

std::unique_ptr<EmptyStateProbe> make_empty_state_probe() {
    return std::make_unique<EmptyStateProbe>();
}

struct ChartIdentityProbe::Impl {
    QQmlEngine engine;
    QObject* identity{nullptr};
    ChartContext* context{nullptr};
    BarFeed* feed{nullptr};
};

ChartIdentityProbe::ChartIdentityProbe() : m_impl(std::make_unique<Impl>()) {
    ensure_test_app();
    m_impl->engine.addImportPath(QStringLiteral("target/cxxqt/qml_modules"));

    QQmlComponent component(&m_impl->engine);
    QFileInfo fileInfo(QStringLiteral("qml/components/ChartIdentity.qml"));
    if (fileInfo.exists()) {
        component.loadUrl(QUrl::fromLocalFile(fileInfo.absoluteFilePath()));
    } else {
        component.loadUrl(QUrl(QStringLiteral("qrc:/qt/qml/qml/components/ChartIdentity.qml")));
    }
    m_impl->identity = component.create();
}

ChartIdentityProbe::~ChartIdentityProbe() {
    if (m_impl->identity) {
        delete m_impl->identity;
    }
}

void ChartIdentityProbe::set_context(ChartContext* context) {
    m_impl->context = context;
    if (m_impl->identity) {
        m_impl->identity->setProperty("context", QVariant::fromValue(static_cast<QObject*>(context)));
    }
}

void ChartIdentityProbe::set_feed(BarFeed* feed) {
    m_impl->feed = feed;
    if (m_impl->identity) {
        m_impl->identity->setProperty("feed", QVariant::fromValue(static_cast<QObject*>(feed)));
    }
    if (m_impl->context && feed) {
        m_impl->context->sync();
    }
}

ChartIdentityProbeResult ChartIdentityProbe::result() const {
    ChartIdentityProbeResult out{};
    if (m_impl->identity) {
        auto* symbolObj = m_impl->identity->findChild<QObject*>("symbolLineText");
        if (symbolObj) {
            out.symbol_line = rust::String(symbolObj->property("text").toString().toStdString());
        }
        auto* sourceObj = m_impl->identity->findChild<QObject*>("sourceLabelText");
        if (sourceObj) {
            out.source_label = rust::String(sourceObj->property("text").toString().toStdString());
        }
        auto* conditionObj = m_impl->identity->findChild<QObject*>("conditionBadge");
        if (conditionObj) {
            out.condition_label = rust::String(conditionObj->property("text").toString().toStdString());
        }
        auto* lastBarObj = m_impl->identity->findChild<QObject*>("lastBarLabelText");
        if (lastBarObj) {
            auto text = lastBarObj->property("text").toString().toStdString();
            const std::string prefix = "Last bar: ";
            if (text.rfind(prefix, 0) == 0) {
                text = text.substr(prefix.size());
            }
            out.last_bar_label = rust::String(text);
        }
        if (m_impl->context) {
            out.is_switching = m_impl->context->getIs_switching();
            out.source_label =
                rust::String(m_impl->context->getSource_label().toStdString());
            out.symbol_line = rust::String(m_impl->context->getSymbol_line().toStdString());
            out.last_bar_label =
                rust::String(m_impl->context->getLast_bar_label().toStdString());
            out.condition_label =
                rust::String(m_impl->context->getCondition_label().toStdString());
        }
    }
    return out;
}

std::unique_ptr<ChartIdentityProbe> make_chart_identity_probe() {
    return std::make_unique<ChartIdentityProbe>();
}

ChartContext* find_window_chart_context(QQmlApplicationEngine& engine) {
    for (QObject* root : engine.rootObjects()) {
        if (auto* context = root->findChild<ChartContext*>("chartContext")) {
            return context;
        }
        if (auto* context = root->findChild<ChartContext*>()) {
            return context;
        }
    }
    return nullptr;
}

ChartContext* make_test_chart_context() {
    return new ChartContext();
}

BarFeed* make_test_feed() {
    return new BarFeed();
}

void feed_set_connection_state(BarFeed* feed, rust::Str state) {
    feed->setConnection_state(QString::fromUtf8(state.data(), static_cast<int>(state.size())));
}

void feed_set_last_error(BarFeed* feed, rust::Str error) {
    feed->setLast_error(QString::fromUtf8(error.data(), static_cast<int>(error.size())));
}

void feed_set_stale(BarFeed* feed, bool stale) {
    feed->setStale(stale);
}

void feed_set_data_age_ms(BarFeed* feed, std::int64_t ms) {
    feed->setData_age_ms(ms);
}

void feed_set_live_only(BarFeed* feed, bool live_only) {
    feed->setLive_only(live_only);
}

void feed_set_history(BarFeed* feed, rust::Str source, std::int64_t shortfall) {
    feed->setHistory_source(QString::fromUtf8(source.data(), static_cast<int>(source.size())));
    feed->setHistory_shortfall(shortfall);
}

void feed_set_bar_count(BarFeed* feed, std::int64_t count) {
    feed->setBar_count(count);
}

BarFeed* find_window_feed(QQmlApplicationEngine& engine) {
    for (QObject* root : engine.rootObjects()) {
        if (auto* feed = root->findChild<BarFeed*>("barFeed")) {
            return feed;
        }
        if (auto* feed = root->findChild<BarFeed*>()) {
            return feed;
        }
    }
    return nullptr;
}

void setup_window_feed(QQmlApplicationEngine& engine, BarFeed* feed) {
    if (!feed) return;
    for (QObject* root : engine.rootObjects()) {
        root->setProperty("feed", QVariant::fromValue(static_cast<QObject*>(feed)));
    }
}

void setup_window_auto_close(QQmlApplicationEngine&, int ms) {
    if (ms <= 0) return;
    // Closing every window ends the application, as a user closing the last one would.
    QTimer::singleShot(ms, qApp, []() {
        const auto windows = QGuiApplication::topLevelWindows();
        for (QWindow* window : windows) {
            window->close();
        }
    });
}

void feed_set_symbol(BarFeed* feed, rust::Str symbol) {
    if (!feed) return;
    feed->setSymbol(QString::fromUtf8(symbol.data(), static_cast<int>(symbol.size())));
}

void feed_set_timeframe(BarFeed* feed, rust::Str timeframe, std::int64_t timeframe_ms) {
    if (!feed) return;
    feed->setTimeframe(QString::fromUtf8(timeframe.data(), static_cast<int>(timeframe.size())));
    feed->setTimeframe_ms(timeframe_ms);
}

void feed_setup_and_load(BarFeed* feed, rust::Str api_base, rust::Str symbol, rust::Str timeframe) {
    if (!feed) return;
    feed->setup_config(
        QString::fromUtf8(api_base.data(), static_cast<int>(api_base.size())),
        QString::fromUtf8(symbol.data(), static_cast<int>(symbol.size())),
        QString::fromUtf8(timeframe.data(), static_cast<int>(timeframe.size()))
    );
    feed->load_history();
}

void post_feed_stream_state(
    BarFeed* feed,
    rust::Str state,
    rust::Str last_error,
    std::int64_t applied,
    std::int64_t dropped,
    std::int64_t gaps_closed,
    std::int64_t resnapshots,
    std::int64_t rest_calls
) {
    if (!feed) return;
    QString qstate = QString::fromUtf8(state.data(), static_cast<int>(state.size()));
    QString qerr = QString::fromUtf8(last_error.data(), static_cast<int>(last_error.size()));
    QMetaObject::invokeMethod(feed, [feed, qstate, qerr, applied, dropped, gaps_closed, resnapshots, rest_calls]() {
        feed->setConnection_state(qstate);
        feed->setLast_error(qerr);
        feed->setApplied(applied);
        feed->setDropped(dropped);
        feed->setGaps_closed(gaps_closed);
        feed->setResnapshots(resnapshots);
        feed->setRest_calls(rest_calls);
    }, Qt::QueuedConnection);
}

void post_feed_completed_bar(
    BarFeed* feed,
    std::int64_t generation,
    std::int64_t time,
    double open,
    double high,
    double low,
    double close
) {
    if (!feed) return;
    QMetaObject::invokeMethod(feed, [feed, generation, time, open, high, low, close]() {
        feed->ingest_completed_bar_gen(generation, time, open, high, low, close);
    }, Qt::QueuedConnection);
}

void post_feed_forming_bar(
    BarFeed* feed,
    std::int64_t generation,
    std::int64_t time,
    double open,
    double high,
    double low,
    double close
) {
    if (!feed) return;
    QMetaObject::invokeMethod(feed, [feed, generation, time, open, high, low, close]() {
        feed->ingest_forming_bar_gen(generation, time, open, high, low, close);
    }, Qt::QueuedConnection);
}

int feed_bar_times_len(BarFeed* feed) {
    if (!feed) return 0;
    return feed->bar_times_len();
}

std::int64_t feed_bar_time_at(BarFeed* feed, int index) {
    if (!feed) return 0;
    return feed->bar_time_at(index);
}

int feed_vertex_len(BarFeed* feed) {
    if (!feed) return 0;
    return static_cast<int>(feed->vertex_len());
}

ProbeVertex feed_vertex_at(BarFeed* feed, std::size_t index) {
    const auto* source = reinterpret_cast<const BarVertex*>(feed->vertex_ptr());
    const BarVertex& vertex = source[index];
    return ProbeVertex{vertex.x, vertex.y, vertex.direction, vertex.forming};
}

void feed_rebuild_geometry(BarFeed* feed, int first_bar, int last_bar, double low, double high, float width, float height) {
    if (!feed) return;
    feed->set_surface(width, height);
    feed->set_viewport(first_bar, last_bar, low, high);
    feed->rebuild_geometry();
}

rust::String feed_history_source(BarFeed* feed) {
    if (!feed) return "";
    return rust::String(feed->getHistory_source().toStdString());
}

rust::String feed_history_error(BarFeed* feed) {
    if (!feed) return "";
    return rust::String(feed->getHistory_error().toStdString());
}

bool feed_history_loading(BarFeed* feed) {
    if (!feed) return false;
    return feed->getHistory_loading();
}

std::int64_t feed_bar_count(BarFeed* feed) {
    if (!feed) return 0;
    return feed->getBar_count();
}

std::int64_t feed_rest_calls(BarFeed* feed) {
    if (!feed) return 0;
    return feed->getRest_calls();
}

bool feed_retarget(BarFeed* feed, rust::Str symbol, rust::Str timeframe, std::int64_t generation) {
    if (!feed) return false;
    return feed->retarget(
        QString::fromUtf8(symbol.data(), static_cast<int>(symbol.size())),
        QString::fromUtf8(timeframe.data(), static_cast<int>(timeframe.size())),
        generation);
}

std::int64_t feed_target_generation(BarFeed* feed) {
    return feed ? feed->target_generation() : -1;
}

std::int64_t feed_stale_dropped(BarFeed* feed) {
    return feed ? feed->stale_dropped() : -1;
}

rust::String feed_symbol(BarFeed* feed) {
    return feed ? rust::String(feed->getSymbol().toStdString()) : rust::String();
}

#include "q_terminal/src/execution_controls.cxxqt.h"
#include "q_terminal/src/execution_models.cxxqt.h"
#include "q_terminal/src/ops_status.cxxqt.h"

ExecutionModels* find_window_execution_models(QQmlApplicationEngine& engine) {
    for (QObject* root : engine.rootObjects()) {
        if (auto* models = root->findChild<ExecutionModels*>("executionModels")) {
            return models;
        }
        if (auto* models = root->findChild<ExecutionModels*>()) {
            return models;
        }
    }
    return nullptr;
}

void post_execution_models_sync(ExecutionModels* models) {
    if (!models) return;
    QMetaObject::invokeMethod(models, [models]() { models->sync(); }, Qt::QueuedConnection);
}

void post_feed_overlays(BarFeed* feed, std::int64_t generation, rust::Str json) {
    if (!feed) return;
    QString text = QString::fromUtf8(json.data(), static_cast<int>(json.size()));
    QMetaObject::invokeMethod(feed, [feed, generation, text]() {
        feed->set_overlays_json(generation, text);
    }, Qt::QueuedConnection);
}

void feed_set_execution_rows(BarFeed* feed, rust::Str decisions, rust::Str fills) {
    if (!feed) return;
    feed->set_execution_rows(
        QString::fromUtf8(decisions.data(), static_cast<int>(decisions.size())),
        QString::fromUtf8(fills.data(), static_cast<int>(fills.size())));
}

std::int64_t feed_marker_count(BarFeed* feed) {
    return feed ? feed->marker_count() : 0;
}

std::int64_t feed_rebuild_overlays(BarFeed* feed, int first_bar, int last_bar, double low,
                                   double high, float width, float height) {
    if (!feed) return 0;
    feed->rebuild_overlays(first_bar, last_bar, low, high, width, height);
    return feed->overlay_layer_count();
}

void feed_populate_bench(BarFeed* feed, std::int64_t bars, std::int64_t markers, std::int64_t overlays) {
    if (feed) {
        feed->bench_populate(bars, markers, overlays);
    }
}

std::uintptr_t find_window_ops_status(QQmlApplicationEngine& engine) {
    for (QObject* root : engine.rootObjects()) {
        if (auto* status = root->findChild<OpsStatus*>("opsStatus")) {
            return reinterpret_cast<std::uintptr_t>(status);
        }
        if (auto* status = root->findChild<OpsStatus*>()) {
            return reinterpret_cast<std::uintptr_t>(status);
        }
    }
    return 0;
}

std::uintptr_t find_window_execution_controls(QQmlApplicationEngine& engine) {
    for (QObject* root : engine.rootObjects()) {
        if (auto* controls = root->findChild<ExecutionControls*>("executionControls")) {
            return reinterpret_cast<std::uintptr_t>(controls);
        }
        if (auto* controls = root->findChild<ExecutionControls*>()) {
            return reinterpret_cast<std::uintptr_t>(controls);
        }
    }
    return 0;
}

void execution_models_setup(ExecutionModels* models, rust::Str api_base) {
    if (!models) {
        return;
    }
    models->setup(QString::fromUtf8(api_base.data(), static_cast<int>(api_base.size())));
}

void execution_models_bind_handle(ExecutionModels* models) {
    if (!models) {
        return;
    }
    models->bind_store();
}

void ops_status_mark_api_offline(std::uintptr_t status) {
    if (!status) {
        return;
    }
    reinterpret_cast<OpsStatus*>(status)->mark_api_offline();
}

void ops_status_mark_postgres_down(std::uintptr_t status) {
    if (!status) {
        return;
    }
    reinterpret_cast<OpsStatus*>(status)->mark_postgres_down();
}

void ops_status_apply_health(std::uintptr_t status, rust::Str health_json) {
    if (!status) {
        return;
    }
    reinterpret_cast<OpsStatus*>(status)->apply_health_json(
        QString::fromUtf8(health_json.data(), static_cast<int>(health_json.size())));
}

void ops_status_apply_positions(std::uintptr_t status, rust::Str positions_json) {
    if (!status) {
        return;
    }
    reinterpret_cast<OpsStatus*>(status)->apply_positions_json(
        QString::fromUtf8(positions_json.data(), static_cast<int>(positions_json.size())));
}

void ops_status_set_stream(std::uintptr_t status, rust::Str state, double age_s) {
    if (!status) {
        return;
    }
    reinterpret_cast<OpsStatus*>(status)->set_stream_info(
        QString::fromUtf8(state.data(), static_cast<int>(state.size())), age_s);
}

void execution_controls_setup(std::uintptr_t controls, rust::Str api_base, rust::Str operator_name) {
    if (!controls) {
        return;
    }
    reinterpret_cast<ExecutionControls*>(controls)->setup(
        QString::fromUtf8(api_base.data(), static_cast<int>(api_base.size())),
        QString::fromUtf8(operator_name.data(), static_cast<int>(operator_name.size())));
}

void execution_controls_bind_handle(std::uintptr_t controls) {
    if (!controls) {
        return;
    }
    reinterpret_cast<ExecutionControls*>(controls)->bind_store();
}

void execution_controls_update_health(
    std::uintptr_t controls,
    bool api_offline,
    bool postgres_available,
    rust::Str worker_status,
    double worker_heartbeat_age_s,
    bool edge_reachable,
    bool edge_mt5_connected) {
    if (!controls) {
        return;
    }
    reinterpret_cast<ExecutionControls*>(controls)->update_health(
        api_offline,
        postgres_available,
        QString::fromUtf8(worker_status.data(), static_cast<int>(worker_status.size())),
        worker_heartbeat_age_s,
        edge_reachable,
        edge_mt5_connected);
}

std::uintptr_t make_test_execution_controls() {
    ensure_test_app();
    return reinterpret_cast<std::uintptr_t>(new ExecutionControls());
}

// ---- Shell test seams (Q-052) ----------------------------------------------------------

#include <QtCore/QRegularExpression>
#include <QtGui/QImage>
#include <QtQml/QQmlContext>
#include <QtQml/QQmlExpression>

rust::String shell_eval(QQmlApplicationEngine& engine, rust::Str js) {
    const auto roots = engine.rootObjects();
    if (roots.isEmpty()) {
        return rust::String("error: no root object");
    }
    QQmlExpression expression(engine.rootContext(), roots.first(),
                              QString::fromUtf8(js.data(), static_cast<int>(js.size())));
    bool is_undefined = false;
    const QVariant value = expression.evaluate(&is_undefined);
    if (expression.hasError()) {
        return rust::String("error: " + expression.error().toString().toStdString());
    }
    return rust::String(value.toString().toStdString());
}

std::int32_t shell_visible_window_count() {
    std::int32_t count = 0;
    for (QWindow* window : QGuiApplication::topLevelWindows()) {
        if (qobject_cast<QQuickWindow*>(window) && window->isVisible()) {
            ++count;
        }
    }
    return count;
}

// Renders every visible window to `<dir>/<title>.png`; returns how many were written.
std::int32_t shell_grab_windows(rust::Str dir) {
    const QString base = QString::fromUtf8(dir.data(), static_cast<int>(dir.size()));
    std::int32_t written = 0;
    for (QWindow* w : QGuiApplication::topLevelWindows()) {
        auto* window = qobject_cast<QQuickWindow*>(w);
        if (!window || !window->isVisible()) continue;
        for (int i = 0; i < 4; ++i) {
            QCoreApplication::processEvents(QEventLoop::AllEvents, 50);
            window->update();
        }
        const QImage image = window->grabWindow();
        QString name = window->title();
        name.replace(QRegularExpression(QStringLiteral("[^A-Za-z0-9]+")), QStringLiteral("-"));
        if (!image.isNull() && image.save(base + QStringLiteral("/") + name + QStringLiteral(".png"), "PNG")) {
            ++written;
        }
    }
    return written;
}

bool shell_quit_on_last_window_closed() {
    return QGuiApplication::quitOnLastWindowClosed();
}
