use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VaultDbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultEntry {
    pub id: i64,
    pub path: String,
    pub entry_type: String, // "file" or "directory"
    pub parent_id: i64,     // 0 = no parent
    pub pw_hash: Vec<u8>,
    pub pw_salt: Vec<u8>,
    pub enc_key: Option<Vec<u8>>,
    pub enc_iv: Option<Vec<u8>>,
    pub enc_salt: Option<Vec<u8>>,
    pub original_perms: Option<String>,
    pub locked_at: i64,
    pub is_own_password: bool,
    pub inode: i64,
}

impl Default for VaultEntry {
    fn default() -> Self {
        Self {
            id: 0,
            path: String::new(),
            entry_type: "file".to_string(),
            parent_id: 0,
            pw_hash: Vec::new(),
            pw_salt: Vec::new(),
            enc_key: None,
            enc_iv: None,
            enc_salt: None,
            original_perms: None,
            locked_at: 0,
            is_own_password: false,
            inode: 0,
        }
    }
}

pub struct VaultDatabase {
    conn: Mutex<Connection>,
}

impl VaultDatabase {
    pub fn open(db_path: &Path) -> Result<Self, VaultDbError> {
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let conn = Connection::open(db_path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.create_tables()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self, VaultDbError> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.create_tables()?;
        Ok(db)
    }

    fn create_tables(&self) -> Result<(), VaultDbError> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS locked_items (
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
            )",
            [],
        )?;

        // Ensure backward compatibility if upgrading an older database
        let _ = conn.execute(
            "ALTER TABLE locked_items ADD COLUMN inode INTEGER DEFAULT 0",
            [],
        );

        conn.execute(
            "CREATE TABLE IF NOT EXISTS active_sessions (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                item_id     INTEGER REFERENCES locked_items(id) ON DELETE CASCADE,
                unlocked_at INTEGER NOT NULL,
                session_token TEXT NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    pub fn add_entry(&self, entry: &VaultEntry) -> Result<i64, VaultDbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO locked_items (path, type, parent_id, pw_hash, pw_salt, enc_key, enc_iv, enc_salt, original_perms, locked_at, is_own_password, inode)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                entry.path,
                entry.entry_type,
                entry.parent_id,
                entry.pw_hash,
                entry.pw_salt,
                entry.enc_key,
                entry.enc_iv,
                entry.enc_salt,
                entry.original_perms,
                entry.locked_at,
                if entry.is_own_password { 1 } else { 0 },
                entry.inode,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn update_entry(&self, entry: &VaultEntry) -> Result<(), VaultDbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE locked_items SET pw_hash = ?1, pw_salt = ?2, enc_key = ?3, enc_iv = ?4, enc_salt = ?5 WHERE path = ?6",
            params![
                entry.pw_hash,
                entry.pw_salt,
                entry.enc_key,
                entry.enc_iv,
                entry.enc_salt,
                entry.path,
            ],
        )?;
        Ok(())
    }

    pub fn remove_entry(&self, path: &str) -> Result<bool, VaultDbError> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute("DELETE FROM locked_items WHERE path = ?1", params![path])?;
        Ok(count > 0)
    }

    pub fn remove_entry_by_id(&self, id: i64) -> Result<bool, VaultDbError> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute("DELETE FROM locked_items WHERE id = ?1", params![id])?;
        Ok(count > 0)
    }

    pub fn find_by_path(&self, path: &str) -> Result<Option<VaultEntry>, VaultDbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, path, type, parent_id, pw_hash, pw_salt, enc_key, enc_iv, enc_salt, original_perms, locked_at, is_own_password, inode
             FROM locked_items WHERE path = ?1",
        )?;

        let entry = stmt
            .query_row(params![path], |row| {
                Ok(VaultEntry {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    entry_type: row.get(2)?,
                    parent_id: row.get(3)?,
                    pw_hash: row.get(4)?,
                    pw_salt: row.get(5)?,
                    enc_key: row.get(6)?,
                    enc_iv: row.get(7)?,
                    enc_salt: row.get(8)?,
                    original_perms: row.get(9)?,
                    locked_at: row.get(10)?,
                    is_own_password: row.get::<_, i32>(11)? != 0,
                    inode: row.get(12)?,
                })
            })
            .optional()?;

        Ok(entry)
    }

    pub fn find_by_parent_id(&self, parent_id: i64) -> Result<Vec<VaultEntry>, VaultDbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, path, type, parent_id, pw_hash, pw_salt, enc_key, enc_iv, enc_salt, original_perms, locked_at, is_own_password, inode
             FROM locked_items WHERE parent_id = ?1",
        )?;

        let rows = stmt.query_map(params![parent_id], |row| {
            Ok(VaultEntry {
                id: row.get(0)?,
                path: row.get(1)?,
                entry_type: row.get(2)?,
                parent_id: row.get(3)?,
                pw_hash: row.get(4)?,
                pw_salt: row.get(5)?,
                enc_key: row.get(6)?,
                enc_iv: row.get(7)?,
                enc_salt: row.get(8)?,
                original_perms: row.get(9)?,
                locked_at: row.get(10)?,
                is_own_password: row.get::<_, i32>(11)? != 0,
                inode: row.get(12)?,
            })
        })?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item?);
        }
        Ok(list)
    }

    pub fn all_entries(&self) -> Result<Vec<VaultEntry>, VaultDbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, path, type, parent_id, pw_hash, pw_salt, enc_key, enc_iv, enc_salt, original_perms, locked_at, is_own_password, inode
             FROM locked_items",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(VaultEntry {
                id: row.get(0)?,
                path: row.get(1)?,
                entry_type: row.get(2)?,
                parent_id: row.get(3)?,
                pw_hash: row.get(4)?,
                pw_salt: row.get(5)?,
                enc_key: row.get(6)?,
                enc_iv: row.get(7)?,
                enc_salt: row.get(8)?,
                original_perms: row.get(9)?,
                locked_at: row.get(10)?,
                is_own_password: row.get::<_, i32>(11)? != 0,
                inode: row.get(12)?,
            })
        })?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item?);
        }
        Ok(list)
    }

    pub fn has_entry(&self, path: &str) -> Result<bool, VaultDbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT 1 FROM locked_items WHERE path = ?1 LIMIT 1")?;
        let exists = stmt.exists(params![path])?;
        Ok(exists)
    }

    pub fn remove_entries_recursive(&self, directory_path: &str) -> Result<(), VaultDbError> {
        let conn = self.conn.lock().unwrap();
        let mut prefix = directory_path.to_string();
        if !prefix.ends_with('/') {
            prefix.push('/');
        }
        let wildcard = format!("{}%", prefix);

        conn.execute(
            "DELETE FROM locked_items WHERE path = ?1 OR path LIKE ?2",
            params![directory_path, wildcard],
        )?;
        Ok(())
    }

    pub fn find_child_entries(&self, directory_path: &str) -> Result<Vec<VaultEntry>, VaultDbError> {
        let conn = self.conn.lock().unwrap();
        let mut prefix = directory_path.to_string();
        if !prefix.ends_with('/') {
            prefix.push('/');
        }
        let wildcard = format!("{}%", prefix);

        let mut stmt = conn.prepare(
            "SELECT id, path, type, parent_id, pw_hash, pw_salt, enc_key, enc_iv, enc_salt, original_perms, locked_at, is_own_password, inode
             FROM locked_items WHERE path LIKE ?1",
        )?;

        let rows = stmt.query_map(params![wildcard], |row| {
            Ok(VaultEntry {
                id: row.get(0)?,
                path: row.get(1)?,
                entry_type: row.get(2)?,
                parent_id: row.get(3)?,
                pw_hash: row.get(4)?,
                pw_salt: row.get(5)?,
                enc_key: row.get(6)?,
                enc_iv: row.get(7)?,
                enc_salt: row.get(8)?,
                original_perms: row.get(9)?,
                locked_at: row.get(10)?,
                is_own_password: row.get::<_, i32>(11)? != 0,
                inode: row.get(12)?,
            })
        })?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item?);
        }
        Ok(list)
    }

    pub fn add_session(&self, item_id: i64, session_token: &str) -> Result<(), VaultDbError> {
        let conn = self.conn.lock().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        conn.execute(
            "INSERT INTO active_sessions (item_id, unlocked_at, session_token) VALUES (?1, ?2, ?3)",
            params![item_id, now, session_token],
        )?;
        Ok(())
    }

    pub fn remove_session(&self, item_id: i64) -> Result<bool, VaultDbError> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute(
            "DELETE FROM active_sessions WHERE item_id = ?1",
            params![item_id],
        )?;
        Ok(count > 0)
    }

    pub fn has_active_session(&self, item_id: i64) -> Result<bool, VaultDbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT 1 FROM active_sessions WHERE item_id = ?1 LIMIT 1")?;
        let exists = stmt.exists(params![item_id])?;
        Ok(exists)
    }

    pub fn clear_all_sessions(&self) -> Result<(), VaultDbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM active_sessions", [])?;
        Ok(())
    }

    pub fn all_locked_paths(&self) -> Result<Vec<String>, VaultDbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT path FROM locked_items")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut paths = Vec::new();
        for p in rows {
            paths.push(p?);
        }
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_db_crud() {
        let db = VaultDatabase::open_in_memory().unwrap();

        let mut entry = VaultEntry {
            path: "/home/user/vault/secret.txt".to_string(),
            entry_type: "file".to_string(),
            parent_id: 0,
            pw_hash: vec![1, 2, 3],
            pw_salt: vec![4, 5, 6],
            enc_key: Some(vec![7, 8, 9]),
            enc_iv: Some(vec![10, 11, 12]),
            enc_salt: Some(vec![13, 14, 15]),
            original_perms: Some("0644".to_string()),
            locked_at: 1000,
            is_own_password: true,
            inode: 12345,
            ..Default::default()
        };

        let id = db.add_entry(&entry).unwrap();
        assert!(id > 0);
        entry.id = id;

        assert!(db.has_entry(&entry.path).unwrap());
        assert_eq!(db.all_locked_paths().unwrap(), vec![entry.path.clone()]);

        let retrieved = db.find_by_path(&entry.path).unwrap().unwrap();
        assert_eq!(retrieved, entry);

        // Update
        entry.pw_hash = vec![9, 9, 9];
        db.update_entry(&entry).unwrap();
        let updated = db.find_by_path(&entry.path).unwrap().unwrap();
        assert_eq!(updated.pw_hash, vec![9, 9, 9]);

        // Sessions
        assert!(!db.has_active_session(id).unwrap());
        db.add_session(id, "test-token").unwrap();
        assert!(db.has_active_session(id).unwrap());
        db.remove_session(id).unwrap();
        assert!(!db.has_active_session(id).unwrap());

        // Remove
        assert!(db.remove_entry(&entry.path).unwrap());
        assert!(!db.has_entry(&entry.path).unwrap());
    }

    #[test]
    fn test_recursive_remove() {
        let db = VaultDatabase::open_in_memory().unwrap();

        let parent = VaultEntry {
            path: "/home/user/vault/dir".to_string(),
            entry_type: "directory".to_string(),
            ..Default::default()
        };
        let child1 = VaultEntry {
            path: "/home/user/vault/dir/a.txt".to_string(),
            entry_type: "file".to_string(),
            ..Default::default()
        };
        let child2 = VaultEntry {
            path: "/home/user/vault/dir/sub/b.txt".to_string(),
            entry_type: "file".to_string(),
            ..Default::default()
        };

        db.add_entry(&parent).unwrap();
        db.add_entry(&child1).unwrap();
        db.add_entry(&child2).unwrap();

        let children = db.find_child_entries("/home/user/vault/dir").unwrap();
        assert_eq!(children.len(), 2);

        db.remove_entries_recursive("/home/user/vault/dir").unwrap();
        assert_eq!(db.all_entries().unwrap().len(), 0);
    }
}
