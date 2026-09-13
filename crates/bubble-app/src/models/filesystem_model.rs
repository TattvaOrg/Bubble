use qmetaobject::*;
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Local};

#[derive(Clone, Default)]
pub struct FileEntry {
    pub file_name: String,
    pub file_path: String,
    pub file_size: u64,
    pub file_size_text: String,
    pub file_type: String,
    pub file_modified: u64,
    pub file_modified_text: String,
    pub file_permissions: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub file_icon_name: String,
    pub git_status: String,
    pub git_status_icon: String,
    pub has_image_preview: bool,
    pub has_video_preview: bool,
    pub has_pdf_preview: bool,
    pub file_owner: String,
    pub file_group: String,
    pub file_created_text: String,
    pub file_accessed_text: String,
    pub file_extension: String,
    pub mime_type: String,
    pub symlink_target: String,
    pub is_locked: bool,
    pub is_session_unlocked: bool,
}

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct FileSystemModel {
    _base: qt_base_class!(trait QAbstractListModel),

    pub rootPath: qt_property!(QString; NOTIFY rootPathChanged),
    pub showHidden: qt_property!(bool; NOTIFY showHiddenChanged),
    pub fileCount: qt_property!(i32; NOTIFY countsChanged),
    pub folderCount: qt_property!(i32; NOTIFY countsChanged),
    pub diskFree: qt_property!(i64; NOTIFY countsChanged),
    pub diskTotal: qt_property!(i64; NOTIFY countsChanged),
    pub isLoading: qt_property!(bool; NOTIFY isLoadingChanged),

    pub rootPathChanged: qt_signal!(),
    pub showHiddenChanged: qt_signal!(),
    pub countsChanged: qt_signal!(),
    pub isLoadingChanged: qt_signal!(),
    pub watchedDirectoryChanged: qt_signal!(path: QString),

    pub setRootPath: qt_method!(fn(&mut self, path: QString)),
    pub setShowHidden: qt_method!(fn(&mut self, show: bool)),
    pub filePath: qt_method!(fn(&self, row: i32) -> QString),
    pub isDir: qt_method!(fn(&self, row: i32) -> bool),
    pub fileName: qt_method!(fn(&self, row: i32) -> QString),
    pub sortByColumn: qt_method!(fn(&mut self, column: QString, ascending: bool)),
    pub refresh: qt_method!(fn(&mut self)),
    pub fileProperties: qt_method!(fn(&self, path: QString) -> QVariant),
    pub folderItemCounts: qt_method!(fn(&self, paths: QVariant) -> QVariant),
    pub homePath: qt_method!(fn(&self) -> QString),
    pub standardPath: qt_method!(fn(&self, key: QString) -> QString),
    pub pathSuggestions: qt_method!(fn(&self, input: QString, limit: i32) -> QVariant),
    pub availableApps: qt_method!(fn(&self, mime: QString) -> QVariant),
    pub defaultApp: qt_method!(fn(&self, mime: QString) -> QString),
    pub setDefaultApp: qt_method!(fn(&self, mime: QString, desktop_file: QString)),
    pub allInstalledApps: qt_method!(fn(&self) -> QVariant),
    pub setFilePermissions: qt_method!(fn(&mut self, path: QString, owner: i32, group: i32, other: i32) -> bool),

    entries: Vec<FileEntry>,
    sort_column: String,
    sort_ascending: bool,
}

impl FileSystemModel {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| Path::new("/").to_path_buf());
        let mut model = Self {
            rootPath: QString::from(home.to_string_lossy().as_ref()),
            showHidden: false,
            fileCount: 0,
            folderCount: 0,
            diskFree: 100 * 1024 * 1024 * 1024,
            diskTotal: 500 * 1024 * 1024 * 1024,
            isLoading: false,
            sort_column: "name".into(),
            sort_ascending: true,
            ..Default::default()
        };
        model.reload();
        model
    }

    pub fn setRootPath(&mut self, path: QString) {
        let p = path.to_string();
        if p.is_empty() { return; }
        self.rootPath = QString::from(p.as_str());
        self.rootPathChanged();
        self.reload();
    }

    pub fn setShowHidden(&mut self, show: bool) {
        if self.showHidden != show {
            self.showHidden = show;
            self.showHiddenChanged();
            self.reload();
        }
    }

    pub fn filePath(&self, row: i32) -> QString {
        if row >= 0 && (row as usize) < self.entries.len() {
            QString::from(self.entries[row as usize].file_path.as_str())
        } else {
            QString::default()
        }
    }

    pub fn isDir(&self, row: i32) -> bool {
        if row >= 0 && (row as usize) < self.entries.len() {
            self.entries[row as usize].is_dir
        } else {
            false
        }
    }

    pub fn fileName(&self, row: i32) -> QString {
        if row >= 0 && (row as usize) < self.entries.len() {
            QString::from(self.entries[row as usize].file_name.as_str())
        } else {
            QString::default()
        }
    }

    pub fn sortByColumn(&mut self, column: QString, ascending: bool) {
        self.sort_column = column.to_string();
        self.sort_ascending = ascending;
        self.sort_entries();
        (self as &mut dyn QAbstractListModel).begin_reset_model();
        (self as &mut dyn QAbstractListModel).end_reset_model();
    }

    pub fn refresh(&mut self) {
        self.reload();
    }

    fn sort_entries(&mut self) {
        let asc = self.sort_ascending;
        let col = self.sort_column.clone();
        self.entries.sort_by(|a, b| {
            if a.is_dir != b.is_dir {
                return b.is_dir.cmp(&a.is_dir); // Dirs first
            }
            let ord = match col.as_str() {
                "size" => a.file_size.cmp(&b.file_size),
                "modified" => a.file_modified.cmp(&b.file_modified),
                "type" => a.file_type.cmp(&b.file_type),
                _ => a.file_name.to_lowercase().cmp(&b.file_name.to_lowercase()),
            };
            if asc { ord } else { ord.reverse() }
        });
    }

    pub fn reload(&mut self) {
        (self as &mut dyn QAbstractListModel).begin_reset_model();
        self.entries.clear();
        let path_str = self.rootPath.to_string();
        let dir_path = Path::new(&path_str);

        let mut files = 0;
        let mut folders = 0;

        if let Ok(read_dir) = fs::read_dir(dir_path) {
            for entry_res in read_dir.flatten() {
                let name = entry_res.file_name().to_string_lossy().to_string();
                if !self.showHidden && name.starts_with('.') {
                    continue;
                }

                let path = entry_res.path();
                let path_string = path.to_string_lossy().to_string();
                let metadata = entry_res.metadata().ok();

                let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                let is_symlink = entry_res.file_type().map(|t| t.is_symlink()).unwrap_or(false);
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

                let modified = metadata.as_ref().and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs()).unwrap_or(0);

                let modified_text = if modified > 0 {
                    DateTime::from_timestamp(modified as i64, 0)
                        .map(|dt: DateTime<chrono::Utc>| dt.with_timezone(&Local).format("%b %d, %Y %H:%M").to_string())
                        .unwrap_or_default()
                } else {
                    String::new()
                };

                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
                let size_text = if is_dir {
                    String::new()
                } else {
                    format_file_size(size)
                };

                let icon = if is_dir {
                    "folder".to_string()
                } else {
                    resolve_icon(&name, &ext)
                };

                let has_img = ["jpg", "jpeg", "png", "webp", "gif", "svg"].contains(&ext.to_lowercase().as_str());
                let has_vid = ["mp4", "mkv", "webm", "avi", "mov"].contains(&ext.to_lowercase().as_str());
                let has_pdf = ext.to_lowercase() == "pdf";

                if is_dir {
                    folders += 1;
                } else {
                    files += 1;
                }

                self.entries.push(FileEntry {
                    file_name: name,
                    file_path: path_string,
                    file_size: size,
                    file_size_text: size_text,
                    file_type: if is_dir { "Folder".into() } else { ext.to_uppercase() },
                    file_modified: modified,
                    file_modified_text: modified_text,
                    file_permissions: "rw-r--r--".into(),
                    is_dir,
                    is_symlink,
                    file_icon_name: icon,
                    git_status: String::new(),
                    git_status_icon: String::new(),
                    has_image_preview: has_img,
                    has_video_preview: has_vid,
                    has_pdf_preview: has_pdf,
                    file_owner: String::new(),
                    file_group: String::new(),
                    file_created_text: String::new(),
                    file_accessed_text: String::new(),
                    file_extension: ext,
                    mime_type: String::new(),
                    symlink_target: String::new(),
                    is_locked: false,
                    is_session_unlocked: false,
                });
            }
        }

        self.sort_entries();
        (self as &mut dyn QAbstractListModel).end_reset_model();

        self.fileCount = files;
        self.folderCount = folders;
        self.countsChanged();
    }

    pub fn fileProperties(&self, path: QString) -> QVariant {
        let p_str = path.to_string();
        let p = Path::new(&p_str);
        let metadata = fs::metadata(p).ok();
        let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let is_symlink = fs::symlink_metadata(p).map(|m| m.file_type().is_symlink()).unwrap_or(false);
        let symlink_target = if is_symlink {
            fs::read_link(p).map(|l| l.to_string_lossy().to_string()).unwrap_or_default()
        } else {
            String::new()
        };

        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or(&p_str).to_string();
        let parent_dir = p.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();

        let mut props = QVariantMap::default();
        props.insert(QString::from("name"), QString::from(name.as_str()).to_qvariant());
        props.insert(QString::from("path"), QString::from(p_str.as_str()).to_qvariant());
        props.insert(QString::from("parentDir"), QString::from(parent_dir.as_str()).to_qvariant());
        props.insert(QString::from("isDir"), is_dir.to_qvariant());
        props.insert(QString::from("isSymlink"), is_symlink.to_qvariant());
        props.insert(QString::from("isLocked"), false.to_qvariant());
        props.insert(QString::from("symlinkTarget"), QString::from(symlink_target.as_str()).to_qvariant());

        let icon = if is_dir { "folder".to_string() } else { resolve_icon(&name, &ext) };
        props.insert(QString::from("iconName"), QString::from(icon.as_str()).to_qvariant());

        if is_dir {
            let mut contained_items = 0;
            let mut contained_files = 0;
            let mut contained_folders = 0;
            if let Ok(rd) = fs::read_dir(p) {
                for e in rd.flatten() {
                    contained_items += 1;
                    if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        contained_folders += 1;
                    } else {
                        contained_files += 1;
                    }
                }
            }
            props.insert(QString::from("containedItems"), contained_items.to_qvariant());
            props.insert(QString::from("containedFiles"), contained_files.to_qvariant());
            props.insert(QString::from("containedFolders"), contained_folders.to_qvariant());
            let content_text = format!("{} items ({} files, {} folders)", contained_items, contained_files, contained_folders);
            props.insert(QString::from("contentText"), QString::from(content_text.as_str()).to_qvariant());
            props.insert(QString::from("sizeText"), QString::from(format!("{} items", contained_items).as_str()).to_qvariant());
            props.insert(QString::from("size"), (-1i64).to_qvariant());
        } else {
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
            props.insert(QString::from("size"), (size as i64).to_qvariant());
            props.insert(QString::from("sizeText"), QString::from(format_file_size(size).as_str()).to_qvariant());
        }

        // Storage
        let c_path = std::ffi::CString::new(p_str.as_str()).unwrap_or_default();
        let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        if unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) } == 0 {
            let s = unsafe { stat.assume_init() };
            let total = (s.f_blocks as u64).saturating_mul(s.f_frsize as u64);
            let free = (s.f_bavail as u64).saturating_mul(s.f_frsize as u64);
            let used = total.saturating_sub(free);
            let used_pct = if total > 0 { (used as f64) / (total as f64) } else { 0.0 };
            let free_pct = if total > 0 { (free as f64) / (total as f64) } else { 0.0 };
            props.insert(QString::from("diskTotal"), QString::from(format_file_size(total).as_str()).to_qvariant());
            props.insert(QString::from("diskFree"), QString::from(format_file_size(free).as_str()).to_qvariant());
            props.insert(QString::from("diskUsed"), QString::from(format_file_size(used).as_str()).to_qvariant());
            props.insert(QString::from("diskUsedPct"), used_pct.to_qvariant());
            props.insert(QString::from("diskFreePct"), free_pct.to_qvariant());
        }

        let mime = if is_dir {
            "inode/directory".to_string()
        } else {
            match ext.to_lowercase().as_str() {
                "txt" => "text/plain".into(),
                "html" | "htm" => "text/html".into(),
                "pdf" => "application/pdf".into(),
                "png" => "image/png".into(),
                "jpg" | "jpeg" => "image/jpeg".into(),
                "svg" => "image/svg+xml".into(),
                "mp4" => "video/mp4".into(),
                "mp3" => "audio/mpeg".into(),
                "zip" => "application/zip".into(),
                "json" => "application/json".into(),
                _ => "application/octet-stream".into(),
            }
        };
        let ftype = if is_dir { "Folder".to_string() } else { ext.to_uppercase() };
        props.insert(QString::from("mimeType"), QString::from(mime.as_str()).to_qvariant());
        props.insert(QString::from("fileType"), QString::from(ftype.as_str()).to_qvariant());

        let (created_text, modified_text, accessed_text) = if let Some(m) = &metadata {
            let mod_t = m.modified().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()).unwrap_or(0);
            let acc_t = m.accessed().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()).unwrap_or(0);
            let cr_t = m.created().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()).unwrap_or(mod_t);
            let fmt_time = |secs: u64| -> String {
                if secs > 0 {
                    DateTime::from_timestamp(secs as i64, 0)
                        .map(|dt: DateTime<chrono::Utc>| dt.with_timezone(&Local).format("%b %d, %Y %H:%M").to_string())
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            };
            (fmt_time(cr_t), fmt_time(mod_t), fmt_time(acc_t))
        } else {
            (String::new(), String::new(), String::new())
        };
        props.insert(QString::from("created"), QString::from(created_text.as_str()).to_qvariant());
        props.insert(QString::from("modified"), QString::from(modified_text.as_str()).to_qvariant());
        props.insert(QString::from("accessed"), QString::from(accessed_text.as_str()).to_qvariant());

        let mode = metadata.as_ref().map(|m| m.mode()).unwrap_or(0o644);
        let perms = format_permissions(mode);
        props.insert(QString::from("permissions"), QString::from(perms.as_str()).to_qvariant());
        props.insert(QString::from("readable"), ((mode & 0o400) != 0).to_qvariant());
        props.insert(QString::from("writable"), ((mode & 0o200) != 0).to_qvariant());
        props.insert(QString::from("executable"), ((mode & 0o100) != 0).to_qvariant());

        QVariant::from(props)
    }

    pub fn folderItemCounts(&self, paths: QVariant) -> QVariant {
        let mut result = QVariantMap::default();
        if let Some(qlist) = <QVariantList as QMetaType>::from_qvariant(paths) {
            for i in 0..qlist.len() {
                let p = qlist[i].to_qbytearray().to_string();
                if let Ok(rd) = fs::read_dir(&p) {
                    let count = rd.flatten().count();
                    result.insert(QString::from(p.as_str()), (count as i32).to_qvariant());
                }
            }
        }
        QVariant::from(result)
    }

    pub fn homePath(&self) -> QString {
        let home = dirs::home_dir().unwrap_or_else(|| Path::new("/").to_path_buf());
        QString::from(home.to_string_lossy().as_ref())
    }

    pub fn standardPath(&self, key: QString) -> QString {
        let home = dirs::home_dir().unwrap_or_else(|| Path::new("/").to_path_buf());
        let k = key.to_string();
        let path = match k.as_str() {
            "documents" => home.join("Documents"),
            "downloads" => home.join("Downloads"),
            "pictures" => home.join("Pictures"),
            "music" => home.join("Music"),
            "videos" => home.join("Videos"),
            _ => home,
        };
        QString::from(path.to_string_lossy().as_ref())
    }

    pub fn pathSuggestions(&self, input: QString, limit: i32) -> QVariant {
        let mut list = QVariantList::default();
        let inp = input.to_string();
        if inp.is_empty() || limit <= 0 {
            return QVariant::from(list);
        }
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let expanded = if inp.starts_with("~/") {
            home.join(&inp[2..])
        } else if inp == "~" {
            home.clone()
        } else {
            PathBuf::from(&inp)
        };

        let (parent, prefix) = if inp.ends_with('/') {
            (expanded.clone(), String::new())
        } else {
            (expanded.parent().unwrap_or_else(|| Path::new("/")).to_path_buf(),
             expanded.file_name().and_then(|f| f.to_str()).unwrap_or("").to_string())
        };

        if let Ok(rd) = fs::read_dir(parent) {
            let mut count = 0;
            for entry in rd.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                    let full = entry.path().to_string_lossy().to_string();
                    list.push(QString::from(full.as_str()).to_qvariant());
                    count += 1;
                    if count >= limit {
                        break;
                    }
                }
            }
        }
        QVariant::from(list)
    }

    pub fn availableApps(&self, mime: QString) -> QVariant {
        let def = self.defaultApp(mime);
        let def_str = def.to_string();
        let mut filtered = QVariantList::default();

        for (desktop_file, name, icon) in scan_installed_apps() {
            let mut app_map = QVariantMap::default();
            let is_def = desktop_file == def_str;
            app_map.insert(QString::from("desktopFile"), QString::from(desktop_file.as_str()).to_qvariant());
            app_map.insert(QString::from("name"), QString::from(name.as_str()).to_qvariant());
            app_map.insert(QString::from("iconName"), QString::from(icon.as_str()).to_qvariant());
            app_map.insert(QString::from("isDefault"), is_def.to_qvariant());
            filtered.push(QVariant::from(app_map));
        }
        QVariant::from(filtered)
    }

    pub fn defaultApp(&self, mime: QString) -> QString {
        let m = mime.to_string();
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        let mimeapps = config_dir.join("mimeapps.list");
        if let Ok(content) = fs::read_to_string(&mimeapps) {
            let mut in_defaults = false;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('[') {
                    in_defaults = trimmed == "[Default Applications]";
                } else if in_defaults && trimmed.starts_with(&m) {
                    if let Some(eq) = trimmed.find('=') {
                        let app = trimmed[eq + 1..].trim().split(';').next().unwrap_or("");
                        return QString::from(app);
                    }
                }
            }
        }
        QString::default()
    }

    pub fn setDefaultApp(&self, mime: QString, desktop_file: QString) {
        let m = mime.to_string();
        let dt = desktop_file.to_string();
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        let mimeapps = config_dir.join("mimeapps.list");
        let mut lines = Vec::new();
        let mut section_found = false;
        let mut updated = false;

        if let Ok(content) = fs::read_to_string(&mimeapps) {
            let mut in_defaults = false;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed == "[Default Applications]" {
                    in_defaults = true;
                    section_found = true;
                    lines.push(line.to_string());
                } else if in_defaults && trimmed.starts_with('[') {
                    if !updated {
                        lines.push(format!("{}={};", m, dt));
                        updated = true;
                    }
                    in_defaults = false;
                    lines.push(line.to_string());
                } else if in_defaults && trimmed.starts_with(&m) {
                    lines.push(format!("{}={};", m, dt));
                    updated = true;
                } else {
                    lines.push(line.to_string());
                }
            }
            if in_defaults && !updated {
                lines.push(format!("{}={};", m, dt));
            }
        }

        if !section_found {
            lines.push("[Default Applications]".to_string());
            lines.push(format!("{}={};", m, dt));
        }

        let _ = fs::write(mimeapps, lines.join("\n") + "\n");
    }

    pub fn allInstalledApps(&self) -> QVariant {
        let mut apps = QVariantList::default();
        for (desktop_file, name, icon) in scan_installed_apps() {
            let mut app = QVariantMap::default();
            app.insert(QString::from("desktopFile"), QString::from(desktop_file.as_str()).to_qvariant());
            app.insert(QString::from("name"), QString::from(name.as_str()).to_qvariant());
            app.insert(QString::from("iconName"), QString::from(icon.as_str()).to_qvariant());
            apps.push(QVariant::from(app));
        }
        QVariant::from(apps)
    }

    pub fn setFilePermissions(&mut self, path: QString, owner: i32, group: i32, other: i32) -> bool {
        let mode = ((owner & 7) << 6) | ((group & 7) << 3) | (other & 7);
        use std::os::unix::fs::PermissionsExt;
        let p = path.to_string();
        let perms = std::fs::Permissions::from_mode(mode as u32);
        std::fs::set_permissions(p, perms).is_ok()
    }
}

fn scan_installed_apps() -> Vec<(String, String, String)> {
    let mut apps = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let dirs = [
        dirs::home_dir().map(|h| h.join(".local/share/applications")).unwrap_or_else(|| PathBuf::from("/")),
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
    ];
    for d in &dirs {
        if let Ok(rd) = fs::read_dir(d) {
            for entry in rd.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) == Some("desktop") {
                    let file_name = p.file_name().unwrap().to_string_lossy().to_string();
                    if seen.insert(file_name.clone()) {
                        if let Ok(content) = fs::read_to_string(&p) {
                            let mut name = String::new();
                            let mut icon = String::new();
                            let mut no_display = false;
                            for line in content.lines() {
                                if line.starts_with("Name=") && name.is_empty() {
                                    name = line["Name=".len()..].trim().to_string();
                                } else if line.starts_with("Icon=") && icon.is_empty() {
                                    icon = line["Icon=".len()..].trim().to_string();
                                } else if line.starts_with("NoDisplay=true") {
                                    no_display = true;
                                }
                            }
                            if !no_display && !name.is_empty() {
                                apps.push((file_name, name, icon));
                            }
                        }
                    }
                }
            }
        }
    }
    apps
}

fn format_permissions(mode: u32) -> String {
    let r = |bit, ch| if (mode & bit) != 0 { ch } else { '-' };
    let mut s = String::with_capacity(9);
    s.push(r(0o400, 'r'));
    s.push(r(0o200, 'w'));
    s.push(r(0o100, 'x'));
    s.push(r(0o040, 'r'));
    s.push(r(0o020, 'w'));
    s.push(r(0o010, 'x'));
    s.push(r(0o004, 'r'));
    s.push(r(0o002, 'w'));
    s.push(r(0o001, 'x'));
    s
}

fn format_file_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let b = bytes as f64;
    let i = (b.log10() / 1024_f64.log10()).floor() as usize;
    let i = i.min(UNITS.len() - 1);
    let v = b / 1024_f64.powi(i as i32);
    if i == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.1} {}", v, UNITS[i])
    }
}

fn resolve_icon(_name: &str, ext: &str) -> String {
    let e = ext.to_lowercase();
    match e.as_str() {
        "rs" | "cpp" | "c" | "h" | "py" | "js" | "ts" | "qml" | "html" | "css" | "json" | "toml" | "sh" => "text-x-script".into(),
        "pdf" => "application-pdf".into(),
        "zip" | "tar" | "gz" | "xz" | "7z" => "package-x-generic".into(),
        "png" | "jpg" | "jpeg" | "svg" | "webp" => "image-x-generic".into(),
        "mp3" | "flac" | "wav" | "ogg" => "audio-x-generic".into(),
        "mp4" | "mkv" | "webm" | "avi" => "video-x-generic".into(),
        _ => "text-x-generic".into(),
    }
}

impl QAbstractListModel for FileSystemModel {
    fn row_count(&self) -> i32 {
        self.entries.len() as i32
    }

    fn data(&self, index: QModelIndex, role: i32) -> QVariant {
        let r = index.row();
        if r < 0 || (r as usize) >= self.entries.len() {
            return QVariant::default();
        }
        let it = &self.entries[r as usize];
        match role {
            257 => QString::from(it.file_name.as_str()).to_qvariant(),
            258 => QString::from(it.file_path.as_str()).to_qvariant(),
            259 => it.file_size.to_qvariant(),
            260 => QString::from(it.file_size_text.as_str()).to_qvariant(),
            261 => QString::from(it.file_type.as_str()).to_qvariant(),
            262 => it.file_modified.to_qvariant(),
            263 => QString::from(it.file_modified_text.as_str()).to_qvariant(),
            264 => QString::from(it.file_permissions.as_str()).to_qvariant(),
            265 => it.is_dir.to_qvariant(),
            266 => it.is_symlink.to_qvariant(),
            267 => QString::from(it.file_icon_name.as_str()).to_qvariant(),
            268 => QString::from(it.git_status.as_str()).to_qvariant(),
            269 => QString::from(it.git_status_icon.as_str()).to_qvariant(),
            270 => it.has_image_preview.to_qvariant(),
            271 => it.has_video_preview.to_qvariant(),
            272 => it.has_pdf_preview.to_qvariant(),
            273 => QString::from(it.file_owner.as_str()).to_qvariant(),
            274 => QString::from(it.file_group.as_str()).to_qvariant(),
            275 => QString::from(it.file_created_text.as_str()).to_qvariant(),
            276 => QString::from(it.file_accessed_text.as_str()).to_qvariant(),
            277 => QString::from(it.file_extension.as_str()).to_qvariant(),
            278 => QString::from(it.mime_type.as_str()).to_qvariant(),
            279 => QString::from(it.symlink_target.as_str()).to_qvariant(),
            280 => it.is_locked.to_qvariant(),
            281 => it.is_session_unlocked.to_qvariant(),
            _ => QVariant::default(),
        }
    }

    fn role_names(&self) -> HashMap<i32, QByteArray> {
        let mut m = HashMap::new();
        m.insert(257, "fileName".into());
        m.insert(258, "filePath".into());
        m.insert(259, "fileSize".into());
        m.insert(260, "fileSizeText".into());
        m.insert(261, "fileType".into());
        m.insert(262, "fileModified".into());
        m.insert(263, "fileModifiedText".into());
        m.insert(264, "filePermissions".into());
        m.insert(265, "isDir".into());
        m.insert(266, "isSymlink".into());
        m.insert(267, "fileIconName".into());
        m.insert(268, "gitStatus".into());
        m.insert(269, "gitStatusIcon".into());
        m.insert(270, "hasImagePreview".into());
        m.insert(271, "hasVideoPreview".into());
        m.insert(272, "hasPdfPreview".into());
        m.insert(273, "fileOwner".into());
        m.insert(274, "fileGroup".into());
        m.insert(275, "fileCreatedText".into());
        m.insert(276, "fileAccessedText".into());
        m.insert(277, "fileExtension".into());
        m.insert(278, "mimeType".into());
        m.insert(279, "symlinkTarget".into());
        m.insert(280, "isLocked".into());
        m.insert(281, "isSessionUnlocked".into());
        m
    }
}
