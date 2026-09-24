#pragma once

#include "bar_vertex.h"
#include "q-qt/src/bar_series.cxxqt.h"

#include <vector>

struct BarChartFrameStats {
    long long syncs = 0;
    long long completedVertices = 0;
    long long formingVertices = 0;
    long long rendererAllocations = 0;
    long long geometryPrepNs = 0;
};

#include <QtGui/QColor>
#include <QtQuick/QSGGeometryNode>
#include <QtQuick/QSGNode>

class BucketGeometryNode : public QSGGeometryNode {
public:
    BucketGeometryNode();
    ~BucketGeometryNode() override;

    void syncVertices(const BarVertex* vertices, int count, const QColor& color);
    int allocationCount() const { return m_allocationCount; }

private:
    int m_allocationCount = 0;
};

class BarChartNode : public QSGNode {
public:
    BarChartNode();

    void sync(const BarVertexView& view, const QColor& rising, const QColor& falling,
              const QColor& forming);

    long long uploadedRevision() const { return m_uploadedRevision; }
    int uploadCount() const { return m_uploadCount; }
    int takeFrameUploads();
    int geometryAllocationCount() const;
    BarChartFrameStats takeFrameStats();

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

    long long m_uploadedRevision = -1;
    int m_uploadCount = 0;
    int m_risingCount = 0;
    int m_fallingCount = 0;
    int m_formingCount = 0;
    int m_frameUploads = 0;
    int m_frameCompletedVertices = 0;
    int m_frameFormingVertices = 0;
    int m_frameRendererAllocations = 0;
    std::vector<BarVertex> m_lastUploadedVertices;
};
