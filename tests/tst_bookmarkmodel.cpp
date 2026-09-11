#include <QTest>
#include <QSignalSpy>
#include <QAbstractItemModelTester>
#include "models/bookmarkmodel.h"

class TestBookmarkModel : public QObject
{
    Q_OBJECT

private slots:
    void testModelConsistency()
    {
        BookmarkModel model;
        auto *tester = new QAbstractItemModelTester(&model,
            QAbstractItemModelTester::FailureReportingMode::QtTest);
        Q_UNUSED(tester)

        model.setBookmarks({"~/Documents", "~/Downloads", "~/Pictures"});
        model.setBookmarks({"~/Music"});
        model.setBookmarks({});
    }

    void testLoadBookmarks()
    {
        BookmarkModel model;
        model.setBookmarks({"~/Documents", "~/Downloads"});
        QCOMPARE(model.rowCount(), 2);
    }

    void testBookmarkData()
    {
        BookmarkModel model;
        model.setBookmarks({"~/Documents"});
        QModelIndex idx = model.index(0);
        QString name = model.data(idx, BookmarkModel::NameRole).toString();
        QCOMPARE(name, QString("Documents"));
    }

    void testExpandTilde()
    {
        BookmarkModel model;
        model.setBookmarks({"~/Documents"});
        QModelIndex idx = model.index(0);
        QString path = model.data(idx, BookmarkModel::PathRole).toString();
        QVERIFY(path.startsWith("/"));
        QVERIFY(path.endsWith("/Documents"));
        QVERIFY(!path.contains("~"));
    }

    void testIconForKnownPaths_data()
    {
        QTest::addColumn<QString>("bookmark");
        QTest::addColumn<bool>("hasIcon");

        QTest::newRow("Documents") << "~/Documents" << true;
        QTest::newRow("Downloads") << "~/Downloads" << true;
        QTest::newRow("Pictures") << "~/Pictures" << true;
        QTest::newRow("Music") << "~/Music" << true;
        QTest::newRow("Videos") << "~/Videos" << true;
        QTest::newRow("Unknown") << "~/RandomDir" << true; // should have fallback icon
    }

    void testIconForKnownPaths()
    {
        QFETCH(QString, bookmark);
        QFETCH(bool, hasIcon);

        BookmarkModel model;
        model.setBookmarks({bookmark});
        QModelIndex idx = model.index(0);
        QString icon = model.data(idx, BookmarkModel::IconRole).toString();
        QCOMPARE(!icon.isEmpty(), hasIcon);
    }

    void testEmptyBookmarkList()
    {
        BookmarkModel model;
        model.setBookmarks({});
        QCOMPARE(model.rowCount(), 0);
    }

    void testReplaceBookmarks()
    {
        BookmarkModel model;
        model.setBookmarks({"~/Documents", "~/Downloads"});
        QCOMPARE(model.rowCount(), 2);

        model.setBookmarks({"~/Music"});
        QCOMPARE(model.rowCount(), 1);

        QModelIndex idx = model.index(0);
        QCOMPARE(model.data(idx, BookmarkModel::NameRole).toString(), QString("Music"));
    }

    void testRoleNames()
    {
        BookmarkModel model;
        auto roles = model.roleNames();
        QCOMPARE(roles[BookmarkModel::NameRole], QByteArray("name"));
        QCOMPARE(roles[BookmarkModel::PathRole], QByteArray("path"));
        QCOMPARE(roles[BookmarkModel::IconRole], QByteArray("icon"));
    }

    void testInvalidIndex()
    {
        BookmarkModel model;
        model.setBookmarks({"~/Documents"});
        QModelIndex bad = model.index(999);
        QVERIFY(!model.data(bad, BookmarkModel::NameRole).isValid());
    }

    void testAbsolutePath()
    {
        BookmarkModel model;
        model.setBookmarks({"/tmp"});
        QModelIndex idx = model.index(0);
        QCOMPARE(model.data(idx, BookmarkModel::PathRole).toString(), QString("/tmp"));
        QCOMPARE(model.data(idx, BookmarkModel::NameRole).toString(), QString("tmp"));
    }

    void testRemoteBookmarkName()
    {
        BookmarkModel model;
        model.setBookmarks({"sftp://example.com/home/jim"});
        QModelIndex idx = model.index(0);
        QCOMPARE(model.data(idx, BookmarkModel::PathRole).toString(), QString("sftp://example.com/home/jim"));
        QCOMPARE(model.data(idx, BookmarkModel::NameRole).toString(), QString("jim"));
    }

    void testRenameBookmarkOverridesName()
    {
        BookmarkModel model;
        model.setBookmarks({"~/Documents", "~/Downloads"});
        QSignalSpy dataSpy(&model, &QAbstractItemModel::dataChanged);
        QSignalSpy changedSpy(&model, &BookmarkModel::bookmarksChanged);

        model.renameBookmark(1, "  Stuff  ");

        QCOMPARE(model.data(model.index(1), BookmarkModel::NameRole).toString(), QString("Stuff"));
        QCOMPARE(model.data(model.index(0), BookmarkModel::NameRole).toString(), QString("Documents"));
        QCOMPARE(dataSpy.count(), 1);
        QCOMPARE(changedSpy.count(), 1);
        // Stored names are keyed by the portable path form paths() emits.
        QCOMPARE(model.names(), QVariantMap({{"~/Downloads", "Stuff"}}));
    }

    void testRenameBookmarkEmptyRevertsToAutoName()
    {
        BookmarkModel model;
        model.setBookmarks({"~/Downloads"});
        model.renameBookmark(0, "Stuff");
        model.renameBookmark(0, "   ");
        QCOMPARE(model.data(model.index(0), BookmarkModel::NameRole).toString(), QString("Downloads"));
        QVERIFY(model.names().isEmpty());
    }

    void testRenameBookmarkInvalidIndexIgnored()
    {
        BookmarkModel model;
        model.setBookmarks({"~/Downloads"});
        QSignalSpy changedSpy(&model, &BookmarkModel::bookmarksChanged);
        model.renameBookmark(-1, "x");
        model.renameBookmark(1, "x");
        QCOMPARE(changedSpy.count(), 0);
    }

    void testSetBookmarksAppliesNames()
    {
        BookmarkModel model;
        model.setBookmarks({"~/Documents", "~/Downloads"}, QVariantMap{{"~/Downloads", "Stuff"}});
        QCOMPARE(model.data(model.index(1), BookmarkModel::NameRole).toString(), QString("Stuff"));
        QCOMPARE(model.data(model.index(1), BookmarkModel::IconRole).toString(), QString("folder-download"));

        // Same paths, name dropped: must not be treated as "unchanged".
        model.setBookmarks({"~/Documents", "~/Downloads"});
        QCOMPARE(model.data(model.index(1), BookmarkModel::NameRole).toString(), QString("Downloads"));
    }

    void testRowsResetSignal()
    {
        BookmarkModel model;
        QSignalSpy resetSpy(&model, &QAbstractItemModel::modelReset);

        model.setBookmarks({"~/Documents"});
        QVERIFY(resetSpy.count() >= 1);
    }
};

QTEST_MAIN(TestBookmarkModel)
#include "tst_bookmarkmodel.moc"
