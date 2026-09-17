#include "bar_chart_node.h"

#include <QtQuick/QSGFlatColorMaterial>
#include <QtQuick/QSGGeometry>

namespace {

void copyPoint2D(QSGGeometry* geometry, const BarVertex* vertices, int count) {
    QSGGeometry::Point2D* points = geometry->vertexDataAsPoint2D();
    for (int i = 0; i < count; ++i) {
        points[i].set(vertices[i].x, vertices[i].y);
    }
    geometry->markVertexDataDirty();
}

} // namespace

BucketGeometryNode::BucketGeometryNode() {
    setMaterial(new QSGFlatColorMaterial());
    setFlag(QSGNode::OwnedByParent, true);
}

void BucketGeometryNode::syncVertices(const BarVertex* vertices, int count,
                                      const QColor& color) {
    if (count <= 0) {
        if (geometry() != nullptr) {
            delete geometry();
            setGeometry(nullptr);
        }
        markDirty(QSGNode::DirtyGeometry);
        return;
    }

    if (geometry() == nullptr || geometry()->vertexCount() != count) {
        delete geometry();
        auto* geom =
            new QSGGeometry(QSGGeometry::defaultAttributes_Point2D(), count);
        geom->setDrawingMode(QSGGeometry::DrawTriangles);
        setGeometry(geom);
        m_allocationCount++;
    }

    copyPoint2D(geometry(), vertices, count);
    static_cast<QSGFlatColorMaterial*>(material())->setColor(color);
    markDirty(QSGNode::DirtyGeometry | QSGNode::DirtyMaterial);
}

BarChartNode::BarChartNode() {
    m_rising = new BucketGeometryNode();
    m_falling = new BucketGeometryNode();
    m_forming = new BucketGeometryNode();
    appendChildNode(m_rising);
    appendChildNode(m_falling);
    appendChildNode(m_forming);
}

void BarChartNode::sync(const BarVertexView& view, const QColor& rising,
                        const QColor& falling, const QColor& forming) {
    if (view.revision == m_uploadedRevision) {
        return;
    }

    m_uploadCount++;
    m_frameUploads++;
    m_uploadedRevision = view.revision;

    std::vector<BarVertex> risingVerts;
    std::vector<BarVertex> fallingVerts;
    std::vector<BarVertex> formingVerts;

    m_lastUploadedVertices.clear();
    if (view.data != nullptr && view.count > 0) {
        m_lastUploadedVertices.assign(view.data, view.data + view.count);
        risingVerts.reserve(view.count);
        fallingVerts.reserve(view.count);
        formingVerts.reserve(view.count);

        for (size_t i = 0; i < view.count; ++i) {
            const BarVertex& vertex = view.data[i];
            if (vertex.forming >= 0.5f) {
                formingVerts.push_back(vertex);
            } else if (vertex.direction >= 0.0f) {
                risingVerts.push_back(vertex);
            } else {
                fallingVerts.push_back(vertex);
            }
        }
    }

    m_risingCount = static_cast<int>(risingVerts.size());
    m_fallingCount = static_cast<int>(fallingVerts.size());
    m_formingCount = static_cast<int>(formingVerts.size());

    m_rising->syncVertices(risingVerts.data(), m_risingCount, rising);
    m_falling->syncVertices(fallingVerts.data(), m_fallingCount, falling);
    m_forming->syncVertices(formingVerts.data(), m_formingCount, forming);
}

int BarChartNode::geometryAllocationCount() const {
    return m_rising->allocationCount() + m_falling->allocationCount() +
           m_forming->allocationCount();
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
    m_uploadedRevision = -1;
    m_uploadCount = 0;
    m_risingCount = 0;
    m_fallingCount = 0;
    m_formingCount = 0;
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
