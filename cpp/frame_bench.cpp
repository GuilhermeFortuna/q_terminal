#include "frame_bench.h"

#include "bar_chart_item.h"
#include "overlay_chart_item.h"
#include "q_terminal/src/bar_feed.cxxqt.h"

#include <algorithm>
#include <iostream>
#include <vector>

#include <QtCore/QElapsedTimer>
#include <QtCore/QMetaObject>
#include <QtGui/QGuiApplication>
#include <QtCore/QTimer>
#include <QtQuick/QQuickWindow>
#include <QtQuick/QSGRendererInterface>
#include "q-qt/src/bar_series.cxxqt.h"

#include "table_model.h"
#include "q_terminal/src/execution_models.cxxqt.h"

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
    std::vector<int> uploads;
    QElapsedTimer timer;
};

void print_stats(const WindowStats& stats, int visible_buckets, int execution_rows, int markers,
                 int overlays) {
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
    double upload_sum = 0.0;
    for (int uploads : stats.uploads) {
        upload_sum += static_cast<double>(uploads);
    }
    const double avg_uploads =
        stats.uploads.empty() ? 0.0 : upload_sum / static_cast<double>(stats.uploads.size());
    QSGRendererInterface* rif = stats.window->rendererInterface();
    const QSGRendererInterface::GraphicsApi api =
        rif ? rif->graphicsApi() : stats.window->graphicsApi();
    std::cout << "frame_bench window=\"" << name.toStdString() << "\" buckets=" << visible_buckets
              << " execution_rows=" << execution_rows << " markers=" << markers
              << " overlays=" << overlays << " frames=" << sorted.size() << " p50_ms=" << pct(0.50)
              << " p95_ms=" << pct(0.95) << " p99_ms=" << pct(0.99) << " max_ms=" << sorted.back()
              << " avg_uploads_per_frame=" << avg_uploads
              << " graphics_api=" << graphics_api_name(api) << std::endl;
}

}  // namespace

void run_frame_bench(QQmlApplicationEngine& engine, int visible_buckets, int bar_count,
                     int duration_ms, int execution_rows, int markers, int overlays) {
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

    BarChartItem* chart = root ? root->findChild<BarChartItem*>("benchChart") : nullptr;
    if (chart == nullptr && root != nullptr) {
        chart = root->findChild<BarChartItem*>();
    }
    QQuickWindow* window = chart ? chart->window() : nullptr;
    BarSeries* series = nullptr;
    if (chart != nullptr) {
        if (auto* s = qobject_cast<BarSeries*>(chart->series())) {
            series = s;
        } else {
            series = new BarSeries(chart);
            chart->setSeries(series);
        }
    }

    if (window == nullptr || chart == nullptr || series == nullptr) {
        std::cerr << "frame bench: chart scene not found" << std::endl;
        return;
    }
    series->load_history_sample(bar_count);
    chart->setFirstBar(0);
    chart->setLastBar(visible_buckets);
    chart->setLowPrice(series->getLow());
    chart->setHighPrice(series->getHigh());

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
    if (markers > 0 || overlays > 0) {
        // Markers and overlays sit on their own feed and item, over the bar chart.
        overlayFeed = new BarFeed(window);
        overlayFeed->bench_populate(visible_buckets, markers, overlays);
        auto* item = new OverlayChartItem(chart->parentItem());
        item->setParentItem(chart->parentItem());
        item->setSize(chart->size());
        QObject::connect(chart, &QQuickItem::widthChanged, item, [chart, item]() { item->setWidth(chart->width()); });
        QObject::connect(chart, &QQuickItem::heightChanged, item, [chart, item]() { item->setHeight(chart->height()); });
        item->setSeries(overlayFeed);
        item->setFirstBar(0);
        item->setLastBar(visible_buckets);
        item->setLowPrice(overlayFeed->getLow() - 1.0);
        item->setHighPrice(overlayFeed->getHigh() + 1.0);
    }

    // One frame series per visible window: a second window costs a second render loop.
    auto* all = new std::vector<WindowStats*>();
    for (QWindow* w : QGuiApplication::topLevelWindows()) {
        auto* quick = qobject_cast<QQuickWindow*>(w);
        if (quick == nullptr || !quick->isVisible()) {
            continue;
        }
        auto* stats = new WindowStats{quick, {}, {}, {}};
        stats->timer.start();
        all->push_back(stats);
        QObject::connect(quick, &QQuickWindow::afterRendering, quick, [stats, chart]() {
            stats->frame_ms.push_back(static_cast<double>(stats->timer.nsecsElapsed()) / 1'000'000.0);
            stats->uploads.push_back(stats->window == chart->window() ? chart->takeFrameUploads() : 0);
            stats->timer.restart();
        });
    }

    QObject::connect(window, &QQuickWindow::beforeRendering, window, [series, overlayFeed]() {
        if (overlayFeed != nullptr) {
            // Worst case: the overlay buffers are rebuilt and uploaded every frame.
            overlayFeed->setOverlay_revision(overlayFeed->getOverlay_revision() + 1);
        }
        series->ingest_forming_bar(series->getLast_time() + 60, series->getLast_price(),
                                   series->getLast_price() + 1.0, series->getLast_price() - 1.0,
                                   series->getLast_price() + 0.5, 1.0);
    });

    QTimer* stopTimer = new QTimer(window);
    stopTimer->setSingleShot(true);
    QObject::connect(stopTimer, &QTimer::timeout, window,
                     [all, visible_buckets, execution_rows, markers, overlays]() {
                         for (WindowStats* stats : *all) {
                             print_stats(*stats, visible_buckets, execution_rows, markers, overlays);
                         }
                         const auto windows = QGuiApplication::topLevelWindows();
                         for (QWindow* w : windows) {
                             w->close();
                         }
                     });
    int dur = duration_ms > 0 ? duration_ms : 5000;
    stopTimer->start(dur);
}
