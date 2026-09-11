#include "services/undomanager.h"
#include "services/xdgtrash.h"
#include "services/fileoperations.h"

#include <QFileInfo>
#include <QDir>
#include <QFile>
#include <QDateTime>
#include <QUrl>

#include <algorithm>

namespace {

// Finds the most recently trashed copy of a path by reading the trash
// directories, not by asking gvfs for a trash:// listing — undoing a trash
// has to work on installs with no gvfs session daemon, same as the trash
// view itself. Returns the real path under <trash>/files.
QString latestTrashedPathForOriginalPath(const QString &originalPath)
{
    QString latestPath;
    QDateTime latestDeletedAt;

    const QList<XdgTrash::Entry> entries = XdgTrash::scan();
    for (const XdgTrash::Entry &entry : entries) {
        if (QDir::cleanPath(entry.originalPath) != originalPath)
            continue;

        // An entry with no readable .trashinfo has no deletion date to
        // compare, so it only wins if nothing better has been seen.
        if (latestPath.isEmpty() || !latestDeletedAt.isValid()
            || (entry.deletedAt.isValid() && entry.deletedAt > latestDeletedAt)) {
            latestPath = entry.filesPath;
            latestDeletedAt = entry.deletedAt;
        }
    }

    return latestPath;
}

QVariantList prepareOperations(const QVariantList &operations, FileOperations *fileOps)
{
    QVariantList prepared;
    for (const QVariant &variant : operations) {
        QVariantMap item = variant.toMap();
        if (item.value("overwrite").toBool() && item.value("backupPath").toString().isEmpty())
            item["backupPath"] = fileOps->conflictBackupPath(item.value("targetPath").toString());
        prepared.append(item);
    }
    return prepared;
}

QStringList operationsField(const QVariantList &operations, const QString &field)
{
    QStringList values;
    for (const QVariant &variant : operations)
        values.append(variant.toMap().value(field).toString());
    return values;
}

QVariantList buildOperations(const QStringList &sources, const QStringList &targets,
                             const QStringList &backupPaths = {})
{
    QVariantList operations;
    const int count = std::min(sources.size(), targets.size());
    for (int i = 0; i < count; ++i) {
        QVariantMap item;
        item["sourcePath"] = sources.at(i);
        item["targetPath"] = targets.at(i);
        const QString backupPath = i < backupPaths.size() ? backupPaths.at(i) : QString();
        item["overwrite"] = !backupPath.isEmpty();
        if (!backupPath.isEmpty())
            item["backupPath"] = backupPath;
        operations.append(item);
    }
    return operations;
}

QVariantList changedRenameOperations(const QVariantList &operations)
{
    QVariantList changed;
    for (const QVariant &variant : operations) {
        const QVariantMap item = variant.toMap();
        const QString sourcePath = QDir::cleanPath(item.value("sourcePath").toString());
        const QString targetPath = QDir::cleanPath(item.value("targetPath").toString());
        if (!sourcePath.isEmpty() && !targetPath.isEmpty() && sourcePath != targetPath)
            changed.append(item);
    }

    return changed;
}

bool hasBackupPaths(const QStringList &backupPaths)
{
    for (const QString &backupPath : backupPaths) {
        if (!backupPath.isEmpty())
            return true;
    }
    return false;
}

}

UndoManager::UndoManager(FileOperations *fileOps, QObject *parent)
    : QObject(parent)
    , m_fileOps(fileOps)
{
}

bool UndoManager::canUndo() const { return !m_undoStack.isEmpty() && !m_fileOps->busy(); }
bool UndoManager::canRedo() const { return !m_redoStack.isEmpty() && !m_fileOps->busy(); }

void UndoManager::record(const UndoRecord &rec)
{
    m_undoStack.push(rec);
    if (m_undoStack.size() > kMaxUndoDepth)
        m_undoStack.removeFirst();
    if (!m_isUndoRedo)
        m_redoStack.clear();
    emit stackChanged();
}

QStringList UndoManager::computeCreatedPaths(const QStringList &sources, const QString &dest)
{
    QStringList result;
    for (const auto &src : sources)
        result.append(dest + "/" + QFileInfo(src).fileName());
    return result;
}

// ── Undoable wrappers ────────────────────────────────────────────────────

int UndoManager::copyFiles(const QStringList &sources, const QString &destination)
{
    return copyResolvedItems(buildOperations(sources, computeCreatedPaths(sources, destination)));
}

int UndoManager::copyResolvedItems(const QVariantList &operations)
{
    const QVariantList preparedOperations = prepareOperations(operations, m_fileOps);
    const QStringList sourcePaths = operationsField(preparedOperations, "sourcePath");
    const QStringList targetPaths = operationsField(preparedOperations, "targetPath");
    const QStringList backupPaths = operationsField(preparedOperations, "backupPath");

    const int opId = m_fileOps->copyResolvedItems(preparedOperations);
    if (opId < 0)
        return opId;   // failed synchronously; nothing ran, nothing to record

    auto conn = std::make_shared<QMetaObject::Connection>();
    *conn = connect(m_fileOps, &FileOperations::operationFinished,
                    this, [this, sourcePaths, targetPaths, backupPaths, conn, opId](bool success, const QString &, int id) {
        if (id != opId)
            return;
        disconnect(*conn);
        if (success)
            record({UndoRecord::Copy, sourcePaths, {}, {}, {}, targetPaths, backupPaths});
    });
    return opId;
}

int UndoManager::moveFiles(const QStringList &sources, const QString &destination)
{
    return moveResolvedItems(buildOperations(sources, computeCreatedPaths(sources, destination)));
}

int UndoManager::moveResolvedItems(const QVariantList &operations)
{
    const QVariantList preparedOperations = prepareOperations(operations, m_fileOps);
    const QStringList sourcePaths = operationsField(preparedOperations, "sourcePath");
    const QStringList targetPaths = operationsField(preparedOperations, "targetPath");
    const QStringList backupPaths = operationsField(preparedOperations, "backupPath");

    const int opId = m_fileOps->moveResolvedItems(preparedOperations);
    if (opId < 0)
        return opId;   // failed synchronously; nothing ran, nothing to record

    auto conn = std::make_shared<QMetaObject::Connection>();
    *conn = connect(m_fileOps, &FileOperations::operationFinished,
                    this, [this, sourcePaths, targetPaths, backupPaths, conn, opId](bool success, const QString &, int id) {
        if (id != opId)
            return;
        disconnect(*conn);
        if (success)
            record({UndoRecord::Move, sourcePaths, {}, {}, {}, targetPaths, backupPaths});
    });
    return opId;
}

int UndoManager::trashFiles(const QStringList &paths)
{
    const int opId = m_fileOps->trashFiles(paths);
    if (opId < 0)
        return opId;   // failed synchronously; nothing ran, nothing to record

    auto conn = std::make_shared<QMetaObject::Connection>();
    *conn = connect(m_fileOps, &FileOperations::operationFinished,
                    this, [this, paths, conn, opId](bool success, const QString &, int id) {
        if (id != opId)
            return;
        disconnect(*conn);
        if (success)
            record({UndoRecord::Trash, paths, {}, {}, {}, {}, {}});
    });
    return opId;
}

bool UndoManager::rename(const QString &path, const QString &newName)
{
    const QFileInfo info(path);
    const QVariantMap result = renameResolvedItems({QVariantMap {
        {"sourcePath", path},
        {"targetPath", info.dir().filePath(newName)}
    }});
    return result.value("success").toBool();
}

QVariantMap UndoManager::renameResolvedItems(const QVariantList &operations)
{
    const QVariantList changedOperations = changedRenameOperations(operations);
    const QVariantMap result = m_fileOps->renameResolvedItems(operations);
    if (result.value("success").toBool() && !changedOperations.isEmpty()) {
        const QStringList sourcePaths = operationsField(changedOperations, "sourcePath");
        const QStringList targetPaths = operationsField(changedOperations, "targetPath");
        record({UndoRecord::Rename, sourcePaths, {}, {}, {}, targetPaths, {}});
    }

    return result;
}

void UndoManager::createFolder(const QString &parentPath, const QString &name)
{
    m_fileOps->createFolder(parentPath, name);
    QString created = QDir(parentPath).filePath(name);
    if (QDir(created).exists())
        record({UndoRecord::CreateFolder, {}, parentPath, {}, name, {created}, {}});
}

void UndoManager::createFile(const QString &parentPath, const QString &name)
{
    m_fileOps->createFile(parentPath, name);
    QString created = QDir(parentPath).filePath(name);
    if (QFile::exists(created))
        record({UndoRecord::CreateFile, {}, parentPath, {}, name, {created}, {}});
}

// ── Undo ─────────────────────────────────────────────────────────────────

void UndoManager::undo()
{
    if (m_undoStack.isEmpty() || m_fileOps->busy())
        return;

    UndoRecord rec = m_undoStack.pop();
    m_isUndoRedo = true;
    executeUndo(rec);
    m_redoStack.push(rec);
    m_isUndoRedo = false;
    emit stackChanged();
}

void UndoManager::executeUndo(const UndoRecord &rec)
{
    switch (rec.type) {
    case UndoRecord::Copy: {
        const int opId = m_fileOps->deleteFiles(rec.createdPaths);
        if (opId >= 0 && hasBackupPaths(rec.backupPaths)) {
            auto conn = std::make_shared<QMetaObject::Connection>();
            *conn = connect(m_fileOps, &FileOperations::operationFinished,
                            this, [this, rec, conn, opId](bool success, const QString &, int id) {
                if (id != opId)
                    return;
                disconnect(*conn);
                if (success)
                    restoreBackupPaths(rec.createdPaths, rec.backupPaths);
            });
        }
        break;
    }

    case UndoRecord::Move: {
        const int opId = m_fileOps->moveResolvedItems(buildOperations(rec.createdPaths, rec.sourcePaths));
        if (opId >= 0 && hasBackupPaths(rec.backupPaths)) {
            auto conn = std::make_shared<QMetaObject::Connection>();
            *conn = connect(m_fileOps, &FileOperations::operationFinished,
                            this, [this, rec, conn, opId](bool success, const QString &, int id) {
                if (id != opId)
                    return;
                disconnect(*conn);
                if (success)
                    restoreBackupPaths(rec.createdPaths, rec.backupPaths);
            });
        }
        break;
    }

    case UndoRecord::Rename: {
        m_fileOps->renameResolvedItems(buildOperations(rec.createdPaths, rec.sourcePaths));
        break;
    }

    case UndoRecord::Trash:
        // Undo trash = restore from trash
        restoreFromTrash(rec.sourcePaths);
        break;

    case UndoRecord::CreateFolder:
        for (const auto &p : rec.createdPaths)
            QDir(p).removeRecursively();
        break;

    case UndoRecord::CreateFile:
        for (const auto &p : rec.createdPaths)
            QFile::remove(p);
        break;
    }
}

// ── Redo ─────────────────────────────────────────────────────────────────

void UndoManager::redo()
{
    if (m_redoStack.isEmpty() || m_fileOps->busy())
        return;

    UndoRecord rec = m_redoStack.pop();
    m_isUndoRedo = true;
    executeRedo(rec);
    m_undoStack.push(rec);
    m_isUndoRedo = false;
    emit stackChanged();
}

void UndoManager::executeRedo(const UndoRecord &rec)
{
    switch (rec.type) {
    case UndoRecord::Copy:
        m_fileOps->copyResolvedItems(buildOperations(rec.sourcePaths, rec.createdPaths, rec.backupPaths));
        break;

    case UndoRecord::Move:
        m_fileOps->moveResolvedItems(buildOperations(rec.sourcePaths, rec.createdPaths, rec.backupPaths));
        break;

    case UndoRecord::Rename: {
        m_fileOps->renameResolvedItems(buildOperations(rec.sourcePaths, rec.createdPaths));
        break;
    }

    case UndoRecord::Trash:
        m_fileOps->trashFiles(rec.sourcePaths);
        break;

    case UndoRecord::CreateFolder:
        if (!rec.createdPaths.isEmpty())
            m_fileOps->createFolder(rec.destination, rec.newName);
        break;

    case UndoRecord::CreateFile:
        if (!rec.createdPaths.isEmpty())
            m_fileOps->createFile(rec.destination, rec.newName);
        break;
    }
}

// ── Trash restore ────────────────────────────────────────────────────────

void UndoManager::restoreFromTrash(const QStringList &originalPaths)
{
    QStringList restoreTargets;

    for (const QString &origPath : originalPaths) {
        const QString trashUri = latestTrashedPathForOriginalPath(QDir::cleanPath(origPath));
        if (!trashUri.isEmpty())
            restoreTargets.append(trashUri);
    }

    if (!restoreTargets.isEmpty())
        m_fileOps->restoreFromTrash(restoreTargets);
}

void UndoManager::restoreBackupPaths(const QStringList &targets, const QStringList &backupPaths)
{
    QVariantList restoreOperations;
    const int count = std::min(targets.size(), backupPaths.size());
    for (int i = 0; i < count; ++i) {
        if (backupPaths.at(i).isEmpty() || !QFileInfo::exists(backupPaths.at(i)))
            continue;

        QVariantMap item;
        item["sourcePath"] = backupPaths.at(i);
        item["targetPath"] = targets.at(i);
        item["overwrite"] = false;
        restoreOperations.append(item);
    }

    if (!restoreOperations.isEmpty())
        m_fileOps->moveResolvedItems(restoreOperations);
}
