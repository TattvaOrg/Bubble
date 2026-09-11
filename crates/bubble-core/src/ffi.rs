use crate::vault::VaultService;
use crate::crypto;
use crate::config::BubbleConfig;
use crate::file_ops;
use crate::trash;
use crate::undo::UndoManager;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::path::{Path, PathBuf};

#[no_mangle]
pub unsafe extern "C" fn bubble_vault_new(config_dir: *const c_char) -> *mut VaultService {
    if config_dir.is_null() {
        return std::ptr::null_mut();
    }
    let c_str = CStr::from_ptr(config_dir);
    let path_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    match VaultService::new(Path::new(path_str)) {
        Ok(service) => Box::into_raw(Box::new(service)),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn bubble_vault_free(vault: *mut VaultService) {
    if !vault.is_null() {
        drop(Box::from_raw(vault));
    }
}

#[no_mangle]
pub unsafe extern "C" fn bubble_vault_lock_item(
    vault: *mut VaultService,
    path: *const c_char,
    password: *const c_char,
) -> bool {
    if vault.is_null() || path.is_null() || password.is_null() {
        return false;
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    let pass = match CStr::from_ptr(password).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    (*vault).lock_item(p, pass).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn bubble_vault_unlock_item(
    vault: *mut VaultService,
    path: *const c_char,
    password: *const c_char,
) -> bool {
    if vault.is_null() || path.is_null() || password.is_null() {
        return false;
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    let pass = match CStr::from_ptr(password).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    (*vault).unlock_item(p, pass).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn bubble_vault_is_locked(
    vault: *mut VaultService,
    path: *const c_char,
) -> bool {
    if vault.is_null() || path.is_null() {
        return false;
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    (*vault).is_locked(p)
}

#[no_mangle]
pub unsafe extern "C" fn bubble_vault_is_session_unlocked(
    vault: *mut VaultService,
    path: *const c_char,
) -> bool {
    if vault.is_null() || path.is_null() {
        return false;
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    (*vault).is_session_unlocked(p)
}

#[no_mangle]
pub unsafe extern "C" fn bubble_vault_session_unlock_folder(
    vault: *mut VaultService,
    path: *const c_char,
    password: *const c_char,
) -> bool {
    if vault.is_null() || path.is_null() || password.is_null() {
        return false;
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    let pass = match CStr::from_ptr(password).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    (*vault).session_unlock_folder(p, pass).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn bubble_vault_session_relock_folder(
    vault: *mut VaultService,
    path: *const c_char,
) -> bool {
    if vault.is_null() || path.is_null() {
        return false;
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    (*vault).session_relock_folder(p).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn bubble_vault_session_unlock_file(
    vault: *mut VaultService,
    path: *const c_char,
    password: *const c_char,
) -> bool {
    if vault.is_null() || path.is_null() || password.is_null() {
        return false;
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    let pass = match CStr::from_ptr(password).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    (*vault).session_unlock_file(p, pass).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn bubble_vault_session_relock_file(
    vault: *mut VaultService,
    path: *const c_char,
) -> bool {
    if vault.is_null() || path.is_null() {
        return false;
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    (*vault).session_relock_file(p).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn bubble_vault_change_password(
    vault: *mut VaultService,
    path: *const c_char,
    old_pass: *const c_char,
    new_pass: *const c_char,
) -> bool {
    if vault.is_null() || path.is_null() || old_pass.is_null() || new_pass.is_null() {
        return false;
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    let old_p = match CStr::from_ptr(old_pass).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    let new_p = match CStr::from_ptr(new_pass).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    (*vault).change_password(p, old_p, new_p).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn bubble_vault_get_lockout_seconds(
    vault: *mut VaultService,
    path: *const c_char,
) -> u64 {
    if vault.is_null() || path.is_null() {
        return 0;
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    (*vault).get_remaining_lockout_seconds(p)
}

#[no_mangle]
pub unsafe extern "C" fn bubble_vault_last_error(
    vault: *mut VaultService,
    out_buf: *mut c_char,
    max_len: usize,
) -> bool {
    if vault.is_null() || out_buf.is_null() || max_len == 0 {
        return false;
    }
    if let Some(err) = (*vault).last_error() {
        if let Ok(c_err) = CString::new(err) {
            let bytes = c_err.as_bytes_with_nul();
            let copy_len = std::cmp::min(bytes.len(), max_len);
            std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, out_buf, copy_len);
            *out_buf.add(copy_len - 1) = 0;
            return true;
        }
    }
    *out_buf = 0;
    false
}

#[no_mangle]
pub unsafe extern "C" fn bubble_vault_clear_last_error(vault: *mut VaultService) {
    if !vault.is_null() {
        (*vault).clear_last_error();
    }
}

#[no_mangle]
pub unsafe extern "C" fn bubble_vault_shred_file(path: *const c_char) -> bool {
    if path.is_null() {
        return false;
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    crypto::shred_file(Path::new(p)).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn bubble_vault_relock_all_sessions(vault: *mut VaultService) {
    if !vault.is_null() {
        (*vault).relock_all_sessions();
    }
}

#[no_mangle]
pub unsafe extern "C" fn bubble_vault_has_own_password(
    vault: *mut VaultService,
    path: *const c_char,
) -> bool {
    if vault.is_null() || path.is_null() {
        return false;
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    (*vault).has_own_password(p)
}

#[no_mangle]
pub unsafe extern "C" fn bubble_vault_all_locked_paths(
    vault: *mut VaultService,
    callback: Option<extern "C" fn(path: *const c_char, user_data: *mut c_void)>,
    user_data: *mut c_void,
) {
    if vault.is_null() {
        return;
    }
    if let Some(cb) = callback {
        for path in (*vault).all_locked_paths() {
            if let Ok(c_path) = CString::new(path) {
                cb(c_path.as_ptr(), user_data);
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn bubble_vault_get_session_data_key(
    vault: *mut VaultService,
    path: *const c_char,
    out_key: *mut u8,
    key_len: usize,
) -> bool {
    if vault.is_null() || path.is_null() || out_key.is_null() || key_len < crypto::KEY_SIZE {
        return false;
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    if let Some(key) = (*vault).get_session_data_key(p) {
        std::ptr::copy_nonoverlapping(key.as_ptr(), out_key, crypto::KEY_SIZE);
        return true;
    }
    false
}

fn to_c_string(s: impl AsRef<str>) -> *mut c_char {
    match CString::new(s.as_ref()) {
        Ok(c) => c.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn bubble_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

// ==========================================
// TRASH C-ABI
// ==========================================

#[no_mangle]
pub unsafe extern "C" fn bubble_trash_item(path: *const c_char) -> bool {
    if path.is_null() {
        return false;
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    trash::trash_item(Path::new(p)).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn bubble_trash_restore_item(files_path: *const c_char) -> *mut c_char {
    if files_path.is_null() {
        return std::ptr::null_mut();
    }
    let p = match CStr::from_ptr(files_path).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    match trash::restore_item(Path::new(p)) {
        Ok(restored) => to_c_string(restored.to_string_lossy()),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn bubble_trash_empty() -> usize {
    trash::empty_trash().unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn bubble_trash_scan() -> *mut c_char {
    let entries = trash::scan_trash();
    #[derive(serde::Serialize)]
    struct TrashEntryJson {
        name: String,
        #[serde(rename = "filesPath")]
        files_path: String,
        #[serde(rename = "originalPath")]
        original_path: String,
        #[serde(rename = "deletedAt")]
        deleted_at: String,
    }
    let json_entries: Vec<TrashEntryJson> = entries
        .into_iter()
        .map(|e| TrashEntryJson {
            name: e.name,
            files_path: e.files_path.to_string_lossy().into_owned(),
            original_path: e.original_path.to_string_lossy().into_owned(),
            deleted_at: e.deleted_at,
        })
        .collect();

    match serde_json::to_string(&json_entries) {
        Ok(j) => to_c_string(j),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn bubble_trash_home_root() -> *mut c_char {
    to_c_string(trash::home_trash_root().to_string_lossy())
}

#[no_mangle]
pub unsafe extern "C" fn bubble_trash_roots() -> *mut c_char {
    let roots = trash::trash_roots();
    let strings: Vec<String> = roots
        .into_iter()
        .map(|r| r.to_string_lossy().into_owned())
        .collect();
    match serde_json::to_string(&strings) {
        Ok(j) => to_c_string(j),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn bubble_trash_remove_info(files_path: *const c_char) -> bool {
    if files_path.is_null() {
        return false;
    }
    let p = match CStr::from_ptr(files_path).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    trash::remove_info(Path::new(p))
}

// ==========================================
// FILE OPERATIONS C-ABI
// ==========================================

#[no_mangle]
pub unsafe extern "C" fn bubble_fileops_copy_items(
    sources_json: *const c_char,
    dest_dir: *const c_char,
) -> i64 {
    if sources_json.is_null() || dest_dir.is_null() {
        return -1;
    }
    let s_str = match CStr::from_ptr(sources_json).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let d_str = match CStr::from_ptr(dest_dir).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let sources: Vec<String> = match serde_json::from_str(s_str) {
        Ok(v) => v,
        Err(_) => return -1,
    };
    let path_bufs: Vec<PathBuf> = sources.into_iter().map(PathBuf::from).collect();
    match file_ops::copy_items(&path_bufs, Path::new(d_str)) {
        Ok(bytes) => bytes as i64,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn bubble_fileops_move_items(
    sources_json: *const c_char,
    dest_dir: *const c_char,
) -> bool {
    if sources_json.is_null() || dest_dir.is_null() {
        return false;
    }
    let s_str = match CStr::from_ptr(sources_json).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    let d_str = match CStr::from_ptr(dest_dir).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    let sources: Vec<String> = match serde_json::from_str(s_str) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let path_bufs: Vec<PathBuf> = sources.into_iter().map(PathBuf::from).collect();
    file_ops::move_items(&path_bufs, Path::new(d_str)).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn bubble_fileops_delete_items(paths_json: *const c_char) -> usize {
    if paths_json.is_null() {
        return 0;
    }
    let p_str = match CStr::from_ptr(paths_json).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let paths: Vec<String> = match serde_json::from_str(p_str) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    let path_bufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    file_ops::delete_items(&path_bufs).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn bubble_fileops_create_folder(
    parent: *const c_char,
    name: *const c_char,
) -> bool {
    if parent.is_null() || name.is_null() {
        return false;
    }
    let p = match CStr::from_ptr(parent).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    let n = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    file_ops::create_folder(Path::new(p), n).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn bubble_fileops_create_file(
    parent: *const c_char,
    name: *const c_char,
) -> bool {
    if parent.is_null() || name.is_null() {
        return false;
    }
    let p = match CStr::from_ptr(parent).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    let n = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    file_ops::create_file(Path::new(p), n).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn bubble_fileops_unique_name(
    dest_dir: *const c_char,
    desired_name: *const c_char,
    blocked_json: *const c_char,
) -> *mut c_char {
    if dest_dir.is_null() || desired_name.is_null() {
        return std::ptr::null_mut();
    }
    let d = match CStr::from_ptr(dest_dir).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let n = match CStr::from_ptr(desired_name).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let blocked: Vec<String> = if !blocked_json.is_null() {
        CStr::from_ptr(blocked_json)
            .to_str()
            .ok()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let unique = file_ops::unique_name_for_destination(Path::new(d), n, &blocked);
    to_c_string(unique)
}

#[no_mangle]
pub unsafe extern "C" fn bubble_fileops_conflict_backup(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let backup = file_ops::conflict_backup_path(Path::new(p));
    to_c_string(backup.to_string_lossy())
}

#[no_mangle]
pub unsafe extern "C" fn bubble_fileops_breadcrumb_segments(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let segs = file_ops::breadcrumb_segments(p);
    #[derive(serde::Serialize)]
    struct BreadcrumbJson {
        name: String,
        path: String,
    }
    let json_segs: Vec<BreadcrumbJson> = segs
        .into_iter()
        .map(|s| BreadcrumbJson {
            name: s.name,
            path: s.path,
        })
        .collect();
    match serde_json::to_string(&json_segs) {
        Ok(j) => to_c_string(j),
        Err(_) => std::ptr::null_mut(),
    }
}

// ==========================================
// CONFIG C-ABI
// ==========================================

#[no_mangle]
pub unsafe extern "C" fn bubble_config_load(path: *const c_char) -> *mut BubbleConfig {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let cfg = BubbleConfig::load_from_path(Path::new(p)).unwrap_or_default();
    Box::into_raw(Box::new(cfg))
}

#[no_mangle]
pub unsafe extern "C" fn bubble_config_save(
    config: *mut BubbleConfig,
    path: *const c_char,
) -> bool {
    if config.is_null() || path.is_null() {
        return false;
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    (*config).save_to_path(Path::new(p)).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn bubble_config_free(config: *mut BubbleConfig) {
    if !config.is_null() {
        drop(Box::from_raw(config));
    }
}

#[no_mangle]
pub unsafe extern "C" fn bubble_config_to_json(config: *mut BubbleConfig) -> *mut c_char {
    if config.is_null() {
        return std::ptr::null_mut();
    }
    match serde_json::to_string(&*config) {
        Ok(j) => to_c_string(j),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn bubble_config_update_from_json(
    config: *mut BubbleConfig,
    json_str: *const c_char,
) -> bool {
    if config.is_null() || json_str.is_null() {
        return false;
    }
    let j = match CStr::from_ptr(json_str).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    match serde_json::from_str::<BubbleConfig>(j) {
        Ok(updated) => {
            *config = updated;
            true
        }
        Err(_) => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn bubble_config_sample_template() -> *mut c_char {
    to_c_string(BubbleConfig::documented_template())
}

#[no_mangle]
pub unsafe extern "C" fn bubble_config_seed_sample(path: *const c_char) -> bool {
    if path.is_null() {
        return false;
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    BubbleConfig::seed_sample_file(Path::new(p)).is_ok()
}

// ==========================================
// UNDO C-ABI
// ==========================================

#[no_mangle]
pub unsafe extern "C" fn bubble_undo_new(max_depth: usize) -> *mut UndoManager {
    Box::into_raw(Box::new(UndoManager::new(max_depth)))
}

#[no_mangle]
pub unsafe extern "C" fn bubble_undo_free(undo: *mut UndoManager) {
    if !undo.is_null() {
        drop(Box::from_raw(undo));
    }
}

#[no_mangle]
pub unsafe extern "C" fn bubble_undo_can_undo(undo: *mut UndoManager) -> bool {
    if undo.is_null() {
        return false;
    }
    (*undo).can_undo()
}

#[no_mangle]
pub unsafe extern "C" fn bubble_undo_can_redo(undo: *mut UndoManager) -> bool {
    if undo.is_null() {
        return false;
    }
    (*undo).can_redo()
}

#[no_mangle]
pub unsafe extern "C" fn bubble_undo_undo(undo: *mut UndoManager) -> *mut c_char {
    if undo.is_null() {
        return std::ptr::null_mut();
    }
    match (*undo).undo() {
        Ok(Some(desc)) => to_c_string(desc),
        _ => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn bubble_undo_redo(undo: *mut UndoManager) -> *mut c_char {
    if undo.is_null() {
        return std::ptr::null_mut();
    }
    match (*undo).redo() {
        Ok(Some(desc)) => to_c_string(desc),
        _ => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn bubble_undo_record_rename(
    undo: *mut UndoManager,
    old_path: *const c_char,
    new_path: *const c_char,
) {
    if undo.is_null() || old_path.is_null() || new_path.is_null() {
        return;
    }
    let old_p = match CStr::from_ptr(old_path).to_str() {
        Ok(s) => PathBuf::from(s),
        Err(_) => return,
    };
    let new_p = match CStr::from_ptr(new_path).to_str() {
        Ok(s) => PathBuf::from(s),
        Err(_) => return,
    };
    (*undo).record(
        crate::undo::UndoAction::Rename {
            old_path: old_p,
            new_path: new_p,
        },
        "Rename",
    );
}

#[no_mangle]
pub unsafe extern "C" fn bubble_undo_clear(undo: *mut UndoManager) {
    if !undo.is_null() {
        (*undo).clear();
    }
}

#[no_mangle]
pub unsafe extern "C" fn bubble_theme_load_colors(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    let p = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let colors = crate::config::load_theme_colors(Path::new(p));
    match serde_json::to_string(&colors) {
        Ok(j) => to_c_string(j),
        Err(_) => std::ptr::null_mut(),
    }
}

