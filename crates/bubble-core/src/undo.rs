use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UndoAction {
    Copy {
        sources: Vec<PathBuf>,
        destination_dir: PathBuf,
        created_paths: Vec<PathBuf>,
    },
    Move {
        pairs: Vec<(PathBuf, PathBuf)>, // (original_src, current_dest)
    },
    Rename {
        old_path: PathBuf,
        new_path: PathBuf,
    },
    Trash {
        pairs: Vec<(PathBuf, PathBuf)>, // (original_path, files_path_in_trash)
    },
    CreateFolder {
        path: PathBuf,
    },
    CreateFile {
        path: PathBuf,
    },
}

#[derive(Debug, Clone)]
pub struct UndoRecord {
    pub action: UndoAction,
    pub description: String,
}

pub struct UndoManager {
    undo_stack: Vec<UndoRecord>,
    redo_stack: Vec<UndoRecord>,
    max_depth: usize,
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new(50)
    }
}

impl UndoManager {
    pub fn new(max_depth: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth,
        }
    }

    pub fn record(&mut self, action: UndoAction, description: impl Into<String>) {
        self.redo_stack.clear();
        if self.undo_stack.len() >= self.max_depth {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(UndoRecord {
            action,
            description: description.into(),
        });
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo(&mut self) -> Result<Option<String>, std::io::Error> {
        let record = match self.undo_stack.pop() {
            Some(r) => r,
            None => return Ok(None),
        };

        let desc = record.description.clone();

        match &record.action {
            UndoAction::Copy { created_paths, .. } => {
                for p in created_paths {
                    if p.is_dir() {
                        let _ = fs::remove_dir_all(p);
                    } else if p.exists() || p.is_symlink() {
                        let _ = fs::remove_file(p);
                    }
                }
            }
            UndoAction::Move { pairs } => {
                for (orig_src, current_dest) in pairs {
                    if current_dest.exists() {
                        let _ = fs::rename(current_dest, orig_src);
                    }
                }
            }
            UndoAction::Rename { old_path, new_path } => {
                if new_path.exists() {
                    let _ = fs::rename(new_path, old_path);
                }
            }
            UndoAction::Trash { pairs } => {
                for (orig_path, files_path) in pairs {
                    if files_path.exists() {
                        if let Some(parent) = orig_path.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        let _ = fs::rename(files_path, orig_path);
                        crate::trash::remove_info(files_path);
                    }
                }
            }
            UndoAction::CreateFolder { path } => {
                if path.is_dir() {
                    let _ = fs::remove_dir_all(path);
                }
            }
            UndoAction::CreateFile { path } => {
                if path.exists() {
                    let _ = fs::remove_file(path);
                }
            }
        }

        self.redo_stack.push(record);
        Ok(Some(desc))
    }

    pub fn redo(&mut self) -> Result<Option<String>, std::io::Error> {
        let record = match self.redo_stack.pop() {
            Some(r) => r,
            None => return Ok(None),
        };

        let desc = record.description.clone();

        match &record.action {
            UndoAction::Copy { sources, destination_dir, .. } => {
                let _ = crate::file_ops::copy_items(sources, destination_dir);
            }
            UndoAction::Move { pairs } => {
                for (orig_src, current_dest) in pairs {
                    if orig_src.exists() {
                        let _ = fs::rename(orig_src, current_dest);
                    }
                }
            }
            UndoAction::Rename { old_path, new_path } => {
                if old_path.exists() {
                    let _ = fs::rename(old_path, new_path);
                }
            }
            UndoAction::Trash { pairs } => {
                for (orig_path, _) in pairs {
                    if orig_path.exists() {
                        let _ = crate::trash::trash_item(orig_path);
                    }
                }
            }
            UndoAction::CreateFolder { path } => {
                let _ = fs::create_dir_all(path);
            }
            UndoAction::CreateFile { path } => {
                if !path.exists() {
                    let _ = fs::File::create(path);
                }
            }
        }

        self.undo_stack.push(record);
        Ok(Some(desc))
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_undo_rename() {
        let tmp = tempdir().unwrap();
        let old = tmp.path().join("first.txt");
        let new = tmp.path().join("second.txt");
        fs::write(&old, "data").unwrap();
        fs::rename(&old, &new).unwrap();

        let mut mgr = UndoManager::default();
        mgr.record(
            UndoAction::Rename {
                old_path: old.clone(),
                new_path: new.clone(),
            },
            "Rename file",
        );

        assert!(mgr.can_undo());
        mgr.undo().unwrap();
        assert!(old.exists());
        assert!(!new.exists());

        assert!(mgr.can_redo());
        mgr.redo().unwrap();
        assert!(!old.exists());
        assert!(new.exists());
    }
}
