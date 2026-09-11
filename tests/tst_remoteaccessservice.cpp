#include <QTest>
#include <QSignalSpy>
#include <QStandardPaths>
#include "services/remoteaccessservice.h"

class TestRemoteAccessService : public QObject
{
    Q_OBJECT

private slots:
    void testReconnectWhilePendingSettlesCleanly()
    {
        if (QStandardPaths::findExecutable("gio").isEmpty())
            QSKIP("gio not found in PATH");

        RemoteAccessService service;
        QSignalSpy finished(&service, &RemoteAccessService::connectionFinished);

        // Nothing listens on port 1, so gio fails fast with "connection refused".
        service.connectToLocation("sftp://127.0.0.1:1/first");
        service.connectToLocation("sftp://127.0.0.1:1/second");

        QTRY_VERIFY_WITH_TIMEOUT(!service.busy(), 15000);
        QCOMPARE(finished.count(), 1);
        QCOMPARE(finished.at(0).at(1).toString(), QString("sftp://127.0.0.1:1/second"));
        QCOMPARE(finished.at(0).at(0).toBool(), false);
    }
};

QTEST_GUILESS_MAIN(TestRemoteAccessService)
#include "tst_remoteaccessservice.moc"
