#include "table_bridge.h"

#include <QtCore/QStringList>

TableModel* make_table_model() {
    return new TableModel();
}

void delete_table_model(TableModel* model) {
    delete model;
}

void table_model_set_roles_csv(TableModel* model, rust::Str roles_csv) {
    if (!model) return;
    QString str = QString::fromUtf8(roles_csv.data(), static_cast<int>(roles_csv.size()));
    QStringList parts = str.split(QLatin1Char(','), Qt::SkipEmptyParts);
    QList<QByteArray> roles;
    roles.reserve(parts.size());
    for (const QString& part : parts) {
        roles.append(part.trimmed().toUtf8());
    }
    model->setRoles(roles);
}

void table_model_reset_json(TableModel* model, rust::Str json_array) {
    if (!model) return;
    QByteArray bytes(json_array.data(), static_cast<int>(json_array.size()));
    QJsonDocument doc = QJsonDocument::fromJson(bytes);
    if (!doc.isArray()) {
        model->clear();
        return;
    }
    QJsonArray array = doc.array();
    QList<QVariantMap> rows;
    rows.reserve(array.size());
    for (const auto& val : array) {
        if (val.isObject()) {
            rows.append(val.toObject().toVariantMap());
        }
    }
    model->resetRows(rows);
}

void table_model_append_json(TableModel* model, rust::Str json_array) {
    if (!model) return;
    QByteArray bytes(json_array.data(), static_cast<int>(json_array.size()));
    QJsonDocument doc = QJsonDocument::fromJson(bytes);
    if (!doc.isArray()) return;
    QJsonArray array = doc.array();
    QList<QVariantMap> rows;
    rows.reserve(array.size());
    for (const auto& val : array) {
        if (val.isObject()) {
            rows.append(val.toObject().toVariantMap());
        }
    }
    model->appendRows(rows);
}

int table_model_count(TableModel* model) {
    return model ? model->count() : 0;
}

void table_model_clear(TableModel* model) {
    if (model) model->clear();
}

QVariant table_model_to_variant(TableModel* model) {
    if (!model) return QVariant();
    return QVariant::fromValue(static_cast<QObject*>(model));
}
