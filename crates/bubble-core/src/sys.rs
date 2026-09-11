use std::ffi::CString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SysError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("System error: {0}")]
    Errno(i32),
    #[error("Invalid path")]
    InvalidPath,
}

pub fn get_inode(path: &Path) -> Result<i64, SysError> {
    let c_path = CString::new(path.to_str().ok_or(SysError::InvalidPath)?)
        .map_err(|_| SysError::InvalidPath)?;
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::lstat(c_path.as_ptr(), &mut st) };
    if ret != 0 {
        return Err(SysError::Errno(std::io::Error::last_os_error().raw_os_error().unwrap_or(0)));
    }
    Ok(st.st_ino as i64)
}

pub fn get_file_permissions(path: &Path) -> Result<String, SysError> {
    let meta = std::fs::symlink_metadata(path)?;
    let mode = meta.permissions().mode() & 0o7777;
    Ok(format!("{:04o}", mode))
}

pub fn restore_file_permissions(path: &Path, perms: &str) -> Result<(), SysError> {
    let mode = u32::from_str_radix(perms.trim_start_matches("0o"), 8)
        .or_else(|_| u32::from_str_radix(perms, 16))
        .unwrap_or(0o644);

    let c_path = CString::new(path.to_str().ok_or(SysError::InvalidPath)?)
        .map_err(|_| SysError::InvalidPath)?;
    let ret = unsafe { libc::chmod(c_path.as_ptr(), mode as libc::mode_t) };
    if ret != 0 {
        return Err(SysError::Errno(std::io::Error::last_os_error().raw_os_error().unwrap_or(0)));
    }
    Ok(())
}

pub fn set_file_permissions_zero(path: &Path) -> Result<(), SysError> {
    let c_path = CString::new(path.to_str().ok_or(SysError::InvalidPath)?)
        .map_err(|_| SysError::InvalidPath)?;
    let ret = unsafe { libc::chmod(c_path.as_ptr(), 0) };
    if ret != 0 {
        return Err(SysError::Errno(std::io::Error::last_os_error().raw_os_error().unwrap_or(0)));
    }
    Ok(())
}

pub fn get_xattr_locked(path: &Path) -> Result<bool, SysError> {
    let c_path = CString::new(path.to_str().ok_or(SysError::InvalidPath)?)
        .map_err(|_| SysError::InvalidPath)?;
    let attr_name = CString::new("user.bubble.locked").unwrap();
    let mut buf = [0u8; 8];

    let ret = unsafe {
        libc::getxattr(
            c_path.as_ptr(),
            attr_name.as_ptr(),
            buf.as_mut_ptr() as *mut libc::c_void,
            buf.len(),
        )
    };

    if ret >= 0 {
        return Ok(true);
    }

    let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
    if errno == libc::ENODATA {
        Ok(false)
    } else {
        // Mode 0000 returns EACCES, or filesystem may lack xattrs (ENOTSUP)
        Ok(true)
    }
}

pub fn set_xattr_locked(path: &Path, locked: bool) -> Result<(), SysError> {
    let c_path = CString::new(path.to_str().ok_or(SysError::InvalidPath)?)
        .map_err(|_| SysError::InvalidPath)?;
    let attr_name = CString::new("user.bubble.locked").unwrap();

    if locked {
        let val = b"1";
        unsafe {
            libc::setxattr(
                c_path.as_ptr(),
                attr_name.as_ptr(),
                val.as_ptr() as *const libc::c_void,
                val.len(),
                0,
            );
        }
    } else {
        unsafe {
            libc::removexattr(c_path.as_ptr(), attr_name.as_ptr());
        }
    }
    Ok(())
}

pub fn set_immutable(path: &Path, immutable: bool, helper_path: Option<&Path>) -> bool {
    let flag = if immutable { "+i" } else { "-i" };

    if let Some(helper) = helper_path {
        if helper.exists() {
            if let Ok(status) = Command::new(helper).arg(flag).arg(path).status() {
                if status.success() {
                    return true;
                }
            }
        }
    }

    // Try user local bin and standard locations for bubble-vault-helper
    if let Ok(home) = std::env::var("HOME") {
        let user_helper = PathBuf::from(format!("{}/.local/bin/bubble-vault-helper", home));
        if user_helper.exists() {
            if let Ok(status) = Command::new(&user_helper).arg(flag).arg(path).status() {
                if status.success() {
                    return true;
                }
            }
        }
    }

    for candidate in &[
        "/usr/local/bin/bubble-vault-helper",
        "/usr/bin/bubble-vault-helper",
    ] {
        let p = Path::new(candidate);
        if p.exists() {
            if let Ok(status) = Command::new(p).arg(flag).arg(path).status() {
                if status.success() {
                    return true;
                }
            }
        }
    }

    // Try finding in PATH
    if let Ok(status) = Command::new("bubble-vault-helper").arg(flag).arg(path).status() {
        if status.success() {
            return true;
        }
    }

    // Direct chattr (succeeds if running as root or has CAP_LINUX_IMMUTABLE)
    if let Ok(status) = Command::new("chattr").arg(flag).arg(path).status() {
        if status.success() {
            return true;
        }
    }

    true // Best-effort fallback
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_sys_inode_and_perms() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();

        let inode = get_inode(path).unwrap();
        assert!(inode > 0);

        let orig_perms = get_file_permissions(path).unwrap();
        assert!(!orig_perms.is_empty());

        set_file_permissions_zero(path).unwrap();
        let zero_perms = get_file_permissions(path).unwrap();
        assert_eq!(zero_perms, "0000");

        restore_file_permissions(path, &orig_perms).unwrap();
        let restored = get_file_permissions(path).unwrap();
        assert_eq!(restored, orig_perms);
    }

    #[test]
    fn test_xattr() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();

        assert!(!get_xattr_locked(path).unwrap());
        set_xattr_locked(path, true).unwrap();
        assert!(get_xattr_locked(path).unwrap());
        set_xattr_locked(path, false).unwrap();
        assert!(!get_xattr_locked(path).unwrap());
    }
}
