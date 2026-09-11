#include "providers/pdfpreviewprovider.h"

#include <QDateTime>
#include <QFileInfo>
#include <QCache>
#include <QHash>
#include <QMutex>
#include <QProcess>
#include <QQuickTextureFactory>
#include <QRegularExpression>
#include <QStandardPaths>
#include <QThreadPool>
#include <QUrl>
#include <QUrlQuery>

#include <algorithm>

namespace {

struct PdfRequest {
    QString path;
    int page = 0;
};

PdfRequest parseRequest(const QString &id)
{
    PdfRequest request;
    const int queryIndex = id.indexOf('?');
    const QString encodedPath = queryIndex >= 0 ? id.left(queryIndex) : id;
    request.path = QUrl::fromPercentEncoding(encodedPath.toUtf8());

    if (queryIndex >= 0) {
        QUrlQuery query;
        query.setQuery(id.mid(queryIndex + 1));
        request.page = query.queryItemValue("page").toInt();
    }

    return request;
}

// pdfinfo prints "Page size:   595.28 x 841.89 pts (A4)" on one line.
// Measured ~17 ms, and it used to run on every single page change even
// though the answer is a property of the document, not the page.
QSizeF pageSizeUncached(const QString &path)
{
    QProcess proc;
    proc.start(QStringLiteral("pdfinfo"), {path});
    if (!proc.waitForFinished(5000) || proc.exitCode() != 0)
        return {};

    const QString out = QString::fromUtf8(proc.readAllStandardOutput());
    static const QRegularExpression re(
        QStringLiteral(R"(Page size:\s*([0-9.]+)\s*x\s*([0-9.]+)\s*pts)"));
    const auto m = re.match(out);
    if (!m.hasMatch())
        return {};
    return QSizeF(m.captured(1).toDouble(), m.captured(2).toDouble());
}

// ponytail: unbounded map, but it holds one QSizeF per PDF previewed this
// session. Add an LRU cap if someone ever previews thousands of documents.
QSizeF pageSizePoints(const QString &path)
{
    const QString key = path + QLatin1Char('\0')
        + QString::number(QFileInfo(path).lastModified().toMSecsSinceEpoch());

    static QMutex mutex;
    static QHash<QString, QSizeF> cache;

    {
        QMutexLocker locker(&mutex);
        const auto it = cache.constFind(key);
        if (it != cache.constEnd())
            return *it;
    }

    // Deliberately computed outside the lock: two threads racing on a cold
    // cache just both run pdfinfo and store the same answer, which is far
    // cheaper than serialising every render behind one mutex.
    const QSizeF size = pageSizeUncached(path);

    QMutexLocker locker(&mutex);
    cache.insert(key, size);
    return size;
}

double dpiForRequest(const QSizeF &pageSizePts, const QSize &requestedSize)
{
    if (!pageSizePts.isValid() || pageSizePts.isEmpty())
        return 120.0;

    double scale = 1.0;
    if (requestedSize.width() > 0 && requestedSize.height() > 0) {
        const double xScale = requestedSize.width() / pageSizePts.width();
        const double yScale = requestedSize.height() / pageSizePts.height();
        scale = std::min(xScale, yScale);
    } else if (requestedSize.width() > 0) {
        scale = requestedSize.width() / pageSizePts.width();
    } else if (requestedSize.height() > 0) {
        scale = requestedSize.height() / pageSizePts.height();
    }

    scale = std::max(0.2, scale);
    return 72.0 * scale;
}


// Rendered-page cache.
//
// QML asks for the same page more than once: an Image's sourceSize is part
// of Qt's own pixmap-cache key, and it changes as the item lays out, so the
// first paint routinely renders a page twice. The prefetch of the adjacent
// pages doubles that again. Keying on the quantised dpi we actually pass to
// pdftoppm collapses all of it to a single render.
//
// ponytail: cost is the decoded image size, capped at 64 MB -- roughly 20
// A4 pages at 150 dpi, comfortably more than the prefetch window needs.
QString renderKey(const QString &path, int page, int dpi)
{
    return path + QLatin1Char('\0')
        + QString::number(QFileInfo(path).lastModified().toMSecsSinceEpoch())
        + QLatin1Char('\0') + QString::number(page)
        + QLatin1Char('@') + QString::number(dpi);
}

QMutex &renderCacheMutex()
{
    static QMutex mutex;
    return mutex;
}

QCache<QString, QImage> &renderCache()
{
    static QCache<QString, QImage> cache(64 * 1024 * 1024);
    return cache;
}

// Serialises the actual pdftoppm runs. Without it the cache never helps on
// first paint: QML issues the duplicate requests concurrently, so both miss
// the cache and both render. Holding this across the subprocess is safe --
// it is only ever taken by pool threads, never by the GUI thread.
QMutex &renderLock()
{
    static QMutex mutex;
    return mutex;
}

}

PdfPreviewResponse::PdfPreviewResponse(const QString &id, const QSize &requestedSize)
    : m_id(id)
    , m_requestedSize(requestedSize)
{
    setAutoDelete(false);
    QThreadPool::globalInstance()->start(this);
}

bool PdfPreviewResponse::tryCache(const QString &key)
{
    QMutexLocker locker(&renderCacheMutex());
    if (const QImage *cached = renderCache().object(key)) {
        m_image = *cached;
        return true;
    }
    return false;
}

void PdfPreviewResponse::run()
{
    const PdfRequest request = parseRequest(m_id);
    if (request.path.isEmpty() || !QFileInfo::exists(request.path)) {
        emit finished();
        return;
    }

    if (QStandardPaths::findExecutable(QStringLiteral("pdftoppm")).isEmpty()) {
        emit finished();
        return;
    }

    const QSizeF sizePts = pageSizePoints(request.path);
    const int dpi = static_cast<int>(dpiForRequest(sizePts, m_requestedSize) + 0.5);
    const QString key = renderKey(request.path, request.page, dpi);

    if (tryCache(key)) {
        emit finished();
        return;
    }

    QMutexLocker renderLocker(&renderLock());

    // Re-check: another thread may have rendered this exact page while we
    // were queued behind it. This is the check that actually collapses the
    // duplicate first-paint requests.
    if (tryCache(key)) {
        emit finished();
        return;
    }

    // pdftoppm writes to stdout only when no output-prefix argument is
    // given. Passing "-" as the prefix (a common assumption) makes recent
    // poppler write a file named "--1.png" in the current working
    // directory instead — silently producing no stdout output.
    // -png spends ~87% of its wall time in zlib, not in rendering: 570 ms
    // vs 70 ms for -jpeg on the same page at the same dpi. The pages are
    // photographic-quality raster either way, and at quality=90 the mean
    // per-pixel difference from the PNG is 0.42/255 — invisible in a
    // preview pane, 8x faster to produce.
    QProcess proc;
    proc.start(QStringLiteral("pdftoppm"), {
        QStringLiteral("-jpeg"),
        QStringLiteral("-jpegopt"), QStringLiteral("quality=90"),
        QStringLiteral("-singlefile"),
        QStringLiteral("-f"), QString::number(request.page + 1),
        QStringLiteral("-l"), QString::number(request.page + 1),
        QStringLiteral("-r"), QString::number(dpi),
        request.path,
    });

    if (!proc.waitForFinished(15000) || proc.exitCode() != 0) {
        emit finished();
        return;
    }

    const QByteArray jpeg = proc.readAllStandardOutput();
    if (jpeg.isEmpty()) {
        emit finished();
        return;
    }

    m_image.loadFromData(jpeg, "JPEG");

    if (!m_image.isNull()) {
        const qsizetype cost = m_image.sizeInBytes();
        QMutexLocker locker(&renderCacheMutex());
        renderCache().insert(key, new QImage(m_image), static_cast<qsizetype>(cost));
    }

    emit finished();
}

QQuickTextureFactory *PdfPreviewResponse::textureFactory() const
{
    return QQuickTextureFactory::textureFactoryForImage(m_image);
}

QQuickImageResponse *PdfPreviewProvider::requestImageResponse(const QString &id,
                                                             const QSize &requestedSize)
{
    return new PdfPreviewResponse(id, requestedSize);
}
