#include "bar_chart_node.h"

#include <QtQuick/QSGFlatColorMaterial>

namespace {

void copyPoint2D(QSGGeometry* geometry, const BarVertex* vertices, int count) {
    QSGGeometry::Point2D* points = geometry->vertexDataAsPoint2D();
    for (int i = 0; i < count; ++i) {
        points[i].set(vertices[i].x, vertices[i].y);
    }
    geometry->markVertexDataDirty();
}

} // namespace

BucketGeometryNode::BucketGeometryNode(QSGGeometry::DataPattern pattern)
    : m_pattern(pattern) {
    auto* geom = new QSGGeometry(QSGGeometry::defaultAttributes_Point2D(), 0);
    geom->setDrawingMode(QSGGeometry::DrawTriangles);
    geom->setVertexDataPattern(m_pattern);
    setGeometry(geom);
    setMaterial(new QSGFlatColorMaterial());
    setFlag(QSGNode::OwnedByParent, true);
}

BucketGeometryNode::~BucketGeometryNode() {
    delete geometry();
}

bool BucketGeometryNode::setColor(const QColor& color) {
    auto* flat = static_cast<QSGFlatColorMaterial*>(material());
    if (flat->color() == color) {
        return false;
    }
    flat->setColor(color);
    markDirty(QSGNode::DirtyMaterial);
    return true;
}

void BucketGeometryNode::syncVertices(const BarVertex* vertices, int count) {
    if (count <= 0) {
        if (geometry() != nullptr && geometry()->vertexCount() != 0) {
            delete geometry();
            auto* geom = new QSGGeometry(QSGGeometry::defaultAttributes_Point2D(), 0);
            geom->setDrawingMode(QSGGeometry::DrawTriangles);
            geom->setVertexDataPattern(m_pattern);
            setGeometry(geom);
            m_allocationCount++;
            markDirty(QSGNode::DirtyGeometry);
        }
        return;
    }

    if (geometry() == nullptr || geometry()->vertexCount() != count) {
        delete geometry();
        auto* geom = new QSGGeometry(QSGGeometry::defaultAttributes_Point2D(), count);
        geom->setDrawingMode(QSGGeometry::DrawTriangles);
        geom->setVertexDataPattern(m_pattern);
        setGeometry(geom);
        m_allocationCount++;
    }

    copyPoint2D(geometry(), vertices, count);
    markDirty(QSGNode::DirtyGeometry);
}

void BucketGeometryNode::syncVertices(const BarVertex* vertices, int count,
                                      const QColor& color) {
    setColor(color);
    syncVertices(vertices, count);
}

BarChartNode::BarChartNode() {
    m_rising = new BucketGeometryNode(QSGGeometry::StaticPattern);
    m_falling = new BucketGeometryNode(QSGGeometry::StaticPattern);
    m_forming = new BucketGeometryNode(QSGGeometry::StreamPattern);
    appendChildNode(m_rising);
    appendChildNode(m_falling);
    appendChildNode(m_forming);
}

void BarChartNode::sync(const BarVertexView& completed, const BarVertexView& forming,
                        const BarChartSourceKey& sourceKey, const QColor& rising,
                        const QColor& falling, const QColor& formingColor) {
    const bool sourceChanged = !m_hasSourceKey || sourceKey != m_sourceKey;
    const bool completedChanged = completed.revision != m_completedRevision ||
                                  (sourceChanged &&
                                   (completed.count > 0 || m_risingCount > 0 || m_fallingCount > 0));
    const bool formingChanged = forming.revision != m_formingRevision ||
                                (sourceChanged && (forming.count > 0 || m_formingCount > 0));
    const int allocationsBefore = geometryAllocationCount();
    bool changed = false;

    if (completedChanged) {
        size_t risingCount = 0;
        size_t fallingCount = 0;
        for (size_t i = 0; completed.data != nullptr && i < completed.count; ++i) {
            if (completed.data[i].direction >= 0.0f) {
                ++risingCount;
            } else {
                ++fallingCount;
            }
        }
        const size_t risingCapacity = m_risingScratch.capacity();
        const size_t fallingCapacity = m_fallingScratch.capacity();
        m_risingScratch.resize(risingCount);
        m_fallingScratch.resize(fallingCount);
        if (m_risingScratch.capacity() != risingCapacity) {
            ++m_frameRendererAllocations;
        }
        if (m_fallingScratch.capacity() != fallingCapacity) {
            ++m_frameRendererAllocations;
        }

        size_t risingIndex = 0;
        size_t fallingIndex = 0;
        for (size_t i = 0; completed.data != nullptr && i < completed.count; ++i) {
            const BarVertex& vertex = completed.data[i];
            if (vertex.direction >= 0.0f) {
                m_risingScratch[risingIndex++] = vertex;
            } else {
                m_fallingScratch[fallingIndex++] = vertex;
            }
        }

        m_risingCount = static_cast<int>(risingCount);
        m_fallingCount = static_cast<int>(fallingCount);
        m_rising->syncVertices(m_risingScratch.data(), m_risingCount);
        m_falling->syncVertices(m_fallingScratch.data(), m_fallingCount);
        m_completedRevision = completed.revision;
        m_completedLayerUpdateCount++;
        m_frameCompletedVertices += m_risingCount + m_fallingCount;
        changed = true;
    }

    if (formingChanged) {
        m_formingCount = static_cast<int>(forming.count);
        m_forming->syncVertices(forming.data, m_formingCount);
        m_formingRevision = forming.revision;
        m_formingLayerUpdateCount++;
        m_frameFormingVertices += m_formingCount;
        changed = true;
    }

    changed |= m_rising->setColor(rising);
    changed |= m_falling->setColor(falling);
    changed |= m_forming->setColor(formingColor);
    m_sourceKey = sourceKey;
    m_hasSourceKey = true;

    m_frameRendererAllocations += geometryAllocationCount() - allocationsBefore;
    if (changed) {
        ++m_uploadCount;
        ++m_frameUploads;
        ++m_uploadedRevision;
    }
}

int BarChartNode::geometryAllocationCount() const {
    return m_rising->allocationCount() + m_falling->allocationCount() +
           m_forming->allocationCount();
}

void BarChartNode::captureProbeVertices(const BarVertex* vertices, size_t count) {
    if (!m_probeCaptureEnabled) {
        return;
    }
    m_lastUploadedVertices.clear();
    if (vertices != nullptr && count > 0) {
        m_lastUploadedVertices.assign(vertices, vertices + count);
    }
}

void BarChartNode::collectVertices(std::vector<BarVertex>& out) const {
    out.clear();
    const BucketGeometryNode* buckets[] = {m_rising, m_falling, m_forming};
    for (const BucketGeometryNode* bucket : buckets) {
        const QSGGeometry* geom = bucket->geometry();
        if (geom == nullptr || geom->vertexCount() <= 0) {
            continue;
        }
        const QSGGeometry::Point2D* points = geom->vertexDataAsPoint2D();
        const int count = geom->vertexCount();
        const size_t base = out.size();
        out.resize(base + static_cast<size_t>(count));
        for (int i = 0; i < count; ++i) {
            out[base + static_cast<size_t>(i)] = BarVertex{points[i].x, points[i].y, 0.0f,
                                                           0.0f};
        }
    }
}

void BarChartNode::resetForTests() {
    m_uploadedRevision = 0;
    m_completedRevision = 0;
    m_formingRevision = 0;
    m_hasSourceKey = false;
    m_uploadCount = 0;
    m_risingCount = 0;
    m_fallingCount = 0;
    m_formingCount = 0;
    m_frameUploads = 0;
    m_completedLayerUpdateCount = 0;
    m_formingLayerUpdateCount = 0;
    m_frameCompletedVertices = 0;
    m_frameFormingVertices = 0;
    m_frameRendererAllocations = 0;
    m_lastUploadedVertices.clear();
    m_rising->syncVertices(nullptr, 0, QColor(0, 180, 0));
    m_falling->syncVertices(nullptr, 0, QColor(220, 0, 0));
    m_forming->syncVertices(nullptr, 0, QColor(255, 165, 0));
}

int BarChartNode::takeFrameUploads() {
    const int uploads = m_frameUploads;
    m_frameUploads = 0;
    return uploads;
}

BarChartFrameStats BarChartNode::takeFrameStats() {
    BarChartFrameStats stats;
    stats.syncs = takeFrameUploads();
    stats.completedVertices = m_frameCompletedVertices;
    stats.formingVertices = m_frameFormingVertices;
    stats.rendererAllocations = m_frameRendererAllocations;
    m_frameCompletedVertices = 0;
    m_frameFormingVertices = 0;
    m_frameRendererAllocations = 0;
    return stats;
}
