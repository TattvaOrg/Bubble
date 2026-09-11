#pragma once

#include <QObject>
#include <QHash>
#include <QLocale>
#include <QSet>
#include <QStringList>
#include <QThread>
#include <QVariantList>
#include <QVariantMap>

#include <atomic>

class DiskUsageWorker : public QObject
{
    Q_OBJECT

public:
    explicit DiskUsageWorker(QObject *parent = nullptr);

    struct FileId {
        quint64 device = 0;
        quint64 inode = 0;

        bool operator==(const FileId &other) const
        {
            return device == other.device && inode == other.inode;
        }
    };

    void calculate(int requestId, const QStringList &paths);
    void cancel();

signals:
    void finished(int requestId, qint64 size, const QVariantMap &pathSizes, int unreadableCount,
                  bool cancelled);

private:
    qint64 pathSize(const QString &path, QSet<FileId> *seenInodes, int *unreadableCount) const;
    qint64 directorySize(const QString &path, QSet<FileId> *seenInodes, int *unreadableCount) const;

    std::atomic<bool> m_cancelled{false};
};

class DiskUsageService : public QObject
{
    Q_OBJECT

public:
    static constexpr int MaxCachedPaths = 256;
    static constexpr qint64 CacheTtlMs = 30000;

    explicit DiskUsageService(QObject *parent = nullptr);
    ~DiskUsageService() override;

    Q_INVOKABLE int requestSize(const QVariantList &paths);
    Q_INVOKABLE void cancelRequest(int requestId);
    Q_INVOKABLE void clearCache();
    Q_INVOKABLE void invalidatePath(const QString &path);
    Q_INVOKABLE void invalidatePaths(const QStringList &paths);

signals:
    void requestFinished(int requestId, const QVariantMap &result);

private:
    struct CacheEntry {
        qint64 size = 0;
        qint64 storedAtMs = 0;
    };

    struct ActiveRequest {
        QThread *thread = nullptr;
        DiskUsageWorker *worker = nullptr;
        bool cacheResult = false;
        QString cachePath;
    };

    static QString normalizePath(const QString &path);

public:
    // Locale-aware ("1,5 KB" in de_DE). Header-inline so models can share it
    // without linking the service.
    static QString formattedSize(qint64 size, bool verbose = false)
    {
        if (size < 0)
            return {};
        const QLocale locale;
        auto fmt = [&](double value, int precision, const char *unit) {
            QString text = QStringLiteral("%1 %2").arg(locale.toString(value, 'f', precision), QLatin1String(unit));
            if (verbose)
                text += QStringLiteral(" (%1 bytes)").arg(locale.toString(size));
            return text;
        };
        if (size < 1024)
            return verbose ? QStringLiteral("%1 B (%2 bytes)").arg(size).arg(locale.toString(size))
                           : QStringLiteral("%1 B").arg(size);
        if (size < 1024 * 1024)
            return fmt(size / 1024.0, 1, "KB");
        if (size < 1024LL * 1024 * 1024)
            return fmt(size / (1024.0 * 1024.0), 1, "MB");
        return fmt(size / (1024.0 * 1024.0 * 1024.0), verbose ? 2 : 1, "GB");
    }

private:
    bool tryCachedSize(const QString &path, qint64 *size);
    void rememberCachedSize(const QString &path, qint64 size);
    void removeCachedPath(const QString &path);
    void cleanupRequest(int requestId);

    int m_nextRequestId = 1;
    QHash<int, ActiveRequest> m_requests;
    QHash<QString, CacheEntry> m_cache;
    QStringList m_cacheOrder;
};
