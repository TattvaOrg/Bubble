#include <QTest>
#include <QSignalSpy>
#include <unistd.h>
#include <QThread>
#include <QTimer>
#include "testdir.h"
#include "services/giotransferworker.h"

class TestGioTransferWorker : public QObject
{
    Q_OBJECT

private slots:
    void testCopySingleFile()
    {
        TestDir src, dst;
        src.createFile("test.bin", QByteArray(4096, 'x'));

        GioTransferWorker worker;
        QList<GioTransferWorker::TransferItem> items;
        items.append({src.path() + "/test.bin", dst.path() + "/test.bin", {}, false});

        QSignalSpy finishSpy(&worker, &GioTransferWorker::finished);

        QThread thread;
        worker.moveToThread(&thread);
        connect(&thread, &QThread::started, &worker, [&]() {
            worker.execute(items, false);
        });
        connect(&worker, &GioTransferWorker::finished, &thread, &QThread::quit);
        thread.start();

        QVERIFY(finishSpy.wait(10000));
        QCOMPARE(finishSpy.constFirst().at(0).toBool(), true);
        QVERIFY(QFile::exists(dst.path() + "/test.bin"));
        QCOMPARE(QFileInfo(dst.path() + "/test.bin").size(), 4096);
        // finished() and the queued thread.quit() race inside spy.wait(): quit
        // explicitly or wait() can block on a loop that never got the call.
        thread.quit();
        QVERIFY(thread.wait(5000));
    }

    void testCopyDirectory()
    {
        TestDir src, dst;
        src.createFile("dir/a.txt", QByteArray(1000, 'a'));
        src.createFile("dir/b.txt", QByteArray(2000, 'b'));
        src.createFile("dir/sub/c.txt", QByteArray(3000, 'c'));

        GioTransferWorker worker;
        QList<GioTransferWorker::TransferItem> items;
        items.append({src.path() + "/dir", dst.path() + "/dir", {}, false});

        QSignalSpy finishSpy(&worker, &GioTransferWorker::finished);

        QThread thread;
        worker.moveToThread(&thread);
        connect(&thread, &QThread::started, &worker, [&]() {
            worker.execute(items, false);
        });
        connect(&worker, &GioTransferWorker::finished, &thread, &QThread::quit);
        thread.start();

        QVERIFY(finishSpy.wait(10000));
        QCOMPARE(finishSpy.constFirst().at(0).toBool(), true);
        QVERIFY(QFile::exists(dst.path() + "/dir/a.txt"));
        QVERIFY(QFile::exists(dst.path() + "/dir/b.txt"));
        QVERIFY(QFile::exists(dst.path() + "/dir/sub/c.txt"));
        QCOMPARE(QFileInfo(dst.path() + "/dir/sub/c.txt").size(), 3000);
        // finished() and the queued thread.quit() race inside spy.wait(): quit
        // explicitly or wait() can block on a loop that never got the call.
        thread.quit();
        QVERIFY(thread.wait(5000));
    }

    // Overwrite with a backup that cannot be created must not touch the
    // target: the undo entry would point at a backup that does not exist.
    void testOverwriteAbortsWhenBackupFails()
    {
        TestDir src, dst;
        src.createFile("new.txt", "new");
        dst.createFile("target.txt", "keep me");
        dst.createFile("notadir", "x");   // backup parent is a file → mkdir and move fail

        GioTransferWorker worker;
        QList<GioTransferWorker::TransferItem> items;
        items.append({src.path() + "/new.txt", dst.path() + "/target.txt",
                      dst.path() + "/notadir/backup.txt", true});
        QSignalSpy finishSpy(&worker, &GioTransferWorker::finished);
        QThread thread;
        worker.moveToThread(&thread);
        connect(&thread, &QThread::started, &worker, [&]() { worker.execute(items, false); });
        connect(&worker, &GioTransferWorker::finished, &thread, &QThread::quit);
        thread.start();
        QVERIFY(finishSpy.wait(10000));
        QCOMPARE(finishSpy.constFirst().at(0).toBool(), false);
        QVERIFY(!finishSpy.constFirst().at(1).toString().isEmpty());
        QFile f(dst.path() + "/target.txt");
        QVERIFY(f.open(QIODevice::ReadOnly));
        QCOMPARE(f.readAll(), QByteArray("keep me"));
        thread.quit();
        QVERIFY(thread.wait(5000));
    }

    // A tree that cannot be fully read is not a finished copy, so a move
    // must leave the source alone.
    void testMoveKeepsSourceWhenTreeCannotBeRead()
    {
        if (geteuid() == 0)
            QSKIP("root ignores directory permissions");
        TestDir src, dst;
        src.createDir("tree");
        src.createFile("tree/a.txt", "a");
        src.createDir("tree/locked");
        src.createFile("tree/locked/b.txt", "b");
        const QString locked = src.path() + "/tree/locked";
        QVERIFY(QFile::setPermissions(locked, QFileDevice::Permissions()));

        GioTransferWorker worker;
        QList<GioTransferWorker::TransferItem> items;
        items.append({src.path() + "/tree", dst.path() + "/tree", {}, false});
        QSignalSpy finishSpy(&worker, &GioTransferWorker::finished);
        QThread thread;
        worker.moveToThread(&thread);
        connect(&thread, &QThread::started, &worker, [&]() { worker.execute(items, true); });
        connect(&worker, &GioTransferWorker::finished, &thread, &QThread::quit);
        thread.start();
        const bool finished = finishSpy.wait(10000);
        QFile::setPermissions(locked, QFileDevice::ReadOwner | QFileDevice::WriteOwner | QFileDevice::ExeOwner);
        QVERIFY(finished);
        QCOMPARE(finishSpy.constFirst().at(0).toBool(), false);
        QVERIFY(QFile::exists(src.path() + "/tree/a.txt"));
        QVERIFY(QFile::exists(src.path() + "/tree/locked/b.txt"));
        thread.quit();
        QVERIFY(thread.wait(5000));
    }

    void testMoveFile()
    {
        TestDir src, dst;
        src.createFile("move_me.txt", QByteArray(2048, 'm'));

        GioTransferWorker worker;
        QList<GioTransferWorker::TransferItem> items;
        items.append({src.path() + "/move_me.txt", dst.path() + "/move_me.txt", {}, false});

        QSignalSpy finishSpy(&worker, &GioTransferWorker::finished);

        QThread thread;
        worker.moveToThread(&thread);
        connect(&thread, &QThread::started, &worker, [&]() {
            worker.execute(items, true);
        });
        connect(&worker, &GioTransferWorker::finished, &thread, &QThread::quit);
        thread.start();

        QVERIFY(finishSpy.wait(10000));
        QCOMPARE(finishSpy.constFirst().at(0).toBool(), true);
        QVERIFY(QFile::exists(dst.path() + "/move_me.txt"));
        QVERIFY(!QFile::exists(src.path() + "/move_me.txt"));
        // finished() and the queued thread.quit() race inside spy.wait(): quit
        // explicitly or wait() can block on a loop that never got the call.
        thread.quit();
        QVERIFY(thread.wait(5000));
    }

    void testMoveTreeWithSymlinkKeepsLinkTarget()
    {
        TestDir src, dst;
        src.createDir("outside");
        src.createFile("outside/keep.txt", "keep");
        src.createDir("tree");
        src.createSymlink(src.path() + "/outside", "tree/lnk");

        GioTransferWorker worker;
        QList<GioTransferWorker::TransferItem> items;
        items.append({src.path() + "/tree", dst.path() + "/tree", {}, false});

        QSignalSpy finishSpy(&worker, &GioTransferWorker::finished);

        QThread thread;
        worker.moveToThread(&thread);
        connect(&thread, &QThread::started, &worker, [&]() {
            worker.execute(items, true);
        });
        connect(&worker, &GioTransferWorker::finished, &thread, &QThread::quit);
        thread.start();

        QVERIFY(finishSpy.wait(10000));
        // finished() and the queued thread.quit() race inside spy.wait(): quit
        // explicitly or wait() can block on a loop that never got the call.
        thread.quit();
        QVERIFY(thread.wait(5000));
        QCOMPARE(finishSpy.constFirst().at(0).toBool(), true);
        QVERIFY(QFile::exists(src.path() + "/outside/keep.txt"));
        QVERIFY(QFileInfo(dst.path() + "/tree/lnk").isSymLink());
        QVERIFY(!QFile::exists(src.path() + "/tree"));
    }

    void testCopySymlink()
    {
        TestDir src, dst;
        src.createFile("real.txt", "hello");
        src.createSymlink(src.path() + "/real.txt", "link.txt");

        GioTransferWorker worker;
        QList<GioTransferWorker::TransferItem> items;
        items.append({src.path() + "/link.txt", dst.path() + "/link.txt", {}, false});

        QSignalSpy finishSpy(&worker, &GioTransferWorker::finished);

        QThread thread;
        worker.moveToThread(&thread);
        connect(&thread, &QThread::started, &worker, [&]() {
            worker.execute(items, false);
        });
        connect(&worker, &GioTransferWorker::finished, &thread, &QThread::quit);
        thread.start();

        QVERIFY(finishSpy.wait(10000));
        QCOMPARE(finishSpy.constFirst().at(0).toBool(), true);
        QFileInfo linkInfo(dst.path() + "/link.txt");
        QVERIFY(linkInfo.isSymLink());
        QCOMPARE(linkInfo.symLinkTarget(), src.path() + "/real.txt");
        // finished() and the queued thread.quit() race inside spy.wait(): quit
        // explicitly or wait() can block on a loop that never got the call.
        thread.quit();
        QVERIFY(thread.wait(5000));
    }

    void testCancel()
    {
        TestDir src, dst;
        src.createFile("big.bin", QByteArray(16 * 1024 * 1024, 'x'));

        GioTransferWorker worker;
        QList<GioTransferWorker::TransferItem> items;
        items.append({src.path() + "/big.bin", dst.path() + "/big.bin", {}, false});

        QSignalSpy finishSpy(&worker, &GioTransferWorker::finished);

        QThread thread;
        worker.moveToThread(&thread);
        connect(&thread, &QThread::started, &worker, [&]() {
            worker.execute(items, false);
        });
        connect(&worker, &GioTransferWorker::finished, &thread, &QThread::quit);

        // Cancel after first progress update
        connect(&worker, &GioTransferWorker::progressUpdated, &worker, [&](double, const QString &, const QString &) {
            worker.cancel();
        }, Qt::DirectConnection);

        thread.start();
        QVERIFY(finishSpy.wait(10000));
        QCOMPARE(finishSpy.constFirst().at(0).toBool(), false);
        // finished() and the queued thread.quit() race inside spy.wait(): quit
        // explicitly or wait() can block on a loop that never got the call.
        thread.quit();
        QVERIFY(thread.wait(5000));
    }

    void testPauseResume()
    {
        TestDir src, dst;
        src.createFile("pause_test.bin", QByteArray(8 * 1024 * 1024, 'p'));

        GioTransferWorker worker;
        QList<GioTransferWorker::TransferItem> items;
        items.append({src.path() + "/pause_test.bin", dst.path() + "/pause_test.bin", {}, false});

        QSignalSpy finishSpy(&worker, &GioTransferWorker::finished);
        bool paused = false;

        QThread thread;
        worker.moveToThread(&thread);
        connect(&thread, &QThread::started, &worker, [&]() {
            worker.execute(items, false);
        });
        connect(&worker, &GioTransferWorker::finished, &thread, &QThread::quit);

        // progressUpdated is queued to the main thread; call pause() directly,
        // then schedule resume() on the main thread via a single-shot timer so
        // it is posted back to the worker thread via an auto-connection.
        connect(&worker, &GioTransferWorker::progressUpdated, this, [&](double, const QString &, const QString &) {
            if (!paused) {
                paused = true;
                worker.pause();
                QTimer::singleShot(100, this, [&]() {
                    worker.resume();
                });
            }
        });

        thread.start();
        QVERIFY(finishSpy.wait(15000));
        QCOMPARE(finishSpy.constFirst().at(0).toBool(), true);
        QVERIFY(QFile::exists(dst.path() + "/pause_test.bin"));
        QCOMPARE(QFileInfo(dst.path() + "/pause_test.bin").size(), 8 * 1024 * 1024);
        // finished() and the queued thread.quit() race inside spy.wait(): quit
        // explicitly or wait() can block on a loop that never got the call.
        thread.quit();
        QVERIFY(thread.wait(5000));
    }
};

QTEST_GUILESS_MAIN(TestGioTransferWorker)
#include "tst_giotransferworker.moc"
