use std::ffi::{CStr, CString};
use std::path::Path;
use std::sync::Mutex;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid path")]
    InvalidPath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultEntry {
    pub id: i64,
    pub path: String,
    pub item_type: String,
    pub parent_id: i64,
    pub pw_hash: Vec<u8>,
    pub pw_salt: Vec<u8>,
    pub enc_key: Vec<u8>,
    pub enc_iv: Vec<u8>,
    pub enc_salt: Vec<u8>,
    pub original_perms: String,
    pub locked_at: i64,
    pub is_own_password: bool,
    pub inode: i64,
}

impl Default for VaultEntry {
    fn default() -> Self {
        Self {
            id: 0,
            path: String::new(),
            item_type: "file".into(),
            parent_id: 0,
            pw_hash: Vec::new(),
            pw_salt: Vec::new(),
            enc_key: Vec::new(),
            enc_iv: Vec::new(),
            enc_salt: Vec::new(),
            original_perms: "0644".into(),
            locked_at: 0,
            is_own_password: true,
            inode: 0,
        }
    }
}

// SQLite FFI
#[allow(non_camel_case_types)]
mod ffi {
    use libc::{c_char, c_int, c_void};

    pub const SQLITE_OK: c_int = 0;
    pub const SQLITE_ROW: c_int = 100;
    pub const SQLITE_DONE: c_int = 101;

    pub const SQLITE_OPEN_READWRITE: c_int = 0x00000002;
    pub const SQLITE_OPEN_CREATE: c_int = 0x00000004;

    pub const SQLITE_TRANSIENT: *mut c_void = !0 as *mut c_void;

    pub enum sqlite3 {}
    pub enum sqlite3_stmt {}

    extern "C" {
        pub fn sqlite3_open_v2(
            filename: *const c_char,
            ppDb: *mut *mut sqlite3,
            flags: c_int,
            zVfs: *const c_char,
        ) -> c_int;
        pub fn sqlite3_close_v2(db: *mut sqlite3) -> c_int;
        pub fn sqlite3_errmsg(db: *mut sqlite3) -> *const c_char;
        pub fn sqlite3_exec(
            db: *mut sqlite3,
            sql: *const c_char,
            callback: *mut c_void,
            arg: *mut c_void,
            errmsg: *mut *mut c_char,
        ) -> c_int;
        pub fn sqlite3_last_insert_rowid(db: *mut sqlite3) -> i64;

        pub fn sqlite3_prepare_v2(
            db: *mut sqlite3,
            zSql: *const c_char,
            nByte: c_int,
            ppStmt: *mut *mut sqlite3_stmt,
            pzTail: *mut *const c_char,
        ) -> c_int;
        pub fn sqlite3_step(stmt: *mut sqlite3_stmt) -> c_int;
        pub fn sqlite3_finalize(stmt: *mut sqlite3_stmt) -> c_int;

        pub fn sqlite3_bind_int64(stmt: *mut sqlite3_stmt, idx: c_int, val: i64) -> c_int;
        pub fn sqlite3_bind_text(
            stmt: *mut sqlite3_stmt,
            idx: c_int,
            val: *const c_char,
            len: c_int,
            destructor: *mut c_void,
        ) -> c_int;
        pub fn sqlite3_bind_blob(
            stmt: *mut sqlite3_stmt,
            idx: c_int,
            val: *const c_void,
            len: c_int,
            destructor: *mut c_void,
        ) -> c_int;

        pub fn sqlite3_column_int64(stmt: *mut sqlite3_stmt, iCol: c_int) -> i64;
        pub fn sqlite3_column_text(stmt: *mut sqlite3_stmt, iCol: c_int) -> *const c_char;
        pub fn sqlite3_column_blob(stmt: *mut sqlite3_stmt, iCol: c_int) -> *const c_void;
        pub fn sqlite3_column_bytes(stmt: *mut sqlite3_stmt, iCol: c_int) -> c_int;
    }
}

pub struct VaultDatabase {
    db: Mutex<*mut ffi::sqlite3>,
}

unsafe impl Send for VaultDatabase {}
unsafe impl Sync for VaultDatabase {}

impl Drop for VaultDatabase {
    fn drop(&mut self) {
        if let Ok(guard) = self.db.lock() {
            if !guard.is_null() {
                unsafe { ffi::sqlite3_close_v2(*guard) };
            }
        }
    }
}

impl VaultDatabase {
    pub fn open(path: &Path) -> Result<Self, DbError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let c_path = CString::new(path.to_str().ok_or(DbError::InvalidPath)?)
            .map_err(|_| DbError::InvalidPath)?;

        let mut db_ptr: *mut ffi::sqlite3 = std::ptr::null_mut();
        let flags = ffi::SQLITE_OPEN_READWRITE | ffi::SQLITE_OPEN_CREATE;

        let ret = unsafe {
            ffi::sqlite3_open_v2(c_path.as_ptr(), &mut db_ptr, flags, std::ptr::null())
        };

        if ret != ffi::SQLITE_OK || db_ptr.is_null() {
            let msg = if !db_ptr.is_null() {
                unsafe { CStr::from_ptr(ffi::sqlite3_errmsg(db_ptr)).to_string_lossy().into_owned() }
            } else {
                format!("Failed to open SQLite database: error {}", ret)
            };
            if !db_ptr.is_null() {
                unsafe { ffi::sqlite3_close_v2(db_ptr) };
            }
            return Err(DbError::Sqlite(msg));
        }

        let db = Self {
            db: Mutex::new(db_ptr),
        };

        db.init_schema()?;
        Ok(db)
    }

    fn exec_raw(&self, sql: &str) -> Result<(), DbError> {
        let guard = self.db.lock().unwrap();
        let c_sql = CString::new(sql).map_err(|_| DbError::Sqlite("Invalid SQL".into()))?;
        let ret = unsafe {
            ffi::sqlite3_exec(
                *guard,
                c_sql.as_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if ret != ffi::SQLITE_OK {
            let err = unsafe { CStr::from_ptr(ffi::sqlite3_errmsg(*guard)).to_string_lossy().into_owned() };
            return Err(DbError::Sqlite(err));
        }
        Ok(())
    }

    fn init_schema(&self) -> Result<(), DbError> {
        self.exec_raw("PRAGMA journal_mode=WAL;")?;
        self.exec_raw("PRAGMA foreign_keys=ON;")?;

        let schema = r#"
            CREATE TABLE IF NOT EXISTS locked_items (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                path        TEXT NOT NULL UNIQUE,
                type        TEXT NOT NULL,
                parent_id   INTEGER DEFAULT 0,
                pw_hash     BLOB NOT NULL,
                pw_salt     BLOB NOT NULL,
                enc_key     BLOB,
                enc_iv      BLOB,
                enc_salt    BLOB,
                original_perms TEXT,
                locked_at   INTEGER NOT NULL,
                is_own_password INTEGER DEFAULT 0,
                inode       INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS active_sessions (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                item_id     INTEGER REFERENCES locked_items(id) ON DELETE CASCADE,
                unlocked_at INTEGER NOT NULL,
                session_token TEXT NOT NULL
            );
        "#;
        self.exec_raw(schema)?;

        // Ensure inode column exists for legacy databases
        let _ = self.exec_raw("ALTER TABLE locked_items ADD COLUMN inode INTEGER DEFAULT 0;");
        Ok(())
    }

    pub fn add_entry(&self, entry: &VaultEntry) -> Result<i64, DbError> {
        let guard = self.db.lock().unwrap();
        let sql = r#"
            INSERT INTO locked_items (path, type, parent_id, pw_hash, pw_salt, enc_key, enc_iv, enc_salt, original_perms, locked_at, is_own_password, inode)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
        "#;
        let c_sql = CString::new(sql).unwrap();
        let mut stmt: *mut ffi::sqlite3_stmt = std::ptr::null_mut();

        unsafe {
            if ffi::sqlite3_prepare_v2(*guard, c_sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) != ffi::SQLITE_OK {
                let err = CStr::from_ptr(ffi::sqlite3_errmsg(*guard)).to_string_lossy().into_owned();
                return Err(DbError::Sqlite(err));
            }

            struct StmtGuard(*mut ffi::sqlite3_stmt);
            impl Drop for StmtGuard {
                fn drop(&mut self) {
                    unsafe { ffi::sqlite3_finalize(self.0) };
                }
            }
            let _stmt_guard = StmtGuard(stmt);

            let c_path = CString::new(entry.path.as_str()).unwrap_or_default();
            let c_type = CString::new(entry.item_type.as_str()).unwrap_or_default();
            let c_perms = CString::new(entry.original_perms.as_str()).unwrap_or_default();

            ffi::sqlite3_bind_text(stmt, 1, c_path.as_ptr(), -1, ffi::SQLITE_TRANSIENT);
            ffi::sqlite3_bind_text(stmt, 2, c_type.as_ptr(), -1, ffi::SQLITE_TRANSIENT);
            ffi::sqlite3_bind_int64(stmt, 3, entry.parent_id);
            ffi::sqlite3_bind_blob(stmt, 4, entry.pw_hash.as_ptr() as *const libc::c_void, entry.pw_hash.len() as libc::c_int, ffi::SQLITE_TRANSIENT);
            ffi::sqlite3_bind_blob(stmt, 5, entry.pw_salt.as_ptr() as *const libc::c_void, entry.pw_salt.len() as libc::c_int, ffi::SQLITE_TRANSIENT);
            ffi::sqlite3_bind_blob(stmt, 6, entry.enc_key.as_ptr() as *const libc::c_void, entry.enc_key.len() as libc::c_int, ffi::SQLITE_TRANSIENT);
            ffi::sqlite3_bind_blob(stmt, 7, entry.enc_iv.as_ptr() as *const libc::c_void, entry.enc_iv.len() as libc::c_int, ffi::SQLITE_TRANSIENT);
            ffi::sqlite3_bind_blob(stmt, 8, entry.enc_salt.as_ptr() as *const libc::c_void, entry.enc_salt.len() as libc::c_int, ffi::SQLITE_TRANSIENT);
            ffi::sqlite3_bind_text(stmt, 9, c_perms.as_ptr(), -1, ffi::SQLITE_TRANSIENT);
            ffi::sqlite3_bind_int64(stmt, 10, entry.locked_at);
            ffi::sqlite3_bind_int64(stmt, 11, if entry.is_own_password { 1 } else { 0 });
            ffi::sqlite3_bind_int64(stmt, 12, entry.inode);

            if ffi::sqlite3_step(stmt) != ffi::SQLITE_DONE {
                let err = CStr::from_ptr(ffi::sqlite3_errmsg(*guard)).to_string_lossy().into_owned();
                return Err(DbError::Sqlite(err));
            }

            Ok(ffi::sqlite3_last_insert_rowid(*guard))
        }
    }

    pub fn update_entry(&self, entry: &VaultEntry) -> Result<(), DbError> {
        let guard = self.db.lock().unwrap();
        let sql = r#"
            UPDATE locked_items
            SET pw_hash = ?, pw_salt = ?, enc_key = ?, enc_iv = ?, enc_salt = ?, original_perms = ?, inode = ?
            WHERE path = ?;
        "#;
        let c_sql = CString::new(sql).unwrap();
        let mut stmt: *mut ffi::sqlite3_stmt = std::ptr::null_mut();

        unsafe {
            if ffi::sqlite3_prepare_v2(*guard, c_sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) != ffi::SQLITE_OK {
                let err = CStr::from_ptr(ffi::sqlite3_errmsg(*guard)).to_string_lossy().into_owned();
                return Err(DbError::Sqlite(err));
            }

            struct StmtGuard(*mut ffi::sqlite3_stmt);
            impl Drop for StmtGuard {
                fn drop(&mut self) {
                    unsafe { ffi::sqlite3_finalize(self.0) };
                }
            }
            let _stmt_guard = StmtGuard(stmt);

            let c_perms = CString::new(entry.original_perms.as_str()).unwrap_or_default();
            let c_path = CString::new(entry.path.as_str()).unwrap_or_default();

            ffi::sqlite3_bind_blob(stmt, 1, entry.pw_hash.as_ptr() as *const libc::c_void, entry.pw_hash.len() as libc::c_int, ffi::SQLITE_TRANSIENT);
            ffi::sqlite3_bind_blob(stmt, 2, entry.pw_salt.as_ptr() as *const libc::c_void, entry.pw_salt.len() as libc::c_int, ffi::SQLITE_TRANSIENT);
            ffi::sqlite3_bind_blob(stmt, 3, entry.enc_key.as_ptr() as *const libc::c_void, entry.enc_key.len() as libc::c_int, ffi::SQLITE_TRANSIENT);
            ffi::sqlite3_bind_blob(stmt, 4, entry.enc_iv.as_ptr() as *const libc::c_void, entry.enc_iv.len() as libc::c_int, ffi::SQLITE_TRANSIENT);
            ffi::sqlite3_bind_blob(stmt, 5, entry.enc_salt.as_ptr() as *const libc::c_void, entry.enc_salt.len() as libc::c_int, ffi::SQLITE_TRANSIENT);
            ffi::sqlite3_bind_text(stmt, 6, c_perms.as_ptr(), -1, ffi::SQLITE_TRANSIENT);
            ffi::sqlite3_bind_int64(stmt, 7, entry.inode);
            ffi::sqlite3_bind_text(stmt, 8, c_path.as_ptr(), -1, ffi::SQLITE_TRANSIENT);

            if ffi::sqlite3_step(stmt) != ffi::SQLITE_DONE {
                let err = CStr::from_ptr(ffi::sqlite3_errmsg(*guard)).to_string_lossy().into_owned();
                return Err(DbError::Sqlite(err));
            }
            Ok(())
        }
    }

    pub fn remove_entry(&self, path: &str) -> Result<bool, DbError> {
        let guard = self.db.lock().unwrap();
        let sql = "DELETE FROM locked_items WHERE path = ?;";
        let c_sql = CString::new(sql).unwrap();
        let mut stmt: *mut ffi::sqlite3_stmt = std::ptr::null_mut();

        unsafe {
            if ffi::sqlite3_prepare_v2(*guard, c_sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) != ffi::SQLITE_OK {
                let err = CStr::from_ptr(ffi::sqlite3_errmsg(*guard)).to_string_lossy().into_owned();
                return Err(DbError::Sqlite(err));
            }
            struct StmtGuard(*mut ffi::sqlite3_stmt);
            impl Drop for StmtGuard {
                fn drop(&mut self) {
                    unsafe { ffi::sqlite3_finalize(self.0) };
                }
            }
            let _stmt_guard = StmtGuard(stmt);

            let c_path = CString::new(path).unwrap_or_default();
            ffi::sqlite3_bind_text(stmt, 1, c_path.as_ptr(), -1, ffi::SQLITE_TRANSIENT);

            if ffi::sqlite3_step(stmt) != ffi::SQLITE_DONE {
                let err = CStr::from_ptr(ffi::sqlite3_errmsg(*guard)).to_string_lossy().into_owned();
                return Err(DbError::Sqlite(err));
            }
            Ok(true)
        }
    }

    unsafe fn parse_entry_row(stmt: *mut ffi::sqlite3_stmt) -> VaultEntry {
        let id = ffi::sqlite3_column_int64(stmt, 0);
        let path = CStr::from_ptr(ffi::sqlite3_column_text(stmt, 1)).to_string_lossy().into_owned();
        let item_type = CStr::from_ptr(ffi::sqlite3_column_text(stmt, 2)).to_string_lossy().into_owned();
        let parent_id = ffi::sqlite3_column_int64(stmt, 3);

        let read_blob = |col: libc::c_int| -> Vec<u8> {
            let ptr = ffi::sqlite3_column_blob(stmt, col);
            let len = ffi::sqlite3_column_bytes(stmt, col);
            if ptr.is_null() || len <= 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(ptr as *const u8, len as usize).to_vec()
            }
        };

        let pw_hash = read_blob(4);
        let pw_salt = read_blob(5);
        let enc_key = read_blob(6);
        let enc_iv = read_blob(7);
        let enc_salt = read_blob(8);
        let original_perms = CStr::from_ptr(ffi::sqlite3_column_text(stmt, 9)).to_string_lossy().into_owned();
        let locked_at = ffi::sqlite3_column_int64(stmt, 10);
        let is_own_password = ffi::sqlite3_column_int64(stmt, 11) != 0;
        let inode = ffi::sqlite3_column_int64(stmt, 12);

        VaultEntry {
            id,
            path,
            item_type,
            parent_id,
            pw_hash,
            pw_salt,
            enc_key,
            enc_iv,
            enc_salt,
            original_perms,
            locked_at,
            is_own_password,
            inode,
        }
    }

    pub fn find_by_path(&self, path: &str) -> Result<Option<VaultEntry>, DbError> {
        let guard = self.db.lock().unwrap();
        let sql = r#"
            SELECT id, path, type, parent_id, pw_hash, pw_salt, enc_key, enc_iv, enc_salt, original_perms, locked_at, is_own_password, inode
            FROM locked_items WHERE path = ?;
        "#;
        let c_sql = CString::new(sql).unwrap();
        let mut stmt: *mut ffi::sqlite3_stmt = std::ptr::null_mut();

        unsafe {
            if ffi::sqlite3_prepare_v2(*guard, c_sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) != ffi::SQLITE_OK {
                let err = CStr::from_ptr(ffi::sqlite3_errmsg(*guard)).to_string_lossy().into_owned();
                return Err(DbError::Sqlite(err));
            }
            struct StmtGuard(*mut ffi::sqlite3_stmt);
            impl Drop for StmtGuard {
                fn drop(&mut self) {
                    unsafe { ffi::sqlite3_finalize(self.0) };
                }
            }
            let _stmt_guard = StmtGuard(stmt);

            let c_path = CString::new(path).unwrap_or_default();
            ffi::sqlite3_bind_text(stmt, 1, c_path.as_ptr(), -1, ffi::SQLITE_TRANSIENT);

            if ffi::sqlite3_step(stmt) == ffi::SQLITE_ROW {
                Ok(Some(Self::parse_entry_row(stmt)))
            } else {
                Ok(None)
            }
        }
    }

    pub fn has_entry(&self, path: &str) -> Result<bool, DbError> {
        Ok(self.find_by_path(path)?.is_some())
    }

    pub fn find_by_parent_id(&self, parent_id: i64) -> Result<Vec<VaultEntry>, DbError> {
        let guard = self.db.lock().unwrap();
        let sql = r#"
            SELECT id, path, type, parent_id, pw_hash, pw_salt, enc_key, enc_iv, enc_salt, original_perms, locked_at, is_own_password, inode
            FROM locked_items WHERE parent_id = ?;
        "#;
        let c_sql = CString::new(sql).unwrap();
        let mut stmt: *mut ffi::sqlite3_stmt = std::ptr::null_mut();

        unsafe {
            if ffi::sqlite3_prepare_v2(*guard, c_sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) != ffi::SQLITE_OK {
                let err = CStr::from_ptr(ffi::sqlite3_errmsg(*guard)).to_string_lossy().into_owned();
                return Err(DbError::Sqlite(err));
            }
            struct StmtGuard(*mut ffi::sqlite3_stmt);
            impl Drop for StmtGuard {
                fn drop(&mut self) {
                    unsafe { ffi::sqlite3_finalize(self.0) };
                }
            }
            let _stmt_guard = StmtGuard(stmt);

            ffi::sqlite3_bind_int64(stmt, 1, parent_id);

            let mut results = Vec::new();
            while ffi::sqlite3_step(stmt) == ffi::SQLITE_ROW {
                results.push(Self::parse_entry_row(stmt));
            }
            Ok(results)
        }
    }

    pub fn all_entries(&self) -> Result<Vec<VaultEntry>, DbError> {
        let guard = self.db.lock().unwrap();
        let sql = r#"
            SELECT id, path, type, parent_id, pw_hash, pw_salt, enc_key, enc_iv, enc_salt, original_perms, locked_at, is_own_password, inode
            FROM locked_items;
        "#;
        let c_sql = CString::new(sql).unwrap();
        let mut stmt: *mut ffi::sqlite3_stmt = std::ptr::null_mut();

        unsafe {
            if ffi::sqlite3_prepare_v2(*guard, c_sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) != ffi::SQLITE_OK {
                let err = CStr::from_ptr(ffi::sqlite3_errmsg(*guard)).to_string_lossy().into_owned();
                return Err(DbError::Sqlite(err));
            }
            struct StmtGuard(*mut ffi::sqlite3_stmt);
            impl Drop for StmtGuard {
                fn drop(&mut self) {
                    unsafe { ffi::sqlite3_finalize(self.0) };
                }
            }
            let _stmt_guard = StmtGuard(stmt);

            let mut results = Vec::new();
            while ffi::sqlite3_step(stmt) == ffi::SQLITE_ROW {
                results.push(Self::parse_entry_row(stmt));
            }
            Ok(results)
        }
    }

    pub fn all_locked_paths(&self) -> Result<Vec<String>, DbError> {
        let guard = self.db.lock().unwrap();
        let sql = "SELECT path FROM locked_items;";
        let c_sql = CString::new(sql).unwrap();
        let mut stmt: *mut ffi::sqlite3_stmt = std::ptr::null_mut();

        unsafe {
            if ffi::sqlite3_prepare_v2(*guard, c_sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) != ffi::SQLITE_OK {
                let err = CStr::from_ptr(ffi::sqlite3_errmsg(*guard)).to_string_lossy().into_owned();
                return Err(DbError::Sqlite(err));
            }
            struct StmtGuard(*mut ffi::sqlite3_stmt);
            impl Drop for StmtGuard {
                fn drop(&mut self) {
                    unsafe { ffi::sqlite3_finalize(self.0) };
                }
            }
            let _stmt_guard = StmtGuard(stmt);

            let mut paths = Vec::new();
            while ffi::sqlite3_step(stmt) == ffi::SQLITE_ROW {
                paths.push(CStr::from_ptr(ffi::sqlite3_column_text(stmt, 0)).to_string_lossy().into_owned());
            }
            Ok(paths)
        }
    }

    pub fn add_session(&self, item_id: i64, session_token: &str) -> Result<(), DbError> {
        let guard = self.db.lock().unwrap();
        let sql = "INSERT INTO active_sessions (item_id, unlocked_at, session_token) VALUES (?, strftime('%s','now'), ?);";
        let c_sql = CString::new(sql).unwrap();
        let mut stmt: *mut ffi::sqlite3_stmt = std::ptr::null_mut();

        unsafe {
            if ffi::sqlite3_prepare_v2(*guard, c_sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) != ffi::SQLITE_OK {
                let err = CStr::from_ptr(ffi::sqlite3_errmsg(*guard)).to_string_lossy().into_owned();
                return Err(DbError::Sqlite(err));
            }
            struct StmtGuard(*mut ffi::sqlite3_stmt);
            impl Drop for StmtGuard {
                fn drop(&mut self) {
                    unsafe { ffi::sqlite3_finalize(self.0) };
                }
            }
            let _stmt_guard = StmtGuard(stmt);

            let c_token = CString::new(session_token).unwrap_or_default();
            ffi::sqlite3_bind_int64(stmt, 1, item_id);
            ffi::sqlite3_bind_text(stmt, 2, c_token.as_ptr(), -1, ffi::SQLITE_TRANSIENT);

            if ffi::sqlite3_step(stmt) != ffi::SQLITE_DONE {
                let err = CStr::from_ptr(ffi::sqlite3_errmsg(*guard)).to_string_lossy().into_owned();
                return Err(DbError::Sqlite(err));
            }
            Ok(())
        }
    }

    pub fn remove_session(&self, item_id: i64) -> Result<(), DbError> {
        let guard = self.db.lock().unwrap();
        let sql = "DELETE FROM active_sessions WHERE item_id = ?;";
        let c_sql = CString::new(sql).unwrap();
        let mut stmt: *mut ffi::sqlite3_stmt = std::ptr::null_mut();

        unsafe {
            if ffi::sqlite3_prepare_v2(*guard, c_sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) != ffi::SQLITE_OK {
                let err = CStr::from_ptr(ffi::sqlite3_errmsg(*guard)).to_string_lossy().into_owned();
                return Err(DbError::Sqlite(err));
            }
            struct StmtGuard(*mut ffi::sqlite3_stmt);
            impl Drop for StmtGuard {
                fn drop(&mut self) {
                    unsafe { ffi::sqlite3_finalize(self.0) };
                }
            }
            let _stmt_guard = StmtGuard(stmt);

            ffi::sqlite3_bind_int64(stmt, 1, item_id);
            if ffi::sqlite3_step(stmt) != ffi::SQLITE_DONE {
                let err = CStr::from_ptr(ffi::sqlite3_errmsg(*guard)).to_string_lossy().into_owned();
                return Err(DbError::Sqlite(err));
            }
            Ok(())
        }
    }

    pub fn has_active_session(&self, item_id: i64) -> Result<bool, DbError> {
        let guard = self.db.lock().unwrap();
        let sql = "SELECT id FROM active_sessions WHERE item_id = ? LIMIT 1;";
        let c_sql = CString::new(sql).unwrap();
        let mut stmt: *mut ffi::sqlite3_stmt = std::ptr::null_mut();

        unsafe {
            if ffi::sqlite3_prepare_v2(*guard, c_sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) != ffi::SQLITE_OK {
                let err = CStr::from_ptr(ffi::sqlite3_errmsg(*guard)).to_string_lossy().into_owned();
                return Err(DbError::Sqlite(err));
            }
            struct StmtGuard(*mut ffi::sqlite3_stmt);
            impl Drop for StmtGuard {
                fn drop(&mut self) {
                    unsafe { ffi::sqlite3_finalize(self.0) };
                }
            }
            let _stmt_guard = StmtGuard(stmt);

            ffi::sqlite3_bind_int64(stmt, 1, item_id);
            Ok(ffi::sqlite3_step(stmt) == ffi::SQLITE_ROW)
        }
    }

    pub fn clear_all_sessions(&self) -> Result<(), DbError> {
        self.exec_raw("DELETE FROM active_sessions;")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_database_crud() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();

        let db = VaultDatabase::open(path).unwrap();

        let mut entry = VaultEntry {
            id: 0,
            path: "/path/to/secret.txt".into(),
            item_type: "file".into(),
            parent_id: 0,
            pw_hash: vec![1, 2, 3],
            pw_salt: vec![4, 5, 6],
            enc_key: vec![7, 8, 9],
            enc_iv: vec![10, 11, 12],
            enc_salt: vec![13, 14, 15],
            original_perms: "0644".into(),
            locked_at: 100000,
            is_own_password: true,
            inode: 987654321,
        };

        let row_id = db.add_entry(&entry).unwrap();
        assert!(row_id > 0);

        let found = db.find_by_path("/path/to/secret.txt").unwrap().unwrap();
        assert_eq!(found.path, "/path/to/secret.txt");
        assert_eq!(found.inode, 987654321);
        assert_eq!(found.original_perms, "0644");

        entry.original_perms = "0600".into();
        entry.inode = 112233;
        db.update_entry(&entry).unwrap();

        let updated = db.find_by_path("/path/to/secret.txt").unwrap().unwrap();
        assert_eq!(updated.original_perms, "0600");
        assert_eq!(updated.inode, 112233);

        let all_paths = db.all_locked_paths().unwrap();
        assert_eq!(all_paths.len(), 1);
        assert_eq!(all_paths[0], "/path/to/secret.txt");

        assert!(db.remove_entry("/path/to/secret.txt").unwrap());
        assert!(db.find_by_path("/path/to/secret.txt").unwrap().is_none());
    }
}
