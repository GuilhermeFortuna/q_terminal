#include "frame_bench.h"

#include "bar_chart_item.h"
#include "bar_chart_node.h"
#include "overlay_chart_item.h"
#include "q_terminal/src/bar_feed.cxxqt.h"

#include <algorithm>
#include <cmath>
#include <iostream>
#include <numeric>
#include <string>
#include <vector>

#include <QtCore/QElapsedTimer>
#include <QtCore/QMetaObject>
#include <QtGui/QGuiApplication>
#include <QtCore/QTimer>
#include <QtCore/QSysInfo>
#include <QtGui/QScreen>
#include <QtQuick/QQuickWindow>
#include <QtQuick/QSGRendererInterface>
#include "q-qt/src/bar_series.cxxqt.h"

#include "table_model.h"
#include "q_terminal/src/execution_models.cxxqt.h"

extern "C" void q_terminal_bench_allocations_start();
extern "C" unsigned long long q_terminal_bench_allocations_stop();

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

namespace {

struct WindowStats {
    QQuickWindow* window;
    std::vector<double> frame_ms;
    std::vector<BarChartFrameStats> frame_stats;
    QElapsedTimer timer;
    QElapsedTimer runtime;
    bool chart_window = false;
};

void print_stats(const WindowStats& stats, int visible_buckets, int execution_rows, int markers,
                 int overlays, const QString& scenario, unsigned long long rust_allocations) {
    const QString name = stats.window->title();
    if (stats.frame_ms.empty()) {
        std::cout << "frame_bench window=\"" << name.toStdString() << "\" buckets="
                  << visible_buckets << " execution_rows=" << execution_rows << " frames=0"
                  << std::endl;
        return;
    }
    std::vector<double> sorted = stats.frame_ms;
    std::sort(sorted.begin(), sorted.end());
    auto pct = [&](double q) {
        return sorted[static_cast<std::size_t>(q * static_cast<double>(sorted.size() - 1))];
    };
    BarChartFrameStats totals;
    long long max_forming_vertices = 0;
    for (const auto& sample : stats.frame_stats) {
        totals.syncs += sample.syncs;
        totals.completedVertices += sample.completedVertices;
        totals.formingVertices += sample.formingVertices;
        totals.rendererAllocations += sample.rendererAllocations;
        totals.geometryPrepNs += sample.geometryPrepNs;
        max_forming_vertices = std::max(max_forming_vertices, sample.formingVertices);
    }
    const double frames = static_cast<double>(stats.frame_stats.size());
    QSGRendererInterface* rif = stats.window->rendererInterface();
    const QSGRendererInterface::GraphicsApi api =
        rif ? rif->graphicsApi() : stats.window->graphicsApi();
    const QSize window_size = stats.window->size();
    std::cout << "frame_bench window=\"" << name.toStdString() << "\" scenario="
              << scenario.toStdString() << " buckets=" << visible_buckets
              << " execution_rows=" << execution_rows << " markers=" << markers
              << " overlays=" << overlays << " frames=" << sorted.size() << " p50_ms=" << pct(0.50)
              << " p95_ms=" << pct(0.95) << " p99_ms=" << pct(0.99) << " max_ms=" << sorted.back()
              << " scene_syncs=" << totals.syncs
              << " avg_completed_vertices_per_frame=" << totals.completedVertices / frames
              << " avg_forming_vertices_per_frame=" << totals.formingVertices / frames
              << " avg_renderer_allocations_per_frame=" << totals.rendererAllocations / frames
              << " avg_geometry_prep_us=" << totals.geometryPrepNs / (frames * 1000.0)
              << " forming_visible=" << (max_forming_vertices > 0 ? "true" : "false")
              << " rust_process_allocations=" << rust_allocations
              << " avg_rust_process_allocations_per_frame=" << rust_allocations / frames
              << " gpu_transfers=not_measured"
              << " graphics_api=" << graphics_api_name(api)
              << " window_size=" << window_size.width() << "x" << window_size.height()
              << " machine=\"" << QSysInfo::machineHostName().toStdString() << "\""
              << " os=\"" << QSysInfo::prettyProductName().toStdString() << "\"" << std::endl;
}

}  // namespace

void run_frame_bench(QQmlApplicationEngine& engine, int visible_buckets, int bar_count,
                     int duration_ms, int execution_rows, int markers, int overlays,
                     const QString& scenario) {
    // The shell root is not a window. With Q_BENCH_WINDOWS=both both compositions are
    // opened, so the chart and the tables render in separate windows.
    QObject* root = engine.rootObjects().isEmpty() ? nullptr : engine.rootObjects().first();
    if (root != nullptr && qEnvironmentVariable("Q_BENCH_WINDOWS") == QStringLiteral("both")) {
        QVariant ignored;
        QMetaObject::invokeMethod(root, "openWindow", Q_RETURN_ARG(QVariant, ignored),
                                  Q_ARG(QVariant, QStringLiteral("market")));
        QMetaObject::invokeMethod(root, "openWindow", Q_RETURN_ARG(QVariant, ignored),
                                  Q_ARG(QVariant, QStringLiteral("operations")));
    }

    BarChartItem* chart_template = root ? root->findChild<BarChartItem*>("benchChart") : nullptr;
    if (chart_template == nullptr && root != nullptr) {
        chart_template = root->findChild<BarChartItem*>();
    }
    QQuickWindow* window = chart_template ? chart_template->window() : nullptr;
    if (window == nullptr) {
        std::cerr << "frame bench: chart scene not found" << std::endl;
        return;
    }
    // Use the same renderer item in a direct scene-graph host. ChartPane's production
    // visibility depends on the live BarFeed, which the synthetic benchmark does not own.
    auto* chart = new BarChartItem(window->contentItem());
    chart->setObjectName(QStringLiteral("benchChartRenderer"));
    chart->setSize(QSizeF(window->size()));
    chart->setPosition(QPointF(0.0, 0.0));
    chart->setZ(1000.0);
    chart->setVisible(true);
    window->contentItem()->setVisible(true);
    auto* series = new BarSeries(chart);
    chart->setSeries(series);
    const int safe_bars = std::max(1, bar_count);
    const int safe_buckets = std::max(1, visible_buckets);
    const bool live_edge = scenario != QStringLiteral("historical");
    if (scenario != QStringLiteral("historical") && scenario != QStringLiteral("live-edge") &&
        scenario != QStringLiteral("pan-zoom") && scenario != QStringLiteral("completion") &&
        scenario != QStringLiteral("price-expansion")) {
        std::cerr << "frame bench: unknown scenario '" << scenario.toStdString() << "'" << std::endl;
        return;
    }
    series->load_history_sample(safe_bars);
    const int first_bar = live_edge ? std::max(0, safe_bars - safe_buckets) : 0;
    const int last_bar = live_edge ? safe_bars + 1 : safe_buckets;
    chart->setFirstBar(first_bar);
    chart->setLastBar(last_bar);
    double low_price = series->getLow();
    double high_price = series->getHigh();
    if (live_edge) {
        const double last_price = series->getLast_price();
        low_price = last_price - 8.0;
        high_price = last_price + 8.0;
    }
    chart->setLowPrice(low_price);
    chart->setHighPrice(high_price);

    const qint64 forming_time = series->getLast_time() + 60;
    const double forming_open = series->getLast_price();
    series->ingest_forming_bar(forming_time, forming_open, forming_open + 1.0,
                               forming_open - 1.0, forming_open + 0.25, 1.0);

    if (execution_rows > 0) {
        auto* models = root->findChild<ExecutionModels*>("executionModels");
        if (models == nullptr) {
            models = root->findChild<ExecutionModels*>();
        }
        if (models != nullptr) {
            QObject* depObj = qvariant_cast<QObject*>(models->getDeployments());
            if (auto* depModel = qobject_cast<TableModel*>(depObj)) {
                QList<QVariantMap> depRows;
                QVariantMap dep;
                dep[QStringLiteral("id")] = QStringLiteral("dep-bench-01");
                dep[QStringLiteral("name")] = QStringLiteral("Bench Strategy");
                dep[QStringLiteral("symbol")] = QStringLiteral("BTCUSD");
                dep[QStringLiteral("timeframe")] = QStringLiteral("1m");
                dep[QStringLiteral("broker_mode")] = QStringLiteral("paper");
                dep[QStringLiteral("lifecycle")] = QStringLiteral("running");
                dep[QStringLiteral("pos_quantity")] = QStringLiteral("1.50");
                dep[QStringLiteral("pos_side")] = QStringLiteral("long");
                dep[QStringLiteral("pos_entry_price")] = QStringLiteral("65000.00");
                dep[QStringLiteral("pos_mark_price")] = QStringLiteral("65100.00");
                dep[QStringLiteral("pos_unrealized_pnl")] = QStringLiteral("150.00");
                depRows.append(dep);
                depModel->resetRows(depRows);
            }
            models->setSelected_deployment_id(QStringLiteral("dep-bench-01"));

            QObject* ordersObj = qvariant_cast<QObject*>(models->getOrders());
            if (auto* ordersModel = qobject_cast<TableModel*>(ordersObj)) {
                QList<QVariantMap> orderRows;
                orderRows.reserve(execution_rows);
                for (int i = 0; i < execution_rows; ++i) {
                    QVariantMap ord;
                    ord[QStringLiteral("id")] = QStringLiteral("ord-%1").arg(i);
                    ord[QStringLiteral("created_at")] = QStringLiteral("2026-09-18T10:00:00Z");
                    ord[QStringLiteral("intent_id")] = QStringLiteral("int-%1").arg(i);
                    ord[QStringLiteral("side")] = (i % 2 == 0) ? QStringLiteral("buy") : QStringLiteral("sell");
                    ord[QStringLiteral("order_type")] = QStringLiteral("limit");
                    ord[QStringLiteral("quantity")] = QStringLiteral("100.00");
                    ord[QStringLiteral("status")] = QStringLiteral("filled");
                    ord[QStringLiteral("reconciliation_state")] = QStringLiteral("reconciled");
                    orderRows.append(ord);
                }
                ordersModel->resetRows(orderRows);
            }
        }
    }

    BarFeed* overlayFeed = nullptr;
    OverlayChartItem* overlay_item = nullptr;
    if (markers > 0 || overlays > 0) {
        // Markers and overlays sit on their own feed and item, over the bar chart.
        overlayFeed = new BarFeed(window);
        overlayFeed->bench_populate(live_edge ? safe_bars : safe_buckets, markers, overlays);
        overlay_item = new OverlayChartItem(chart->parentItem());
        overlay_item->setParentItem(chart->parentItem());
        overlay_item->setSize(chart->size());
        QObject::connect(chart, &QQuickItem::widthChanged, overlay_item, [chart, overlay_item]() { overlay_item->setWidth(chart->width()); });
        QObject::connect(chart, &QQuickItem::heightChanged, overlay_item, [chart, overlay_item]() { overlay_item->setHeight(chart->height()); });
        overlay_item->setSeries(overlayFeed);
        overlay_item->setFirstBar(first_bar);
        overlay_item->setLastBar(last_bar);
        overlay_item->setLowPrice(overlayFeed->getLow() - 1.0);
        overlay_item->setHighPrice(overlayFeed->getHigh() + 1.0);
    }

    // One frame series per visible window: a second window costs a second render loop.
    auto* all = new std::vector<WindowStats*>();
    auto* allocation_started = new bool(false);
    for (QWindow* w : QGuiApplication::topLevelWindows()) {
        auto* quick = qobject_cast<QQuickWindow*>(w);
        if (quick == nullptr || !quick->isVisible()) {
            continue;
        }
        auto* stats = new WindowStats{quick, {}, {}, {}, {}, quick == window};
        stats->timer.start();
        stats->runtime.start();
        all->push_back(stats);
        QObject::connect(quick, &QQuickWindow::afterRendering, quick,
                         [stats, chart, allocation_started]() {
            const double frame_ms = static_cast<double>(stats->timer.nsecsElapsed()) / 1'000'000.0;
            const BarChartFrameStats frame_stats = stats->chart_window ? chart->takeFrameStats() : BarChartFrameStats{};
            // Ignore startup and initial buffer allocations; count only warmed-up frames.
            if (stats->runtime.elapsed() >= 1000) {
                if (!*allocation_started) {
                    q_terminal_bench_allocations_start();
                    *allocation_started = true;
                }
                stats->frame_ms.push_back(frame_ms);
                stats->frame_stats.push_back(frame_stats);
            }
            stats->timer.restart();
        });
    }

    auto* tick = new int(0);
    auto* current_forming_time = new qint64(forming_time);
    QObject::connect(window, &QQuickWindow::beforeRendering, window,
                     [series, overlayFeed, overlay_item, chart, tick, scenario, forming_open,
                      current_forming_time, safe_bars, safe_buckets, low_price, high_price]() mutable {
        ++*tick;
        if (overlayFeed != nullptr) {
            // Worst case: the overlay buffers are rebuilt and uploaded every frame.
            overlayFeed->setOverlay_revision(overlayFeed->getOverlay_revision() + 1);
        }
        const double delta = std::sin(static_cast<double>(*tick) * 0.07) * 0.7;
        const double close = forming_open + delta;
        qint64 time = *current_forming_time;
        double high = std::max(forming_open, close) + 1.0;
        double low = std::min(forming_open, close) - 1.0;
        if (scenario == QStringLiteral("historical")) {
            time = series->getLast_time() + 60;
        } else if (scenario == QStringLiteral("price-expansion")) {
            high += static_cast<double>(*tick) * 0.002;
            chart->setHighPrice(high_price + static_cast<double>(*tick) * 0.002);
        } else if (scenario == QStringLiteral("pan-zoom")) {
            const int span = std::max(2, safe_buckets - ((*tick / 90) % std::max(1, safe_buckets / 4)));
            const int first = std::max(0, safe_bars - span + ((*tick / 30) % 15));
            chart->setFirstBar(first);
            chart->setLastBar(std::min(safe_bars + 1, first + span));
            chart->setLowPrice(low_price - (span % 7));
            chart->setHighPrice(high_price + (span % 7));
            if (overlay_item != nullptr) {
                // Keep the marker/overlay layer on the same scripted viewport.
                overlay_item->setFirstBar(chart->firstBar());
                overlay_item->setLastBar(chart->lastBar());
                overlay_item->setLowPrice(chart->lowPrice());
                overlay_item->setHighPrice(chart->highPrice());
            }
        } else if (scenario == QStringLiteral("completion") && *tick % 120 == 0) {
            series->ingest_completed_bar(time, forming_open, high, low, close, 1.0);
            *current_forming_time += 60;
            time = *current_forming_time;
        }
        series->ingest_forming_bar(time, forming_open, high, low, close, 1.0);
        chart->update();
    });

    QTimer* stopTimer = new QTimer(window);
    stopTimer->setSingleShot(true);
    QObject::connect(stopTimer, &QTimer::timeout, window,
                     [all, allocation_started, visible_buckets, execution_rows, markers, overlays,
                      scenario]() {
                         const unsigned long long rust_allocations =
                             *allocation_started ? q_terminal_bench_allocations_stop() : 0;
                         for (WindowStats* stats : *all) {
                             print_stats(*stats, visible_buckets, execution_rows, markers, overlays,
                                         scenario, rust_allocations);
                         }
                         const auto windows = QGuiApplication::topLevelWindows();
                         for (QWindow* w : windows) {
                             w->close();
                         }
                     });
    int dur = duration_ms > 0 ? duration_ms : 5000;
    stopTimer->start(dur);
}
