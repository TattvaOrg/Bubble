#pragma once

#include <QObject>
#include <QString>
#include <QByteArray>
#include <QVariantMap>
#include <QVariantList>
#include <QStringList>

class QSqlDatabase;

struct VaultEntry {
    qint64 id = 0;
    QString path;
    QString type;           // "file" or "directory"
    qint64 parentId = 0;    // 0 = no parent
    QByteArray pwHash;
    QByteArray pwSalt;
    QByteArray encKey;      // encrypted data key
    QByteArray encIv;
    QByteArray encSalt;
    QString originalPerms;
    qint64 lockedAt = 0;
    bool isOwnPassword = false;
    qint64 inode = 0;
};

class VaultDatabase : public QObject
{
    Q_OBJECT

public:
    explicit VaultDatabase(QObject *parent = nullptr);
    ~VaultDatabase() override;

    bool open(const QString &dbPath);
    void close();
    bool isOpen() const;

    // CRUD operations
    bool addEntry(const VaultEntry &entry);
    bool updateEntry(const VaultEntry &entry);
    bool removeEntry(const QString &path);
    bool removeEntryById(qint64 id);
    VaultEntry findByPath(const QString &path) const;
    QList<VaultEntry> findByParentId(qint64 parentId) const;
    QList<VaultEntry> allEntries() const;
    bool hasEntry(const QString &path) const;
    
    // Batch operations
    bool removeEntriesRecursive(const QString &directoryPath);
    QList<VaultEntry> findChildEntries(const QString &directoryPath) const;

    // Session tracking
    bool addSession(qint64 itemId, const QString &sessionToken);
    bool removeSession(qint64 itemId);
    bool hasActiveSession(qint64 itemId) const;
    void clearAllSessions();

    // For vault-destroy: get all locked file paths
    QStringList allLockedPaths() const;

private:
    bool createTables();
    bool m_isOpen = false;
    QString m_connectionName;
};
