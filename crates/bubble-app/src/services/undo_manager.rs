use qmetaobject::*;
use bubble_core::undo::{UndoManager as CoreUndoManager, UndoAction};
use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct UndoManager {
    _base: qt_base_class!(trait QObject),

    pub canUndo: qt_property!(bool; NOTIFY canUndoChanged),
    pub canRedo: qt_property!(bool; NOTIFY canRedoChanged),

    pub canUndoChanged: qt_signal!(),
    pub canRedoChanged: qt_signal!(),

    pub undo: qt_method!(fn(&mut self)),
    pub redo: qt_method!(fn(&mut self)),
    pub rename: qt_method!(fn(&mut self, path: QString, new_name: QString)),
    pub createFolder: qt_method!(fn(&mut self, parent: QString, name: QString)),
    pub createFile: qt_method!(fn(&mut self, parent: QString, name: QString)),
    pub trashFiles: qt_method!(fn(&mut self, paths: QVariant)),
    pub moveResolvedItems: qt_method!(fn(&mut self, items: QVariant) -> i64),
    pub copyResolvedItems: qt_method!(fn(&mut self, items: QVariant) -> i64),
    pub renameResolvedItems: qt_method!(fn(&mut self, operations: QVariant) -> bool),

    core: Arc<Mutex<CoreUndoManager>>,
}

impl UndoManager {
    pub fn new() -> Self {
        Self {
            core: Arc::new(Mutex::new(CoreUndoManager::default())),
            ..Default::default()
        }
    }

    pub fn sync_state(&mut self) {
        let u = self.core.lock().unwrap();
        let cu = u.can_undo();
        let cr = u.can_redo();
        drop(u);
        if self.canUndo != cu {
            self.canUndo = cu;
            self.canUndoChanged();
        }
        if self.canRedo != cr {
            self.canRedo = cr;
            self.canRedoChanged();
        }
    }

    pub fn undo(&mut self) {
        let _ = self.core.lock().unwrap().undo();
        self.sync_state();
    }

    pub fn redo(&mut self) {
        let _ = self.core.lock().unwrap().redo();
        self.sync_state();
    }

    pub fn rename(&mut self, path: QString, new_name: QString) {
        let p = path.to_string();
        let n = new_name.to_string();
        let parent = Path::new(&p).parent().unwrap_or_else(|| Path::new("/"));
        let target = parent.join(&n);
        let _ = std::fs::rename(&p, &target);
        self.core.lock().unwrap().record(UndoAction::Rename {
            old_path: PathBuf::from(&p),
            new_path: target,
        }, "Rename");
        self.sync_state();
    }

    pub fn createFolder(&mut self, parent: QString, name: QString) {
        let p = Path::new(&parent.to_string()).join(&name.to_string());
        let _ = std::fs::create_dir_all(&p);
        self.core.lock().unwrap().record(UndoAction::CreateFolder { path: p }, "Create folder");
        self.sync_state();
    }

    pub fn createFile(&mut self, parent: QString, name: QString) {
        let p = Path::new(&parent.to_string()).join(&name.to_string());
        let _ = std::fs::File::create(&p);
        self.core.lock().unwrap().record(UndoAction::CreateFile { path: p }, "Create file");
        self.sync_state();
    }

    pub fn trashFiles(&mut self, _paths: QVariant) {
        self.sync_state();
    }

    pub fn moveResolvedItems(&mut self, _items: QVariant) -> i64 {
        self.sync_state();
        1
    }

    pub fn copyResolvedItems(&mut self, _items: QVariant) -> i64 {
        self.sync_state();
        1
    }

    pub fn renameResolvedItems(&mut self, _operations: QVariant) -> bool {
        self.sync_state();
        true
    }
}
