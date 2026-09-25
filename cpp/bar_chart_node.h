#pragma once

#include "bar_vertex.h"
#include "q-qt/src/bar_series.cxxqt.h"

#include <QtQuick/QSGGeometry>

#include <vector>

struct BarChartFrameStats {
    long long syncs = 0;
    long long completedVertices = 0;
    long long formingVertices = 0;
    long long rendererAllocations = 0;
    long long geometryPrepNs = 0;
};

struct BarChartSourceKey {
    const QObject* source = nullptr;
    long long targetGeneration = 0;
    int firstBar = 0;
    int lastBar = 0;
    double lowPrice = 0.0;
    double highPrice = 0.0;
    float width = 0.0f;
    float height = 0.0f;

    bool operator==(const BarChartSourceKey& other) const {
        return source == other.source && targetGeneration == other.targetGeneration &&
               firstBar == other.firstBar && lastBar == other.lastBar &&
               lowPrice == other.lowPrice && highPrice == other.highPrice &&
               width == other.width && height == other.height;
    }
    bool operator!=(const BarChartSourceKey& other) const { return !(*this == other); }
};

#include <QtGui/QColor>
#include <QtQuick/QSGGeometryNode>
#include <QtQuick/QSGNode>

class BucketGeometryNode : public QSGGeometryNode {
public:
    explicit BucketGeometryNode(QSGGeometry::DataPattern pattern);
    ~BucketGeometryNode() override;

    void syncVertices(const BarVertex* vertices, int count, const QColor& color);
    void syncVertices(const BarVertex* vertices, int count);
    bool setColor(const QColor& color);
    int allocationCount() const { return m_allocationCount; }

private:
    int m_allocationCount = 0;
    QSGGeometry::DataPattern m_pattern;
};

class BarChartNode : public QSGNode {
public:
    BarChartNode();

    void sync(const BarVertexView& completed, const BarVertexView& forming,
              const BarChartSourceKey& sourceKey, const QColor& rising,
              const QColor& falling, const QColor& formingColor);

    long long uploadedRevision() const { return m_uploadedRevision; }
    int uploadCount() const { return m_uploadCount; }
    int takeFrameUploads();
    int geometryAllocationCount() const;
    int completedLayerUpdateCount() const { return m_completedLayerUpdateCount; }
    int formingLayerUpdateCount() const { return m_formingLayerUpdateCount; }
    BarChartFrameStats takeFrameStats();
    void setProbeCaptureEnabled(bool enabled) { m_probeCaptureEnabled = enabled; }
    void captureProbeVertices(const BarVertex* vertices, size_t count);

    int risingVertexCount() const { return m_risingCount; }
    int fallingVertexCount() const { return m_fallingCount; }
    int formingVertexCount() const { return m_formingCount; }

    void collectVertices(std::vector<BarVertex>& out) const;
    const std::vector<BarVertex>& lastUploadedVertices() const { return m_lastUploadedVertices; }
    void resetForTests();

private:
    BucketGeometryNode* m_rising = nullptr;
    BucketGeometryNode* m_falling = nullptr;
    BucketGeometryNode* m_forming = nullptr;

    long long m_uploadedRevision = 0;
    long long m_completedRevision = 0;
    long long m_formingRevision = 0;
    BarChartSourceKey m_sourceKey;
    bool m_hasSourceKey = false;
    int m_uploadCount = 0;
    int m_risingCount = 0;
    int m_fallingCount = 0;
    int m_formingCount = 0;
    int m_frameUploads = 0;
    int m_frameCompletedVertices = 0;
    int m_frameFormingVertices = 0;
    int m_frameRendererAllocations = 0;
    int m_completedLayerUpdateCount = 0;
    int m_formingLayerUpdateCount = 0;
    bool m_probeCaptureEnabled = false;
    std::vector<BarVertex> m_lastUploadedVertices;
    std::vector<BarVertex> m_risingScratch;
    std::vector<BarVertex> m_fallingScratch;
};
