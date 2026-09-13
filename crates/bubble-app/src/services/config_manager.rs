use qmetaobject::*;
use bubble_core::config::BubbleConfig;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const SHORTCUT_SPECS: &[(&str, &str, &str)] = &[
    ("open", "Open", "Return"),
    ("back", "Back", "Alt+Left"),
    ("forward", "Forward", "Alt+Right"),
    ("parent", "Go to Parent", "Alt+Up"),
    ("home", "Home", "Alt+Home"),
    ("refresh", "Refresh", "F5"),
    ("new_tab", "New Tab", "Ctrl+T"),
    ("new_window", "New Window", "Ctrl+Alt+N"),
    ("close_tab", "Close Tab", "Ctrl+W"),
    ("next_tab", "Next Tab", "Ctrl+Tab"),
    ("previous_tab", "Previous Tab", "Ctrl+Shift+Tab"),
    ("reopen_tab", "Reopen Closed Tab", "Ctrl+Shift+T"),
    ("open_in_new_tab", "Open in New Tab", "Ctrl+Return"),
    ("open_in_split", "Open in Split View", "Ctrl+Shift+Return"),
    ("copy", "Copy", "Ctrl+C"),
    ("cut", "Cut", "Ctrl+X"),
    ("paste", "Paste", "Ctrl+V"),
    ("rename", "Rename", "F2"),
    ("new_folder", "New Folder", "Ctrl+Shift+N"),
    ("new_file", "New File", "Ctrl+N"),
    ("trash", "Move to Trash", "Delete"),
    ("permanent_delete", "Permanent Delete", "Shift+Delete"),
    ("toggle_hidden", "Toggle Hidden Files", "Ctrl+H"),
    ("toggle_transparency", "Toggle Transparency", "Ctrl+Shift+B"),
    ("quick_preview", "Quick Preview", "Space"),
    ("search", "Search", "Ctrl+F"),
    ("context_menu", "Show Context Menu", "Shift+F10"),
    ("open_terminal", "Open in Terminal", "Ctrl+Alt+T"),
    ("properties", "Properties", "Alt+Return"),
    ("path_bar", "Focus Path Bar", "Ctrl+L"),
    ("vault_lock", "Lock / Unlock Item", "Ctrl+Shift+L"),
    ("toggle_sidebar", "Toggle Sidebar", "F9"),
    ("split_view", "Toggle Split View", "F3"),
    ("focus_next_pane", "Focus Next Pane", "F6"),
    ("focus_previous_pane", "Focus Previous Pane", "Shift+F6"),
    ("focus_left_pane", "Focus Left Pane", "Ctrl+Alt+Left"),
    ("focus_right_pane", "Focus Right Pane", "Ctrl+Alt+Right"),
    ("grid_view", "Grid View", "Ctrl+1"),
    ("miller_view", "Miller View", "Ctrl+2"),
    ("detailed_view", "Detailed View", "Ctrl+3"),
    ("select_all", "Select All", "Ctrl+A"),
    ("undo", "Undo", "Ctrl+Z"),
    ("redo", "Redo", "Ctrl+Shift+Z"),
    ("settings", "Open Settings", "Ctrl+,"),
    ("edit_config", "Edit config.toml", "Ctrl+Shift+,"),
    ("keyboard_shortcuts", "Open Keyboard Shortcuts", "Ctrl+?"),
];

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct ConfigManager {
    _base: qt_base_class!(trait QObject),

    pub theme: qt_property!(QString; NOTIFY configChanged),
    pub lightTheme: qt_property!(QString; NOTIFY configChanged),
    pub darkTheme: qt_property!(QString; NOTIFY configChanged),
    pub iconTheme: qt_property!(QString; NOTIFY configChanged),
    pub fontFamily: qt_property!(QString; NOTIFY configChanged),
    pub defaultView: qt_property!(QString; NOTIFY configChanged),
    pub showHidden: qt_property!(bool; NOTIFY configChanged),
    pub rightClickToEditPath: qt_property!(bool; NOTIFY configChanged),
    pub sortBy: qt_property!(QString; NOTIFY configChanged),
    pub sortAscending: qt_property!(bool; NOTIFY configChanged),
    pub rememberSortPerFolder: qt_property!(bool; NOTIFY configChanged),
    pub dependencyStartupCheck: qt_property!(bool; NOTIFY configChanged),
    pub sidebarPosition: qt_property!(QString; NOTIFY configChanged),
    pub sidebarWidth: qt_property!(i32; NOTIFY configChanged),
    pub sidebarVisible: qt_property!(bool; NOTIFY configChanged),
    pub hiddenQuickAccess: qt_property!(QVariant; NOTIFY configChanged),

    pub radiusSmall: qt_property!(i32; NOTIFY configChanged),
    pub radiusMedium: qt_property!(i32; NOTIFY configChanged),
    pub radiusLarge: qt_property!(i32; NOTIFY configChanged),
    pub transparencyEnabled: qt_property!(bool; NOTIFY configChanged),
    pub transparencyLevel: qt_property!(f64; NOTIFY configChanged),
    pub animationsEnabled: qt_property!(bool; NOTIFY configChanged),
    pub animDurationFast: qt_property!(i32; NOTIFY configChanged),
    pub animDuration: qt_property!(i32; NOTIFY configChanged),
    pub animDurationSlow: qt_property!(i32; NOTIFY configChanged),
    pub animCurveEnter: qt_property!(QString; NOTIFY configChanged),
    pub animCurveExit: qt_property!(QString; NOTIFY configChanged),
    pub animCurveTransition: qt_property!(QString; NOTIFY configChanged),

    pub showWindowControls: qt_property!(bool; NOTIFY configChanged),
    pub windowButtonLayout: qt_property!(QString; NOTIFY configChanged),
    pub configPath: qt_property!(QString; NOTIFY configChanged),
    pub configError: qt_property!(QString; NOTIFY configErrorChanged),

    pub listColumns: qt_property!(QVariant; NOTIFY listColumnsChanged),
    pub listColumnWidths: qt_property!(QVariant; NOTIFY listColumnsChanged),
    pub millerFractions: qt_property!(QVariant; NOTIFY millerFractionsChanged),
    pub shortcutMap: qt_property!(QVariant; NOTIFY configChanged),
    pub customContextActions: qt_property!(QVariant; NOTIFY configChanged),
    pub shortcutDefinitions: qt_property!(QVariant; NOTIFY configChanged),

    pub configChanged: qt_signal!(),
    pub configErrorChanged: qt_signal!(),
    pub listColumnsChanged: qt_signal!(),
    pub millerFractionsChanged: qt_signal!(),

    pub reload: qt_method!(fn(&mut self)),
    pub saveSettings: qt_method!(fn(&mut self, settings: QVariant)),
    pub saveShortcuts: qt_method!(fn(&mut self, shortcuts: QVariant)),
    pub saveBookmarks: qt_method!(fn(&mut self, paths: QVariant, names: QVariant)),
    pub saveListColumns: qt_method!(fn(&mut self, columns: QVariant, widths: QVariant)),
    pub saveMillerFractions: qt_method!(fn(&mut self, parent: f64, current: f64)),
    pub saveSidebarWidth: qt_method!(fn(&mut self, width: i32)),
    pub folderSortBy: qt_method!(fn(&self, path: QString) -> QString),
    pub folderSortAscending: qt_method!(fn(&self, path: QString) -> bool),
    pub setFolderSort: qt_method!(fn(&mut self, path: QString, sort_by: QString, ascending: bool)),
    pub shortcut: qt_method!(fn(&self, action: QString) -> QString),
    pub keyEventMatches: qt_method!(fn(&self, action: QString, key: i32, modifiers: i32) -> bool),
    pub availableThemes: qt_method!(fn(&self) -> QVariant),
    pub availableFonts: qt_method!(fn(&self) -> QVariant),
    pub availableIconThemes: qt_method!(fn(&self) -> QVariant),

    cfg: BubbleConfig,
    file_path: PathBuf,
    folder_sort: HashMap<String, (String, bool)>,
}

impl ConfigManager {
    pub fn new(path: PathBuf) -> Self {
        let loaded = BubbleConfig::load_from_path(&path).unwrap_or_default();
        let _ = BubbleConfig::seed_sample_file(&path);

        let mut mgr = Self {
            cfg: loaded,
            file_path: path.clone(),
            configPath: QString::from(path.to_string_lossy().as_ref()),
            folder_sort: HashMap::new(),
            ..Default::default()
        };
        mgr.sync_from_cfg();
        mgr
    }

    pub fn sync_from_cfg(&mut self) {
        let c = &self.cfg;
        self.theme = QString::from(c.general.theme.as_deref().unwrap_or("catppuccin-mocha"));
        self.lightTheme = QString::from(c.general.light_theme.as_str());
        self.darkTheme = QString::from(c.general.dark_theme.as_str());
        self.iconTheme = QString::from(c.general.icon_theme.as_str());
        self.fontFamily = QString::from(c.general.font_family.as_str());
        self.defaultView = QString::from(c.general.default_view.as_str());
        self.showHidden = c.general.show_hidden;
        self.rightClickToEditPath = c.general.right_click_to_edit_path;
        self.sortBy = QString::from(c.general.sort_by.as_str());
        self.sortAscending = c.general.sort_ascending;
        self.rememberSortPerFolder = c.general.remember_sort_per_folder;
        self.dependencyStartupCheck = c.general.dependency_startup_check;

        self.sidebarPosition = QString::from(c.sidebar.position.as_str());
        self.sidebarWidth = c.sidebar.width;
        self.sidebarVisible = c.sidebar.visible;

        self.radiusSmall = c.appearance.radius_small;
        self.radiusMedium = c.appearance.radius_medium;
        self.radiusLarge = c.appearance.radius_large;
        self.transparencyEnabled = c.appearance.transparency_enabled;
        self.transparencyLevel = c.appearance.transparency_level;
        self.animationsEnabled = c.appearance.animations_enabled;
        self.animDurationFast = c.appearance.anim_duration_fast;
        self.animDuration = c.appearance.anim_duration;
        self.animDurationSlow = c.appearance.anim_duration_slow;
        self.animCurveEnter = QString::from(c.appearance.anim_curve_enter.as_str());
        self.animCurveExit = QString::from(c.appearance.anim_curve_exit.as_str());
        self.animCurveTransition = QString::from(c.appearance.anim_curve_transition.as_str());

        self.showWindowControls = c.window.show_controls.unwrap_or(true);
        self.windowButtonLayout = QString::from(c.window.button_layout.as_str());

        // Shortcuts map and definitions
        let mut sc_map = QVariantMap::default();
        let mut defs = QVariantList::default();
        for &(act, lbl, def) in SHORTCUT_SPECS {
            let seq = c.shortcuts.get(act).cloned().unwrap_or_else(|| def.to_string());
            sc_map.insert(QString::from(act), QString::from(seq.as_str()).to_qvariant());

            let mut def_item = QVariantMap::default();
            def_item.insert(QString::from("action"), QString::from(act).to_qvariant());
            def_item.insert(QString::from("label"), QString::from(lbl).to_qvariant());
            def_item.insert(QString::from("defaultSequence"), QString::from(def).to_qvariant());
            def_item.insert(QString::from("sequence"), QString::from(seq.as_str()).to_qvariant());
            defs.push(QVariant::from(def_item));
        }
        self.shortcutMap = QVariant::from(sc_map);
        self.shortcutDefinitions = QVariant::from(defs);

        // List columns
        let mut cols = QVariantList::default();
        for col in &c.list_view.columns {
            cols.push(QString::from(col.as_str()).to_qvariant());
        }
        self.listColumns = QVariant::from(cols);

        let mut widths = QVariantMap::default();
        for (col, w) in &c.list_view.column_widths {
            widths.insert(QString::from(col.as_str()), (*w).to_qvariant());
        }
        self.listColumnWidths = QVariant::from(widths);

        // Miller view fractions
        let mut fractions = QVariantMap::default();
        fractions.insert(QString::from("parent"), c.miller_view.parent_fraction.to_qvariant());
        fractions.insert(QString::from("current"), c.miller_view.current_fraction.to_qvariant());
        self.millerFractions = QVariant::from(fractions);

        // Sidebar hidden quick access
        let mut hqa = QVariantList::default();
        for item in &c.sidebar.hidden_quick_access {
            hqa.push(QString::from(item.as_str()).to_qvariant());
        }
        self.hiddenQuickAccess = QVariant::from(hqa);

        // Custom context actions
        let mut acts = QVariantList::default();
        for a in &c.context_menu.actions {
            let mut item = QVariantMap::default();
            item.insert(QString::from("name"), QString::from(a.name.as_str()).to_qvariant());
            item.insert(QString::from("command"), QString::from(a.command.as_str()).to_qvariant());
            acts.push(QVariant::from(item));
        }
        self.customContextActions = QVariant::from(acts);
    }

    pub fn reload(&mut self) {
        if let Ok(c) = BubbleConfig::load_from_path(&self.file_path) {
            self.cfg = c;
            self.configError = QString::default();
            self.sync_from_cfg();
            self.configChanged();
            self.listColumnsChanged();
            self.millerFractionsChanged();
        } else if let Err(e) = BubbleConfig::load_from_path(&self.file_path) {
            self.configError = QString::from(e.as_str());
            self.configErrorChanged();
        }
    }

    pub fn saveSettings(&mut self, _settings: QVariant) {
        let _ = self.cfg.save_to_path(&self.file_path);
        self.configChanged();
    }

    pub fn saveShortcuts(&mut self, _shortcuts: QVariant) {
        let _ = self.cfg.save_to_path(&self.file_path);
        self.configChanged();
    }

    pub fn saveBookmarks(&mut self, _paths: QVariant, _names: QVariant) {
        let _ = self.cfg.save_to_path(&self.file_path);
        self.configChanged();
    }

    pub fn saveListColumns(&mut self, _columns: QVariant, _widths: QVariant) {
        let _ = self.cfg.save_to_path(&self.file_path);
        self.listColumnsChanged();
    }

    pub fn saveMillerFractions(&mut self, parent: f64, current: f64) {
        self.cfg.miller_view.parent_fraction = parent;
        self.cfg.miller_view.current_fraction = current;
        let _ = self.cfg.save_to_path(&self.file_path);
        let mut fractions = QVariantMap::default();
        fractions.insert(QString::from("parent"), parent.to_qvariant());
        fractions.insert(QString::from("current"), current.to_qvariant());
        self.millerFractions = QVariant::from(fractions);
        self.millerFractionsChanged();
    }

    pub fn saveSidebarWidth(&mut self, width: i32) {
        self.cfg.sidebar.width = width;
        self.sidebarWidth = width;
        let _ = self.cfg.save_to_path(&self.file_path);
        self.configChanged();
    }

    pub fn folderSortBy(&self, path: QString) -> QString {
        let p = path.to_string();
        if let Some((by, _)) = self.folder_sort.get(&p) {
            QString::from(by.as_str())
        } else {
            self.sortBy.clone()
        }
    }

    pub fn folderSortAscending(&self, path: QString) -> bool {
        let p = path.to_string();
        if let Some((_, asc)) = self.folder_sort.get(&p) {
            *asc
        } else {
            self.sortAscending
        }
    }

    pub fn setFolderSort(&mut self, path: QString, sort_by: QString, ascending: bool) {
        self.folder_sort.insert(path.to_string(), (sort_by.to_string(), ascending));
    }

    pub fn shortcut(&self, action: QString) -> QString {
        let act = action.to_string();
        if let Some(s) = self.cfg.shortcuts.get(&act) {
            return QString::from(s.as_str());
        }
        for &(a, _, def) in SHORTCUT_SPECS {
            if a == act {
                return QString::from(def);
            }
        }
        QString::default()
    }

    pub fn keyEventMatches(&self, _action: QString, _key: i32, _modifiers: i32) -> bool {
        false
    }

    pub fn availableThemes(&self) -> QVariant {
        let themes = vec![
            "catppuccin-mocha", "catppuccin-latte", "catppuccin-frappe", "catppuccin-macchiato",
            "nord", "dracula", "tokyo-night", "gruvbox-dark", "solarized-dark", "solarized-light",
            "rose-pine", "one-dark",
        ];
        let mut list = QVariantList::default();
        for t in themes {
            list.push(QString::from(t).to_qvariant());
        }
        QVariant::from(list)
    }

    pub fn availableFonts(&self) -> QVariant {
        let fonts = vec![
            "Inter", "Roboto", "Noto Sans", "Cantarell", "Ubuntu", "DejaVu Sans",
            "JetBrains Mono", "Fira Code", "Source Code Pro", "Monospace",
        ];
        let mut list = QVariantList::default();
        for f in fonts {
            list.push(QString::from(f).to_qvariant());
        }
        QVariant::from(list)
    }

    pub fn availableIconThemes(&self) -> QVariant {
        let icons = vec!["Papirus", "Papirus-Dark", "Adwaita", "breeze", "breeze-dark", "hicolor"];
        let mut list = QVariantList::default();
        for i in icons {
            list.push(QString::from(i).to_qvariant());
        }
        QVariant::from(list)
    }
}
