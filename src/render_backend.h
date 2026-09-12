#pragma once
#include <QtCore/QString>
#include <QtQml/QQmlApplicationEngine>

void setup_window(QQmlApplicationEngine& engine);
QString query_graphics_api(QQmlApplicationEngine& engine);
