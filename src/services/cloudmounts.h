#pragma once

#include <QDir>
#include <QString>

// Where the rclone mounts live. Five call sites across the model, the git
// service and the file operations need to recognise these paths, so the
// directory is defined once here rather than spelled out in each of them.
inline QString cloudMountsBaseDir()
{
    static const QString dir = QDir::homePath() + QStringLiteral("/.local/share/bubble/mounts");
    return dir;
}

// True for a path inside a mount, not for the mounts directory itself: the
// container is an ordinary local folder and must stay browsable.
inline bool isCloudMountPath(const QString &path)
{
    static const QString prefix = cloudMountsBaseDir() + QLatin1Char('/');
    return path.startsWith(prefix) && path.length() > prefix.length();
}
