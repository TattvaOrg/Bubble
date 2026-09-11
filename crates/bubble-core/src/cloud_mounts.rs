use std::path::{Path, PathBuf};

// Where the rclone mounts live. Five call sites across the model, the git
// service and the file operations need to recognise these paths, so the
// directory is defined once here rather than spelled out in each of them.
pub fn cloud_mounts_base_dir() -> PathBuf {
    dirs::home_dir()
        .map(|home| home.join(".local/share/bubble/mounts"))
        .unwrap_or_else(|| PathBuf::from("/tmp/bubble/mounts"))
}

// True for a path inside a mount, not for the mounts directory itself: the
// container is an ordinary local folder and must stay browsable.
pub fn is_cloud_mount_path(path: &Path) -> bool {
    let base = cloud_mounts_base_dir();
    if let Ok(stripped) = path.strip_prefix(&base) {
        !stripped.as_os_str().is_empty()
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_mounts_base_dir() {
        let base = cloud_mounts_base_dir();
        assert!(base.ends_with(".local/share/bubble/mounts"));
    }

    #[test]
    fn test_is_cloud_mount_path() {
        let base = cloud_mounts_base_dir();
        assert!(!is_cloud_mount_path(&base));
        assert!(is_cloud_mount_path(&base.join("remote1")));
        assert!(is_cloud_mount_path(&base.join("remote1/subfolder")));
        assert!(!is_cloud_mount_path(Path::new("/tmp/some/other/path")));
    }
}
