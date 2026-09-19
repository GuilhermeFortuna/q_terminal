#pragma once

#include <QtQml/QQmlApplicationEngine>

struct FrameBenchReport {
    double percentile50Ms = 0.0;
    double percentile95Ms = 0.0;
    double percentile99Ms = 0.0;
    double maxMs = 0.0;
    double avgUploadsPerFrame = 0.0;
    int frameCount = 0;
};

void run_frame_bench(QQmlApplicationEngine& engine, int visible_buckets, int bar_count,
                     int duration_ms, int execution_rows);
