#include "services/themeloader.h"
#define TOML_HEADER_ONLY 1
#include "third_party/toml.hpp"
#include <QFile>
#include <QDir>
#include <QFileInfo>
#include <QDebug>

QMap<QString, QColor> ThemeLoader::s_defaults = {
    {"base", QColor("#1e1e2e")}, {"mantle", QColor("#181825")},
    {"crust", QColor("#11111b")}, {"surface", QColor("#313244")},
    {"overlay", QColor("#45475a")}, {"text", QColor("#cdd6f4")},
    {"subtext", QColor("#bac2de")}, {"muted", QColor("#6c7086")},
    {"accent", QColor("#89b4fa")}, {"success", QColor("#a6e3a1")},
    {"warning", QColor("#f9e2af")}, {"error", QColor("#f38ba8")},
};

ThemeLoader::ThemeLoader(QObject *parent) : QObject(parent), m_colors(s_defaults)
{
    // Same trick ConfigManager uses: editors save by writing a temp file and
    // renaming over the target, which gives the theme a new inode and drops a
    // file-only watch. Watch the containing directory too and re-arm from
    // either signal.
    const auto rearmAndReload = [this]() {
        if (m_watchedPath.isEmpty())
            return;
        const QFileInfo info(m_watchedPath);
        if (!info.exists())
            return;
        if (!m_watcher.files().contains(m_watchedPath))
            m_watcher.addPath(m_watchedPath);
        // The directory signal also fires for sibling themes, so only reload
        // when the theme actually in use moved on.
        if (info.lastModified() == m_watchedModified)
            return;
        m_watchedModified = info.lastModified();
        applyThemeFile(m_watchedPath);
    };
    connect(&m_watcher, &QFileSystemWatcher::fileChanged, this, rearmAndReload);
    connect(&m_watcher, &QFileSystemWatcher::directoryChanged, this, rearmAndReload);
}

void ThemeLoader::watchThemeFile(const QString &filePath)
{
    if (filePath == m_watchedPath)
        return;
    if (!m_watcher.files().isEmpty())
        m_watcher.removePaths(m_watcher.files());
    if (!m_watcher.directories().isEmpty())
        m_watcher.removePaths(m_watcher.directories());
    m_watchedPath = filePath;
    m_watchedModified = QFileInfo(filePath).lastModified();
    m_watcher.addPath(filePath);
    m_watcher.addPath(QFileInfo(filePath).absolutePath());
}

void ThemeLoader::resetEffects()
{
    m_hasEffects = false;
    m_sidebarOpacity = 1.0;
    m_contentOpacity = 1.0;
    m_toolbarOpacity = 1.0;
    m_gradientEnabled = false;
    m_gradientColor = QColor("#000000");
    m_gradientDirection = QStringLiteral("top_to_bottom");
    m_glowEnabled = false;
    m_glowColor = QColor("#ffffff");
    m_glowRadius = 0.0;
    m_glowOpacity = 0.0;
    m_blurEnabled = false;
    m_blurRadius = 0.0;
    m_noiseEnabled = false;
    m_noiseOpacity = 0.0;
    m_saturation = 1.0;
    m_refractionEnabled = false;
    m_refractionStrength = 0.0;
}

void ThemeLoader::applyThemeFile(const QString &filePath)
{
    m_colors = s_defaults;
    resetEffects();
    try {
        auto config = toml::parse_file(filePath.toStdString());
        if (auto colors = config["colors"].as_table()) {
            for (const auto &[key, val] : *colors) {
                if (auto v = val.value<std::string>()) {
                    QString colorStr = QString::fromStdString(*v);
                    QColor c(colorStr);
                    if (c.isValid())
                        m_colors[QString::fromStdString(std::string(key))] = c;
                }
            }
        }
        // Parse optional [effects] section for glass themes
        if (auto effects = config["effects"].as_table()) {
            m_hasEffects = true;

            if (auto v = effects->get("sidebar_opacity"))
                m_sidebarOpacity = qBound(0.0, v->value_or(1.0), 1.0);
            if (auto v = effects->get("content_opacity"))
                m_contentOpacity = qBound(0.0, v->value_or(1.0), 1.0);
            if (auto v = effects->get("toolbar_opacity"))
                m_toolbarOpacity = qBound(0.0, v->value_or(1.0), 1.0);

            if (auto v = effects->get("gradient_enabled"))
                m_gradientEnabled = v->value_or(false);
            if (auto v = effects->get("gradient_color")) {
                QColor c(QString::fromStdString(v->value_or(std::string("#000000"))));
                if (c.isValid()) m_gradientColor = c;
            }
            if (auto v = effects->get("gradient_direction"))
                m_gradientDirection = QString::fromStdString(v->value_or(std::string("top_to_bottom")));

            if (auto v = effects->get("glow_enabled"))
                m_glowEnabled = v->value_or(false);
            if (auto v = effects->get("glow_color")) {
                QColor c(QString::fromStdString(v->value_or(std::string("#ffffff"))));
                if (c.isValid()) m_glowColor = c;
            }
            if (auto v = effects->get("glow_radius"))
                m_glowRadius = qBound(0.0, v->value_or(0.0), 64.0);
            if (auto v = effects->get("glow_opacity"))
                m_glowOpacity = qBound(0.0, v->value_or(0.0), 1.0);

            if (auto v = effects->get("blur_enabled"))
                m_blurEnabled = v->value_or(false);
            if (auto v = effects->get("blur_radius"))
                m_blurRadius = qBound(0.0, v->value_or(0.0), 64.0);

            if (auto v = effects->get("noise_enabled"))
                m_noiseEnabled = v->value_or(false);
            if (auto v = effects->get("noise_opacity"))
                m_noiseOpacity = qBound(0.0, v->value_or(0.0), 0.2);

            if (auto v = effects->get("saturation"))
                m_saturation = qBound(0.5, v->value_or(1.0), 2.0);

            if (auto v = effects->get("refraction_enabled"))
                m_refractionEnabled = v->value_or(false);
            if (auto v = effects->get("refraction_strength"))
                m_refractionStrength = qBound(0.0, v->value_or(0.0), 0.1);
        }
    } catch (const toml::parse_error &err) {
        qWarning() << "Theme parse error:" << err.what();
    }
    emit themeChanged();
}

void ThemeLoader::loadTheme(const QString &nameOrPath, const QStringList &themesDirs)
{
    QString filePath;
    if (QFile::exists(nameOrPath)) {
        filePath = nameOrPath;
    } else {
        // First directory wins, so the user's ~/.config themes shadow bundled ones.
        for (const QString &dir : themesDirs) {
            if (dir.isEmpty())
                continue;
            const QString candidate = QDir(dir).filePath(nameOrPath + ".toml");
            if (QFile::exists(candidate)) {
                filePath = candidate;
                break;
            }
        }
    }
    if (filePath.isEmpty() || !QFile::exists(filePath)) {
        qWarning() << "Theme not found:" << nameOrPath;
        m_colors = s_defaults;
        resetEffects();
        emit themeChanged();
        return;
    }
    watchThemeFile(QFileInfo(filePath).absoluteFilePath());
    applyThemeFile(filePath);
}

QColor ThemeLoader::color(const QString &name) const
{
    return m_colors.value(name, s_defaults.value(name, QColor("#ff00ff")));
}
