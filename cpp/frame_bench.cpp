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

void run_frame_bench(QQmlApplicationEngine& engine, int visible_buckets, int bar_count,
                     int duration_ms, int execution_rows) {
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
        auto* models = window->findChild<ExecutionModels*>("executionModels");
        if (models == nullptr) {
            models = window->findChild<ExecutionModels*>();
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
                     [window, frame_ms, uploads_per_frame, visible_buckets, execution_rows]() {
                         if (frame_ms->empty()) {
                             std::cout << "frame_bench buckets=" << visible_buckets
                                       << " execution_rows=" << execution_rows
                                       << " frames=0" << std::endl;
                             window->close();
                             return;
                         }

                         std::vector<double> sorted = *frame_ms;
                         std::sort(sorted.begin(), sorted.end());
                         const std::size_t idx50 =
                             static_cast<std::size_t>(0.50 * static_cast<double>(sorted.size() - 1));
                         const std::size_t idx95 =
                             static_cast<std::size_t>(0.95 * static_cast<double>(sorted.size() - 1));
                         const std::size_t idx99 =
                             static_cast<std::size_t>(0.99 * static_cast<double>(sorted.size() - 1));
                         const double p50 = sorted[idx50];
                         const double p95 = sorted[idx95];
                         const double p99 = sorted[idx99];
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
                                   << " execution_rows=" << execution_rows
                                   << " frames=" << sorted.size()
                                   << " p50_ms=" << p50
                                   << " p95_ms=" << p95
                                   << " p99_ms=" << p99
                                   << " max_ms=" << max_ms
                                   << " avg_uploads_per_frame=" << avg_uploads
                                   << " graphics_api=" << graphics_api_name(api) << std::endl;
                         window->close();
                     });
    int dur = duration_ms > 0 ? duration_ms : 5000;
    stopTimer->start(dur);
}
