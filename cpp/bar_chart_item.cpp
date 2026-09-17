#include "bar_chart_item.h"

#include "bar_chart_node.h"

#include <QtQml/qqml.h>

#include "q-qt/src/bar_series.cxxqt.h"

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
    m_series = series;
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
    if (auto* barSeries = qobject_cast<BarSeries*>(m_series)) {
        barSeries->set_surface(static_cast<float>(newGeometry.width()),
                               static_cast<float>(newGeometry.height()));
    }
    scheduleUpdate();
}

namespace {

long long compositeRevision(BarSeries* barSeries, int firstBar, int lastBar, int syncGeneration) {
    const long long geometryRevision = barSeries ? barSeries->geometry_revision() : 0;
    return (geometryRevision << 32) ^ (static_cast<long long>(firstBar) << 16) ^
           static_cast<long long>(lastBar) ^ static_cast<long long>(syncGeneration);
}

} // namespace

QSGNode* BarChartItem::updatePaintNode(QSGNode* oldNode, UpdatePaintNodeData* data) {
    Q_UNUSED(data);
    return testUpdatePaintNode(oldNode);
}

QSGNode* BarChartItem::testUpdatePaintNode(QSGNode* oldNode) {
    m_paintNodeCallCount++;
    auto* node = static_cast<BarChartNode*>(oldNode);
    if (node == nullptr) {
        node = new BarChartNode();
    }
    m_chartNode = node;

    auto* barSeries = qobject_cast<BarSeries*>(m_series);
    if (barSeries == nullptr || width() <= 0.0 || height() <= 0.0) {
        BarVertexView emptyView{nullptr, 0, barSeries ? barSeries->geometry_revision() : 0};
        node->sync(emptyView, m_risingColor, m_fallingColor, m_formingColor);
        return node;
    }

    barSeries->set_viewport(m_firstBar, m_lastBar, m_lowPrice, m_highPrice);
    barSeries->rebuild_geometry();

    const BarVertex* source = barSeries->vertex_ptr();
    BarVertexView view{source, barSeries->vertex_len(),
                       compositeRevision(barSeries, m_firstBar, m_lastBar, m_updateRequestCount)};
    node->sync(view, m_risingColor, m_fallingColor, m_formingColor);
    return node;
}

int BarChartItem::takeFrameUploads() {
    if (m_chartNode == nullptr) {
        return 0;
    }
    return m_chartNode->takeFrameUploads();
}

void register_bar_chart_types() {
    qmlRegisterType<BarSeries>("qml", 1, 0, "BarSeries");
    qmlRegisterType<BarChartItem>("qml", 1, 0, "BarChartItem");
}
