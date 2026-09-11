use crate::crypto::{self, CryptoError, KEY_SIZE};
use crate::database::{DbError, VaultDatabase, VaultEntry};
use crate::sys;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VaultError {
    #[error("Item is already locked")]
    AlreadyLocked,
    #[error("Item is not locked")]
    NotLocked,
    #[error("Item does not exist: {0}")]
    NotFound(String),
    #[error("Access denied: Incorrect password")]
    AccessDenied,
    #[error("Rate limited: Locked for {0} seconds")]
    RateLimited(u64),
    #[error("Tampering detected: File contents were corrupted or modified on disk. Decryption failed.")]
    Tampered,
    #[error("Database error: {0}")]
    Db(#[from] DbError),
    #[error("Crypto error: {0}")]
    Crypto(#[from] CryptoError),
    #[error("System error: {0}")]
    Sys(#[from] sys::SysError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("General error: {0}")]
    Other(String),
}

struct RateLimitEntry {
    attempts: u32,
    last_attempt_time: u64,
}

struct ActiveFileSession {
    data_key: [u8; KEY_SIZE],
    _original_perms: String,
    _pid: i64,
}

struct ActiveFolderSession {
    data_key: [u8; KEY_SIZE],
    _original_perms: String,
}

pub struct VaultService {
    db: VaultDatabase,
    _config_dir: PathBuf,
    active_sessions: Mutex<HashSet<String>>,
    active_file_sessions: Mutex<HashMap<String, ActiveFileSession>>,
    active_folder_sessions: Mutex<HashMap<String, ActiveFolderSession>>,
    rate_limits: Mutex<HashMap<String, RateLimitEntry>>,
    last_error: Mutex<Option<String>>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl VaultService {
    pub fn new(config_dir: &Path) -> Result<Self, VaultError> {
        let db_path = config_dir.join("vault.db");
        let db = VaultDatabase::open(&db_path)?;
        let _ = db.clear_all_sessions();

        Ok(Self {
            db,
            _config_dir: config_dir.to_path_buf(),
            active_sessions: Mutex::new(HashSet::new()),
            active_file_sessions: Mutex::new(HashMap::new()),
            active_folder_sessions: Mutex::new(HashMap::new()),
            rate_limits: Mutex::new(HashMap::new()),
            last_error: Mutex::new(None),
        })
    }

    pub fn last_error(&self) -> Option<String> {
        self.last_error.lock().unwrap().clone()
    }

    pub fn clear_last_error(&self) {
        *self.last_error.lock().unwrap() = None;
    }

    fn set_last_error(&self, err: &str) {
        *self.last_error.lock().unwrap() = Some(err.to_string());
    }

    pub fn get_remaining_lockout_seconds(&self, path: &str) -> u64 {
        let limits = self.rate_limits.lock().unwrap();
        if let Some(entry) = limits.get(path) {
            if entry.attempts <= 3 {
                return 0;
            }
            let delay_secs = match entry.attempts {
                4 => 5,
                5 => 30,
                6 => 60,
                _ => 300,
            };
            let elapsed = now_secs().saturating_sub(entry.last_attempt_time);
            if elapsed < delay_secs {
                return delay_secs - elapsed;
            }
        }
        0
    }

    fn record_failed_attempt(&self, path: &str) {
        let mut limits = self.rate_limits.lock().unwrap();
        let entry = limits.entry(path.to_string()).or_insert(RateLimitEntry {
            attempts: 0,
            last_attempt_time: 0,
        });
        entry.attempts += 1;
        entry.last_attempt_time = now_secs();
    }

    fn clear_failed_attempts(&self, path: &str) {
        let mut limits = self.rate_limits.lock().unwrap();
        limits.remove(path);
    }

    pub fn is_locked(&self, path: &str) -> bool {
        let _p = Path::new(path);
        let has_entry = match self.db.has_entry(path) {
            Ok(has) => has,
            Err(_) => return false,
        };
        if !has_entry {
            return false;
        }

        // Check if file exists on disk via lstat
        let c_path = match std::ffi::CString::new(path) {
            Ok(c) => c,
            Err(_) => return false,
        };
        let mut st: libc::stat = unsafe { std::mem::zeroed() };
        let ret = unsafe { libc::lstat(c_path.as_ptr(), &mut st) };
        if ret != 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            if errno == libc::ENOENT {
                let _ = self.db.remove_entry(path);
            }
            // If parent folder has mode 0000, lstat returns EACCES, meaning item is locked
            return errno == libc::EACCES;
        }

        let entry = match self.db.find_by_path(path) {
            Ok(Some(e)) => e,
            _ => return false,
        };

        // Inode check: if file replaced on disk, inode will not match
        if entry.inode > 0 && (st.st_ino as i64) != entry.inode {
            let _ = self.db.remove_entry(path);
            return false;
        }

        // Extended attribute check if not in active session
        if !self.is_session_unlocked(path) {
            let attr_name = std::ffi::CString::new("user.bubble.locked").unwrap();
            let mut buf = [0u8; 8];
            let xret = unsafe {
                libc::getxattr(
                    c_path.as_ptr(),
                    attr_name.as_ptr(),
                    buf.as_mut_ptr() as *mut libc::c_void,
                    buf.len(),
                )
            };
            if xret < 0 {
                let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                if errno == libc::ENODATA {
                    let _ = self.db.remove_entry(path);
                    return false;
                }
            }
        }

        true
    }

    pub fn is_session_unlocked(&self, path: &str) -> bool {
        self.active_sessions.lock().unwrap().contains(path)
    }

    pub fn lock_item(&self, path: &str, password: &str) -> Result<(), VaultError> {
        if self.is_locked(path) {
            self.set_last_error("Item is already locked");
            return Err(VaultError::AlreadyLocked);
        }

        let p = Path::new(path);
        if !p.exists() {
            self.set_last_error("Path does not exist");
            return Err(VaultError::NotFound(path.into()));
        }

        if p.is_dir() {
            self.lock_directory(path, password)
        } else {
            self.lock_single_file(path, password, 0, true)
        }
    }

    fn lock_single_file(&self, path: &str, password: &str, parent_id: i64, is_own_password: bool) -> Result<(), VaultError> {
        let p = Path::new(path);
        let perms = sys::get_file_permissions(p)?;
        let inode = sys::get_inode(p)?;

        let data_key = crypto::generate_key();
        let file_iv = crypto::encrypt_file(p, &data_key)?;

        let enc_salt = crypto::generate_salt();
        let pw_key = crypto::derive_key(password, &enc_salt)?;
        let enc_data_key = crypto::encrypt_key(&data_key, &pw_key)?;

        let pw_salt = crypto::generate_salt();
        let pw_hash = crypto::hash_password(password, &pw_salt)?;

        let entry = VaultEntry {
            id: 0,
            path: path.into(),
            item_type: "file".into(),
            parent_id,
            pw_hash: pw_hash.to_vec(),
            pw_salt: pw_salt.to_vec(),
            enc_key: enc_data_key,
            enc_iv: file_iv.to_vec(),
            enc_salt: enc_salt.to_vec(),
            original_perms: perms,
            locked_at: now_secs() as i64,
            is_own_password,
            inode,
        };

        if let Err(e) = self.db.add_entry(&entry) {
            let mut nonce = [0u8; crypto::NONCE_SIZE];
            nonce.copy_from_slice(&file_iv);
            let _ = crypto::decrypt_file(p, &data_key, &nonce);
            return Err(VaultError::Db(e));
        }

        let _ = sys::set_xattr_locked(p, true);
        let _ = sys::set_file_permissions_zero(p);
        let _ = sys::set_immutable(p, true, None);

        Ok(())
    }

    fn lock_directory(&self, path: &str, password: &str) -> Result<(), VaultError> {
        let p = Path::new(path);
        let perms = sys::get_file_permissions(p)?;
        let dir_inode = sys::get_inode(p)?;

        let pw_salt = crypto::generate_salt();
        let pw_hash = crypto::hash_password(password, &pw_salt)?;

        let enc_salt = crypto::generate_salt();
        let pw_key = crypto::derive_key(password, &enc_salt)?;
        let folder_data_key = crypto::generate_key();
        let enc_key = crypto::encrypt_key(&folder_data_key, &pw_key)?;

        let dir_entry = VaultEntry {
            id: 0,
            path: path.into(),
            item_type: "directory".into(),
            parent_id: 0,
            pw_hash: pw_hash.to_vec(),
            pw_salt: pw_salt.to_vec(),
            enc_key,
            enc_iv: Vec::new(),
            enc_salt: enc_salt.to_vec(),
            original_perms: perms,
            locked_at: now_secs() as i64,
            is_own_password: true,
            inode: dir_inode,
        };

        let dir_entry_id = self.db.add_entry(&dir_entry)?;

        // Recursively encrypt children with fast folder master key
        fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        collect_files(&path, files);
                    } else if path.is_file() {
                        files.push(path);
                    }
                }
            }
        }

        let mut child_files = Vec::new();
        collect_files(p, &mut child_files);

        for child in child_files {
            let child_path_str = child.to_string_lossy().into_owned();
            let child_perms = sys::get_file_permissions(&child)?;
            let child_inode = sys::get_inode(&child)?;

            if let Ok(file_iv) = crypto::encrypt_file(&child, &folder_data_key) {
                let child_entry = VaultEntry {
                    id: 0,
                    path: child_path_str,
                    item_type: "file".into(),
                    parent_id: dir_entry_id,
                    pw_hash: pw_hash.to_vec(),
                    pw_salt: pw_salt.to_vec(),
                    enc_key: Vec::new(),
                    enc_iv: file_iv.to_vec(),
                    enc_salt: Vec::new(),
                    original_perms: child_perms,
                    locked_at: dir_entry.locked_at,
                    is_own_password: false,
                    inode: child_inode,
                };
                let _ = self.db.add_entry(&child_entry);
                let _ = sys::set_xattr_locked(&child, true);
                let _ = sys::set_file_permissions_zero(&child);
                let _ = sys::set_immutable(&child, true, None);
            }
        }

        let _ = sys::set_xattr_locked(p, true);
        let _ = sys::set_file_permissions_zero(p);
        let _ = sys::set_immutable(p, true, None);

        Ok(())
    }

    pub fn unlock_item(&self, path: &str, password: &str) -> Result<(), VaultError> {
        let entry = self.db.find_by_path(path)?.ok_or(VaultError::NotLocked)?;

        let lockout = self.get_remaining_lockout_seconds(path);
        if lockout > 0 {
            let msg = format!("Too many failed attempts. Try again in {}s.", lockout);
            self.set_last_error(&msg);
            return Err(VaultError::RateLimited(lockout));
        }

        if !crypto::verify_password(password, &entry.pw_hash, &entry.pw_salt) {
            self.record_failed_attempt(path);
            let next_lockout = self.get_remaining_lockout_seconds(path);
            if next_lockout > 0 {
                let msg = format!("Incorrect password. Locked for {}s.", next_lockout);
                self.set_last_error(&msg);
                return Err(VaultError::RateLimited(next_lockout));
            } else {
                self.set_last_error("Access Denied: Incorrect password.");
                return Err(VaultError::AccessDenied);
            }
        }
        self.clear_failed_attempts(path);

        if entry.item_type == "directory" {
            self.unlock_directory(path, password, &entry)
        } else {
            self.unlock_single_file(path, password, &entry)
        }
    }

    fn unlock_single_file(&self, path: &str, password: &str, entry: &VaultEntry) -> Result<(), VaultError> {
        let p = Path::new(path);
        let pw_key = crypto::derive_key(password, &entry.enc_salt)?;
        let data_key = crypto::decrypt_key(&entry.enc_key, &pw_key)?;

        let _ = sys::set_immutable(p, false, None);
        let _ = sys::restore_file_permissions(p, &entry.original_perms);

        let mut nonce = [0u8; crypto::NONCE_SIZE];
        if entry.enc_iv.len() == crypto::NONCE_SIZE {
            nonce.copy_from_slice(&entry.enc_iv);
        }

        if let Err(_e) = crypto::decrypt_file(p, &data_key, &nonce) {
            let _ = sys::set_file_permissions_zero(p);
            let _ = sys::set_immutable(p, true, None);
            let msg = "Tampering detected: File contents were corrupted or modified on disk. Decryption failed.";
            self.set_last_error(msg);
            return Err(VaultError::Tampered);
        }

        let _ = sys::set_xattr_locked(p, false);
        let _ = self.db.remove_entry(path);
        self.clear_last_error();

        Ok(())
    }

    fn unlock_directory(&self, path: &str, password: &str, dir_entry: &VaultEntry) -> Result<(), VaultError> {
        let p = Path::new(path);
        let pw_key = crypto::derive_key(password, &dir_entry.enc_salt)?;
        let folder_data_key = crypto::decrypt_key(&dir_entry.enc_key, &pw_key)?;

        let _ = sys::set_immutable(p, false, None);
        let _ = sys::restore_file_permissions(p, if dir_entry.original_perms.is_empty() { "0755" } else { &dir_entry.original_perms });
        let _ = sys::set_xattr_locked(p, false);

        let children = self.db.find_by_parent_id(dir_entry.id)?;
        for child in children {
            let child_path = Path::new(&child.path);
            let _ = sys::set_immutable(child_path, false, None);
            let _ = sys::restore_file_permissions(child_path, if child.original_perms.is_empty() { "0644" } else { &child.original_perms });
            let _ = sys::set_xattr_locked(child_path, false);

            if child.enc_iv.len() == crypto::NONCE_SIZE {
                let mut nonce = [0u8; crypto::NONCE_SIZE];
                nonce.copy_from_slice(&child.enc_iv);
                let _ = crypto::decrypt_file(child_path, &folder_data_key, &nonce);
            }
            let _ = self.db.remove_entry(&child.path);
        }

        let _ = self.db.remove_entry(path);
        self.active_sessions.lock().unwrap().remove(path);
        self.active_folder_sessions.lock().unwrap().remove(path);
        self.clear_last_error();

        Ok(())
    }

    pub fn session_unlock_folder(&self, path: &str, password: &str) -> Result<(), VaultError> {
        let entry = self.db.find_by_path(path)?.ok_or(VaultError::NotLocked)?;
        if entry.item_type != "directory" {
            return Err(VaultError::Other("Item is not a folder".into()));
        }

        let lockout = self.get_remaining_lockout_seconds(path);
        if lockout > 0 {
            return Err(VaultError::RateLimited(lockout));
        }

        if !crypto::verify_password(password, &entry.pw_hash, &entry.pw_salt) {
            self.record_failed_attempt(path);
            return Err(VaultError::AccessDenied);
        }
        self.clear_failed_attempts(path);

        let pw_key = crypto::derive_key(password, &entry.enc_salt)?;
        let folder_data_key = crypto::decrypt_key(&entry.enc_key, &pw_key)?;

        let p = Path::new(path);
        let _ = sys::set_immutable(p, false, None);
        let _ = sys::restore_file_permissions(p, if entry.original_perms.is_empty() { "0755" } else { &entry.original_perms });

        let children = self.db.find_by_parent_id(entry.id)?;
        for child in children {
            let child_path = Path::new(&child.path);
            let _ = sys::set_immutable(child_path, false, None);
            let _ = sys::restore_file_permissions(child_path, if child.original_perms.is_empty() { "0644" } else { &child.original_perms });
            if child.enc_iv.len() == crypto::NONCE_SIZE {
                let mut nonce = [0u8; crypto::NONCE_SIZE];
                nonce.copy_from_slice(&child.enc_iv);
                let _ = crypto::decrypt_file(child_path, &folder_data_key, &nonce);
            }
            let _ = sys::set_xattr_locked(child_path, false);
        }

        self.active_folder_sessions.lock().unwrap().insert(
            path.to_string(),
            ActiveFolderSession {
                data_key: folder_data_key,
                _original_perms: entry.original_perms.clone(),
            },
        );
        self.active_sessions.lock().unwrap().insert(path.to_string());
        let _ = self.db.add_session(entry.id, &format!("{}", now_secs()));

        Ok(())
    }

    pub fn session_relock_folder(&self, path: &str) -> Result<(), VaultError> {
        let folder_sess = self.active_folder_sessions.lock().unwrap().remove(path);
        self.active_sessions.lock().unwrap().remove(path);

        let p = Path::new(path);
        if let Some(entry) = self.db.find_by_path(path)? {
            if let Some(sess) = folder_sess {
                let children = self.db.find_by_parent_id(entry.id)?;
                for mut child in children {
                    let child_path = Path::new(&child.path);
                    if child_path.exists() {
                        if let Ok(new_iv) = crypto::encrypt_file(child_path, &sess.data_key) {
                            child.enc_iv = new_iv.to_vec();
                            let _ = self.db.update_entry(&child);
                        }
                        let _ = sys::set_xattr_locked(child_path, true);
                        let _ = sys::set_file_permissions_zero(child_path);
                        let _ = sys::set_immutable(child_path, true, None);
                    }
                }
            }

            let _ = sys::set_file_permissions_zero(p);
            let _ = sys::set_immutable(p, true, None);
            let _ = self.db.remove_session(entry.id);
        }

        Ok(())
    }

    pub fn session_unlock_file(&self, path: &str, password: &str) -> Result<(), VaultError> {
        let entry = self.db.find_by_path(path)?.ok_or(VaultError::NotLocked)?;
        if entry.item_type != "file" {
            return Err(VaultError::Other("Item is not a file".into()));
        }

        let lockout = self.get_remaining_lockout_seconds(path);
        if lockout > 0 {
            return Err(VaultError::RateLimited(lockout));
        }

        if !crypto::verify_password(password, &entry.pw_hash, &entry.pw_salt) {
            self.record_failed_attempt(path);
            return Err(VaultError::AccessDenied);
        }
        self.clear_failed_attempts(path);

        let pw_key = crypto::derive_key(password, &entry.enc_salt)?;
        let data_key = crypto::decrypt_key(&entry.enc_key, &pw_key)?;

        let p = Path::new(path);
        let _ = sys::set_immutable(p, false, None);
        let _ = sys::restore_file_permissions(p, &entry.original_perms);

        let mut nonce = [0u8; crypto::NONCE_SIZE];
        if entry.enc_iv.len() == crypto::NONCE_SIZE {
            nonce.copy_from_slice(&entry.enc_iv);
        }

        if let Err(_) = crypto::decrypt_file(p, &data_key, &nonce) {
            let _ = sys::set_file_permissions_zero(p);
            let _ = sys::set_immutable(p, true, None);
            let msg = "Tampering detected: File contents were corrupted or modified on disk. Decryption failed.";
            self.set_last_error(msg);
            return Err(VaultError::Tampered);
        }

        let _ = sys::set_xattr_locked(p, false);

        self.active_file_sessions.lock().unwrap().insert(
            path.to_string(),
            ActiveFileSession {
                data_key,
                _original_perms: entry.original_perms.clone(),
                _pid: 0,
            },
        );
        self.active_sessions.lock().unwrap().insert(path.to_string());
        let _ = self.db.add_session(entry.id, &format!("{}", now_secs()));
        self.clear_last_error();

        Ok(())
    }

    pub fn session_relock_file(&self, path: &str) -> Result<(), VaultError> {
        let file_sess = self.active_file_sessions.lock().unwrap().remove(path);
        self.active_sessions.lock().unwrap().remove(path);

        let p = Path::new(path);
        if let Some(mut entry) = self.db.find_by_path(path)? {
            if let Some(sess) = file_sess {
                if p.exists() {
                    if let Ok(new_iv) = crypto::encrypt_file(p, &sess.data_key) {
                        entry.enc_iv = new_iv.to_vec();
                        let _ = self.db.update_entry(&entry);
                    }
                    let _ = sys::set_xattr_locked(p, true);
                    let _ = sys::set_file_permissions_zero(p);
                    let _ = sys::set_immutable(p, true, None);
                }
            }
            let _ = self.db.remove_session(entry.id);
        }

        Ok(())
    }

    pub fn change_password(&self, path: &str, old_password: &str, new_password: &str) -> Result<(), VaultError> {
        let mut entry = self.db.find_by_path(path)?.ok_or(VaultError::NotLocked)?;

        if !crypto::verify_password(old_password, &entry.pw_hash, &entry.pw_salt) {
            self.set_last_error("Access denied: Current password incorrect");
            return Err(VaultError::AccessDenied);
        }

        let old_pw_key = crypto::derive_key(old_password, &entry.enc_salt)?;
        let data_key = crypto::decrypt_key(&entry.enc_key, &old_pw_key)?;

        // Verify file integrity on disk before changing password
        if entry.item_type == "file" && entry.enc_iv.len() == crypto::NONCE_SIZE {
            let p = Path::new(path);
            let _ = sys::restore_file_permissions(p, "0644");
            let mut file = File::open(p)?;
            let mut ciphertext = Vec::new();
            file.read_to_end(&mut ciphertext)?;
            let _ = sys::set_file_permissions_zero(p);

            let mut nonce = [0u8; crypto::NONCE_SIZE];
            nonce.copy_from_slice(&entry.enc_iv);
            if !ciphertext.is_empty() && crypto::decrypt(&ciphertext, &data_key, &nonce).is_err() {
                let msg = "Tampering detected: Cannot change password because file contents on disk have been corrupted or modified.";
                self.set_last_error(msg);
                return Err(VaultError::Tampered);
            }
        }

        let new_pw_salt = crypto::generate_salt();
        let new_pw_key = crypto::derive_key(new_password, &new_pw_salt)?;
        let new_enc_key = crypto::encrypt_key(&data_key, &new_pw_key)?;

        let new_hash_salt = crypto::generate_salt();
        let new_pw_hash = crypto::hash_password(new_password, &new_hash_salt)?;

        entry.enc_salt = new_pw_salt.to_vec();
        entry.enc_key = new_enc_key;
        entry.pw_salt = new_hash_salt.to_vec();
        entry.pw_hash = new_pw_hash.to_vec();

        self.db.update_entry(&entry)?;
        self.clear_last_error();

        Ok(())
    }

    pub fn has_own_password(&self, path: &str) -> bool {
        match self.db.find_by_path(path) {
            Ok(Some(entry)) => entry.is_own_password,
            _ => false,
        }
    }

    pub fn all_locked_paths(&self) -> Vec<String> {
        self.db.all_locked_paths().unwrap_or_default()
    }

    pub fn relock_all_sessions(&self) {
        let folder_sessions: Vec<String> = self
            .active_folder_sessions
            .lock()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        for p in folder_sessions {
            let _ = self.session_relock_folder(&p);
        }
        let file_sessions: Vec<String> = self
            .active_file_sessions
            .lock()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        for p in file_sessions {
            let _ = self.session_relock_file(&p);
        }
        self.active_sessions.lock().unwrap().clear();
    }

    pub fn get_session_data_key(&self, path: &str) -> Option<[u8; KEY_SIZE]> {
        self.active_file_sessions
            .lock()
            .unwrap()
            .get(path)
            .map(|s| s.data_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_vault_lock_unlock_file() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config");
        let test_file = dir.path().join("secret.txt");

        std::fs::write(&test_file, b"Confidential notes").unwrap();

        let vault = VaultService::new(&config_dir).unwrap();
        assert!(!vault.is_locked(test_file.to_str().unwrap()));

        vault.lock_item(test_file.to_str().unwrap(), "Pass123").unwrap();
        assert!(vault.is_locked(test_file.to_str().unwrap()));

        // Unlock with wrong pass fails
        assert!(vault.unlock_item(test_file.to_str().unwrap(), "Wrong").is_err());
        assert!(vault.is_locked(test_file.to_str().unwrap()));

        // Unlock with right pass succeeds
        vault.unlock_item(test_file.to_str().unwrap(), "Pass123").unwrap();
        assert!(!vault.is_locked(test_file.to_str().unwrap()));

        let content = std::fs::read(&test_file).unwrap();
        assert_eq!(content, b"Confidential notes");
    }

    #[test]
    fn test_vault_ghost_entry_detection() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config");
        let test_file = dir.path().join("ghost.txt");

        std::fs::write(&test_file, b"Secret content").unwrap();

        let vault = VaultService::new(&config_dir).unwrap();
        vault.lock_item(test_file.to_str().unwrap(), "Pass123").unwrap();
        assert!(vault.is_locked(test_file.to_str().unwrap()));

        // External deletion (sudo rm ghost.txt)
        std::fs::remove_file(&test_file).unwrap();
        assert!(!vault.is_locked(test_file.to_str().unwrap()));

        // Recreating file with same name
        std::fs::write(&test_file, b"New unencrypted file").unwrap();
        // Must NOT be considered locked!
        assert!(!vault.is_locked(test_file.to_str().unwrap()));
    }

    #[test]
    fn test_vault_tamper_detection() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config");
        let test_file = dir.path().join("tamper.txt");

        std::fs::write(&test_file, b"Original valid data").unwrap();

        let vault = VaultService::new(&config_dir).unwrap();
        vault.lock_item(test_file.to_str().unwrap(), "Pass123").unwrap();
        assert!(vault.is_locked(test_file.to_str().unwrap()));

        // Simulate external tampering (sudo nano)
        let _ = sys::restore_file_permissions(&test_file, "0644");
        let mut data = std::fs::read(&test_file).unwrap();
        let len = data.len();
        data[len - 1] ^= 0xFF;
        std::fs::write(&test_file, &data).unwrap();
        let _ = sys::set_file_permissions_zero(&test_file);

        // Unlock with correct password fails with Tampered
        let res = vault.unlock_item(test_file.to_str().unwrap(), "Pass123");
        assert!(matches!(res, Err(VaultError::Tampered)));
        assert!(vault.last_error().unwrap().contains("Tampering detected"));

        // Password change also rejected with Tampered
        let change_res = vault.change_password(test_file.to_str().unwrap(), "Pass123", "NewPass");
        assert!(matches!(change_res, Err(VaultError::Tampered)));
    }

    #[test]
    fn test_vault_rate_limiting() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config");
        let test_file = dir.path().join("rate.txt");

        std::fs::write(&test_file, b"Secret").unwrap();

        let vault = VaultService::new(&config_dir).unwrap();
        vault.lock_item(test_file.to_str().unwrap(), "Pass123").unwrap();

        let path_str = test_file.to_str().unwrap();
        for _ in 0..3 {
            let _ = vault.unlock_item(path_str, "Wrong");
            assert_eq!(vault.get_remaining_lockout_seconds(path_str), 0);
        }

        // 4th attempt triggers lockout
        let res = vault.unlock_item(path_str, "Wrong");
        assert!(matches!(res, Err(VaultError::RateLimited(_))));
        assert!(vault.get_remaining_lockout_seconds(path_str) > 0);
    }

    #[test]
    fn test_vault_folder_lock_unlock() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config");
        let secret_folder = dir.path().join("secret_folder");
        std::fs::create_dir(&secret_folder).unwrap();

        let file1 = secret_folder.join("doc1.txt");
        let file2 = secret_folder.join("doc2.txt");
        std::fs::write(&file1, b"Secret doc 1").unwrap();
        std::fs::write(&file2, b"Secret doc 2").unwrap();

        let vault = VaultService::new(&config_dir).unwrap();
        vault.lock_item(secret_folder.to_str().unwrap(), "FolderPass").unwrap();

        assert!(vault.is_locked(secret_folder.to_str().unwrap()));
        assert!(vault.is_locked(file1.to_str().unwrap()));
        assert!(vault.is_locked(file2.to_str().unwrap()));

        vault.unlock_item(secret_folder.to_str().unwrap(), "FolderPass").unwrap();
        assert!(!vault.is_locked(secret_folder.to_str().unwrap()));
        assert!(!vault.is_locked(file1.to_str().unwrap()));
        assert!(!vault.is_locked(file2.to_str().unwrap()));

        assert_eq!(std::fs::read(&file1).unwrap(), b"Secret doc 1");
        assert_eq!(std::fs::read(&file2).unwrap(), b"Secret doc 2");
    }
}
