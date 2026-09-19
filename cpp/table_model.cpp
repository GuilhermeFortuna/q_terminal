#include "table_model.h"

#include <QtCore/QString>

TableModel::TableModel(QObject* parent) : QAbstractListModel(parent) {
    m_roles[Qt::UserRole] = "modelData";
}

int TableModel::rowCount(const QModelIndex& parent) const {
    if (parent.isValid()) {
        return 0;
    }
    return m_rows.size();
}

QVariant TableModel::data(const QModelIndex& index, int role) const {
    if (!index.isValid() || index.row() < 0 || index.row() >= m_rows.size()) {
        return QVariant();
    }
    const auto& row = m_rows.at(index.row());
    if (role == Qt::UserRole) {
        return row;
    }
    int roleIdx = role - (Qt::UserRole + 1);
    if (roleIdx >= 0 && roleIdx < m_roleNames.size()) {
        return row.value(QString::fromUtf8(m_roleNames.at(roleIdx)));
    }
    if (role == Qt::DisplayRole) {
        return row.value(QStringLiteral("id"));
    }
    return QVariant();
}

QHash<int, QByteArray> TableModel::roleNames() const {
    return m_roles;
}

void TableModel::setRoles(const QList<QByteArray>& roles) {
    m_roleNames = roles;
    m_roles.clear();
    m_roles[Qt::UserRole] = "modelData";
    for (int i = 0; i < roles.size(); ++i) {
        m_roles[Qt::UserRole + 1 + i] = roles.at(i);
    }
}

void TableModel::resetRows(const QList<QVariantMap>& rows) {
    beginResetModel();
    m_rows = rows;
    endResetModel();
    emit countChanged();
}

void TableModel::appendRows(const QList<QVariantMap>& rows) {
    if (rows.isEmpty()) {
        return;
    }
    beginInsertRows(QModelIndex(), m_rows.size(), m_rows.size() + rows.size() - 1);
    m_rows.append(rows);
    endInsertRows();
    emit countChanged();
}

void TableModel::clear() {
    resetRows({});
}

QVariantMap TableModel::get(int index) const {
    if (index >= 0 && index < m_rows.size()) {
        return m_rows.at(index);
    }
    return QVariantMap();
}
