use crate::vault::VaultService;
use crate::crypto;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::path::Path;

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

