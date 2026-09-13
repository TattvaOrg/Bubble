use qmetaobject::*;
use bubble_core::vault::VaultService as CoreVaultService;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct VaultService {
    _base: qt_base_class!(trait QObject),

    pub itemLocked: qt_signal!(path: QString),
    pub itemUnlocked: qt_signal!(path: QString),
    pub lockError: qt_signal!(path: QString, error: QString),
    pub sessionStarted: qt_signal!(path: QString),
    pub sessionEnded: qt_signal!(path: QString),

    pub isLocked: qt_method!(fn(&self, path: QString) -> bool),
    pub isSessionUnlocked: qt_method!(fn(&self, path: QString) -> bool),
    pub lockItem: qt_method!(fn(&mut self, path: QString, password: QString) -> bool),
    pub lockItems: qt_method!(fn(&mut self, paths: QVariant, password: QString) -> bool),
    pub unlockItem: qt_method!(fn(&mut self, path: QString, password: QString) -> bool),
    pub sessionUnlockFolder: qt_method!(fn(&mut self, path: QString, password: QString) -> bool),
    pub sessionRelockFolder: qt_method!(fn(&mut self, path: QString)),
    pub sessionOpenFile: qt_method!(fn(&mut self, path: QString, password: QString) -> bool),
    pub sessionRelockFile: qt_method!(fn(&mut self, path: QString)),
    pub getRemainingLockoutSeconds: qt_method!(fn(&self, path: QString) -> i64),
    pub lastError: qt_method!(fn(&self) -> QString),
    pub changePassword: qt_method!(fn(&mut self, path: QString, old_pass: QString, new_pass: QString) -> bool),
    pub hasOwnPassword: qt_method!(fn(&self, path: QString) -> bool),

    core: Option<Arc<Mutex<CoreVaultService>>>,
    last_err: String,
}

impl VaultService {
    pub fn new(config_dir: PathBuf) -> Self {
        let core = CoreVaultService::new(&config_dir).ok().map(|s| Arc::new(Mutex::new(s)));
        Self {
            core,
            last_err: String::new(),
            ..Default::default()
        }
    }

    pub fn isLocked(&self, path: QString) -> bool {
        let p = path.to_string();
        if let Some(c) = &self.core {
            c.lock().unwrap().is_locked(&p)
        } else {
            false
        }
    }

    pub fn isSessionUnlocked(&self, path: QString) -> bool {
        let p = path.to_string();
        if let Some(c) = &self.core {
            c.lock().unwrap().is_session_unlocked(&p)
        } else {
            false
        }
    }

    pub fn lockItem(&mut self, path: QString, password: QString) -> bool {
        let p = path.to_string();
        let pass = password.to_string();
        if let Some(c) = &self.core {
            let res = c.lock().unwrap().lock_item(&p, &pass);
            match res {
                Ok(_) => {
                    self.itemLocked(path);
                    true
                }
                Err(e) => {
                    self.last_err = e.to_string();
                    self.lockError(path, QString::from(self.last_err.as_str()));
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn lockItems(&mut self, _paths: QVariant, _password: QString) -> bool {
        true
    }

    pub fn unlockItem(&mut self, path: QString, password: QString) -> bool {
        let p = path.to_string();
        let pass = password.to_string();
        if let Some(c) = &self.core {
            let res = c.lock().unwrap().unlock_item(&p, &pass);
            match res {
                Ok(_) => {
                    self.itemUnlocked(path);
                    true
                }
                Err(e) => {
                    self.last_err = e.to_string();
                    self.lockError(path, QString::from(self.last_err.as_str()));
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn sessionUnlockFolder(&mut self, path: QString, password: QString) -> bool {
        let p = path.to_string();
        let pass = password.to_string();
        if let Some(c) = &self.core {
            let res = c.lock().unwrap().session_unlock_folder(&p, &pass);
            match res {
                Ok(_) => {
                    self.sessionStarted(path);
                    true
                }
                Err(e) => {
                    self.last_err = e.to_string();
                    self.lockError(path, QString::from(self.last_err.as_str()));
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn sessionRelockFolder(&mut self, path: QString) {
        let p = path.to_string();
        if let Some(c) = &self.core {
            let _ = c.lock().unwrap().session_relock_folder(&p);
            self.sessionEnded(path);
        }
    }

    pub fn sessionOpenFile(&mut self, path: QString, password: QString) -> bool {
        let p = path.to_string();
        let pass = password.to_string();
        if let Some(c) = &self.core {
            let res = c.lock().unwrap().session_unlock_folder(&p, &pass);
            res.is_ok()
        } else {
            false
        }
    }

    pub fn sessionRelockFile(&mut self, path: QString) {
        let p = path.to_string();
        if let Some(c) = &self.core {
            let _ = c.lock().unwrap().session_relock_folder(&p);
        }
    }

    pub fn getRemainingLockoutSeconds(&self, path: QString) -> i64 {
        let p = path.to_string();
        if let Some(c) = &self.core {
            c.lock().unwrap().get_remaining_lockout_seconds(&p) as i64
        } else {
            0
        }
    }

    pub fn lastError(&self) -> QString {
        QString::from(self.last_err.as_str())
    }

    pub fn changePassword(&mut self, path: QString, old_pass: QString, new_pass: QString) -> bool {
        let p = path.to_string();
        let op = old_pass.to_string();
        let np = new_pass.to_string();
        if let Some(c) = &self.core {
            c.lock().unwrap().change_password(&p, &op, &np).is_ok()
        } else {
            false
        }
    }

    pub fn hasOwnPassword(&self, _path: QString) -> bool {
        true
    }
}
