use qmetaobject::*;
use std::path::{Path, PathBuf};

#[derive(QObject, Default)]
pub struct ThemeLoader {
    _base: qt_base_class!(trait QObject),

    pub base: qt_property!(QString; NOTIFY theme_changed),
    pub mantle: qt_property!(QString; NOTIFY theme_changed),
    pub crust: qt_property!(QString; NOTIFY theme_changed),
    pub surface: qt_property!(QString; NOTIFY theme_changed),
    pub overlay: qt_property!(QString; NOTIFY theme_changed),
    pub text: qt_property!(QString; NOTIFY theme_changed),
    pub subtext: qt_property!(QString; NOTIFY theme_changed),
    pub muted: qt_property!(QString; NOTIFY theme_changed),
    pub accent: qt_property!(QString; NOTIFY theme_changed),
    pub success: qt_property!(QString; NOTIFY theme_changed),
    pub warning: qt_property!(QString; NOTIFY theme_changed),
    pub error: qt_property!(QString; NOTIFY theme_changed),

    pub theme_changed: qt_signal!(),

    pub load_theme_named: qt_method!(fn(&mut self, name: QString)),
}

impl ThemeLoader {
    pub fn new() -> Self {
        Self {
            base: QString::from("#1e1e2e"),
            mantle: QString::from("#181825"),
            crust: QString::from("#11111b"),
            surface: QString::from("#313244"),
            overlay: QString::from("#45475a"),
            text: QString::from("#cdd6f4"),
            subtext: QString::from("#bac2de"),
            muted: QString::from("#6c7086"),
            accent: QString::from("#89b4fa"),
            success: QString::from("#a6e3a1"),
            warning: QString::from("#f9e2af"),
            error: QString::from("#f38ba8"),
            ..Default::default()
        }
    }

    pub fn load_theme_named(&mut self, name: QString) {
        let n = name.to_string();
        let dirs = Self::default_theme_dirs();
        self.load_theme(&n, &dirs);
    }

    pub fn default_theme_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Some(config_dir) = dirs::config_dir() {
            dirs.push(config_dir.join("bubble/themes"));
            dirs.push(config_dir.join("hyprfm/themes"));
        }
        if let Some(data_dir) = dirs::data_dir() {
            dirs.push(data_dir.join("bubble/themes"));
        }
        dirs.push(PathBuf::from("themes"));
        dirs.push(PathBuf::from("/usr/share/bubble/themes"));
        dirs.push(PathBuf::from("/usr/local/share/bubble/themes"));
        dirs
    }

    pub fn load_theme(&mut self, theme_name: &str, theme_dirs: &[PathBuf]) {
        let theme_file = if theme_name.ends_with(".toml") {
            theme_name.to_string()
        } else {
            format!("{}.toml", theme_name)
        };

        for dir in theme_dirs {
            let path = dir.join(&theme_file);
            if path.exists() {
                let colors = bubble_core::config::load_theme_colors(&path);
                if !colors.is_empty() {
                    if let Some(c) = colors.get("base") { self.base = QString::from(c.as_str()); }
                    if let Some(c) = colors.get("mantle") { self.mantle = QString::from(c.as_str()); }
                    if let Some(c) = colors.get("crust") { self.crust = QString::from(c.as_str()); }
                    if let Some(c) = colors.get("surface") { self.surface = QString::from(c.as_str()); }
                    if let Some(c) = colors.get("overlay") { self.overlay = QString::from(c.as_str()); }
                    if let Some(c) = colors.get("text") { self.text = QString::from(c.as_str()); }
                    if let Some(c) = colors.get("subtext") { self.subtext = QString::from(c.as_str()); }
                    if let Some(c) = colors.get("muted") { self.muted = QString::from(c.as_str()); }
                    if let Some(c) = colors.get("accent") { self.accent = QString::from(c.as_str()); }
                    if let Some(c) = colors.get("success") { self.success = QString::from(c.as_str()); }
                    if let Some(c) = colors.get("warning") { self.warning = QString::from(c.as_str()); }
                    if let Some(c) = colors.get("error") { self.error = QString::from(c.as_str()); }
                    self.theme_changed();
                    return;
                }
            }
        }
    }
}
