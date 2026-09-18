#pragma once

#include <QtCore/QObject>
#include <QtGui/QColor>
#include <QtQuick/QQuickItem>
#include <QtQuick/QSGNode>

class BarChartNode;

class BarChartItem : public QQuickItem {
    Q_OBJECT
    Q_PROPERTY(QObject* series READ series WRITE setSeries NOTIFY seriesChanged)
    Q_PROPERTY(int firstBar READ firstBar WRITE setFirstBar NOTIFY firstBarChanged)
    Q_PROPERTY(int lastBar READ lastBar WRITE setLastBar NOTIFY lastBarChanged)
    Q_PROPERTY(double lowPrice READ lowPrice WRITE setLowPrice NOTIFY lowPriceChanged)
    Q_PROPERTY(double highPrice READ highPrice WRITE setHighPrice NOTIFY highPriceChanged)
    Q_PROPERTY(QColor risingColor READ risingColor WRITE setRisingColor NOTIFY risingColorChanged)
    Q_PROPERTY(QColor fallingColor READ fallingColor WRITE setFallingColor NOTIFY fallingColorChanged)
    Q_PROPERTY(QColor formingColor READ formingColor WRITE setFormingColor NOTIFY formingColorChanged)
    Q_PROPERTY(int paintNodeCallCount READ paintNodeCallCount CONSTANT)
    Q_PROPERTY(int updateRequestCount READ updateRequestCount CONSTANT)

public:
    explicit BarChartItem(QQuickItem* parent = nullptr);

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

    QColor risingColor() const { return m_risingColor; }
    void setRisingColor(const QColor& value);

    QColor fallingColor() const { return m_fallingColor; }
    void setFallingColor(const QColor& value);

    QColor formingColor() const { return m_formingColor; }
    void setFormingColor(const QColor& value);

    int paintNodeCallCount() const { return m_paintNodeCallCount; }
    int updateRequestCount() const { return m_updateRequestCount; }
    int takeFrameUploads();

    QSGNode* updatePaintNode(QSGNode* oldNode, UpdatePaintNodeData* data) override;

    QSGNode* testUpdatePaintNode(QSGNode* oldNode);

protected:
    void geometryChange(const QRectF& newGeometry, const QRectF& oldGeometry) override;

signals:
    void seriesChanged();
    void firstBarChanged();
    void lastBarChanged();
    void lowPriceChanged();
    void highPriceChanged();
    void risingColorChanged();
    void fallingColorChanged();
    void formingColorChanged();

private:
    QObject* m_series = nullptr;
    int m_firstBar = 0;
    int m_lastBar = 0;
    double m_lowPrice = 0.0;
    double m_highPrice = 0.0;
    QColor m_risingColor = QColor(0, 180, 0);
    QColor m_fallingColor = QColor(220, 0, 0);
    QColor m_formingColor = QColor(255, 165, 0);
    int m_paintNodeCallCount = 0;
    int m_updateRequestCount = 0;
    BarChartNode* m_chartNode = nullptr;

    void scheduleUpdate();
};

void register_bar_chart_types();
