use qmetaobject::*;
use bubble_core::file_ops;
use bubble_core::trash;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct FileOperations {
    _base: qt_base_class!(trait QObject),

    pub busy: qt_property!(bool; NOTIFY busyChanged),
    pub progress: qt_property!(f64; NOTIFY progressChanged),
    pub statusText: qt_property!(QString; NOTIFY statusTextChanged),
    pub speed: qt_property!(QString; NOTIFY speedChanged),
    pub eta: qt_property!(QString; NOTIFY etaChanged),
    pub paused: qt_property!(bool; NOTIFY pausedChanged),
    pub currentFile: qt_property!(QString; NOTIFY currentFileChanged),
    pub pendingTargetPaths: qt_property!(QVariant; NOTIFY pathsChanged),
    pub activeTransfers: qt_property!(QVariant; NOTIFY activeTransfersChanged),

    pub busyChanged: qt_signal!(),
    pub progressChanged: qt_signal!(),
    pub statusTextChanged: qt_signal!(),
    pub speedChanged: qt_signal!(),
    pub etaChanged: qt_signal!(),
    pub pausedChanged: qt_signal!(),
    pub currentFileChanged: qt_signal!(),
    pub activeTransfersChanged: qt_signal!(),
    pub pathsChanged: qt_signal!(paths: QVariant),
    pub passwordRequested: qt_signal!(archivePath: QString, destination: QString, retry: bool),
    pub operationFinished: qt_signal!(success: bool, error: QString, operationId: i32),

    pub isTrashPath: qt_method!(fn(&self, path: QString) -> bool),
    pub trashFilesPathFor: qt_method!(fn(&self, path: QString) -> QString),
    pub isRemotePath: qt_method!(fn(&self, path: QString) -> bool),
    pub isSlowPath: qt_method!(fn(&self, path: QString) -> bool),
    pub parentPath: qt_method!(fn(&self, path: QString) -> QString),
    pub pathExists: qt_method!(fn(&self, path: QString) -> bool),
    pub displayNameForPath: qt_method!(fn(&self, path: QString) -> QString),
    pub breadcrumbSegments: qt_method!(fn(&self, path: QString) -> QVariant),
    pub isArchive: qt_method!(fn(&self, path: QString) -> bool),
    pub uniqueNameForDestination: qt_method!(fn(&self, dest_dir: QString, name: QString) -> QString),
    pub transferPlan: qt_method!(fn(&self, paths: QVariant, dest_dir: QString) -> QVariant),
    pub copyFiles: qt_method!(fn(&mut self, sources: QVariant, destination: QString) -> i32),
    pub copyResolvedItems: qt_method!(fn(&mut self, items: QVariant) -> i64),
    pub moveFiles: qt_method!(fn(&mut self, sources: QVariant, destination: QString) -> i32),
    pub moveResolvedItems: qt_method!(fn(&mut self, items: QVariant) -> i64),
    pub deleteFiles: qt_method!(fn(&mut self, paths: QVariant) -> i32),
    pub emptyTrash: qt_method!(fn(&mut self) -> i32),
    pub trashFiles: qt_method!(fn(&mut self, paths: QVariant) -> i32),
    pub restoreFromTrash: qt_method!(fn(&mut self, paths: QVariant) -> i32),
    pub rename: qt_method!(fn(&mut self, path: QString, new_name: QString) -> bool),
    pub renameResolvedItems: qt_method!(fn(&mut self, items: QVariant) -> QVariant),
    pub createFolder: qt_method!(fn(&mut self, parent: QString, name: QString) -> bool),
    pub createFile: qt_method!(fn(&mut self, parent: QString, name: QString) -> bool),
    pub openFile: qt_method!(fn(&self, path: QString)),
    pub openFileWith: qt_method!(fn(&self, path: QString, desktop_file: QString)),
    pub copyPathToClipboard: qt_method!(fn(&self, path: QString)),
    pub openInTerminal: qt_method!(fn(&self, path: QString)),
    pub openInEditor: qt_method!(fn(&self, path: QString)),
    pub openNewWindow: qt_method!(fn(&self, path: QString)),
    pub hasClipboardImage: qt_method!(fn(&self) -> bool),
    pub pasteClipboardImage: qt_method!(fn(&self, dest_dir: QString) -> QString),
    pub pauseTransfer: qt_method!(fn(&mut self, transfer_id: i32)),
    pub resumeTransfer: qt_method!(fn(&mut self, transfer_id: i32)),
    pub cancelTransfer: qt_method!(fn(&mut self, transfer_id: i32)),
    pub compressFiles: qt_method!(fn(&mut self, paths: QVariant, format: QString) -> i32),
    pub extractArchive: qt_method!(fn(&mut self, archive_path: QString, destination: QString) -> i32),
    pub archivePassword: qt_method!(fn(&self, path: QString) -> QString),
    pub cacheArchivePassword: qt_method!(fn(&mut self, path: QString, password: QString)),
    pub clearArchivePassword: qt_method!(fn(&mut self, path: QString)),
    pub newExtractionFolder: qt_method!(fn(&mut self, archive_path: QString) -> QString),
    pub archiveRootFolder: qt_method!(fn(&self, archive_path: QString) -> QString),
    pub runCustomAction: qt_method!(fn(&self, command: QString, paths: QVariant)),
    pub setWallpaper: qt_method!(fn(&self, path: QString)),
    pub setHyprlandRounding: qt_method!(fn(&self, title: QString, radius: i32)),
    pub setHyprlandBorder: qt_method!(fn(&self, title: QString, size: i32)),

    archive_passwords: HashMap<String, String>,
}

impl FileOperations {
    pub fn new() -> Self {
        Self {
            pendingTargetPaths: QVariant::from(QVariantList::default()),
            activeTransfers: QVariant::from(QVariantList::default()),
            ..Default::default()
        }
    }

    pub fn isTrashPath(&self, path: QString) -> bool {
        let p = path.to_string();
        p.starts_with("trash://") || p.contains("/.local/share/Trash")
    }

    pub fn trashFilesPathFor(&self, path: QString) -> QString {
        let p = path.to_string();
        if p.starts_with("trash://") {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
            let sub = p.trim_start_matches("trash:///").trim_start_matches("trash://");
            let files = home.join(".local/share/Trash/files").join(sub);
            QString::from(files.to_string_lossy().as_ref())
        } else {
            path
        }
    }

    pub fn isRemotePath(&self, path: QString) -> bool {
        let p = path.to_string();
        p.starts_with("smb://") || p.starts_with("sftp://") || p.starts_with("nfs://") || p.starts_with("webdav://")
    }

    pub fn isSlowPath(&self, _path: QString) -> bool {
        false
    }

    pub fn parentPath(&self, path: QString) -> QString {
        let p_str = path.to_string();
        let p = Path::new(&p_str);
        if let Some(parent) = p.parent() {
            QString::from(parent.to_string_lossy().as_ref())
        } else {
            QString::from("/")
        }
    }

    pub fn pathExists(&self, path: QString) -> bool {
        let p_str = path.to_string();
        Path::new(&p_str).exists()
    }

    pub fn displayNameForPath(&self, path: QString) -> QString {
        let p_str = path.to_string();
        if p_str == "/" {
            return QString::from("File System");
        }
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        if p_str == home.to_string_lossy() {
            return QString::from("Home");
        }
        if p_str.starts_with("trash://") {
            return QString::from("Trash");
        }
        let p = Path::new(&p_str);
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or(&p_str);
        QString::from(name)
    }

    pub fn breadcrumbSegments(&self, path: QString) -> QVariant {
        let p_str = path.to_string();
        let mut segments = QVariantList::default();
        if p_str.is_empty() {
            return QVariant::from(segments);
        }
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let home_str = home.to_string_lossy().to_string();

        if p_str.starts_with("trash://") {
            let mut tr = QVariantMap::default();
            tr.insert(QString::from("label"), QString::from("Trash").to_qvariant());
            tr.insert(QString::from("fullPath"), QString::from("trash:///").to_qvariant());
            segments.push(QVariant::from(tr));
            return QVariant::from(segments);
        }

        if p_str == "/" {
            let mut root = QVariantMap::default();
            root.insert(QString::from("label"), QString::from("File System").to_qvariant());
            root.insert(QString::from("fullPath"), QString::from("/").to_qvariant());
            segments.push(QVariant::from(root));
            return QVariant::from(segments);
        }

        let (mut accumulated, parts) = if !home_str.is_empty() && (p_str == home_str || p_str.starts_with(&(home_str.clone() + "/"))) {
            let mut h = QVariantMap::default();
            h.insert(QString::from("label"), QString::from("Home").to_qvariant());
            h.insert(QString::from("fullPath"), QString::from(home_str.as_str()).to_qvariant());
            segments.push(QVariant::from(h));
            let sub = &p_str[home_str.len()..];
            (home_str.clone(), sub.split('/').filter(|s| !s.is_empty()).collect::<Vec<_>>())
        } else {
            (String::new(), p_str.split('/').filter(|s| !s.is_empty()).collect::<Vec<_>>())
        };

        for part in parts {
            accumulated.push('/');
            accumulated.push_str(part);
            let mut crumb = QVariantMap::default();
            crumb.insert(QString::from("label"), QString::from(part).to_qvariant());
            crumb.insert(QString::from("fullPath"), QString::from(accumulated.as_str()).to_qvariant());
            segments.push(QVariant::from(crumb));
        }

        QVariant::from(segments)
    }

    pub fn isArchive(&self, path: QString) -> bool {
        let p_str = path.to_string().to_lowercase();
        const ARCHIVE_EXTS: &[&str] = &[
            ".zip", ".tar", ".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst",
            ".tgz", ".txz", ".tzst", ".tbz2", ".7z", ".rar", ".gz", ".xz", ".bz2", ".zst",
        ];
        ARCHIVE_EXTS.iter().any(|ext| p_str.ends_with(ext))
    }

    pub fn uniqueNameForDestination(&self, dest_dir: QString, name: QString) -> QString {
        let dir_str = dest_dir.to_string();
        let dir = Path::new(&dir_str);
        let n = name.to_string();
        let unique = file_ops::unique_name_for_destination(dir, &n, &[]);
        QString::from(unique.as_str())
    }

    pub fn transferPlan(&self, _paths: QVariant, _dest_dir: QString) -> QVariant {
        QVariant::from(QVariantList::default())
    }

    pub fn copyFiles(&mut self, _sources: QVariant, _destination: QString) -> i32 {
        1
    }

    pub fn copyResolvedItems(&mut self, _items: QVariant) -> i64 {
        1
    }

    pub fn moveFiles(&mut self, _sources: QVariant, _destination: QString) -> i32 {
        1
    }

    pub fn moveResolvedItems(&mut self, _items: QVariant) -> i64 {
        1
    }

    pub fn deleteFiles(&mut self, _paths: QVariant) -> i32 {
        1
    }

    pub fn emptyTrash(&mut self) -> i32 {
        let _ = trash::empty_trash();
        1
    }

    pub fn trashFiles(&mut self, _paths: QVariant) -> i32 {
        1
    }

    pub fn restoreFromTrash(&mut self, _paths: QVariant) -> i32 {
        1
    }

    pub fn rename(&mut self, path: QString, new_name: QString) -> bool {
        let p = path.to_string();
        let n = new_name.to_string();
        let old_path = Path::new(&p);
        if let Some(parent) = old_path.parent() {
            let new_path = parent.join(&n);
            std::fs::rename(old_path, new_path).is_ok()
        } else {
            false
        }
    }

    pub fn renameResolvedItems(&mut self, _items: QVariant) -> QVariant {
        let mut map = QVariantMap::default();
        map.insert(QString::from("success"), true.to_qvariant());
        QVariant::from(map)
    }

    pub fn openFile(&self, path: QString) {
        let p = path.to_string();
        let _ = Command::new("xdg-open").arg(&p).spawn();
    }

    pub fn openFileWith(&self, path: QString, desktop_file: QString) {
        let p = path.to_string();
        let d = desktop_file.to_string();
        let _ = Command::new("gtk-launch").arg(&d).arg(&p).spawn();
    }

    pub fn copyPathToClipboard(&self, path: QString) {
        let p = path.to_string();
        let _ = Command::new("wl-copy").arg(&p).spawn();
    }

    pub fn openInTerminal(&self, path: QString) {
        let p = path.to_string();
        let terminals = ["ghostty", "foot", "alacritty", "kitty", "konsole", "gnome-terminal", "xterm"];
        for term in terminals {
            if let Ok(status) = Command::new("which").arg(term).output() {
                if status.status.success() {
                    let _ = Command::new(term).current_dir(&p).spawn();
                    return;
                }
            }
        }
    }

    pub fn openInEditor(&self, path: QString) {
        let p = path.to_string();
        let editors = ["code", "kate", "gedit", "nano"];
        for ed in editors {
            if let Ok(status) = Command::new("which").arg(ed).output() {
                if status.status.success() {
                    let _ = Command::new(ed).arg(&p).spawn();
                    return;
                }
            }
        }
    }

    pub fn openNewWindow(&self, path: QString) {
        let p = path.to_string();
        let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("bubble"));
        let _ = Command::new(current_exe).arg("-n").arg(&p).spawn();
    }

    pub fn createFolder(&mut self, parent: QString, name: QString) -> bool {
        let p = Path::new(&parent.to_string()).join(&name.to_string());
        std::fs::create_dir_all(p).is_ok()
    }

    pub fn createFile(&mut self, parent: QString, name: QString) -> bool {
        let p = Path::new(&parent.to_string()).join(&name.to_string());
        std::fs::File::create(p).is_ok()
    }

    pub fn hasClipboardImage(&self) -> bool {
        false
    }

    pub fn pasteClipboardImage(&self, _dest_dir: QString) -> QString {
        QString::default()
    }

    pub fn pauseTransfer(&mut self, _transfer_id: i32) {}
    pub fn resumeTransfer(&mut self, _transfer_id: i32) {}
    pub fn cancelTransfer(&mut self, _transfer_id: i32) {}

    pub fn compressFiles(&mut self, _paths: QVariant, _format: QString) -> i32 {
        1
    }

    pub fn extractArchive(&mut self, _archive_path: QString, _destination: QString) -> i32 {
        1
    }

    pub fn archivePassword(&self, path: QString) -> QString {
        let p = path.to_string();
        if let Some(pass) = self.archive_passwords.get(&p) {
            QString::from(pass.as_str())
        } else {
            QString::default()
        }
    }

    pub fn cacheArchivePassword(&mut self, path: QString, password: QString) {
        self.archive_passwords.insert(path.to_string(), password.to_string());
    }

    pub fn clearArchivePassword(&mut self, path: QString) {
        self.archive_passwords.remove(&path.to_string());
    }

    pub fn newExtractionFolder(&mut self, archive_path: QString) -> QString {
        let p = archive_path.to_string();
        let path = Path::new(&p);
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("extracted");
        let dest = parent.join(stem);
        let _ = std::fs::create_dir_all(&dest);
        QString::from(dest.to_string_lossy().as_ref())
    }

    pub fn archiveRootFolder(&self, _archive_path: QString) -> QString {
        QString::default()
    }

    pub fn runCustomAction(&self, command: QString, paths: QVariant) {
        let cmd = command.to_string();
        let _ = (cmd, paths);
    }

    pub fn setWallpaper(&self, path: QString) {
        let p = path.to_string();
        let _ = Command::new("swww").arg("img").arg(&p).spawn();
    }

    pub fn setHyprlandRounding(&self, _title: QString, _radius: i32) {}
    pub fn setHyprlandBorder(&self, _title: QString, _size: i32) {}
}
