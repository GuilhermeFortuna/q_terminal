#ifndef Q_TERMINAL_PLACEMENT_CXX_H
#define Q_TERMINAL_PLACEMENT_CXX_H

#include <QtCore/QString>

#include "rust/cxx.h"

QString placement_mode_name();
QString displays_json();
void apply_window_placement(
    const QString& title,
    const QString& app_id,
    int x,
    int y,
    int width,
    int height,
    bool direct);

#endif
