// Headless test for the toolbar tab strip: tabs keep a minimum width and the
// strip scrolls instead of squeezing them.
#include <QTest>
#include <QWheelEvent>
#include "viewharness.h"
#include "models/tablistmodel.h"
#include <QJsonArray>

class TestToolbar : public QObject
{
    Q_OBJECT

    struct ToolbarHarness : ViewHarness {
        TabListModel &tabs = *new TabListModel(&view);   // dies with the view, after the QML
        bool loadToolbar(int tabCount)
        {
            while (tabs.rowCount() < tabCount)
                tabs.addTab();
            extraContext = [this](QQmlContext *ctx) { ctx->setContextProperty("tabModel", &tabs); };
            return load(QStringLiteral("src/qml/components/Toolbar.qml"), "activeTab");
        }
    };

private slots:
    void testTabsKeepMinimumWidthAndStripScrolls()
    {
        ToolbarHarness h;
        QVERIFY(h.loadToolbar(12));
        QTest::qWait(400);   // enter animations
        QQuickItem *strip = h.item("tabStrip");
        QVERIFY(strip);
        for (int i = 0; i < 12; ++i) {
            QQuickItem *tab = h.item(QStringLiteral("toolbarTab_%1").arg(i));
            QVERIFY2(tab, qPrintable(QStringLiteral("tab %1 missing").arg(i)));
            QVERIFY2(tab->width() >= 120, qPrintable(QStringLiteral("tab %1 is %2 px").arg(i).arg(tab->width())));
        }
        QVERIFY(strip->property("contentWidth").toReal() > strip->width());

        // Activating the last tab scrolls it into view.
        h.tabs.setActiveIndex(11);
        QQuickItem *last = h.item("toolbarTab_11");
        QTRY_VERIFY(strip->property("contentX").toReal() > 0);
        QTRY_VERIFY(last->x() + last->width() <= strip->property("contentX").toReal() + strip->width() + 0.5);
    }

    void testWheelOverStripScrollsIt()
    {
        ToolbarHarness h;
        QVERIFY(h.loadToolbar(12));
        QTest::qWait(400);
        QQuickItem *strip = h.item("tabStrip");
        QVERIFY(strip);
        h.tabs.setActiveIndex(0);
        QTRY_COMPARE(strip->property("contentX").toReal(), 0.0);
        const QPoint pos = h.center(strip);
        QWheelEvent ev(pos, h.view.mapToGlobal(pos), QPoint(), QPoint(0, -120),
                       Qt::NoButton, Qt::NoModifier, Qt::NoScrollPhase, false);
        QCoreApplication::sendEvent(&h.view, &ev);
        QTRY_VERIFY(strip->property("contentX").toReal() > 0);
    }

    void testHorizontalSwipeScrollsStrip()
    {
        ToolbarHarness h;
        QVERIFY(h.loadToolbar(12));
        QTest::qWait(400);
        QQuickItem *strip = h.item("tabStrip");
        QVERIFY(strip);
        h.tabs.setActiveIndex(0);
        QTRY_COMPARE(strip->property("contentX").toReal(), 0.0);
        const QPoint pos = h.center(strip);
        QWheelEvent ev(pos, h.view.mapToGlobal(pos), QPoint(-60, 0), QPoint(),   // swipe left = scroll right
                       Qt::NoButton, Qt::NoModifier, Qt::ScrollUpdate, false);
        QCoreApplication::sendEvent(&h.view, &ev);
        QTRY_VERIFY(strip->property("contentX").toReal() > 0);
    }

    void testDragTabReordersIt()
    {
        ToolbarHarness h;
        QVERIFY(h.loadToolbar(3));
        for (int i = 0; i < 3; ++i)
            h.tabs.tabAt(i)->navigateTo(QStringLiteral("/t%1").arg(i));
        QTest::qWait(400);
        QQuickItem *first = h.item("toolbarTab_0");
        QQuickItem *third = h.item("toolbarTab_2");
        QVERIFY(first && third);

        h.drag(h.center(first), h.center(third) + QPoint(20, 0));

        QCOMPARE(h.tabs.tabAt(0)->currentPath(), QString("/t1"));
        QCOMPARE(h.tabs.tabAt(1)->currentPath(), QString("/t2"));
        QCOMPARE(h.tabs.tabAt(2)->currentPath(), QString("/t0"));
        QCOMPARE(h.tabs.activeIndex(), 2);
    }

    void testReorderSlidesDisplacedTabs()
    {
        ToolbarHarness h;
        QVERIFY(h.loadToolbar(3));
        for (int i = 0; i < 3; ++i)
            h.tabs.tabAt(i)->navigateTo(QStringLiteral("/t%1").arg(i));
        QTest::qWait(400);
        QQuickItem *first = h.item("toolbarTab_0");
        QQuickItem *second = h.item("toolbarTab_1");
        QVERIFY(first && second);

        // Press tab 0 and move it into tab 1's slot in one step: tab 1 is
        // displaced left and must start from its old position (offset > 0)
        // and animate back to 0.
        const QPoint from = h.center(first);
        QTest::mousePress(&h.view, Qt::LeftButton, {}, from);
        QTest::mouseMove(&h.view, h.center(second) + QPoint(10, 0), 10);
        QCOMPARE(h.tabs.tabAt(0)->currentPath(), QString("/t1"));
        // The layout repositions on the next polish; the displaced tab then
        // starts from its old spot (positive offset) and eases back to 0.
        QTRY_VERIFY(second->property("slideOffset").toReal() > 1);
        QTRY_COMPARE(second->property("slideOffset").toReal(), 0.0);
        QTest::mouseRelease(&h.view, Qt::LeftButton, {}, h.center(second) + QPoint(10, 0));
    }

    // Close tabs quickly with the middle button (each runs an exit animation),
    // "restart" by restoring the saved session into a fresh toolbar, add tabs.
    // Every tab must be visible and together fill the strip — no gap.
    void testRapidClosesThenRestartLeavesNoGap()
    {
        QJsonArray session;
        int activeIndex = 0;
        {
            ToolbarHarness h;
            QVERIFY(h.loadToolbar(8));
            for (int i = 0; i < 8; ++i)
                h.tabs.tabAt(i)->navigateTo(QStringLiteral("/t%1").arg(i));
            QTest::qWait(400);
            // left-to-right so every later close sees shifted indices
            for (int i = 0; i < 7; ++i) {
                QQuickItem *tab = h.item(QStringLiteral("toolbarTab_%1").arg(i));
                QVERIFY(tab);
                QTest::mouseClick(&h.view, Qt::MiddleButton, {}, h.center(tab));
                QTest::qWait(20);
            }
            QTest::qWait(800);   // exit animations
            QCOMPARE(h.tabs.rowCount(), 1);
            QCOMPARE(h.tabs.tabAt(0)->currentPath(), QString("/t7"));   // the one we did not close
            session = h.tabs.saveSession();
            activeIndex = h.tabs.activeIndex();
        }

        ToolbarHarness h;
        h.tabs.restoreSession(session, activeIndex);
        QVERIFY(h.loadToolbar(1));
        h.tabs.addTab();
        h.tabs.addTab();
        QTest::qWait(600);
        QQuickItem *strip = h.item("tabStrip");
        QVERIFY(strip);
        qreal total = 0;
        for (int i = 0; i < 3; ++i) {
            QQuickItem *tab = h.item(QStringLiteral("toolbarTab_%1").arg(i));
            QVERIFY(tab);
            QVERIFY2(tab->opacity() > 0.9 && tab->width() > 100,
                     qPrintable(QStringLiteral("tab %1: opacity %2 width %3").arg(i).arg(tab->opacity()).arg(tab->width())));
            total += tab->width();
        }
        QVERIFY2(qAbs(total - strip->width()) < 2, qPrintable(QStringLiteral("tabs %1 vs strip %2").arg(total).arg(strip->width())));
    }

    // Start like a restored session with a single tab (bar hidden, height 0),
    // then open tabs: the strip must show the first tab at x=0, no gap.
    void testOpeningTabsFromHiddenBarStartsAtZero()
    {
        ToolbarHarness h;
        QVERIFY(h.loadToolbar(1));
        QTest::qWait(200);
        h.tabs.addTab();
        QTest::qWait(50);
        h.tabs.addTab();
        QTest::qWait(700);   // bar height animation + enter animations
        QQuickItem *strip = h.item("tabStrip");
        QQuickItem *first = h.item("toolbarTab_0");
        QVERIFY(strip && first);
        QCOMPARE(strip->property("contentX").toReal(), 0.0);
        QCOMPARE(int(first->mapToItem(strip, QPointF(0, 0)).x()), 0);
        qreal total = 0;
        for (int i = 0; i < 3; ++i)
            total += h.item(QStringLiteral("toolbarTab_%1").arg(i))->width();
        QVERIFY2(qAbs(total - strip->width()) < 2, qPrintable(QStringLiteral("tabs %1 strip %2").arg(total).arg(strip->width())));
    }

    void testFewTabsShareTheWidth()
    {
        ToolbarHarness h;
        QVERIFY(h.loadToolbar(3));
        QTest::qWait(400);
        QQuickItem *strip = h.item("tabStrip");
        QQuickItem *tab = h.item("toolbarTab_0");
        QVERIFY(strip && tab);
        QVERIFY(tab->width() > 200);   // 900 px / 3 tabs
        QCOMPARE(strip->property("contentX").toReal(), 0.0);
    }
};

QTEST_MAIN(TestToolbar)
#include "tst_toolbar.moc"
