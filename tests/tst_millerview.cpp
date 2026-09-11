// Headless interaction test for the Miller view's draggable column dividers.
#include <QTest>
#include "viewharness.h"
#include "services/metadataextractor.h"
#include "services/previewservice.h"
#include "services/runtimefeaturesservice.h"

class TestMillerView : public QObject
{
    Q_OBJECT

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

private slots:
    void testLeftDividerDragChangesParentFraction()
    {
        MillerHarness h;
        QVERIFY(h.loadMiller());
        QQuickItem *divider = h.item("millerDivider_left");
        QQuickItem *parentColumn = h.item("millerParentColumn");
        QVERIFY(divider && parentColumn);
        QCOMPARE(int(parentColumn->width()), int(900 * 0.2));

        const QPoint start = h.center(divider);
        h.drag(start, start + QPoint(90, 0));   // +10% of a 900 px view

        QTRY_COMPARE(int(parentColumn->width()), int(900 * 0.3));
        QVERIFY(qAbs(h.config->millerFractions().value("parent").toDouble() - 0.3) < 0.005);
        QVERIFY(qAbs(h.config->millerFractions().value("current").toDouble() - 0.4) < 0.005);
    }

    void testRightDividerDragChangesCurrentFraction()
    {
        MillerHarness h;
        QVERIFY(h.loadMiller());
        QQuickItem *divider = h.item("millerDivider_right");
        QQuickItem *preview = h.item("millerPreviewColumn");
        QVERIFY(divider && preview);
        const int previewBefore = int(preview->width());

        const QPoint start = h.center(divider);
        h.drag(start, start + QPoint(-90, 0));

        QTRY_COMPARE(int(preview->width()), previewBefore + 90);
        QVERIFY(qAbs(h.config->millerFractions().value("current").toDouble() - 0.4) < 0.005);
        QVERIFY(qAbs(h.config->millerFractions().value("parent").toDouble() - 0.2) < 0.005);
    }

    void testDividersRespectMinimumWidth()
    {
        MillerHarness h;
        QVERIFY(h.loadMiller());
        QQuickItem *divider = h.item("millerDivider_left");
        QVERIFY(divider);
        h.drag(h.center(divider), h.center(divider) + QPoint(-400, 0));
        QVERIFY(h.config->millerFractions().value("parent").toDouble() >= ConfigManager::kMillerMinFraction - 1e-9);
    }
};

QTEST_MAIN(TestMillerView)
#include "tst_millerview.moc"
