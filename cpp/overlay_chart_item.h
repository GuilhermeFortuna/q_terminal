#pragma once

#include <QtCore/QObject>
#include <QtQuick/QQuickItem>
#include <QtQuick/QSGNode>

/// Draws the deployment's marker glyphs and indicator overlays. The vertex buffers are
/// built in Rust (BarFeed::rebuild_overlays) from the same viewport the bar item draws;
/// this item only moves them into the scene graph, one geometry node per layer.
class OverlayChartItem : public QQuickItem {
    Q_OBJECT
    Q_PROPERTY(QObject* series READ series WRITE setSeries NOTIFY seriesChanged)
    Q_PROPERTY(int firstBar READ firstBar WRITE setFirstBar NOTIFY firstBarChanged)
    Q_PROPERTY(int lastBar READ lastBar WRITE setLastBar NOTIFY lastBarChanged)
    Q_PROPERTY(double lowPrice READ lowPrice WRITE setLowPrice NOTIFY lowPriceChanged)
    Q_PROPERTY(double highPrice READ highPrice WRITE setHighPrice NOTIFY highPriceChanged)

public:
    explicit OverlayChartItem(QQuickItem* parent = nullptr);

    QObject* series() const { return m_series; }
    void setSeries(QObject* series);
    int firstBar() const { return m_firstBar; }
    void setFirstBar(int value);
    int lastBar() const { return m_lastBar; }
    void setLastBar(int value);
    double lowPrice() const { return m_lowPrice; }
    void setLowPrice(double value);
    double highPrice() const { return m_highPrice; }
    void setHighPrice(double value);

    /// Number of layers and vertices the last paint uploaded, for the probes.
    int layerCount() const { return m_layerCount; }
    long long vertexCount() const { return m_vertexCount; }

    QSGNode* updatePaintNode(QSGNode* oldNode, UpdatePaintNodeData* data) override;

protected:
    void geometryChange(const QRectF& newGeometry, const QRectF& oldGeometry) override;

signals:
    void seriesChanged();
    void firstBarChanged();
    void lastBarChanged();
    void lowPriceChanged();
    void highPriceChanged();

private:
    QObject* m_series = nullptr;
    int m_firstBar = 0;
    int m_lastBar = 0;
    double m_lowPrice = 0.0;
    double m_highPrice = 0.0;
    int m_layerCount = 0;
    long long m_vertexCount = 0;
};
