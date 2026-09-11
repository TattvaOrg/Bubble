// Right-clicking a row/tile that is not selected must select it and open the
// item menu in one action (issue #20). Before the fix every view emitted an
// empty path in that case, which Main.qml reads as "empty space" and shows the
// folder menu (Sort By, New Folder...) instead.
//
// The same delegate handler exists three times, once per view, so all three are
// driven here through the real QML with QTest mouse events.
#include <QTest>
#include <QSignalSpy>
#include "viewharness.h"
#include "services/metadataextractor.h"
#include "services/previewservice.h"
#include "services/runtimefeaturesservice.h"

class TestViewContextMenu : public QObject
{
    Q_OBJECT

    // Miller needs the parent/preview columns wired up; the other two views
    // load on the common harness context alone.
    struct MillerHarness : ViewHarness {
        FileSystemModel parentModel;
        FileSystemModel previewModel;
        PreviewService previewService;
        MetadataExtractor metadataExtractor;
        RuntimeFeaturesService runtimeFeatures;

        bool loadMiller()
        {
            parentModel.setSynchronousReload(true);
            previewModel.setSynchronousReload(true);
            extraContext = [this](QQmlContext *ctx) {
                ctx->setContextProperty("millerParentModel", &parentModel);
                ctx->setContextProperty("millerPreviewModel", &previewModel);
                ctx->setContextProperty("previewService", &previewService);
                ctx->setContextProperty("metadataExtractor", &metadataExtractor);
                ctx->setContextProperty("runtimeFeatures", &runtimeFeatures);
            };
            return load(QStringLiteral("src/qml/views/FileMillerView.qml"), "fileModel");
        }
    };

    // The delegate for `index` in an ItemView, materialized and laid out.
    static QQuickItem *delegateAt(QQuickItem *itemView, int index)
    {
        QQuickItem *delegate = nullptr;
        if (!QMetaObject::invokeMethod(itemView, "itemAtIndex", Qt::DirectConnection,
                                       Q_RETURN_ARG(QQuickItem *, delegate),
                                       Q_ARG(int, index)))
            return nullptr;
        return delegate;
    }

    // filePath of the last contextMenuRequested, or a null string if none.
    static QString emittedPath(const QSignalSpy &spy)
    {
        return spy.isEmpty() ? QString() : spy.last().at(0).toString();
    }

    // One right-click on an unselected delegate: it replaces the selection and
    // the menu is asked for that file, not for empty space. Selection starts on
    // a.txt because the Miller column always has a current row of its own.
    void checkUnselectedRightClickSelects(ViewHarness &h, QQuickItem *itemView,
                                          QObject *selectionOwner, int index)
    {
        QQuickItem *root = h.view.rootObject();
        QSignalSpy spy(root, SIGNAL(contextMenuRequested(QString, bool, QPointF)));
        QVERIFY(spy.isValid());
        selectionOwner->setProperty("selectedIndices", QVariantList{0});

        QQuickItem *delegate = delegateAt(itemView, index);
        QVERIFY(delegate);
        QTest::mouseClick(&h.view, Qt::RightButton, {}, h.center(delegate));

        QTRY_COMPARE(spy.count(), 1);
        const QVariantList selected = selectionOwner->property("selectedIndices").toList();
        QCOMPARE(selected.size(), 1);
        QCOMPARE(selected.first().toInt(), index);
        QCOMPARE(emittedPath(spy), h.files.path() + "/b.txt");
        QCOMPARE(spy.last().at(1).toBool(), false);   // b.txt is not a directory
    }

    // Right-clicking inside an existing multi-selection must not collapse it:
    // "Copy" on three selected files still copies all three.
    void checkRightClickKeepsMultiSelection(ViewHarness &h, QQuickItem *itemView,
                                            QObject *selectionOwner)
    {
        QQuickItem *root = h.view.rootObject();
        QSignalSpy spy(root, SIGNAL(contextMenuRequested(QString, bool, QPointF)));
        QVERIFY(spy.isValid());

        selectionOwner->setProperty("selectedIndices", QVariantList{0, 1, 2});
        QQuickItem *delegate = delegateAt(itemView, 1);
        QVERIFY(delegate);
        QTest::mouseClick(&h.view, Qt::RightButton, {}, h.center(delegate));

        QTRY_COMPARE(spy.count(), 1);
        QCOMPARE(selectionOwner->property("selectedIndices").toList().size(), 3);
        QCOMPARE(emittedPath(spy), h.files.path() + "/b.txt");
    }

private slots:
    // The harness seeds a.txt, b.txt, c.txt; index 1 is b.txt in every view.
    void testGridRightClickSelectsUnselectedTile()
    {
        ViewHarness h;
        QVERIFY(h.load(QStringLiteral("src/qml/views/FileGridView.qml"), "model"));
        checkUnselectedRightClickSelects(h, h.view.rootObject(), h.view.rootObject(), 1);
    }

    void testGridRightClickKeepsMultiSelection()
    {
        ViewHarness h;
        QVERIFY(h.load(QStringLiteral("src/qml/views/FileGridView.qml"), "model"));
        checkRightClickKeepsMultiSelection(h, h.view.rootObject(), h.view.rootObject());
    }

    void testDetailedRightClickSelectsUnselectedRow()
    {
        ViewHarness h;
        QVERIFY(h.load());
        QQuickItem *list = h.view.rootObject()->findChild<QQuickItem *>("listView");
        QVERIFY(list);
        checkUnselectedRightClickSelects(h, list, h.view.rootObject(), 1);
    }

    void testDetailedRightClickKeepsMultiSelection()
    {
        ViewHarness h;
        QVERIFY(h.load());
        QQuickItem *list = h.view.rootObject()->findChild<QQuickItem *>("listView");
        QVERIFY(list);
        checkRightClickKeepsMultiSelection(h, list, h.view.rootObject());
    }

    // Miller keeps its selection on the current column, not on the view root.
    void testMillerRightClickSelectsUnselectedRow()
    {
        MillerHarness h;
        QVERIFY(h.loadMiller());
        QQuickItem *column = h.item("millerCurrentColumn");
        QVERIFY(column);
        checkUnselectedRightClickSelects(h, column, column, 1);
    }

    void testMillerRightClickKeepsMultiSelection()
    {
        MillerHarness h;
        QVERIFY(h.loadMiller());
        QQuickItem *column = h.item("millerCurrentColumn");
        QVERIFY(column);
        checkRightClickKeepsMultiSelection(h, column, column);
    }
};

QTEST_MAIN(TestViewContextMenu)
#include "tst_viewcontextmenu.moc"
