#include "render_backend.h"
#include <QtCore/QCoreApplication>
#include <QtCore/QDebug>
#include <QtGui/QFontDatabase>
#include <QtGui/QGuiApplication>
#include <QtGui/QFontMetricsF>
#include <QtGui/QImage>
#include <QtQuick/QQuickWindow>
#include <QtQuick/QSGRendererInterface>
#include <iostream>

static const char* get_graphics_api_name(QSGRendererInterface::GraphicsApi api) {
    switch (api) {
        case QSGRendererInterface::Software: return "Software";
        case QSGRendererInterface::OpenGL: return "OpenGL";
        case QSGRendererInterface::Vulkan: return "Vulkan";
        case QSGRendererInterface::Direct3D11: return "Direct3D11";
        case QSGRendererInterface::Direct3D12: return "Direct3D12";
        case QSGRendererInterface::Metal: return "Metal";
        case QSGRendererInterface::Null: return "Null";
        default: return "Unknown";
    }
}

// The shell root is not a window: the top-level windows are created by it. The first one
// stands for the render backend, which every window of the process shares.
static QQuickWindow* first_window(QQmlApplicationEngine& engine) {
    for (QObject* obj : engine.rootObjects()) {
        if (auto* window = qobject_cast<QQuickWindow*>(obj)) {
            return window;
        }
    }
    for (QWindow* window : QGuiApplication::topLevelWindows()) {
        if (auto* quick = qobject_cast<QQuickWindow*>(window)) {
            return quick;
        }
    }
    return nullptr;
}

void setup_window(QQmlApplicationEngine& engine) {
    QQuickWindow* window = first_window(engine);
    if (!window) {
        return;
    }
    QObject* app_info = nullptr;
    for (QObject* obj : engine.rootObjects()) {
        app_info = obj->findChild<QObject*>(QStringLiteral("appInfo"));
        if (app_info) break;
    }
    auto update_backend = [window, app_info]() {
        QSGRendererInterface* rif = window->rendererInterface();
        QSGRendererInterface::GraphicsApi api = rif ? rif->graphicsApi() : window->graphicsApi();
        QString backend = QString::fromUtf8(get_graphics_api_name(api));
        std::cout << "render_backend: " << backend.toStdString() << std::endl;
        if (app_info) {
            app_info->setProperty("render_backend", backend);
        }
    };
    QObject::connect(window, &QQuickWindow::sceneGraphInitialized, update_backend);
    QObject::connect(window, &QQuickWindow::afterRendering, update_backend);
    update_backend();
}

QString query_graphics_api(QQmlApplicationEngine& engine) {
    if (QQuickWindow* window = first_window(engine)) {
        QSGRendererInterface* rif = window->rendererInterface();
        QSGRendererInterface::GraphicsApi api = rif ? rif->graphicsApi() : window->graphicsApi();
        return QString::fromUtf8(get_graphics_api_name(api));
    }
    return QStringLiteral("unknown");
}

bool verify_design_fonts() {
    const int inter_id = QFontDatabase::addApplicationFont(QStringLiteral(":/assets/fonts/Inter-Variable.ttf"));
    const int mono_id = QFontDatabase::addApplicationFont(QStringLiteral(":/assets/fonts/JetBrainsMono-Variable.ttf"));
    if (inter_id < 0 || mono_id < 0) return false;
    const QStringList inter_families = QFontDatabase::applicationFontFamilies(inter_id);
    const QStringList mono_families = QFontDatabase::applicationFontFamilies(mono_id);
    if (inter_families.isEmpty() || mono_families.isEmpty()) return false;
    QFont numeric(mono_families.first());
    numeric.setStyleStrategy(QFont::PreferNoShaping);
    const QFontMetricsF metrics(numeric);
    return qAbs(metrics.horizontalAdvance(QStringLiteral("111111"))
              - metrics.horizontalAdvance(QStringLiteral("908276"))) < 0.01;
}

bool capture_window(QQmlApplicationEngine& engine, const QString& output_path, int width, int height) {
    if (QQuickWindow* window = first_window(engine)) {
        window->setWidth(width);
        window->setHeight(height);
        window->show();
        for (int i = 0; i < 8; ++i) {
            QCoreApplication::processEvents(QEventLoop::AllEvents, 50);
            window->update();
        }
        const QImage image = window->grabWindow();
        return !image.isNull() && image.width() == width && image.height() == height
            && image.save(output_path, "PNG");
    }
    return false;
}
