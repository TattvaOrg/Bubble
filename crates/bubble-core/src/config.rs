use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub const KNOWN_LIST_COLUMNS: &[&str] = &[
    "name", "size", "modified", "type", "permissions", "owner", "group",
    "created", "accessed", "extension", "mime", "git", "symlink",
];

pub const MILLER_MIN_FRACTION: f64 = 0.12;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomContextAction {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeneralConfig {
    pub theme: Option<String>,
    #[serde(default = "default_light_theme")]
    pub light_theme: String,
    #[serde(default = "default_dark_theme")]
    pub dark_theme: String,
    #[serde(default = "default_icon_theme")]
    pub icon_theme: String,
    #[serde(default)]
    pub font_family: String,
    #[serde(default = "default_view")]
    pub default_view: String,
    #[serde(default)]
    pub show_hidden: bool,
    #[serde(default = "default_true")]
    pub right_click_to_edit_path: bool,
    #[serde(default = "default_true")]
    pub dependency_startup_check: bool,
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
    #[serde(default = "default_true")]
    pub sort_ascending: bool,
    #[serde(default = "default_true")]
    pub remember_sort_per_folder: bool,
}

fn default_light_theme() -> String {
    "catppuccin-latte".to_string()
}
fn default_dark_theme() -> String {
    "catppuccin-mocha".to_string()
}
fn default_icon_theme() -> String {
    "Adwaita".to_string()
}
fn default_view() -> String {
    "grid".to_string()
}
fn default_true() -> bool {
    true
}
fn default_sort_by() -> String {
    "name".to_string()
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            theme: None,
            light_theme: default_light_theme(),
            dark_theme: default_dark_theme(),
            icon_theme: default_icon_theme(),
            font_family: String::new(),
            default_view: default_view(),
            show_hidden: false,
            right_click_to_edit_path: true,
            dependency_startup_check: true,
            sort_by: default_sort_by(),
            sort_ascending: true,
            remember_sort_per_folder: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SidebarConfig {
    #[serde(default = "default_sidebar_position")]
    pub position: String,
    #[serde(default = "default_sidebar_width")]
    pub width: i32,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub hidden_quick_access: Vec<String>,
}

fn default_sidebar_position() -> String {
    "left".to_string()
}
fn default_sidebar_width() -> i32 {
    200
}

impl Default for SidebarConfig {
    fn default() -> Self {
        Self {
            position: default_sidebar_position(),
            width: default_sidebar_width(),
            visible: true,
            hidden_quick_access: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppearanceConfig {
    #[serde(default = "default_radius_small")]
    pub radius_small: i32,
    #[serde(default = "default_radius_medium")]
    pub radius_medium: i32,
    #[serde(default = "default_radius_large")]
    pub radius_large: i32,
    #[serde(default = "default_true")]
    pub transparency_enabled: bool,
    #[serde(default = "default_transparency_level")]
    pub transparency_level: f64,
    #[serde(default = "default_true")]
    pub animations_enabled: bool,
    #[serde(default = "default_anim_fast")]
    pub anim_duration_fast: i32,
    #[serde(default = "default_anim_duration")]
    pub anim_duration: i32,
    #[serde(default = "default_anim_slow")]
    pub anim_duration_slow: i32,
    #[serde(default = "default_curve_enter")]
    pub anim_curve_enter: String,
    #[serde(default = "default_curve_exit")]
    pub anim_curve_exit: String,
    #[serde(default = "default_curve_transition")]
    pub anim_curve_transition: String,
}

fn default_radius_small() -> i32 { 4 }
fn default_radius_medium() -> i32 { 8 }
fn default_radius_large() -> i32 { 12 }
fn default_transparency_level() -> f64 { 1.0 }
fn default_anim_fast() -> i32 { 100 }
fn default_anim_duration() -> i32 { 200 }
fn default_anim_slow() -> i32 { 350 }
fn default_curve_enter() -> String { "OutCubic".to_string() }
fn default_curve_exit() -> String { "InCubic".to_string() }
fn default_curve_transition() -> String { "Bezier".to_string() }

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            radius_small: default_radius_small(),
            radius_medium: default_radius_medium(),
            radius_large: default_radius_large(),
            transparency_enabled: true,
            transparency_level: default_transparency_level(),
            animations_enabled: true,
            anim_duration_fast: default_anim_fast(),
            anim_duration: default_anim_duration(),
            anim_duration_slow: default_anim_slow(),
            anim_curve_enter: default_curve_enter(),
            anim_curve_exit: default_curve_exit(),
            anim_curve_transition: default_curve_transition(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowConfig {
    pub show_controls: Option<bool>,
    #[serde(default = "default_button_layout")]
    pub button_layout: String,
}

fn default_button_layout() -> String {
    ":minimize,maximize,close".to_string()
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            show_controls: None,
            button_layout: default_button_layout(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListViewConfig {
    #[serde(default = "default_columns")]
    pub columns: Vec<String>,
    #[serde(default = "default_column_widths")]
    pub column_widths: HashMap<String, i32>,
}

fn default_columns() -> Vec<String> {
    vec!["name".to_string(), "size".to_string(), "modified".to_string(), "type".to_string()]
}

fn default_column_widths() -> HashMap<String, i32> {
    let mut map = HashMap::new();
    map.insert("size".to_string(), 110);
    map.insert("modified".to_string(), 140);
    map.insert("type".to_string(), 80);
    map.insert("permissions".to_string(), 90);
    map.insert("owner".to_string(), 90);
    map.insert("group".to_string(), 90);
    map.insert("created".to_string(), 140);
    map.insert("accessed".to_string(), 140);
    map.insert("extension".to_string(), 70);
    map.insert("mime".to_string(), 160);
    map.insert("git".to_string(), 70);
    map.insert("symlink".to_string(), 180);
    map
}

impl Default for ListViewConfig {
    fn default() -> Self {
        Self {
            columns: default_columns(),
            column_widths: default_column_widths(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MillerViewConfig {
    #[serde(default = "default_miller_parent")]
    pub parent_fraction: f64,
    #[serde(default = "default_miller_current")]
    pub current_fraction: f64,
}

fn default_miller_parent() -> f64 { 0.2 }
fn default_miller_current() -> f64 { 0.5 }

impl Default for MillerViewConfig {
    fn default() -> Self {
        Self {
            parent_fraction: default_miller_parent(),
            current_fraction: default_miller_current(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BookmarksConfig {
    pub paths: Option<Vec<String>>,
    #[serde(default)]
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
pub struct AppConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub sidebar: SidebarConfig,
    #[serde(default)]
    pub appearance: AppearanceConfig,
    #[serde(default)]
    pub window: WindowConfig,
    #[serde(default)]
    pub list_view: ListViewConfig,
    #[serde(default)]
    pub miller_view: MillerViewConfig,
    #[serde(default)]
    pub bookmarks: BookmarksConfig,
    #[serde(default)]
    pub shortcuts: HashMap<String, String>,
    #[serde(default)]
    pub context_menu: Option<ContextMenuConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ContextMenuConfig {
    #[serde(default)]
    pub actions: Vec<CustomContextAction>,
}

pub fn default_shortcuts() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("open".to_string(), "Return".to_string());
    map.insert("back".to_string(), "Alt+Left".to_string());
    map.insert("forward".to_string(), "Alt+Right".to_string());
    map.insert("parent".to_string(), "Alt+Up".to_string());
    map.insert("home".to_string(), "Alt+Home".to_string());
    map.insert("refresh".to_string(), "F5".to_string());
    map.insert("new_tab".to_string(), "Ctrl+T".to_string());
    map.insert("new_window".to_string(), "Ctrl+Alt+N".to_string());
    map.insert("close_tab".to_string(), "Ctrl+W".to_string());
    map.insert("next_tab".to_string(), "Ctrl+Tab".to_string());
    map.insert("previous_tab".to_string(), "Ctrl+Shift+Tab".to_string());
    map.insert("reopen_tab".to_string(), "Ctrl+Shift+T".to_string());
    map.insert("open_in_new_tab".to_string(), "Ctrl+Return".to_string());
    map.insert("open_in_split".to_string(), "Ctrl+Shift+Return".to_string());
    map.insert("copy".to_string(), "Ctrl+C".to_string());
    map.insert("cut".to_string(), "Ctrl+X".to_string());
    map.insert("paste".to_string(), "Ctrl+V".to_string());
    map.insert("rename".to_string(), "F2".to_string());
    map.insert("new_folder".to_string(), "Ctrl+Shift+N".to_string());
    map.insert("new_file".to_string(), "Ctrl+N".to_string());
    map.insert("trash".to_string(), "Delete".to_string());
    map.insert("permanent_delete".to_string(), "Shift+Delete".to_string());
    map.insert("toggle_hidden".to_string(), "Ctrl+H".to_string());
    map.insert("toggle_transparency".to_string(), "Ctrl+Shift+B".to_string());
    map.insert("quick_preview".to_string(), "Space".to_string());
    map.insert("search".to_string(), "Ctrl+F".to_string());
    map.insert("context_menu".to_string(), "Shift+F10".to_string());
    map.insert("open_terminal".to_string(), "Ctrl+Alt+T".to_string());
    map.insert("properties".to_string(), "Alt+Return".to_string());
    map.insert("path_bar".to_string(), "Ctrl+L".to_string());
    map.insert("vault_lock".to_string(), "Ctrl+Shift+L".to_string());
    map.insert("toggle_sidebar".to_string(), "F9".to_string());
    map.insert("split_view".to_string(), "F3".to_string());
    map.insert("focus_next_pane".to_string(), "F6".to_string());
    map.insert("focus_previous_pane".to_string(), "Shift+F6".to_string());
    map.insert("focus_left_pane".to_string(), "Ctrl+Alt+Left".to_string());
    map.insert("focus_right_pane".to_string(), "Ctrl+Alt+Right".to_string());
    map.insert("grid_view".to_string(), "Ctrl+1".to_string());
    map.insert("miller_view".to_string(), "Ctrl+2".to_string());
    map.insert("detailed_view".to_string(), "Ctrl+3".to_string());
    map.insert("select_all".to_string(), "Ctrl+A".to_string());
    map.insert("undo".to_string(), "Ctrl+Z".to_string());
    map.insert("redo".to_string(), "Ctrl+Shift+Z".to_string());
    map.insert("settings".to_string(), "Ctrl+,".to_string());
    map.insert("edit_config".to_string(), "Ctrl+Shift+,".to_string());
    map.insert("keyboard_shortcuts".to_string(), "Ctrl+?".to_string());
    map
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            sidebar: SidebarConfig::default(),
            appearance: AppearanceConfig::default(),
            window: WindowConfig::default(),
            list_view: ListViewConfig::default(),
            miller_view: MillerViewConfig::default(),
            bookmarks: BookmarksConfig::default(),
            shortcuts: default_shortcuts(),
            context_menu: None,
        }
    }
}

impl AppConfig {
    pub fn load_from_str(s: &str) -> Result<Self, toml::de::Error> {
        let mut config: AppConfig = toml::from_str(s)?;
        config.normalize();
        Ok(config)
    }

    pub fn load_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        Ok(Self::load_from_str(&content)?)
    }

    pub fn save_file(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        let serialized = toml::to_string_pretty(self)?;
        fs::write(path, serialized)?;
        Ok(())
    }

    pub fn normalize(&mut self) {
        // Normalize list columns: must include "name", drop unknown/duplicates
        let mut normalized = Vec::new();
        for col in &self.list_view.columns {
            if KNOWN_LIST_COLUMNS.contains(&col.as_str()) && !normalized.contains(col) {
                normalized.push(col.clone());
            }
        }
        if !normalized.contains(&"name".to_string()) {
            normalized.insert(0, "name".to_string());
        }
        if normalized.len() == 1 {
            normalized = default_columns();
        }
        self.list_view.columns = normalized;

        // Ensure widths have min size of 40
        for width in self.list_view.column_widths.values_mut() {
            *width = (*width).max(40);
        }

        // Clamp miller fractions
        self.miller_view.parent_fraction = self
            .miller_view
            .parent_fraction
            .clamp(MILLER_MIN_FRACTION, 1.0 - 2.0 * MILLER_MIN_FRACTION);
        self.miller_view.current_fraction = self
            .miller_view
            .current_fraction
            .clamp(MILLER_MIN_FRACTION, 1.0 - self.miller_view.parent_fraction - MILLER_MIN_FRACTION);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults_and_normalization() {
        let mut config = AppConfig::default();
        config.normalize();
        assert_eq!(config.general.theme, None);
        assert_eq!(config.general.light_theme, "catppuccin-latte");
        assert_eq!(config.sidebar.width, 200);
        assert!(config.list_view.columns.contains(&"name".to_string()));
    }

    #[test]
    fn test_config_load_and_save() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_file = temp_dir.path().join("config.toml");

        let mut config = AppConfig::default();
        config.general.show_hidden = true;
        config.sidebar.width = 250;
        config.save_file(&config_file).unwrap();

        let loaded = AppConfig::load_file(&config_file).unwrap();
        assert_eq!(loaded.general.show_hidden, true);
        assert_eq!(loaded.sidebar.width, 250);
    }
}
