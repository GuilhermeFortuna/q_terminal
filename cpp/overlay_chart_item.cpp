#include "overlay_chart_item.h"

#include <QtGui/QColor>
#include <QtQuick/QSGFlatColorMaterial>
#include <QtQuick/QSGGeometry>
#include <QtQuick/QSGGeometryNode>

#include "q_terminal/src/bar_feed.cxxqt.h"

OverlayChartItem::OverlayChartItem(QQuickItem* parent) : QQuickItem(parent) {
    setFlag(QQuickItem::ItemHasContents, true);
}

void OverlayChartItem::setSeries(QObject* series) {
    if (m_series == series) return;
    if (m_series != nullptr) disconnect(m_series, nullptr, this, nullptr);
    m_series = series;
    if (auto* feed = qobject_cast<BarFeed*>(m_series)) {
        connect(feed, &BarFeed::revisionChanged, this, [this]() { update(); });
        connect(feed, &BarFeed::overlay_revisionChanged, this, [this]() { update(); });
    }
    emit seriesChanged();
    update();
}

#define Q_OVERLAY_SETTER(Name, name, Type)     \
    void OverlayChartItem::set##Name(Type value) { \
        if (m_##name == value) return;          \
        m_##name = value;                       \
        emit name##Changed();                   \
        update();                               \
    }
Q_OVERLAY_SETTER(FirstBar, firstBar, int)
Q_OVERLAY_SETTER(LastBar, lastBar, int)
Q_OVERLAY_SETTER(LowPrice, lowPrice, double)
Q_OVERLAY_SETTER(HighPrice, highPrice, double)

void OverlayChartItem::geometryChange(const QRectF& newGeometry, const QRectF& oldGeometry) {
    QQuickItem::geometryChange(newGeometry, oldGeometry);
    update();
}

namespace {

class LayerNode : public QSGGeometryNode {
public:
    LayerNode() {
        setMaterial(new QSGFlatColorMaterial());
        setFlag(QSGNode::OwnedByParent, true);
        setFlag(QSGNode::OwnsMaterial, true);
        setFlag(QSGNode::OwnsGeometry, true);
    }
};

} // namespace

QSGNode* OverlayChartItem::updatePaintNode(QSGNode* oldNode, UpdatePaintNodeData*) {
    auto* root = oldNode != nullptr ? oldNode : new QSGNode();
    auto* feed = qobject_cast<BarFeed*>(m_series);
    int layers = 0;
    long long vertices = 0;
    if (feed != nullptr && width() > 0.0 && height() > 0.0) {
        feed->rebuild_overlays(m_firstBar, m_lastBar, m_lowPrice, m_highPrice,
                               static_cast<float>(width()), static_cast<float>(height()));
        layers = feed->overlay_layer_count();
    }

    // Reuse one node per layer, drop the extras.
    while (root->childCount() > layers) {
        QSGNode* extra = root->lastChild();
        root->removeChildNode(extra);
        delete extra;
    }
    while (root->childCount() < layers) {
        root->appendChildNode(new LayerNode());
    }

    QSGNode* child = root->firstChild();
    for (int i = 0; i < layers; ++i, child = child->nextSibling()) {
        auto* node = static_cast<LayerNode*>(child);
        const auto count = static_cast<int>(feed->overlay_layer_len(i));
        const auto* xy = reinterpret_cast<const float*>(feed->overlay_layer_ptr(i));
        auto* geom = node->geometry();
        if (geom == nullptr || geom->vertexCount() != count) {
            geom = new QSGGeometry(QSGGeometry::defaultAttributes_Point2D(), count);
            node->setGeometry(geom);
        }
        geom->setDrawingMode(feed->overlay_layer_mode(i) == 1 ? QSGGeometry::DrawLineStrip
                                                             : QSGGeometry::DrawTriangles);
        auto* points = geom->vertexDataAsPoint2D();
        for (int v = 0; v < count; ++v) {
            points[v].set(xy[2 * v], xy[2 * v + 1]);
        }
        geom->markVertexDataDirty();
        const auto rgba = static_cast<quint32>(feed->overlay_layer_color(i));
        static_cast<QSGFlatColorMaterial*>(node->material())
            ->setColor(QColor((rgba >> 24) & 0xff, (rgba >> 16) & 0xff, (rgba >> 8) & 0xff,
                              rgba & 0xff));
        node->markDirty(QSGNode::DirtyGeometry | QSGNode::DirtyMaterial);
        vertices += count;
    }
    m_layerCount = layers;
    m_vertexCount = vertices;
    return root;
}
