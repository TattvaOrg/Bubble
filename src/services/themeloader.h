#pragma once
#include <QObject>
#include <QColor>
#include <QMap>
#include <QDateTime>
#include <QFileSystemWatcher>

class ThemeLoader : public QObject
{
    Q_OBJECT
    Q_PROPERTY(QColor base READ base NOTIFY themeChanged)
    Q_PROPERTY(QColor mantle READ mantle NOTIFY themeChanged)
    Q_PROPERTY(QColor crust READ crust NOTIFY themeChanged)
    Q_PROPERTY(QColor surface READ surface NOTIFY themeChanged)
    Q_PROPERTY(QColor overlay READ overlay NOTIFY themeChanged)
    Q_PROPERTY(QColor text READ text NOTIFY themeChanged)
    Q_PROPERTY(QColor subtext READ subtext NOTIFY themeChanged)
    Q_PROPERTY(QColor muted READ muted NOTIFY themeChanged)
    Q_PROPERTY(QColor accent READ accent NOTIFY themeChanged)
    Q_PROPERTY(QColor success READ success NOTIFY themeChanged)
    Q_PROPERTY(QColor warning READ warning NOTIFY themeChanged)
    Q_PROPERTY(QColor error READ error NOTIFY themeChanged)

    // Glass effects properties — optional [effects] section in theme TOML
    Q_PROPERTY(bool hasEffects READ hasEffects NOTIFY themeChanged)
    Q_PROPERTY(double sidebarOpacity READ sidebarOpacity NOTIFY themeChanged)
    Q_PROPERTY(double contentOpacity READ contentOpacity NOTIFY themeChanged)
    Q_PROPERTY(double toolbarOpacity READ toolbarOpacity NOTIFY themeChanged)
    Q_PROPERTY(bool gradientEnabled READ gradientEnabled NOTIFY themeChanged)
    Q_PROPERTY(QColor gradientColor READ gradientColor NOTIFY themeChanged)
    Q_PROPERTY(QString gradientDirection READ gradientDirection NOTIFY themeChanged)
    Q_PROPERTY(bool glowEnabled READ glowEnabled NOTIFY themeChanged)
    Q_PROPERTY(QColor glowColor READ glowColor NOTIFY themeChanged)
    Q_PROPERTY(double glowRadius READ glowRadius NOTIFY themeChanged)
    Q_PROPERTY(double glowOpacity READ glowOpacity NOTIFY themeChanged)
    Q_PROPERTY(bool blurEnabled READ blurEnabled NOTIFY themeChanged)
    Q_PROPERTY(double blurRadius READ blurRadius NOTIFY themeChanged)
    Q_PROPERTY(bool noiseEnabled READ noiseEnabled NOTIFY themeChanged)
    Q_PROPERTY(double noiseOpacity READ noiseOpacity NOTIFY themeChanged)
    Q_PROPERTY(double saturation READ saturation NOTIFY themeChanged)
    Q_PROPERTY(bool refractionEnabled READ refractionEnabled NOTIFY themeChanged)
    Q_PROPERTY(double refractionStrength READ refractionStrength NOTIFY themeChanged)
    Q_PROPERTY(bool compositorBlurAvailable READ compositorBlurAvailable WRITE setCompositorBlurAvailable NOTIFY compositorBlurAvailableChanged)

public:
    explicit ThemeLoader(QObject *parent = nullptr);
    void loadTheme(const QString &nameOrPath, const QStringList &themesDirs);
    QColor color(const QString &name) const;
    QColor base() const { return color("base"); }
    QColor mantle() const { return color("mantle"); }
    QColor crust() const { return color("crust"); }
    QColor surface() const { return color("surface"); }
    QColor overlay() const { return color("overlay"); }
    QColor text() const { return color("text"); }
    QColor subtext() const { return color("subtext"); }
    QColor muted() const { return color("muted"); }
    QColor accent() const { return color("accent"); }
    QColor success() const { return color("success"); }
    QColor warning() const { return color("warning"); }
    QColor error() const { return color("error"); }

    // Glass effects accessors
    bool hasEffects() const { return m_hasEffects; }
    double sidebarOpacity() const { return m_sidebarOpacity; }
    double contentOpacity() const { return m_contentOpacity; }
    double toolbarOpacity() const { return m_toolbarOpacity; }
    bool gradientEnabled() const { return m_gradientEnabled; }
    QColor gradientColor() const { return m_gradientColor; }
    QString gradientDirection() const { return m_gradientDirection; }
    bool glowEnabled() const { return m_glowEnabled; }
    QColor glowColor() const { return m_glowColor; }
    double glowRadius() const { return m_glowRadius; }
    double glowOpacity() const { return m_glowOpacity; }
    bool blurEnabled() const { return m_blurEnabled; }
    double blurRadius() const { return m_blurRadius; }
    bool noiseEnabled() const { return m_noiseEnabled; }
    double noiseOpacity() const { return m_noiseOpacity; }
    double saturation() const { return m_saturation; }
    bool refractionEnabled() const { return m_refractionEnabled; }
    double refractionStrength() const { return m_refractionStrength; }
    bool compositorBlurAvailable() const { return m_compositorBlurAvailable; }
    void setCompositorBlurAvailable(bool available) {
        if (m_compositorBlurAvailable != available) {
            m_compositorBlurAvailable = available;
            emit compositorBlurAvailableChanged();
        }
    }

signals:
    void themeChanged();
    void compositorBlurAvailableChanged();
private:
    void applyThemeFile(const QString &filePath);
    void watchThemeFile(const QString &filePath);
    void resetEffects();

    QMap<QString, QColor> m_colors;
    QFileSystemWatcher m_watcher;
    QString m_watchedPath;
    QDateTime m_watchedModified;
    static QMap<QString, QColor> s_defaults;

    // Glass effects state
    bool m_hasEffects = false;
    bool m_compositorBlurAvailable = false;
    double m_sidebarOpacity = 1.0;
    double m_contentOpacity = 1.0;
    double m_toolbarOpacity = 1.0;
    bool m_gradientEnabled = false;
    QColor m_gradientColor{"#000000"};
    QString m_gradientDirection{"top_to_bottom"};
    bool m_glowEnabled = false;
    QColor m_glowColor{"#ffffff"};
    double m_glowRadius = 0.0;
    double m_glowOpacity = 0.0;
    bool m_blurEnabled = false;
    double m_blurRadius = 0.0;
    bool m_noiseEnabled = false;
    double m_noiseOpacity = 0.0;
    double m_saturation = 1.0;
    bool m_refractionEnabled = false;
    double m_refractionStrength = 0.0;
};
