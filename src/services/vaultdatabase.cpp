#include "vaultdatabase.h"
#include <QSqlDatabase>
#include <QSqlQuery>
#include <QSqlError>
#include <QDir>
#include <QFileInfo>
#include <QDebug>
#include <QDateTime>
#include <QUuid>

VaultDatabase::VaultDatabase(QObject *parent)
    : QObject(parent)
{
    m_connectionName = QString("bubble_vault_%1").arg(QUuid::createUuid().toString(QUuid::WithoutBraces));
}

VaultDatabase::~VaultDatabase()
{
    close();
}

bool VaultDatabase::open(const QString &dbPath)
{
    if (m_isOpen) {
        return true;
    }

    QFileInfo fileInfo(dbPath);
    QDir dir = fileInfo.absoluteDir();
    if (!dir.exists()) {
        if (!dir.mkpath(".")) {
            qWarning() << "Failed to create database directory:" << dir.absolutePath();
            return false;
        }
    }

    QSqlDatabase db = QSqlDatabase::addDatabase("QSQLITE", m_connectionName);
    db.setDatabaseName(dbPath);

    if (!db.open()) {
        qWarning() << "Failed to open vault database:" << db.lastError().text();
        return false;
    }

    QSqlQuery query(db);
    if (!query.exec("PRAGMA journal_mode=WAL")) {
        qWarning() << "Failed to set WAL journal mode:" << query.lastError().text();
    }
    if (!query.exec("PRAGMA foreign_keys=ON")) {
        qWarning() << "Failed to enable foreign keys:" << query.lastError().text();
    }

    if (!createTables()) {
        db.close();
        return false;
    }

    m_isOpen = true;
    return true;
}

void VaultDatabase::close()
{
    if (m_isOpen) {
        {
            QSqlDatabase db = QSqlDatabase::database(m_connectionName);
            if (db.isOpen()) {
                db.close();
            }
        }
        QSqlDatabase::removeDatabase(m_connectionName);
        m_isOpen = false;
    }
}

bool VaultDatabase::isOpen() const
{
    return m_isOpen;
}

bool VaultDatabase::createTables()
{
    QSqlDatabase db = QSqlDatabase::database(m_connectionName);
    if (!db.isOpen()) return false;

    QSqlQuery query(db);
    
    QString createLockedItems = R"(
        CREATE TABLE IF NOT EXISTS locked_items (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            path        TEXT NOT NULL UNIQUE,
            type        TEXT NOT NULL,
            parent_id   INTEGER DEFAULT 0,
            pw_hash     BLOB NOT NULL,
            pw_salt     BLOB NOT NULL,
            enc_key     BLOB,
            enc_iv      BLOB,
            enc_salt    BLOB,
            original_perms TEXT,
            locked_at   INTEGER NOT NULL,
            is_own_password INTEGER DEFAULT 0,
            inode       INTEGER DEFAULT 0
        )
    )";

    if (!query.exec(createLockedItems)) {
        qWarning() << "Failed to create locked_items table:" << query.lastError().text();
        return false;
    }

    // Ensure inode column exists for backward compatibility with existing databases
    query.exec("ALTER TABLE locked_items ADD COLUMN inode INTEGER DEFAULT 0");

    QString createActiveSessions = R"(
        CREATE TABLE IF NOT EXISTS active_sessions (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            item_id     INTEGER REFERENCES locked_items(id) ON DELETE CASCADE,
            unlocked_at INTEGER NOT NULL,
            session_token TEXT NOT NULL
        )
    )";

    if (!query.exec(createActiveSessions)) {
        qWarning() << "Failed to create active_sessions table:" << query.lastError().text();
        return false;
    }

    return true;
}

bool VaultDatabase::addEntry(const VaultEntry &entry)
{
    QSqlDatabase db = QSqlDatabase::database(m_connectionName);
    if (!db.isOpen()) return false;

    QSqlQuery query(db);
    query.prepare("INSERT INTO locked_items (path, type, parent_id, pw_hash, pw_salt, enc_key, enc_iv, enc_salt, original_perms, locked_at, is_own_password, inode) "
                  "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)");
    query.addBindValue(entry.path);
    query.addBindValue(entry.type);
    query.addBindValue(entry.parentId);
    query.addBindValue(entry.pwHash);
    query.addBindValue(entry.pwSalt);
    query.addBindValue(entry.encKey);
    query.addBindValue(entry.encIv);
    query.addBindValue(entry.encSalt);
    query.addBindValue(entry.originalPerms);
    query.addBindValue(entry.lockedAt);
    query.addBindValue(entry.isOwnPassword ? 1 : 0);
    query.addBindValue(entry.inode);

    if (!query.exec()) {
        qWarning() << "Failed to add vault entry:" << query.lastError().text();
        return false;
    }

    return true;
}

bool VaultDatabase::updateEntry(const VaultEntry &entry)
{
    QSqlDatabase db = QSqlDatabase::database(m_connectionName);
    if (!db.isOpen()) return false;

    QSqlQuery query(db);
    query.prepare("UPDATE locked_items SET pw_hash = ?, pw_salt = ?, enc_key = ?, enc_iv = ?, enc_salt = ? WHERE path = ?");
    query.addBindValue(entry.pwHash);
    query.addBindValue(entry.pwSalt);
    query.addBindValue(entry.encKey);
    query.addBindValue(entry.encIv);
    query.addBindValue(entry.encSalt);
    query.addBindValue(entry.path);

    if (!query.exec()) {
        qWarning() << "Failed to update vault entry:" << query.lastError().text();
        return false;
    }

    return true;
}

bool VaultDatabase::removeEntry(const QString &path)
{
    QSqlDatabase db = QSqlDatabase::database(m_connectionName);
    if (!db.isOpen()) return false;

    QSqlQuery query(db);
    query.prepare("DELETE FROM locked_items WHERE path = ?");
    query.addBindValue(path);

    if (!query.exec()) {
        qWarning() << "Failed to remove vault entry:" << query.lastError().text();
        return false;
    }

    return query.numRowsAffected() > 0;
}

bool VaultDatabase::removeEntryById(qint64 id)
{
    QSqlDatabase db = QSqlDatabase::database(m_connectionName);
    if (!db.isOpen()) return false;

    QSqlQuery query(db);
    query.prepare("DELETE FROM locked_items WHERE id = ?");
    query.addBindValue(id);

    if (!query.exec()) {
        qWarning() << "Failed to remove vault entry by id:" << query.lastError().text();
        return false;
    }

    return query.numRowsAffected() > 0;
}

VaultEntry VaultDatabase::findByPath(const QString &path) const
{
    VaultEntry entry;
    QSqlDatabase db = QSqlDatabase::database(m_connectionName);
    if (!db.isOpen()) return entry;

    QSqlQuery query(db);
    query.prepare("SELECT id, path, type, parent_id, pw_hash, pw_salt, enc_key, enc_iv, enc_salt, original_perms, locked_at, is_own_password, inode "
                  "FROM locked_items WHERE path = ?");
    query.addBindValue(path);

    if (!query.exec()) {
        qWarning() << "Failed to find vault entry by path:" << query.lastError().text();
        return entry;
    }

    if (query.next()) {
        entry.id = query.value(0).toLongLong();
        entry.path = query.value(1).toString();
        entry.type = query.value(2).toString();
        entry.parentId = query.value(3).toLongLong();
        entry.pwHash = query.value(4).toByteArray();
        entry.pwSalt = query.value(5).toByteArray();
        entry.encKey = query.value(6).toByteArray();
        entry.encIv = query.value(7).toByteArray();
        entry.encSalt = query.value(8).toByteArray();
        entry.originalPerms = query.value(9).toString();
        entry.lockedAt = query.value(10).toLongLong();
        entry.isOwnPassword = query.value(11).toInt() != 0;
        entry.inode = query.value(12).toLongLong();
    }

    return entry;
}

QList<VaultEntry> VaultDatabase::findByParentId(qint64 parentId) const
{
    QList<VaultEntry> entries;
    QSqlDatabase db = QSqlDatabase::database(m_connectionName);
    if (!db.isOpen()) return entries;

    QSqlQuery query(db);
    query.prepare("SELECT id, path, type, parent_id, pw_hash, pw_salt, enc_key, enc_iv, enc_salt, original_perms, locked_at, is_own_password, inode "
                  "FROM locked_items WHERE parent_id = ?");
    query.addBindValue(parentId);

    if (!query.exec()) {
        qWarning() << "Failed to find vault entries by parent id:" << query.lastError().text();
        return entries;
    }

    while (query.next()) {
        VaultEntry entry;
        entry.id = query.value(0).toLongLong();
        entry.path = query.value(1).toString();
        entry.type = query.value(2).toString();
        entry.parentId = query.value(3).toLongLong();
        entry.pwHash = query.value(4).toByteArray();
        entry.pwSalt = query.value(5).toByteArray();
        entry.encKey = query.value(6).toByteArray();
        entry.encIv = query.value(7).toByteArray();
        entry.encSalt = query.value(8).toByteArray();
        entry.originalPerms = query.value(9).toString();
        entry.lockedAt = query.value(10).toLongLong();
        entry.isOwnPassword = query.value(11).toInt() != 0;
        entry.inode = query.value(12).toLongLong();
        entries.append(entry);
    }

    return entries;
}

QList<VaultEntry> VaultDatabase::allEntries() const
{
    QList<VaultEntry> entries;
    QSqlDatabase db = QSqlDatabase::database(m_connectionName);
    if (!db.isOpen()) return entries;

    QSqlQuery query(db);
    if (!query.exec("SELECT id, path, type, parent_id, pw_hash, pw_salt, enc_key, enc_iv, enc_salt, original_perms, locked_at, is_own_password, inode FROM locked_items")) {
        qWarning() << "Failed to get all vault entries:" << query.lastError().text();
        return entries;
    }

    while (query.next()) {
        VaultEntry entry;
        entry.id = query.value(0).toLongLong();
        entry.path = query.value(1).toString();
        entry.type = query.value(2).toString();
        entry.parentId = query.value(3).toLongLong();
        entry.pwHash = query.value(4).toByteArray();
        entry.pwSalt = query.value(5).toByteArray();
        entry.encKey = query.value(6).toByteArray();
        entry.encIv = query.value(7).toByteArray();
        entry.encSalt = query.value(8).toByteArray();
        entry.originalPerms = query.value(9).toString();
        entry.lockedAt = query.value(10).toLongLong();
        entry.isOwnPassword = query.value(11).toInt() != 0;
        entry.inode = query.value(12).toLongLong();
        entries.append(entry);
    }

    return entries;
}

bool VaultDatabase::hasEntry(const QString &path) const
{
    QSqlDatabase db = QSqlDatabase::database(m_connectionName);
    if (!db.isOpen()) return false;

    QSqlQuery query(db);
    query.prepare("SELECT 1 FROM locked_items WHERE path = ? LIMIT 1");
    query.addBindValue(path);

    if (!query.exec()) {
        qWarning() << "Failed to check vault entry existence:" << query.lastError().text();
        return false;
    }

    return query.next();
}

bool VaultDatabase::removeEntriesRecursive(const QString &directoryPath)
{
    QSqlDatabase db = QSqlDatabase::database(m_connectionName);
    if (!db.isOpen()) return false;

    QString dirPathPrefix = directoryPath;
    if (!dirPathPrefix.endsWith('/')) {
        dirPathPrefix += '/';
    }
    QString wildcardStr = dirPathPrefix + "%";

    QSqlQuery query(db);
    query.prepare("DELETE FROM locked_items WHERE path = ? OR path LIKE ?");
    query.addBindValue(directoryPath);
    query.addBindValue(wildcardStr);

    if (!query.exec()) {
        qWarning() << "Failed to remove recursive vault entries:" << query.lastError().text();
        return false;
    }

    return true;
}

QList<VaultEntry> VaultDatabase::findChildEntries(const QString &directoryPath) const
{
    QList<VaultEntry> entries;
    QSqlDatabase db = QSqlDatabase::database(m_connectionName);
    if (!db.isOpen()) return entries;

    QString dirPathPrefix = directoryPath;
    if (!dirPathPrefix.endsWith('/')) {
        dirPathPrefix += '/';
    }
    QString wildcardStr = dirPathPrefix + "%";

    QSqlQuery query(db);
    query.prepare("SELECT id, path, type, parent_id, pw_hash, pw_salt, enc_key, enc_iv, enc_salt, original_perms, locked_at, is_own_password "
                  "FROM locked_items WHERE path LIKE ?");
    query.addBindValue(wildcardStr);

    if (!query.exec()) {
        qWarning() << "Failed to find child entries:" << query.lastError().text();
        return entries;
    }

    while (query.next()) {
        VaultEntry entry;
        entry.id = query.value(0).toLongLong();
        entry.path = query.value(1).toString();
        entry.type = query.value(2).toString();
        entry.parentId = query.value(3).toLongLong();
        entry.pwHash = query.value(4).toByteArray();
        entry.pwSalt = query.value(5).toByteArray();
        entry.encKey = query.value(6).toByteArray();
        entry.encIv = query.value(7).toByteArray();
        entry.encSalt = query.value(8).toByteArray();
        entry.originalPerms = query.value(9).toString();
        entry.lockedAt = query.value(10).toLongLong();
        entry.isOwnPassword = query.value(11).toInt() != 0;
        entries.append(entry);
    }

    return entries;
}

bool VaultDatabase::addSession(qint64 itemId, const QString &sessionToken)
{
    QSqlDatabase db = QSqlDatabase::database(m_connectionName);
    if (!db.isOpen()) return false;

    QSqlQuery query(db);
    query.prepare("INSERT INTO active_sessions (item_id, unlocked_at, session_token) VALUES (?, ?, ?)");
    query.addBindValue(itemId);
    query.addBindValue(QDateTime::currentSecsSinceEpoch());
    query.addBindValue(sessionToken);

    if (!query.exec()) {
        qWarning() << "Failed to add active session:" << query.lastError().text();
        return false;
    }

    return true;
}

bool VaultDatabase::removeSession(qint64 itemId)
{
    QSqlDatabase db = QSqlDatabase::database(m_connectionName);
    if (!db.isOpen()) return false;

    QSqlQuery query(db);
    query.prepare("DELETE FROM active_sessions WHERE item_id = ?");
    query.addBindValue(itemId);

    if (!query.exec()) {
        qWarning() << "Failed to remove active session:" << query.lastError().text();
        return false;
    }

    return query.numRowsAffected() > 0;
}

bool VaultDatabase::hasActiveSession(qint64 itemId) const
{
    QSqlDatabase db = QSqlDatabase::database(m_connectionName);
    if (!db.isOpen()) return false;

    QSqlQuery query(db);
    query.prepare("SELECT 1 FROM active_sessions WHERE item_id = ? LIMIT 1");
    query.addBindValue(itemId);

    if (!query.exec()) {
        qWarning() << "Failed to check active session:" << query.lastError().text();
        return false;
    }

    return query.next();
}

void VaultDatabase::clearAllSessions()
{
    QSqlDatabase db = QSqlDatabase::database(m_connectionName);
    if (!db.isOpen()) return;

    QSqlQuery query(db);
    if (!query.exec("DELETE FROM active_sessions")) {
        qWarning() << "Failed to clear all active sessions:" << query.lastError().text();
    }
}

QStringList VaultDatabase::allLockedPaths() const
{
    QStringList paths;
    QSqlDatabase db = QSqlDatabase::database(m_connectionName);
    if (!db.isOpen()) return paths;

    QSqlQuery query(db);
    if (!query.exec("SELECT path FROM locked_items")) {
        qWarning() << "Failed to get all locked paths:" << query.lastError().text();
        return paths;
    }

    while (query.next()) {
        paths.append(query.value(0).toString());
    }

    return paths;
}
