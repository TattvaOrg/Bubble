#pragma once

#include <QLatin1String>
#include <QString>

// Handed to 7z/unzip whenever no password is known. It is deliberately wrong,
// and long enough that no real password collides with it: without a -p/-P the
// tools prompt on stdin and wait forever for input nobody will type, hanging
// the extraction. With it, an encrypted archive fails immediately and an
// unencrypted one ignores it.
inline const QLatin1String archivePasswordSentinel("__bubble_placeholder_password__");

inline QString effectiveArchivePassword(const QString &password)
{
    return password.isEmpty() ? QString(archivePasswordSentinel) : password;
}
