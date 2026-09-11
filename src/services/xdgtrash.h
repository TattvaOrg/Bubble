#pragma once

#include <QDateTime>
#include <QList>
#include <QString>
#include <QStringList>

// Reading the trash straight off disk per the XDG Trash spec, instead of
// asking gvfs for trash:// listings. gvfs is a session daemon, so packaging
// that can't ship one (Nix without services.gvfs.enable, AppImage, Flatpak)
// used to get an empty Trash view even though the files were right there.
// Moving files *into* the trash still goes through g_file_trash(), which is
// plain GLib and needs no gvfs at all.
//
// Remote trash (sftp://, smb://) is out of scope: those really do need gvfs.
namespace XdgTrash {

struct Entry {
    QString name;         // basename inside <root>/files
    QString filesPath;    // absolute path of the trashed file itself
    QString originalPath; // decoded Path= — absolute once resolved
    QDateTime deletedAt;  // parsed DeletionDate=
};

// ~/.local/share/Trash, whether or not it exists yet — callers that need to
// say where a file *would* land use this; roots() only reports real ones.
QString homeRoot();

// Every trash directory currently visible: the home one plus a
// .Trash-<uid> / .Trash/<uid> on each mounted volume that has one.
QStringList roots();

// The .trashinfo that belongs to a file under some trash's files/ dir.
// Empty when the path isn't inside a trash directory at all.
QString infoPathFor(const QString &filesPath);

// Reads the sidecar metadata for one trashed file. An entry with no
// readable .trashinfo still comes back with name/filesPath filled in, so a
// half-deleted item stays visible and deletable rather than vanishing.
Entry readEntry(const QString &filesPath);

// Every top-level trashed item across every root.
QList<Entry> scan();

// Drops the .trashinfo for a trashed file. Call it after the file itself is
// gone, or the trash keeps growing metadata for items that no longer exist.
bool removeInfo(const QString &filesPath);

} // namespace XdgTrash
