#include "frame_bench.h"

#include "bar_chart_item.h"

#include <algorithm>
#include <iostream>
#include <vector>

#include <QtCore/QElapsedTimer>
#include <QtCore/QTimer>
#include <QtQuick/QQuickWindow>
#include <QtQuick/QSGRendererInterface>
#include "q-qt/src/bar_series.cxxqt.h"

namespace {

const char* graphics_api_name(QSGRendererInterface::GraphicsApi api) {
    switch (api) {
        case QSGRendererInterface::Software:
            return "Software";
        case QSGRendererInterface::OpenGL:
            return "OpenGL";
        case QSGRendererInterface::Vulkan:
            return "Vulkan";
        case QSGRendererInterface::Direct3D11:
            return "Direct3D11";
        case QSGRendererInterface::Direct3D12:
            return "Direct3D12";
        case QSGRendererInterface::Metal:
            return "Metal";
        case QSGRendererInterface::Null:
            return "Null";
        default:
            return "Unknown";
    }
}

} // namespace

void run_frame_bench(QQmlApplicationEngine& engine, int visible_buckets, int bar_count,
                     int duration_ms) {
    QQuickWindow* window = nullptr;
    for (QObject* root : engine.rootObjects()) {
        window = qobject_cast<QQuickWindow*>(root);
        if (window != nullptr) {
            break;
        }
    }

    BarChartItem* chart = window ? window->findChild<BarChartItem*>("benchChart") : nullptr;
    if (chart == nullptr && window != nullptr) {
        chart = window->findChild<BarChartItem*>();
    }
    BarSeries* series = chart ? qobject_cast<BarSeries*>(chart->series()) : nullptr;

    if (window == nullptr || chart == nullptr || series == nullptr) {
        std::cerr << "frame bench: chart scene not found" << std::endl;
        return;
    }

    series->load_history_sample(bar_count);
    chart->setFirstBar(0);
    chart->setLastBar(visible_buckets);
    chart->setLowPrice(series->getLow());
    chart->setHighPrice(series->getHigh());

    auto* frame_ms = new std::vector<double>();
    auto* uploads_per_frame = new std::vector<int>();
    auto* timer = new QElapsedTimer();
    timer->start();

    QObject::connect(window, &QQuickWindow::beforeRendering, window, [series]() {
        series->ingest_forming_bar(series->getLast_time() + 60, series->getLast_price(),
                                   series->getLast_price() + 1.0, series->getLast_price() - 1.0,
                                   series->getLast_price() + 0.5, 1.0);
    });

    QObject::connect(window, &QQuickWindow::afterRendering, window,
                     [window, chart, frame_ms, uploads_per_frame, timer, visible_buckets]() {
                         frame_ms->push_back(static_cast<double>(timer->nsecsElapsed()) /
                                             1'000'000.0);
                         uploads_per_frame->push_back(chart->takeFrameUploads());
                         timer->restart();
                     });

    QTimer* stopTimer = new QTimer(window);
    stopTimer->setSingleShot(true);
    QObject::connect(stopTimer, &QTimer::timeout, window,
                     [window, frame_ms, uploads_per_frame, visible_buckets]() {
                         if (frame_ms->empty()) {
                             std::cout << "frame_bench buckets=" << visible_buckets
                                       << " frames=0" << std::endl;
                             window->close();
                             return;
                         }

                         std::vector<double> sorted = *frame_ms;
                         std::sort(sorted.begin(), sorted.end());
                         const std::size_t idx =
                             static_cast<std::size_t>(0.95 * static_cast<double>(sorted.size() - 1));
                         const double p95 = sorted[idx];
                         const double max_ms = sorted.back();
                         double upload_sum = 0.0;
                         for (int uploads : *uploads_per_frame) {
                             upload_sum += static_cast<double>(uploads);
                         }
                         const double avg_uploads =
                             upload_sum / static_cast<double>(uploads_per_frame->size());

                         QSGRendererInterface* rif = window->rendererInterface();
                         const QSGRendererInterface::GraphicsApi api =
                             rif ? rif->graphicsApi() : window->graphicsApi();

                         std::cout << "frame_bench buckets=" << visible_buckets
                                   << " frames=" << sorted.size() << " p95_ms=" << p95
                                   << " max_ms=" << max_ms
                                   << " avg_uploads_per_frame=" << avg_uploads
                                   << " graphics_api=" << graphics_api_name(api) << std::endl;
                         window->close();
                     });
    stopTimer->start(duration_ms);
}
