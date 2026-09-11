#include <QDir>
#include <QTest>
#include <QSignalSpy>
#include "services/cloudmounts.h"
#include "services/rcloneservice.h"

class TestRcloneService : public QObject
{
    Q_OBJECT

private slots:
    // Every navigation runs through isRclonePath(), and the whole cloud path
    // handling -- skipped previews, skipped git status, faked metadata --
    // hangs off it. Getting the boundaries wrong either mounts on ordinary
    // folders or silently treats a mount as local.
    void testOnlyPathsInsideAMountCountAsCloud_data()
    {
        QTest::addColumn<QString>("path");
        QTest::addColumn<bool>("expected");

        const QString base = cloudMountsBaseDir();
        QTest::newRow("inside a remote") << base + "/gdrive/photos/a.jpg" << true;
        QTest::newRow("the remote itself") << base + "/gdrive" << true;
        QTest::newRow("the mounts folder") << base << false;
        QTest::newRow("mounts folder with slash") << base + "/" << false;
        QTest::newRow("a sibling folder") << base + "-backup/gdrive" << false;
        QTest::newRow("an ordinary path") << QStringLiteral("/home/user/Documents") << false;
        QTest::newRow("empty") << QString() << false;
    }

    void testOnlyPathsInsideAMountCountAsCloud()
    {
        QFETCH(QString, path);
        QFETCH(bool, expected);

        RcloneService service;
        QCOMPARE(service.isRclonePath(path), expected);
        QCOMPARE(isCloudMountPath(path), expected);
    }

    void testRemoteNameIsTheFirstSegmentUnderTheMountsFolder()
    {
        RcloneService service;
        const QString base = cloudMountsBaseDir();

        QCOMPARE(service.getRemoteNameFromPath(base + "/gdrive"), QString("gdrive"));
        QCOMPARE(service.getRemoteNameFromPath(base + "/gdrive/photos/a.jpg"), QString("gdrive"));
        QCOMPARE(service.getRemoteNameFromPath(base), QString());
        QCOMPARE(service.getRemoteNameFromPath("/home/user/Documents"), QString());
    }

    void testMountPathRoundTripsThroughTheRemoteName()
    {
        RcloneService service;
        const QString mountPath = service.getMountPath(QStringLiteral("onedrive"));

        QCOMPARE(mountPath, cloudMountsBaseDir() + "/onedrive");
        QVERIFY(service.isRclonePath(mountPath));
        QCOMPARE(service.getRemoteNameFromPath(mountPath), QString("onedrive"));
    }

    // A remote nobody has mounted must not report itself as mounted or
    // mounting: Main.qml reads both before deciding whether to navigate
    // straight away or park the navigation behind a mount.
    void testAnUnknownRemoteIsNeitherMountedNorMounting()
    {
        RcloneService service;

        QVERIFY(!service.isMounted(QStringLiteral("nosuchremote")));
        QVERIFY(!service.isMounting(QStringLiteral("nosuchremote")));
        QVERIFY(!service.isMountedForPath(cloudMountsBaseDir() + "/nosuchremote/file.txt"));
        QVERIFY(service.activeMounts().isEmpty());
    }

    // The navigation that asked for the mount is only released by
    // mountFinished, so a mount that cannot even start has to report failure
    // rather than leave the pane waiting forever.
    void testMountingAnUnknownRemoteReportsFailure()
    {
        RcloneService service;
        QSignalSpy finished(&service, &RcloneService::mountFinished);

        service.mountRemote(QStringLiteral("bubble-test-nosuchremote"));

        QTRY_VERIFY_WITH_TIMEOUT(finished.count() == 1, 20000);
        QCOMPARE(finished.at(0).at(0).toString(), QString("bubble-test-nosuchremote"));
        QCOMPARE(finished.at(0).at(1).toBool(), false);
        QVERIFY(!finished.at(0).at(2).toString().isEmpty());
        QVERIFY(!service.isMounted(QStringLiteral("bubble-test-nosuchremote")));

        // mountRemote() creates the mount point before it knows the mount
        // will fail; don't leave it behind in the user's home.
        QDir().rmdir(service.getMountPath(QStringLiteral("bubble-test-nosuchremote")));
    }
};

QTEST_GUILESS_MAIN(TestRcloneService)
#include "tst_rcloneservice.moc"
