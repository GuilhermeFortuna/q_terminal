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

long long compositeRevision(long long geometryRevision, int firstBar, int lastBar, int syncGeneration) {
    return (geometryRevision << 32) ^ (static_cast<long long>(firstBar) << 16) ^
           static_cast<long long>(lastBar) ^ static_cast<long long>(syncGeneration);
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

    if ((barFeed == nullptr && barSeries == nullptr) || width() <= 0.0 || height() <= 0.0) {
        long long rev = barFeed ? barFeed->geometry_revision() : (barSeries ? barSeries->geometry_revision() : 0);
        BarVertexView emptyView{nullptr, 0, rev};
        node->sync(emptyView, m_risingColor, m_fallingColor, m_formingColor);
        m_frameGeometryPrepNs += geometryTimer.nsecsElapsed();
        return node;
    }

    // The surface is set on every paint, not only on resize: a newly assigned series, or
    // the fresh series a feed makes on retarget, would otherwise keep a zero surface and
    // draw nothing until the item is resized.
    const auto surfaceWidth = static_cast<float>(width());
    const auto surfaceHeight = static_cast<float>(height());

    if (barFeed != nullptr) {
        barFeed->set_surface(surfaceWidth, surfaceHeight);
        barFeed->set_viewport(m_firstBar, m_lastBar, m_lowPrice, m_highPrice);
        barFeed->rebuild_geometry();

        const auto* source = reinterpret_cast<const BarVertex*>(barFeed->vertex_ptr());
        BarVertexView view{source, static_cast<std::size_t>(barFeed->vertex_len()),
                           compositeRevision(barFeed->geometry_revision(), m_firstBar, m_lastBar, m_updateRequestCount)};
        node->sync(view, m_risingColor, m_fallingColor, m_formingColor);
        m_frameGeometryPrepNs += geometryTimer.nsecsElapsed();
        return node;
    }

    barSeries->set_surface(surfaceWidth, surfaceHeight);
    barSeries->set_viewport(m_firstBar, m_lastBar, m_lowPrice, m_highPrice);
    barSeries->rebuild_geometry();

    const BarVertex* source = barSeries->vertex_ptr();
    BarVertexView view{source, barSeries->vertex_len(),
                       compositeRevision(barSeries->geometry_revision(), m_firstBar, m_lastBar, m_updateRequestCount)};
    node->sync(view, m_risingColor, m_fallingColor, m_formingColor);
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
