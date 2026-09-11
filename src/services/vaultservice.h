#pragma once

#include <QObject>
#include <QString>
#include <QStringList>
#include <QSet>
#include <QFileDevice>

class CryptoEngine;
class VaultDatabase;

class VaultService : public QObject
{
    Q_OBJECT

public:
    explicit VaultService(const QString &configDir, QObject *parent = nullptr);
    ~VaultService() override;

    // QML-callable methods
    Q_INVOKABLE bool lockItem(const QString &path, const QString &password);
    Q_INVOKABLE bool lockItems(const QStringList &paths, const QString &password);
    Q_INVOKABLE bool unlockItem(const QString &path, const QString &password);
    Q_INVOKABLE bool isLocked(const QString &path) const;
    Q_INVOKABLE bool isSessionUnlocked(const QString &path) const;
    Q_INVOKABLE bool changePassword(const QString &path, const QString &oldPassword, const QString &newPassword);
    
    // Session management for folders and files
    Q_INVOKABLE bool sessionUnlockFolder(const QString &path, const QString &password);
    Q_INVOKABLE void sessionRelockFolder(const QString &path);
    Q_INVOKABLE bool sessionUnlockFile(const QString &path, const QString &password);
    Q_INVOKABLE bool sessionOpenFile(const QString &path, const QString &password);
    Q_INVOKABLE void sessionRelockFile(const QString &path);
    Q_INVOKABLE void relockAllSessions();
    
    // Check if a path or any ancestor is locked
    Q_INVOKABLE bool isPathBlocked(const QString &path) const;
    
    // For FilesystemModel integration
    Q_INVOKABLE bool hasOwnPassword(const QString &path) const;
    QSet<QString> allLockedPaths() const;

    // Brute force protection
    Q_INVOKABLE int getRemainingLockoutSeconds(const QString &path) const;

    // Error reporting
    Q_INVOKABLE QString lastError() const;
    Q_INVOKABLE void clearLastError();

signals:
    void itemLocked(const QString &path);
    void itemUnlocked(const QString &path);
    void lockError(const QString &path, const QString &error);
    void passwordRequired(const QString &path);
    void accessDenied(const QString &path);
    void sessionStarted(const QString &path);
    void sessionEnded(const QString &path);

private:
    bool lockSingleFile(const QString &path, const QString &password, qint64 parentId = 0, bool isOwnPassword = true);
    bool lockDirectory(const QString &path, const QString &password);
    bool unlockSingleFile(const QString &path, const QString &password);
    bool unlockDirectory(const QString &path, const QString &password);
    
    // OS-level protection
    bool setImmutable(const QString &path, bool immutable);
    bool setExtendedAttribute(const QString &path, bool locked);
    QString getFilePermissions(const QString &path) const;
    bool restoreFilePermissions(const QString &path, const QString &perms);
    bool setPermissionMode(const QString &path, QFileDevice::Permissions p);
    
    struct ActiveFileSession {
        QByteArray dataKey;
        QString originalPerms;
        qint64 pid = 0;
        QString filePath;
    };

    void checkRunningProcesses();
    qint64 launchDefaultApp(const QString &filePath);

    struct ActiveFolderSession {
        QByteArray dataKey;
        QString originalPerms;
    };

    struct RateLimitEntry {
        int attempts = 0;
        qint64 lastAttemptTime = 0;
    };

    void recordFailedAttempt(const QString &path);
    void clearFailedAttempts(const QString &path);

    CryptoEngine *m_crypto;
    VaultDatabase *m_db;
    QString m_configDir;
    QSet<QString> m_activeSessions; // paths with active folder or file sessions
    QHash<QString, ActiveFileSession> m_activeFileSessions;
    QHash<QString, ActiveFolderSession> m_activeFolderSessions;
    mutable QHash<QString, RateLimitEntry> m_rateLimits;
    QString m_lastError;
    class QTimer *m_processMonitorTimer = nullptr;
};
