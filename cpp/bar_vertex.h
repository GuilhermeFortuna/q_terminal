#pragma once

#include <cstddef>

struct BarVertex;

struct BarVertexView {
    const BarVertex* data;
    size_t count;
    long long revision;
};
