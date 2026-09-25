#include "bar_chart_item.h"

#include "bar_chart_node.h"
#include "overlay_chart_item.h"

#include <mutex>
#include <QtCore/QElapsedTimer>
#include <QtQml/qqml.h>

#include "q-qt/src/bar_series.cxxqt.h"
#include "q_terminal/src/bar_feed.cxxqt.h"

BarChartItem::BarChartItem(QQuickItem* parent) : QQuickItem(parent) {
    setFlag(QQuickItem::ItemHasContents, true);
}

void BarChartItem::scheduleUpdate() {
    m_updateRequestCount++;
    update();
}

void BarChartItem::setSeries(QObject* series) {
    if (m_series == series) {
        return;
    }
    if (m_series != nullptr) {
        disconnect(m_series, nullptr, this, nullptr);
    }
    m_series = series;
    if (m_series != nullptr) {
        if (auto* barFeed = qobject_cast<BarFeed*>(m_series)) {
            connect(barFeed, &BarFeed::revisionChanged, this, &BarChartItem::scheduleUpdate);
        } else if (auto* barSeries = qobject_cast<BarSeries*>(m_series)) {
            connect(barSeries, &BarSeries::revisionChanged, this, &BarChartItem::scheduleUpdate);
        }
    }
    emit seriesChanged();
    scheduleUpdate();
}

void BarChartItem::setFirstBar(int value) {
    if (m_firstBar == value) {
        return;
    }
    m_firstBar = value;
    emit firstBarChanged();
    scheduleUpdate();
}

void BarChartItem::setLastBar(int value) {
    if (m_lastBar == value) {
        return;
    }
    m_lastBar = value;
    emit lastBarChanged();
    scheduleUpdate();
}

void BarChartItem::setLowPrice(double value) {
    if (m_lowPrice == value) {
        return;
    }
    m_lowPrice = value;
    emit lowPriceChanged();
    scheduleUpdate();
}

void BarChartItem::setHighPrice(double value) {
    if (m_highPrice == value) {
        return;
    }
    m_highPrice = value;
    emit highPriceChanged();
    scheduleUpdate();
}

void BarChartItem::setRisingColor(const QColor& value) {
    if (m_risingColor == value) {
        return;
    }
    m_risingColor = value;
    emit risingColorChanged();
    scheduleUpdate();
}

void BarChartItem::setFallingColor(const QColor& value) {
    if (m_fallingColor == value) {
        return;
    }
    m_fallingColor = value;
    emit fallingColorChanged();
    scheduleUpdate();
}

void BarChartItem::setFormingColor(const QColor& value) {
    if (m_formingColor == value) {
        return;
    }
    m_formingColor = value;
    emit formingColorChanged();
    scheduleUpdate();
}

void BarChartItem::geometryChange(const QRectF& newGeometry, const QRectF& oldGeometry) {
    QQuickItem::geometryChange(newGeometry, oldGeometry);
    if (auto* barFeed = qobject_cast<BarFeed*>(m_series)) {
        barFeed->set_surface(static_cast<float>(newGeometry.width()),
                             static_cast<float>(newGeometry.height()));
    } else if (auto* barSeries = qobject_cast<BarSeries*>(m_series)) {
        barSeries->set_surface(static_cast<float>(newGeometry.width()),
                               static_cast<float>(newGeometry.height()));
    }
    scheduleUpdate();
}

namespace {

BarChartSourceKey sourceKey(QObject* source, long long generation, int firstBar, int lastBar,
                            double lowPrice, double highPrice, float width, float height) {
    return BarChartSourceKey{source, generation, firstBar, lastBar, lowPrice, highPrice, width,
                             height};
}

} // namespace

QSGNode* BarChartItem::updatePaintNode(QSGNode* oldNode, UpdatePaintNodeData* data) {
    Q_UNUSED(data);
    return testUpdatePaintNode(oldNode);
}

QSGNode* BarChartItem::testUpdatePaintNode(QSGNode* oldNode) {
    QElapsedTimer geometryTimer;
    geometryTimer.start();
    m_paintNodeCallCount++;
    auto* node = static_cast<BarChartNode*>(oldNode);
    if (node == nullptr) {
        node = new BarChartNode();
    }
    m_chartNode = node;

    auto* barFeed = qobject_cast<BarFeed*>(m_series);
    auto* barSeries = qobject_cast<BarSeries*>(m_series);

    // The surface is set on every paint, not only on resize: a newly assigned series, or
    // the fresh series a feed makes on retarget, would otherwise keep a zero surface and
    // draw nothing until the item is resized.
    const auto surfaceWidth = static_cast<float>(width());
    const auto surfaceHeight = static_cast<float>(height());

    auto captureCombinedProbeGeometry = [this, node, barFeed, barSeries]() {
        if (!m_captureProbeVertices) {
            return;
        }
        if (barFeed != nullptr) {
            barFeed->rebuild_geometry();
            node->captureProbeVertices(
                reinterpret_cast<const BarVertex*>(barFeed->vertex_ptr()),
                static_cast<size_t>(barFeed->vertex_len()));
        } else if (barSeries != nullptr) {
            barSeries->rebuild_geometry();
            node->captureProbeVertices(barSeries->vertex_ptr(), barSeries->vertex_len());
        } else {
            node->captureProbeVertices(nullptr, 0);
        }
    };

    if (barFeed != nullptr) {
        barFeed->set_surface(surfaceWidth, surfaceHeight);
        barFeed->set_viewport(m_firstBar, m_lastBar, m_lowPrice, m_highPrice);
        barFeed->rebuild_split_geometry();

        const auto* completed = reinterpret_cast<const BarVertex*>(barFeed->completed_vertex_ptr());
        const auto* forming = reinterpret_cast<const BarVertex*>(barFeed->forming_vertex_ptr());
        const BarVertexView completedView{completed,
                                          static_cast<size_t>(barFeed->completed_vertex_len()),
                                          barFeed->completed_geometry_revision()};
        const BarVertexView formingView{forming,
                                        static_cast<size_t>(barFeed->forming_vertex_len()),
                                        barFeed->forming_geometry_revision()};
        const auto key = sourceKey(m_series, barFeed->target_generation(), m_firstBar, m_lastBar,
                                   m_lowPrice, m_highPrice, surfaceWidth, surfaceHeight);
        node->sync(completedView, formingView, key, m_risingColor, m_fallingColor,
                   m_formingColor);
        captureCombinedProbeGeometry();
        m_frameGeometryPrepNs += geometryTimer.nsecsElapsed();
        return node;
    }

    if (barSeries != nullptr) {
        barSeries->set_surface(surfaceWidth, surfaceHeight);
        barSeries->set_viewport(m_firstBar, m_lastBar, m_lowPrice, m_highPrice);
        barSeries->rebuild_split_geometry();

        const BarVertexView completedView{
            barSeries->completed_vertex_ptr(), barSeries->completed_vertex_len(),
            barSeries->completed_geometry_revision()};
        const BarVertexView formingView{
            barSeries->forming_vertex_ptr(), barSeries->forming_vertex_len(),
            barSeries->forming_geometry_revision()};
        const auto key = sourceKey(m_series, 0, m_firstBar, m_lastBar, m_lowPrice, m_highPrice,
                                   surfaceWidth, surfaceHeight);
        node->sync(completedView, formingView, key, m_risingColor, m_fallingColor,
                   m_formingColor);
    } else {
        const auto key = sourceKey(nullptr, 0, m_firstBar, m_lastBar, m_lowPrice, m_highPrice,
                                   surfaceWidth, surfaceHeight);
        const BarVertexView emptyCompleted{nullptr, 0, 0};
        const BarVertexView emptyForming{nullptr, 0, 0};
        node->sync(emptyCompleted, emptyForming, key, m_risingColor, m_fallingColor,
                   m_formingColor);
    }
    captureCombinedProbeGeometry();
    m_frameGeometryPrepNs += geometryTimer.nsecsElapsed();
    return node;
}

int BarChartItem::takeFrameUploads() {
    if (m_chartNode == nullptr) {
        return 0;
    }
    return m_chartNode->takeFrameUploads();
}

BarChartFrameStats BarChartItem::takeFrameStats() {
    if (m_chartNode == nullptr) {
        return {};
    }
    BarChartFrameStats stats = m_chartNode->takeFrameStats();
    stats.geometryPrepNs = m_frameGeometryPrepNs;
    m_frameGeometryPrepNs = 0;
    return stats;
}

void register_bar_chart_types() {
    static std::once_flag once;
    std::call_once(once, []() {
        qmlRegisterType<BarSeries>("qml", 1, 0, "BarSeries");
        qmlRegisterType<BarChartItem>("qml", 1, 0, "BarChartItem");
        qmlRegisterType<OverlayChartItem>("qml", 1, 0, "OverlayChartItem");
    });
}
