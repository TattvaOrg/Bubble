#include "services/remoteaccessservice.h"

#include <QDir>
#include <QUrl>

RemoteAccessService::RemoteAccessService(QObject *parent)
    : QObject(parent)
{
}

bool RemoteAccessService::busy() const
{
    return m_process != nullptr;
}

QString RemoteAccessService::buildUri(const QString &protocol, const QString &host,
                                      const QString &remotePath, const QString &user,
                                      int port, const QString &share) const
{
    if (host.trimmed().isEmpty())
        return {};

    QUrl url;
    url.setScheme(protocol.trimmed().toLower());
    url.setHost(host.trimmed());
    if (!user.trimmed().isEmpty())
        url.setUserName(user.trimmed());
    if (port > 0)
        url.setPort(port);

    QString path = remotePath.trimmed();
    if (url.scheme() == QStringLiteral("smb")) {
        const QString trimmedShare = share.trimmed();
        if (!trimmedShare.isEmpty())
            path = trimmedShare + (path.isEmpty() ? QString() : QStringLiteral("/") + path);
    }

    if (path.isEmpty())
        path = QStringLiteral("/");
    if (!path.startsWith('/'))
        path.prepend('/');

    url.setPath(QDir::cleanPath(path));
    return url.toString(QUrl::FullyEncoded);
}

void RemoteAccessService::connectToLocation(const QString &uri)
{
    const QString trimmedUri = uri.trimmed();
    if (trimmedUri.isEmpty()) {
        emit connectionFinished(false, QString(), QStringLiteral("Enter a remote location"));
        return;
    }

    if (m_process) {
        // Its signals must not reach the handlers below once a new process
        // owns m_process; ~QProcess emits them synchronously on kill.
        disconnect(m_process, nullptr, this, nullptr);
        m_process->kill();
        m_process->deleteLater();
        m_process = nullptr;
    }

    m_pendingUri = trimmedUri;
    QProcess *process = new QProcess(this);
    m_process = process;
    emit busyChanged();

    // errorOccurred(Crashed) and finished() fire for the same event; whichever
    // runs first settles the connection and the other bails on the guard.
    auto settle = [this, process](bool success, const QString &error) {
        if (process != m_process)
            return;
        emit connectionFinished(success, m_pendingUri, error);
        process->deleteLater();
        m_process = nullptr;
        m_pendingUri.clear();
        emit busyChanged();
    };

    connect(process, qOverload<int, QProcess::ExitStatus>(&QProcess::finished), this,
            [process, settle](int exitCode, QProcess::ExitStatus) {
        settle(exitCode == 0, exitCode == 0
            ? QString()
            : QString::fromUtf8(process->readAllStandardError()).trimmed());
    });

    connect(process, &QProcess::errorOccurred, this, [process, settle](QProcess::ProcessError) {
        settle(false, process->errorString());
    });

    process->start(QStringLiteral("gio"), {QStringLiteral("mount"), trimmedUri});
}
