#![allow(non_snake_case, unused_imports)]

use qmetaobject::*;
use std::env;
use std::path::{Path, PathBuf};
use std::process;

pub mod services;
pub mod models;

use services::*;
use models::*;

const BUBBLE_VERSION: &str = "0.6.1";

fn print_usage() {
    println!(
        "Bubble {} — a modern file manager for Linux\n\n\
        Usage:\n  bubble [options] [path]\n\n\
        Options:\n\
          -n, --new-window   Open a separate window even when a path is given.\n\
          -h, --help         Show this help and exit.\n\
          -v, --version      Show the version and exit.\n",
        BUBBLE_VERSION
    );
}

fn resolve_qml_file() -> PathBuf {
    let mut candidates = vec![
        PathBuf::from("src/qml/Main.qml"),
    ];
    if let Some(data) = dirs::data_dir() {
        candidates.push(data.join("bubble/src/qml/Main.qml"));
        candidates.push(data.join("bubble/qml/Main.qml"));
    }
    candidates.push(PathBuf::from("/usr/local/share/bubble/src/qml/Main.qml"));
    candidates.push(PathBuf::from("/usr/local/share/bubble/qml/Main.qml"));
    candidates.push(PathBuf::from("/usr/share/bubble/src/qml/Main.qml"));
    candidates.push(PathBuf::from("/usr/share/bubble/qml/Main.qml"));

    for c in candidates {
        if c.exists() {
            return c;
        }
    }
    PathBuf::from("src/qml/Main.qml")
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut initial_path: Option<String> = None;
    let mut _new_window = false;

    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage();
                return;
            }
            "-v" | "--version" => {
                println!("bubble {}", BUBBLE_VERSION);
                return;
            }
            "-n" | "--new-window" => {
                _new_window = true;
            }
            other if !other.starts_with('-') => {
                if initial_path.is_none() {
                    let p = PathBuf::from(other);
                    let abs = if p.is_relative() {
                        env::current_dir().map(|cwd| cwd.join(&p)).unwrap_or(p)
                    } else {
                        p
                    };
                    initial_path = Some(abs.to_string_lossy().to_string());
                }
            }
            _ => {}
        }
    }

    println!("Initializing Bubble {} (100% Rust Backend)...", BUBBLE_VERSION);

    // Setup config directory (~/.config/bubble)
    let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    let home_str = home_dir.to_string_lossy().to_string();
    let config_dir = dirs::config_dir()
        .map(|c| c.join("bubble"))
        .unwrap_or_else(|| home_dir.join(".config/bubble"));
    let _ = std::fs::create_dir_all(&config_dir);
    let config_path = config_dir.join("config.toml");

    let initial_open = initial_path.unwrap_or_else(|| home_str.clone());

    // Initialize all models and services
    let config = QObjectBox::new(ConfigManager::new(config_path));
    let theme = QObjectBox::new(ThemeLoader::new());
    let tab_model = QObjectBox::new(TabListModel::new(&initial_open));
    let bookmarks = QObjectBox::new(BookmarkModel::new());
    let file_ops = QObjectBox::new(FileOperations::new());
    let undo_manager = QObjectBox::new(UndoManager::new());
    let clipboard = QObjectBox::new(ClipboardManager::new());
    let drag_helper = QObjectBox::new(DragHelper::new());

    let mut fs_model_obj = FileSystemModel::new();
    fs_model_obj.setRootPath(QString::from(initial_open.as_str()));
    let fs_model = QObjectBox::new(fs_model_obj);

    let split_fs_model = QObjectBox::new(FileSystemModel::new());
    let miller_parent_model = QObjectBox::new(FileSystemModel::new());
    let miller_preview_model = QObjectBox::new(FileSystemModel::new());

    let devices = QObjectBox::new(DeviceModel::new());
    let recent_files = QObjectBox::new(RecentFilesModel::new());
    let search_results = QObjectBox::new(SearchResultsModel::default());
    let search_proxy = QObjectBox::new(SearchProxyModel::new());
    let split_search_results = QObjectBox::new(SearchResultsModel::default());
    let split_search_proxy = QObjectBox::new(SearchProxyModel::new());
    let search_service = QObjectBox::new(SearchService::new());
    let split_search_service = QObjectBox::new(SearchService::new());

    let preview_service = QObjectBox::new(PreviewService::new());
    let metadata_extractor = QObjectBox::new(MetadataExtractor::new());
    let disk_usage_service = QObjectBox::new(DiskUsageService::new());
    let remote_access_service = QObjectBox::new(RemoteAccessService::new());
    let rclone_service = QObjectBox::new(RcloneService::new());
    let runtime_features = QObjectBox::new(RuntimeFeaturesService::new());
    let dependencies = QObjectBox::new(DependencyChecker::new());
    let session_state = QObjectBox::new(SessionState::new());
    let vault_service = QObjectBox::new(VaultService::new(config_dir));

    let mut engine = QmlEngine::new();

    // Add import paths for QML modules (Bubble, Quill, Icons)
    let mut qml_import_paths = vec![
        "src/qml".to_string(),
        "src/qml/Bubble".to_string(),
    ];
    if let Some(data) = dirs::data_dir() {
        qml_import_paths.push(data.join("bubble").to_string_lossy().to_string());
        qml_import_paths.push(data.join("bubble/src/qml").to_string_lossy().to_string());
        qml_import_paths.push(data.join("bubble/src/qml/Bubble").to_string_lossy().to_string());
        qml_import_paths.push(data.join("bubble/qml").to_string_lossy().to_string());
        qml_import_paths.push(data.join("bubble/qml/Bubble").to_string_lossy().to_string());
    }
    qml_import_paths.push("/usr/local/share/bubble".to_string());
    qml_import_paths.push("/usr/local/share/bubble/src/qml".to_string());
    qml_import_paths.push("/usr/local/share/bubble/qml".to_string());
    qml_import_paths.push("/usr/share/bubble".to_string());
    qml_import_paths.push("/usr/share/bubble/src/qml".to_string());
    qml_import_paths.push("/usr/share/bubble/qml".to_string());

    for p in qml_import_paths {
        engine.add_import_path(QString::from(p.as_str()));
    }

    // Register all 29 context properties
    engine.set_object_property("config".into(), config.pinned());
    engine.set_object_property("theme".into(), theme.pinned());
    engine.set_object_property("tabModel".into(), tab_model.pinned());
    engine.set_object_property("bookmarks".into(), bookmarks.pinned());
    engine.set_object_property("fileOps".into(), file_ops.pinned());
    engine.set_object_property("undoManager".into(), undo_manager.pinned());
    engine.set_object_property("clipboard".into(), clipboard.pinned());
    engine.set_object_property("dragHelper".into(), drag_helper.pinned());
    engine.set_object_property("fsModel".into(), fs_model.pinned());
    engine.set_object_property("splitFsModel".into(), split_fs_model.pinned());
    engine.set_object_property("millerParentModel".into(), miller_parent_model.pinned());
    engine.set_object_property("millerPreviewModel".into(), miller_preview_model.pinned());
    engine.set_object_property("devices".into(), devices.pinned());
    engine.set_object_property("recentFiles".into(), recent_files.pinned());
    engine.set_object_property("searchProxy".into(), search_proxy.pinned());
    engine.set_object_property("searchResults".into(), search_results.pinned());
    engine.set_object_property("searchService".into(), search_service.pinned());
    engine.set_object_property("splitSearchProxy".into(), split_search_proxy.pinned());
    engine.set_object_property("splitSearchResults".into(), split_search_results.pinned());
    engine.set_object_property("splitSearchService".into(), split_search_service.pinned());
    engine.set_object_property("previewService".into(), preview_service.pinned());
    engine.set_object_property("metadataExtractor".into(), metadata_extractor.pinned());
    engine.set_object_property("diskUsageService".into(), disk_usage_service.pinned());
    engine.set_object_property("remoteAccessService".into(), remote_access_service.pinned());
    engine.set_object_property("rcloneService".into(), rclone_service.pinned());
    engine.set_object_property("runtimeFeatures".into(), runtime_features.pinned());
    engine.set_object_property("dependencies".into(), dependencies.pinned());
    engine.set_object_property("sessionState".into(), session_state.pinned());
    engine.set_object_property("vault".into(), vault_service.pinned());

    let main_qml = resolve_qml_file();
    println!("Loading QML from: {:?}", main_qml);

    if main_qml.exists() {
        let path_str = main_qml.canonicalize().unwrap_or(main_qml).to_string_lossy().to_string();
        engine.load_file(QString::from(path_str.as_str()));
    } else {
        eprintln!("Warning: Main.qml not found at expected paths.");
    }

    if env::var("BUBBLE_HEADLESS").is_ok() || env::var("CI").is_ok() {
        println!("Headless mode: initialization complete.");
        return;
    }

    engine.exec();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_services_instantiation() {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let home_str = home.to_string_lossy().to_string();
        let config_dir = PathBuf::from("/tmp");

        let _config = ConfigManager::new(config_dir.join("config.toml"));
        let _theme = ThemeLoader::new();
        let _tab_model = TabListModel::new(&home_str);
        let _bookmarks = BookmarkModel::new();
        let _file_ops = FileOperations::new();
        let _undo_manager = UndoManager::new();
        let _clipboard = ClipboardManager::new();
        let _drag_helper = DragHelper::new();
        let _fs_model = FileSystemModel::new();
        let _devices = DeviceModel::new();
        let _recent_files = RecentFilesModel::new();
        let _search_proxy = SearchProxyModel::new();
        let _search_service = SearchService::new();
        let _preview_service = PreviewService::new();
        let _metadata_extractor = MetadataExtractor::new();
        let _disk_usage_service = DiskUsageService::new();
        let _remote_access_service = RemoteAccessService::new();
        let _rclone_service = RcloneService::new();
        let _runtime_features = RuntimeFeaturesService::new();
        let _dependencies = DependencyChecker::new();
        let _session_state = SessionState::new();
        let _vault_service = VaultService::new(config_dir);
    }

    #[test]
    fn test_qml_engine_context_properties() {
        let mut engine = QmlEngine::new();
        engine.add_import_path("../../src/qml".into());

        let theme = QObjectBox::new(ThemeLoader::new());
        let config = QObjectBox::new(ConfigManager::new(PathBuf::from("/tmp/config.toml")));
        let fs_model = QObjectBox::new(FileSystemModel::new());
        let tab_model = QObjectBox::new(TabListModel::new("/tmp"));

        engine.set_object_property("theme".into(), theme.pinned());
        engine.set_object_property("config".into(), config.pinned());
        engine.set_object_property("fsModel".into(), fs_model.pinned());
        engine.set_object_property("tabModel".into(), tab_model.pinned());

        engine.load_data(r#"
            import QtQuick 2.15
            import Bubble 1.0

            Item {
                Component.onCompleted: {
                    console.log("Verified QML execution with Rust backend: theme accent is", Theme.accent);
                }
            }
        "#.into());
    }

    #[test]
    fn test_filesystem_model_operations() {
        let mut fs = FileSystemModel::new();
        fs.setRootPath(QString::from("/tmp"));
        assert_eq!(fs.rootPath.to_string(), "/tmp");

        let props = fs.fileProperties(QString::from("/tmp"));
        let p_map = <QVariantMap as QMetaType>::from_qvariant(props).unwrap();
        assert!(p_map.contains(QString::from("name")));
        assert!(p_map.contains(QString::from("isDir")));
        assert!(p_map.contains(QString::from("permissions")));
        assert!(p_map.contains(QString::from("mimeType")));

        let mut paths_list = QVariantList::default();
        paths_list.push(QString::from("/tmp").to_qvariant());
        let counts = fs.folderItemCounts(paths_list.into());
        assert!(counts.is_valid());

        let apps = fs.allInstalledApps();
        assert!(apps.is_valid());
    }

    #[test]
    fn test_tab_model_lifecycle() {
        let mut tabs = TabListModel::new("/tmp");
        assert_eq!(tabs.count, 1);

        tabs.addTab();
        assert_eq!(tabs.count, 2);

        // Opening an existing path reuses the active tab
        tabs.openPath(QString::from("/tmp"));
        assert_eq!(tabs.count, 2);

        // Opening a new path creates a new tab
        tabs.openPath(QString::from("/usr"));
        assert_eq!(tabs.count, 3);

        tabs.closeTab(2);
        assert_eq!(tabs.count, 2);

        tabs.closeTab(1);
        assert_eq!(tabs.count, 1);
    }

    #[test]
    fn test_config_manager_shortcuts_and_views() {
        let cfg = ConfigManager::new(PathBuf::from("/tmp/test_bubble_config.toml"));
        assert!(!cfg.iconTheme.to_string().is_empty());
        assert!(!cfg.theme.to_string().is_empty());

        let s_map = <QVariantMap as QMetaType>::from_qvariant(cfg.shortcutMap.clone()).unwrap();
        assert!(s_map.contains(QString::from("new_tab")));
        assert!(s_map.contains(QString::from("copy")));
        assert!(s_map.contains(QString::from("toggle_hidden")));

        let s_defs = <QVariantList as QMetaType>::from_qvariant(cfg.shortcutDefinitions.clone()).unwrap();
        assert!(!s_defs.is_empty());

        let cols = <QVariantList as QMetaType>::from_qvariant(cfg.listColumns.clone()).unwrap();
        assert!(!cols.is_empty());
    }

    #[test]
    fn test_bookmark_model_operations() {
        let mut bm = BookmarkModel::new();
        let initial_count = bm.count;
        assert!(initial_count >= 1);

        bm.addBookmark(QString::from("/tmp"));
        assert_eq!(bm.count, initial_count + 1);

        bm.renameBookmark(initial_count, QString::from("Temp Folder"));
        bm.removeBookmark(initial_count);
        assert_eq!(bm.count, initial_count);
    }

    #[test]
    fn test_file_operations_helpers() {
        let fops = FileOperations::new();
        assert!(fops.isArchive(QString::from("package.zip")));
        assert!(fops.isArchive(QString::from("bundle.tar.gz")));
        assert!(!fops.isArchive(QString::from("notes.txt")));

        let name = fops.displayNameForPath(QString::from("/usr/share/bubble"));
        assert_eq!(name.to_string(), "bubble");

        let segs = fops.breadcrumbSegments(QString::from("/usr/share/bubble"));
        let seg_list = <QVariantList as QMetaType>::from_qvariant(segs).unwrap();
        assert!(!seg_list.is_empty());
    }

    #[test]
    fn test_remote_access_service_uri_builder() {
        let ras = RemoteAccessService::new();
        let uri = ras.buildUri(
            QString::from("sftp"),
            QString::from("10.0.0.1"),
            QString::from("/srv/files"),
            QString::from("user"),
            2222,
            QString::default(),
        );
        assert_eq!(uri.to_string(), "sftp://user@10.0.0.1:2222/srv/files");

        let smb_uri = ras.buildUri(
            QString::from("smb"),
            QString::from("nas.local"),
            QString::from("documents"),
            QString::from("guest"),
            0,
            QString::from("public"),
        );
        assert_eq!(smb_uri.to_string(), "smb://guest@nas.local/public/documents");
    }

    #[test]
    fn test_rclone_service_mount_helpers() {
        let rcs = RcloneService::new();
        let mount_path = rcs.getMountPath(QString::from("myremote"));
        assert!(rcs.isRclonePath(mount_path.clone()));
        assert_eq!(rcs.getRemoteNameFromPath(mount_path).to_string(), "myremote");
    }

    #[test]
    fn test_preview_service_loaders() {
        let preview = PreviewService::new();
        let hosts_path = if Path::new("/etc/hosts").exists() { "/etc/hosts" } else { "/etc/passwd" };
        let text_prev = preview.loadTextPreview(QString::from(hosts_path), 2048, 50);
        let map = <QVariantMap as QMetaType>::from_qvariant(text_prev).unwrap();
        assert_eq!(map[QString::from("isBinary")].to_bool(), false);
        let content_str = <QString as QMetaType>::from_qvariant(map[QString::from("content")].clone()).unwrap();
        assert!(!content_str.to_string().is_empty());

        let dir_prev = preview.loadDirectoryPreview(QString::from("/tmp"), 20);
        let dir_map = <QVariantMap as QMetaType>::from_qvariant(dir_prev).unwrap();
        assert!(dir_map.contains(QString::from("entries")));
    }
}
