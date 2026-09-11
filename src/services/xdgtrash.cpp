#include "services/xdgtrash.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QStandardPaths>
#include <QStorageInfo>
#include <QUrl>

#include <unistd.h>

namespace {

QString uidString()
{
    return QString::number(geteuid());
}

// Where GLib itself puts trashed files: $XDG_DATA_HOME/Trash, falling back to
// ~/.local/share/Trash when the variable is unset.
QString xdgDataTrashRoot()
{
    const QString data = QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation);
    return data.isEmpty() ? QString() : QDir::cleanPath(data + QStringLiteral("/Trash"));
}

// A trash directory is only usable if it has the two subdirectories the
// spec mandates. That check doubles as the filter for the volume scan:
// most mounted volumes have no trash at all.
bool looksLikeTrashRoot(const QString &root)
{
    const QDir dir(root);
    return dir.exists(QStringLiteral("files")) && dir.exists(QStringLiteral("info"));
}

// <root>/files/<name> -> <root>. Empty unless the path really is a
// top-level entry of a trash directory: a file nested inside a trashed
// folder has no .trashinfo of its own, and must not borrow its parent's.
QString trashRootForEntry(const QString &filesPath)
{
    const QFileInfo info(filesPath);
    const QDir filesDir = info.absoluteDir();
    if (filesDir.dirName() != QStringLiteral("files"))
        return {};

    const QString root = QFileInfo(filesDir.absolutePath()).absolutePath();
    return looksLikeTrashRoot(root) ? root : QString();
}

// Relative Path= entries are relative to the volume the trash lives on,
// which is the directory holding .Trash-<uid> (or .Trash/<uid>). The home
// trash always stores absolute paths, so it has no volume root here.
QString volumeRootFor(const QString &trashRoot)
{
    const QFileInfo rootInfo(trashRoot);
    if (rootInfo.fileName().startsWith(QStringLiteral(".Trash-")))
        return rootInfo.absolutePath();

    const QFileInfo parent(rootInfo.absolutePath());
    if (parent.fileName() == QStringLiteral(".Trash"))
        return parent.absolutePath();

    return {};
}

} // namespace

namespace XdgTrash {

QString homeRoot()
{
    return QDir::cleanPath(QDir::homePath() + QStringLiteral("/.local/share/Trash"));
}

QStringList roots()
{
    QStringList found;

    // Two home candidates on purpose. GLib's g_file_trash() — what actually
    // moves files in — resolves the trash through XDG_DATA_HOME, so that one
    // has to be scanned or a user who sets it sees an empty Trash. The plain
    // ~/.local/share path is kept as well because under Flatpak XDG_DATA_HOME
    // points into ~/.var/app/<id>/data while the user's real trash is still
    // the one they expect to see.
    for (const QString &candidate : {xdgDataTrashRoot(), homeRoot()}) {
        if (!candidate.isEmpty() && looksLikeTrashRoot(candidate))
            found.append(QDir::cleanPath(candidate));
    }

    const QString uid = uidString();
    const auto volumes = QStorageInfo::mountedVolumes();
    for (const QStorageInfo &volume : volumes) {
        if (!volume.isValid() || !volume.isReady())
            continue;

        const QString mount = QDir::cleanPath(volume.rootPath());
        if (mount.isEmpty())
            continue;

        for (const QString &candidate : {QDir(mount).filePath(QStringLiteral(".Trash-") + uid),
                                         QDir(mount).filePath(QStringLiteral(".Trash/") + uid)}) {
            if (looksLikeTrashRoot(candidate))
                found.append(QDir::cleanPath(candidate));
        }
    }

    found.removeDuplicates();
    return found;
}

QString infoPathFor(const QString &filesPath)
{
    const QString root = trashRootForEntry(filesPath);
    if (root.isEmpty())
        return {};

    return QDir::cleanPath(root + QStringLiteral("/info/")
                           + QFileInfo(filesPath).fileName()
                           + QStringLiteral(".trashinfo"));
}

Entry readEntry(const QString &filesPath)
{
    Entry entry;
    entry.filesPath = QDir::cleanPath(filesPath);
    entry.name = QFileInfo(entry.filesPath).fileName();
    const QString infoPath = infoPathFor(entry.filesPath);
    if (infoPath.isEmpty())
        return entry;

    QFile info(infoPath);
    if (!info.open(QIODevice::ReadOnly | QIODevice::Text))
        return entry;

    // Hand-rolled rather than QSettings: the spec's Path= is percent-encoded
    // and QSettings mangles '%' and unquoted values on the way through.
    while (!info.atEnd()) {
        const QByteArray line = info.readLine().trimmed();
        if (line.startsWith("Path=")) {
            const QString raw = QUrl::fromPercentEncoding(line.mid(5));
            if (QDir::isAbsolutePath(raw)) {
                entry.originalPath = QDir::cleanPath(raw);
            } else {
                const QString volume = volumeRootFor(trashRootForEntry(entry.filesPath));
                if (!volume.isEmpty())
                    entry.originalPath = QDir::cleanPath(volume + QLatin1Char('/') + raw);
            }
        } else if (line.startsWith("DeletionDate=")) {
            const QString raw = QString::fromUtf8(line.mid(13)).trimmed();
            entry.deletedAt = QDateTime::fromString(raw, Qt::ISODate);
            if (!entry.deletedAt.isValid())
                entry.deletedAt = QDateTime::fromString(raw, Qt::ISODateWithMs);
        }
    }

    return entry;
}

QList<Entry> scan()
{
    QList<Entry> entries;
    const QStringList trashRoots = roots();
    for (const QString &root : trashRoots) {
        QDir filesDir(root + QStringLiteral("/files"));
        const auto infos = filesDir.entryInfoList(
            QDir::AllEntries | QDir::NoDotAndDotDot | QDir::Hidden | QDir::System);
        for (const QFileInfo &info : infos)
            entries.append(readEntry(info.absoluteFilePath()));
    }
    return entries;
}

bool removeInfo(const QString &filesPath)
{
    const QString info = infoPathFor(filesPath);
    if (info.isEmpty() || !QFile::exists(info))
        return false;
    return QFile::remove(info);
}

} // namespace XdgTrash
