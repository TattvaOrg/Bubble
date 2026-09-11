#include "services/clipboardmanager.h"

#include <QByteArray>
#include <QClipboard>
#include <QFileInfo>
#include <QGuiApplication>
#include <QMimeData>
#include <QUrl>

namespace {

void writePathsToSystemClipboard(const QStringList &paths, bool cut)
{
    QClipboard *cb = QGuiApplication::clipboard();
    if (!cb)
        return;

    if (paths.isEmpty()) {
        cb->clear();
        return;
    }

    QList<QUrl> urls;
    QStringList plainLines;
    urls.reserve(paths.size());
    plainLines.reserve(paths.size());
    for (const QString &p : paths) {
        QUrl url = p.contains(QStringLiteral("://"))
            ? QUrl(p)
            : QUrl::fromLocalFile(QFileInfo(p).absoluteFilePath());
        urls.append(url);
        plainLines.append(url.toString());
    }

    auto *mime = new QMimeData;
    mime->setUrls(urls);
    mime->setText(plainLines.join(QLatin1Char('\n')));

    // KDE / Dolphin cut marker
    mime->setData(QStringLiteral("application/x-kde-cutselection"),
                  cut ? QByteArrayLiteral("1") : QByteArrayLiteral("0"));

    // GNOME / Nautilus cut marker: "cut\nfile:///..." or "copy\nfile:///..."
    QByteArray gnome = cut ? QByteArrayLiteral("cut") : QByteArrayLiteral("copy");
    for (const QUrl &u : urls) {
        gnome.append('\n');
        gnome.append(u.toEncoded());
    }
    mime->setData(QStringLiteral("x-special/gnome-copied-files"), gnome);

    cb->setMimeData(mime);
}

} // namespace

ClipboardManager::ClipboardManager(QObject *parent)
    : QObject(parent)
{
}

bool ClipboardManager::hasContent() const { return !m_paths.isEmpty(); }
bool ClipboardManager::isCut() const { return m_isCut; }
QStringList ClipboardManager::paths() const { return m_paths; }

void ClipboardManager::copy(const QStringList &paths)
{
    m_paths = paths;
    m_isCut = false;
    writePathsToSystemClipboard(m_paths, false);
    emit changed();
}

void ClipboardManager::cut(const QStringList &paths)
{
    m_paths = paths;
    m_isCut = true;
    writePathsToSystemClipboard(m_paths, true);
    emit changed();
}

void ClipboardManager::clear()
{
    m_paths.clear();
    m_isCut = false;
    if (QClipboard *cb = QGuiApplication::clipboard())
        cb->clear();
    emit changed();
}

bool ClipboardManager::contains(const QString &path) const
{
    return m_paths.contains(path);
}

QStringList ClipboardManager::take()
{
    QStringList result = m_paths;
    if (m_isCut) {
        m_paths.clear();
        m_isCut = false;
        emit changed();
    }
    return result;
}

void ClipboardManager::copyText(const QString &text)
{
    if (QClipboard *cb = QGuiApplication::clipboard())
        cb->setText(text);
}
