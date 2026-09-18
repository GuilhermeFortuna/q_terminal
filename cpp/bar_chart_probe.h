#pragma once

#include "bar_vertex.h"
#include "q-qt/src/bar_series.cxxqt.h"

#include <vector>

class BarChartItem;
class BarSeries;

struct InternalProbeState {
    long long uploadedRevision = -1;
    int uploadCount = 0;
    int geometryAllocations = 0;
};

struct InternalProbeResult {
    std::vector<BarVertex> vertices;
    long long revision = 0;
    int uploads = 0;
    int geometryAllocations = 0;
    int risingVertexCount = 0;
    int fallingVertexCount = 0;
    int formingVertexCount = 0;
    int paintNodeCallCount = 0;
    int updateRequestCount = 0;
};

InternalProbeResult probe_sync(BarSeries* series, int firstBar, int lastBar, double low,
                               double high, float widthPx, float heightPx, InternalProbeState* state);

InternalProbeResult probe_item_paint(BarChartItem* item, int frames);

BarSeries* probe_make_test_series(int barCount);

void reset_probe_node();
