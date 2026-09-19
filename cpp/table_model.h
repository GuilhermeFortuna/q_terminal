#ifndef Q_TERMINAL_TABLE_MODEL_H
#define Q_TERMINAL_TABLE_MODEL_H

#include <QtCore/QAbstractListModel>
#include <QtCore/QByteArray>
#include <QtCore/QHash>
#include <QtCore/QList>
#include <QtCore/QVariant>
#include <QtCore/QVariantMap>

class TableModel : public QAbstractListModel {
    Q_OBJECT
    Q_PROPERTY(int count READ count NOTIFY countChanged)

public:
    explicit TableModel(QObject* parent = nullptr);
    ~TableModel() override = default;

    int rowCount(const QModelIndex& parent = QModelIndex()) const override;
    QVariant data(const QModelIndex& index, int role = Qt::DisplayRole) const override;
    QHash<int, QByteArray> roleNames() const override;

    void setRoles(const QList<QByteArray>& roles);
    void resetRows(const QList<QVariantMap>& rows);
    void appendRows(const QList<QVariantMap>& rows);
    void clear();

    int count() const { return m_rows.size(); }
    Q_INVOKABLE QVariantMap get(int index) const;

signals:
    void countChanged();

private:
    QList<QByteArray> m_roleNames;
    QHash<int, QByteArray> m_roles;
    QList<QVariantMap> m_rows;
};

#endif // Q_TERMINAL_TABLE_MODEL_H
