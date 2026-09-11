#include <QTest>
#include <QDir>
#include <QFile>
#include <QTemporaryDir>
#include "services/xdgtrash.h"

class TestXdgTrash : public QObject
{
    Q_OBJECT

    QTemporaryDir m_home;

    QString trashRoot() const
    {
        return m_home.path() + QStringLiteral("/.local/share/Trash");
    }

    void writeInfo(const QString &name, const QString &encodedPath, const QString &date)
    {
        QFile info(trashRoot() + QStringLiteral("/info/") + name + QStringLiteral(".trashinfo"));
        QVERIFY(info.open(QIODevice::WriteOnly | QIODevice::Text));
        info.write("[Trash Info]\n");
        info.write(QStringLiteral("Path=%1\n").arg(encodedPath).toUtf8());
        info.write(QStringLiteral("DeletionDate=%1\n").arg(date).toUtf8());
    }

    QString trashFile(const QString &name) const
    {
        return trashRoot() + QStringLiteral("/files/") + name;
    }

private slots:
    void initTestCase()
    {
        QVERIFY(m_home.isValid());
        // XdgTrash::roots() resolves the home trash through QDir::homePath(),
        // which follows $HOME on Unix, so the whole suite runs against a
        // throwaway trash instead of the developer's real one.
        qputenv("HOME", m_home.path().toUtf8());
        QCOMPARE(QDir::homePath(), m_home.path());

        QVERIFY(QDir().mkpath(trashRoot() + QStringLiteral("/files")));
        QVERIFY(QDir().mkpath(trashRoot() + QStringLiteral("/info")));
    }

    void testInfoPathForTopLevelEntry()
    {
        QCOMPARE(XdgTrash::infoPathFor(trashFile("report.txt")),
                 trashRoot() + QStringLiteral("/info/report.txt.trashinfo"));
    }

    void testNestedChildHasNoInfoPath()
    {
        // Only the item that was trashed carries metadata. A file inside a
        // trashed folder must not resolve to some other entry's .trashinfo,
        // or deleting it would strip the parent's metadata.
        QVERIFY(XdgTrash::infoPathFor(trashFile("folder/inner.txt")).isEmpty());
    }

    void testPathOutsideTrashHasNoInfoPath()
    {
        QVERIFY(XdgTrash::infoPathFor(m_home.path() + QStringLiteral("/notes.txt")).isEmpty());
    }

    void testReadEntryDecodesPathAndDate()
    {
        QFile f(trashFile("my report.txt"));
        QVERIFY(f.open(QIODevice::WriteOnly));
        f.write("x");
        f.close();
        writeInfo(QStringLiteral("my report.txt"),
                  QStringLiteral("/home/someone/my%20report.txt"),
                  QStringLiteral("2026-08-27T10:11:12"));

        const XdgTrash::Entry entry = XdgTrash::readEntry(trashFile("my report.txt"));
        QCOMPARE(entry.name, QString("my report.txt"));
        QCOMPARE(entry.originalPath, QString("/home/someone/my report.txt"));
        QCOMPARE(entry.deletedAt, QDateTime::fromString("2026-08-27T10:11:12", Qt::ISODate));
    }

    void testEntryWithoutInfoStillReadable()
    {
        // A half-deleted item (file present, metadata gone) has to stay
        // visible in the trash view, otherwise it can never be cleaned up.
        QFile f(trashFile("orphan.bin"));
        QVERIFY(f.open(QIODevice::WriteOnly));
        f.write("x");
        f.close();

        const XdgTrash::Entry entry = XdgTrash::readEntry(trashFile("orphan.bin"));
        QCOMPARE(entry.name, QString("orphan.bin"));
        QCOMPARE(entry.filesPath, trashFile("orphan.bin"));
        QVERIFY(entry.originalPath.isEmpty());
        QVERIFY(!entry.deletedAt.isValid());
    }

    void testScanListsTopLevelEntriesOnly()
    {
        QVERIFY(QDir().mkpath(trashFile("folder")));
        QFile inner(trashFile("folder/inner.txt"));
        QVERIFY(inner.open(QIODevice::WriteOnly));
        inner.write("x");
        inner.close();
        writeInfo(QStringLiteral("folder"), QStringLiteral("/home/someone/folder"),
                  QStringLiteral("2026-08-27T10:11:12"));

        QStringList names;
        const auto entries = XdgTrash::scan();
        for (const XdgTrash::Entry &e : entries)
            names << e.name;

        QVERIFY(names.contains("folder"));
        QVERIFY(names.contains("orphan.bin"));
        QVERIFY(!names.contains("inner.txt"));
    }

    void testRemoveInfoDropsSidecar()
    {
        QFile f(trashFile("gone.txt"));
        QVERIFY(f.open(QIODevice::WriteOnly));
        f.write("x");
        f.close();
        writeInfo(QStringLiteral("gone.txt"), QStringLiteral("/home/someone/gone.txt"),
                  QStringLiteral("2026-08-27T10:11:12"));

        const QString info = XdgTrash::infoPathFor(trashFile("gone.txt"));
        QVERIFY(QFile::exists(info));
        QVERIFY(XdgTrash::removeInfo(trashFile("gone.txt")));
        QVERIFY(!QFile::exists(info));
        // Second call is a no-op, not a crash or a false success.
        QVERIFY(!XdgTrash::removeInfo(trashFile("gone.txt")));
    }

    void testHomeTrashIsDiscovered()
    {
        QVERIFY(XdgTrash::roots().contains(QDir::cleanPath(trashRoot())));
    }
};

QTEST_MAIN(TestXdgTrash)
#include "tst_xdgtrash.moc"
