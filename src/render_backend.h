#pragma once
#include <QtCore/QString>
#include <QtQml/QQmlApplicationEngine>

void setup_window(QQmlApplicationEngine& engine);
QString query_graphics_api(QQmlApplicationEngine& engine);
bool capture_window(QQmlApplicationEngine& engine, const QString& output_path, int width, int height);
bool verify_design_fonts();
