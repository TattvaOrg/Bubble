use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct GeneralConfig {
    pub theme: Option<String>,
    pub light_theme: String,
    pub dark_theme: String,
    pub icon_theme: String,
    pub font_family: String,
    pub default_view: String,
    pub show_hidden: bool,
    pub right_click_to_edit_path: bool,
    pub sort_by: String,
    pub sort_ascending: bool,
    pub remember_sort_per_folder: bool,
    pub dependency_startup_check: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            theme: None,
            light_theme: "catppuccin-latte".to_string(),
            dark_theme: "catppuccin-mocha".to_string(),
            icon_theme: "Adwaita".to_string(),
            font_family: String::new(),
            default_view: "grid".to_string(),
            show_hidden: false,
            right_click_to_edit_path: true,
            sort_by: "name".to_string(),
            sort_ascending: true,
            remember_sort_per_folder: true,
            dependency_startup_check: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SidebarConfig {
    pub position: String,
    pub width: i32,
    pub visible: bool,
    pub hidden_quick_access: Vec<String>,
}

impl Default for SidebarConfig {
    fn default() -> Self {
        Self {
            position: "left".to_string(),
            width: 200,
            visible: true,
            hidden_quick_access: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppearanceConfig {
    pub radius_small: i32,
    pub radius_medium: i32,
    pub radius_large: i32,
    pub transparency_enabled: bool,
    pub transparency_level: f64,
    pub animations_enabled: bool,
    pub anim_duration_fast: i32,
    pub anim_duration: i32,
    pub anim_duration_slow: i32,
    pub anim_curve_enter: String,
    pub anim_curve_exit: String,
    pub anim_curve_transition: String,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            radius_small: 4,
            radius_medium: 8,
            radius_large: 12,
            transparency_enabled: true,
            transparency_level: 1.0,
            animations_enabled: true,
            anim_duration_fast: 100,
            anim_duration: 200,
            anim_duration_slow: 350,
            anim_curve_enter: "OutCubic".to_string(),
            anim_curve_exit: "InCubic".to_string(),
            anim_curve_transition: "Bezier".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct WindowConfig {
    pub show_controls: Option<bool>,
    pub button_layout: String,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            show_controls: None,
            button_layout: ":minimize,maximize,close".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ListViewConfig {
    pub columns: Vec<String>,
    pub column_widths: HashMap<String, f64>,
}

impl Default for ListViewConfig {
    fn default() -> Self {
        let mut widths = HashMap::new();
        widths.insert("size".to_string(), 110.0);
        widths.insert("modified".to_string(), 140.0);
        widths.insert("type".to_string(), 80.0);
        Self {
            columns: vec![
                "name".to_string(),
                "size".to_string(),
                "modified".to_string(),
                "type".to_string(),
            ],
            column_widths: widths,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MillerViewConfig {
    pub parent_fraction: f64,
    pub current_fraction: f64,
}

impl Default for MillerViewConfig {
    fn default() -> Self {
        Self {
            parent_fraction: 0.2,
            current_fraction: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct BookmarksConfig {
    pub paths: Option<Vec<String>>,
    pub names: HashMap<String, String>,
}

impl Default for BookmarksConfig {
    fn default() -> Self {
        Self {
            paths: None,
            names: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomActionConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct ContextMenuConfig {
    pub actions: Vec<CustomActionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct BubbleConfig {
    pub general: GeneralConfig,
    pub sidebar: SidebarConfig,
    pub appearance: AppearanceConfig,
    pub window: WindowConfig,
    pub list_view: ListViewConfig,
    pub miller_view: MillerViewConfig,
    pub bookmarks: BookmarksConfig,
    pub context_menu: ContextMenuConfig,
    pub shortcuts: HashMap<String, String>,
}

impl BubbleConfig {
    pub fn load_from_path(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        toml::from_str(&content).map_err(|e| format!("TOML syntax error: {}", e))
    }

    pub fn save_to_path(&self, path: &Path) -> io::Result<()> {
        let parent = path.parent().unwrap_or_else(|| Path::new("/"));
        fs::create_dir_all(parent)?;

        let serialized = toml::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Atomic write via tempfile in same directory
        let temp_path = parent.join(format!(".config.tmp.{}", std::process::id()));
        {
            let mut file = File::create(&temp_path)?;
            file.write_all(serialized.as_bytes())?;
            file.sync_all()?;
        }

        fs::rename(&temp_path, path)?;
        Ok(())
    }

    pub fn documented_template() -> &'static str {
        r##"# Bubble configuration — ~/.config/bubble/config.toml
#
# Every key below is at its default; delete or comment out anything you do
# not want to override. Changes are picked up live while Bubble runs.
#
# NOTE: when you change a setting inside Bubble (Settings panel, column
# drags, bookmarks…) the app rewrites this file and these comments are lost.
# config.toml.sample next to it is regenerated on every start and always
# has the full documented set of keys for the version you are running.

[general]
# Colour theme: a file name from /usr/share/bubble/themes or
# ~/.config/bubble/themes without ".toml". Unset = follow the system
# light/dark preference (catppuccin-latte / catppuccin-mocha).
# theme = "catppuccin-mocha"

# The pair the Dark Mode switch in Settings flips between. Point them at any
# two themes; a script can also just rewrite "theme" above and Bubble reloads.
light_theme = "catppuccin-latte"
dark_theme = "catppuccin-mocha"

# Icon theme for file and folder icons (a directory name under
# /usr/share/icons or ~/.icons). Toolbar and sidebar icons are built in.
icon_theme = "Adwaita"

# UI font family. Empty = the desktop's UI font.
font_family = ""

# View for new tabs: "grid" | "detailed" | "miller"
default_view = "grid"

show_hidden = false

# Right click anywhere on the address bar enters path edit mode with the
# whole path selected, just like Ctrl+L. Left clicks on breadcrumb segments
# still navigate.
right_click_to_edit_path = true

# Sort order: "name" | "size" | "modified" | "type"
sort_by = "name"
sort_ascending = true

# Remember a different sort per folder (stored in folder_sort.json).
remember_sort_per_folder = true

# Warn at startup when a tool Bubble uses (gvfs, ffmpeg, bat, pdftoppm…) is missing.
dependency_startup_check = true

[sidebar]
# "left" | "right"
position = "left"
width = 200
visible = true
# Quick-access entries to hide. Valid names:
# "Home", "Recents", "Trash", "Network", "Pictures", "Downloads"
hidden_quick_access = []

[appearance]
# Corner radii in pixels.
radius_small = 4
radius_medium = 8
radius_large = 12

# Window transparency (needs compositor blur rules to look good).
# transparency_level: 0.0 (fully transparent) .. 1.0 (opaque)
transparency_enabled = true
transparency_level = 1.0

# Animations. Durations in milliseconds; curves are Qt easing names:
# Linear | InCubic | OutCubic | InOutCubic | OutBack | InOutQuad | OutQuad
# | OutExpo | InOutExpo | Bezier
animations_enabled = true
anim_duration_fast = 100
anim_duration = 200
anim_duration_slow = 350
anim_curve_enter = "OutCubic"
anim_curve_exit = "InCubic"
anim_curve_transition = "Bezier"

[window]
# Draw minimize/maximize/close buttons in Bubble's own title bar.
# Unset = on only when the compositor provides no decorations.
# show_controls = false
# Button order, ":" separates left from right side.
button_layout = ":minimize,maximize,close"

[list_view]
# Columns of the detailed view, in display order. "name" is always present.
# Available: name, size, modified, type, permissions, owner, group, created,
# accessed, extension, mime, git, symlink
columns = ["name", "size", "modified", "type"]
column_widths = { size = 110.0, modified = 140.0, type = 80.0 }

[miller_view]
# Column widths as fractions of the view; the preview column takes the rest.
# Each column keeps at least 0.12. Drag the lines between columns to change.
parent_fraction = 0.2
current_fraction = 0.5

[bookmarks]
# Sidebar bookmarks. Unset = the XDG user folders that exist on this machine.
# paths = ["~/Documents", "~/Downloads", "~/Pictures", "~/Projects"]
# Custom display names, keyed by an entry of paths. Right-click a bookmark
# and choose "Rename" to set one in-app.
# names = { "~/Projects" = "Work" }

[context_menu]
# Extra entries at the bottom of a file's or folder's right-click menu.

[shortcuts]
# Override any shortcut with a Qt key sequence.
path_bar = "Ctrl+L"
"##
    }

    pub fn seed_sample_file(config_path: &Path) -> io::Result<()> {
        let parent = config_path.parent().unwrap_or_else(|| Path::new("/"));
        fs::create_dir_all(parent)?;
        let sample_path = parent.join("config.toml.sample");
        fs::write(sample_path, Self::documented_template())
    }
}

pub fn load_theme_colors(path: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(val) = toml::from_str::<toml::Value>(&content) {
            if let Some(colors) = val.get("colors").and_then(|c| c.as_table()) {
                for (k, v) in colors {
                    if let Some(color_str) = v.as_str() {
                        map.insert(k.clone(), color_str.to_string());
                    }
                }
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config_roundtrip() {
        let cfg = BubbleConfig::default();
        let serialized = toml::to_string(&cfg).unwrap();
        let deserialized: BubbleConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(cfg, deserialized);
    }

    #[test]
    fn test_save_and_load_config() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("config.toml");

        let mut cfg = BubbleConfig::default();
        cfg.general.icon_theme = "Papirus".to_string();
        cfg.sidebar.width = 250;
        cfg.save_to_path(&path).unwrap();

        let loaded = BubbleConfig::load_from_path(&path).unwrap();
        assert_eq!(loaded.general.icon_theme, "Papirus");
        assert_eq!(loaded.sidebar.width, 250);
    }
}
