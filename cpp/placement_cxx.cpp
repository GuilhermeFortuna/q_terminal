#include "placement_cxx.h"

#include <QtCore/QByteArray>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtGui/QGuiApplication>
#include <QtGui/QScreen>
#include <QtGui/QWindow>

QString placement_mode_name() {
    const QByteArray platform = qgetenv("QT_QPA_PLATFORM").toLower();
    const bool wayland = qEnvironmentVariableIsSet("WAYLAND_DISPLAY")
        && !platform.contains("xcb") && !platform.contains("offscreen");
    if (wayland) {
        return QStringLiteral("compositor");
    }
    if (platform.contains("offscreen") || platform.contains("xcb") || platform.isEmpty()) {
        return QStringLiteral("direct");
    }
    return QStringLiteral("direct");
}

QString displays_json() {
    QJsonArray arr;
    QScreen* primary = QGuiApplication::primaryScreen();
    for (QScreen* screen : QGuiApplication::screens()) {
        QJsonObject entry;
        const QRect geom = screen->geometry();
        const bool isPrimary = screen == primary;
        QJsonObject fp;
        fp.insert(QStringLiteral("name"), screen->name());
        fp.insert(QStringLiteral("manufacturer"), screen->manufacturer());
        fp.insert(QStringLiteral("model"), screen->model());
        const QString serial = screen->serialNumber();
        if (serial.isEmpty()) {
            fp.insert(QStringLiteral("serial"), QJsonValue());
        } else {
            fp.insert(QStringLiteral("serial"), serial);
        }
        QJsonObject g;
        g.insert(QStringLiteral("x"), geom.x());
        g.insert(QStringLiteral("y"), geom.y());
        g.insert(QStringLiteral("width"), geom.width());
        g.insert(QStringLiteral("height"), geom.height());
        fp.insert(QStringLiteral("geometry"), g);
        fp.insert(QStringLiteral("device_pixel_ratio"), screen->devicePixelRatio());
        fp.insert(QStringLiteral("primary"), isPrimary);
        entry.insert(QStringLiteral("fingerprint"), fp);
        entry.insert(QStringLiteral("primary"), isPrimary);
        arr.append(entry);
    }
    return QString::fromUtf8(QJsonDocument(arr).toJson(QJsonDocument::Compact));
}

void apply_window_placement(
    const QString& title,
    const QString& app_id,
    int x,
    int y,
    int width,
    int height,
    bool direct) {
    for (QWindow* window : QGuiApplication::topLevelWindows()) {
        if (!window) continue;
        const QString current = window->title();
        if (!current.contains(title) && !title.contains(current)) {
            continue;
        }
        window->setTitle(title);
        if (direct) {
            window->setX(x);
            window->setY(y);
            window->resize(width, height);
        }
        // Under Wayland the compositor places by class/title rules; identity is enough.
        window->setProperty("_q_platform_window_class", app_id);
    }
}
