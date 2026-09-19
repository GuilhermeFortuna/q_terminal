#ifndef Q_TERMINAL_TABLE_BRIDGE_H
#define Q_TERMINAL_TABLE_BRIDGE_H

#include <QtCore/QByteArray>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QVariant>
#include "cxx-qt-lib/qvariant.h"
#include "rust/cxx.h"
#include "table_model.h"

class QQmlApplicationEngine;

TableModel* make_table_model();
void delete_table_model(TableModel* model);

void table_model_set_roles_csv(TableModel* model, rust::Str roles_csv);
void table_model_reset_json(TableModel* model, rust::Str json_array);
void table_model_append_json(TableModel* model, rust::Str json_array);
int table_model_count(TableModel* model);
void table_model_clear(TableModel* model);

QVariant table_model_to_variant(TableModel* model);

#endif // Q_TERMINAL_TABLE_BRIDGE_H
