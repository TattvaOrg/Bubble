use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThemeColors {
    #[serde(default = "default_base")]
    pub base: String,
    #[serde(default = "default_mantle")]
    pub mantle: String,
    #[serde(default = "default_crust")]
    pub crust: String,
    #[serde(default = "default_surface")]
    pub surface: String,
    #[serde(default = "default_overlay")]
    pub overlay: String,
    #[serde(default = "default_text")]
    pub text: String,
    #[serde(default = "default_subtext")]
    pub subtext: String,
    #[serde(default = "default_muted")]
    pub muted: String,
    #[serde(default = "default_accent")]
    pub accent: String,
    #[serde(default = "default_success")]
    pub success: String,
    #[serde(default = "default_warning")]
    pub warning: String,
    #[serde(default = "default_error")]
    pub error: String,
}

fn default_base() -> String {
    "#1e1e2e".to_string()
}
fn default_mantle() -> String {
    "#181825".to_string()
}
fn default_crust() -> String {
    "#11111b".to_string()
}
fn default_surface() -> String {
    "#313244".to_string()
}
fn default_overlay() -> String {
    "#45475a".to_string()
}
fn default_text() -> String {
    "#cdd6f4".to_string()
}
fn default_subtext() -> String {
    "#bac2de".to_string()
}
fn default_muted() -> String {
    "#6c7086".to_string()
}
fn default_accent() -> String {
    "#89b4fa".to_string()
}
fn default_success() -> String {
    "#a6e3a1".to_string()
}
fn default_warning() -> String {
    "#f9e2af".to_string()
}
fn default_error() -> String {
    "#f38ba8".to_string()
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            base: default_base(),
            mantle: default_mantle(),
            crust: default_crust(),
            surface: default_surface(),
            overlay: default_overlay(),
            text: default_text(),
            subtext: default_subtext(),
            muted: default_muted(),
            accent: default_accent(),
            success: default_success(),
            warning: default_warning(),
            error: default_error(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ThemeFile {
    #[serde(default)]
    colors: HashMap<String, String>,
}

impl ThemeColors {
    pub fn get_color(&self, name: &str) -> &str {
        match name {
            "base" => &self.base,
            "mantle" => &self.mantle,
            "crust" => &self.crust,
            "surface" => &self.surface,
            "overlay" => &self.overlay,
            "text" => &self.text,
            "subtext" => &self.subtext,
            "muted" => &self.muted,
            "accent" => &self.accent,
            "success" => &self.success,
            "warning" => &self.warning,
            "error" => &self.error,
            _ => "#ff00ff",
        }
    }

    pub fn parse_toml(content: &str) -> Self {
        let mut theme = Self::default();
        if let Ok(file) = toml::from_str::<ThemeFile>(content) {
            for (key, val) in file.colors {
                match key.as_str() {
                    "base" => theme.base = val,
                    "mantle" => theme.mantle = val,
                    "crust" => theme.crust = val,
                    "surface" => theme.surface = val,
                    "overlay" => theme.overlay = val,
                    "text" => theme.text = val,
                    "subtext" => theme.subtext = val,
                    "muted" => theme.muted = val,
                    "accent" => theme.accent = val,
                    "success" => theme.success = val,
                    "warning" => theme.warning = val,
                    "error" => theme.error = val,
                    _ => {}
                }
            }
        }
        theme
    }

    pub fn load_file(path: &Path) -> Result<Self, std::io::Error> {
        let content = fs::read_to_string(path)?;
        Ok(Self::parse_toml(&content))
    }

    pub fn resolve_and_load(name_or_path: &str, theme_dirs: &[PathBuf]) -> (Self, Option<PathBuf>) {
        let path = Path::new(name_or_path);
        if path.is_file() {
            if let Ok(theme) = Self::load_file(path) {
                return (theme, Some(path.to_path_buf()));
            }
        }

        let filename = if name_or_path.ends_with(".toml") {
            name_or_path.to_string()
        } else {
            format!("{}.toml", name_or_path)
        };

        for dir in theme_dirs {
            let candidate = dir.join(&filename);
            if candidate.is_file() {
                if let Ok(theme) = Self::load_file(&candidate) {
                    return (theme, Some(candidate));
                }
            }
        }

        (Self::default(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_parse() {
        let toml_str = r##"
        [colors]
        base = "#000000"
        accent = "#123456"
        "##;
        let theme = ThemeColors::parse_toml(toml_str);
        assert_eq!(theme.base, "#000000");
        assert_eq!(theme.accent, "#123456");
        // Others should retain default
        assert_eq!(theme.text, "#cdd6f4");
    }

    #[test]
    fn test_theme_color_lookup() {
        let theme = ThemeColors::default();
        assert_eq!(theme.get_color("base"), "#1e1e2e");
        assert_eq!(theme.get_color("non_existent"), "#ff00ff");
    }
}
