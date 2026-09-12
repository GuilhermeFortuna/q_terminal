#include "render_backend.h"
#include <QtCore/QCoreApplication>
#include <QtCore/QDebug>
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

void setup_window(QQmlApplicationEngine& engine) {
    for (QObject* obj : engine.rootObjects()) {
        if (auto* window = qobject_cast<QQuickWindow*>(obj)) {
            auto update_backend = [window]() {
                QSGRendererInterface* rif = window->rendererInterface();
                QSGRendererInterface::GraphicsApi api = rif ? rif->graphicsApi() : window->graphicsApi();
                QString backend = QString::fromUtf8(get_graphics_api_name(api));
                std::cout << "render_backend: " << backend.toStdString() << std::endl;
                for (QObject* child : window->children()) {
                    if (child->inherits("AppInfo") || child->objectName() == QStringLiteral("appInfo")) {
                        child->setProperty("render_backend", backend);
                    }
                }
            };
            QObject::connect(window, &QQuickWindow::sceneGraphInitialized, update_backend);
            QObject::connect(window, &QQuickWindow::afterRendering, update_backend);
            update_backend();
        }
    }
}

QString query_graphics_api(QQmlApplicationEngine& engine) {
    for (QObject* obj : engine.rootObjects()) {
        if (auto* window = qobject_cast<QQuickWindow*>(obj)) {
            QSGRendererInterface* rif = window->rendererInterface();
            QSGRendererInterface::GraphicsApi api = rif ? rif->graphicsApi() : window->graphicsApi();
            return QString::fromUtf8(get_graphics_api_name(api));
        }
    }
    return QStringLiteral("unknown");
}
