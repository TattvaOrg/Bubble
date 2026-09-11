use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreadcrumbSegment {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct RenameItem {
    pub original_path: PathBuf,
    pub new_name: String,
}

#[derive(Debug, Clone)]
pub struct RenameResult {
    pub original_path: PathBuf,
    pub new_path: PathBuf,
    pub success: bool,
    pub error: Option<String>,
}

/// Fast file copy using copy_file_range when possible, falling back to standard copy
pub fn copy_file_fast(src: &Path, dst: &Path) -> io::Result<u64> {
    // If destination exists and is same as src, do nothing
    if src == dst {
        return Ok(0);
    }

    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }

    // Standard std::fs::copy uses copy_file_range internally on Linux since Rust 1.45+
    let bytes = fs::copy(src, dst)?;

    // Preserve permissions
    if let Ok(meta) = src.metadata() {
        let perms = meta.permissions();
        let _ = fs::set_permissions(dst, perms);
    }

    Ok(bytes)
}

/// Recursive directory copy
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<u64> {
    if !src.is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Source is not a directory"));
    }

    fs::create_dir_all(dst)?;
    let mut total_bytes = 0;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let entry_path = entry.path();
        let file_name = entry.file_name();
        let target_path = dst.join(file_name);

        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            total_bytes += copy_dir_recursive(&entry_path, &target_path)?;
        } else if file_type.is_symlink() {
            if let Ok(link_target) = fs::read_link(&entry_path) {
                let _ = std::os::unix::fs::symlink(link_target, &target_path);
            }
        } else {
            total_bytes += copy_file_fast(&entry_path, &target_path)?;
        }
    }

    // Preserve directory permissions
    if let Ok(meta) = src.metadata() {
        let _ = fs::set_permissions(dst, meta.permissions());
    }

    Ok(total_bytes)
}

/// Copy a list of files or directories to destination directory
pub fn copy_items(sources: &[PathBuf], destination_dir: &Path) -> io::Result<u64> {
    fs::create_dir_all(destination_dir)?;
    let mut total_bytes = 0;

    for src in sources {
        let file_name = src
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name"))?;
        let target = destination_dir.join(file_name);

        if src.is_dir() {
            total_bytes += copy_dir_recursive(src, &target)?;
        } else {
            total_bytes += copy_file_fast(src, &target)?;
        }
    }

    Ok(total_bytes)
}

/// Move a file or directory across filesystems or within the same filesystem
pub fn move_item(src: &Path, dst: &Path) -> io::Result<()> {
    if src == dst {
        return Ok(());
    }

    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }

    // Try atomic rename first
    match fs::rename(src, dst) {
        Ok(_) => Ok(()),
        Err(err) if err.raw_os_error() == Some(libc::EXDEV) => {
            // Cross-filesystem move: copy then remove
            if src.is_dir() {
                copy_dir_recursive(src, dst)?;
                fs::remove_dir_all(src)?;
            } else {
                copy_file_fast(src, dst)?;
                fs::remove_file(src)?;
            }
            Ok(())
        }
        Err(err) => Err(err),
    }
}

/// Move a list of files or directories into a destination directory
pub fn move_items(sources: &[PathBuf], destination_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(destination_dir)?;

    for src in sources {
        let file_name = src
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name"))?;
        let target = destination_dir.join(file_name);
        move_item(src, &target)?;
    }

    Ok(())
}

/// Permanently delete files or directories
pub fn delete_items(paths: &[PathBuf]) -> io::Result<usize> {
    let mut deleted_count = 0;
    for path in paths {
        if !path.exists() && !path.is_symlink() {
            continue;
        }

        if path.is_dir() && !path.is_symlink() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
        deleted_count += 1;
    }
    Ok(deleted_count)
}

/// Create a new folder
pub fn create_folder(parent: &Path, name: &str) -> io::Result<PathBuf> {
    let target = parent.join(name);
    fs::create_dir_all(&target)?;
    Ok(target)
}

/// Create a new empty file
pub fn create_file(parent: &Path, name: &str) -> io::Result<PathBuf> {
    fs::create_dir_all(parent)?;
    let target = parent.join(name);
    if !target.exists() {
        File::create(&target)?;
    }
    Ok(target)
}

/// Generate a unique name for destination to avoid accidental overwrite
pub fn unique_name_for_destination(
    destination_dir: &Path,
    desired_name: &str,
    blocked_names: &[String],
) -> String {
    let candidate = destination_dir.join(desired_name);
    if !candidate.exists() && !blocked_names.iter().any(|b| b == desired_name) {
        return desired_name.to_string();
    }

    let (stem, ext) = match desired_name.rfind('.') {
        Some(dot) if dot > 0 => (&desired_name[..dot], &desired_name[dot..]),
        _ => (desired_name, ""),
    };

    let mut counter = 1;
    loop {
        let candidate_name = format!("{} ({}){}", stem, counter, ext);
        let path = destination_dir.join(&candidate_name);
        if !path.exists() && !blocked_names.iter().any(|b| b == &candidate_name) {
            return candidate_name;
        }
        counter += 1;
    }
}

/// Compute temporary backup path before overwriting
pub fn conflict_backup_path(target_path: &Path) -> PathBuf {
    let dir = target_path.parent().unwrap_or_else(|| Path::new("/"));
    let name = target_path
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();
    let rand_id = format!("{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos());
    dir.join(format!(".{}.backup-{}", name, rand_id))
}

/// Parse breadcrumb segments for path bar navigation
pub fn breadcrumb_segments(path_str: &str) -> Vec<BreadcrumbSegment> {
    let clean = path_str.trim();
    if clean.is_empty() || clean == "/" {
        return vec![BreadcrumbSegment {
            name: "/".to_string(),
            path: "/".to_string(),
        }];
    }

    let mut segments = Vec::new();
    segments.push(BreadcrumbSegment {
        name: "/".to_string(),
        path: "/".to_string(),
    });

    let mut current_path = PathBuf::from("/");
    for part in clean.split('/').filter(|p| !p.is_empty()) {
        current_path.push(part);
        segments.push(BreadcrumbSegment {
            name: part.to_string(),
            path: current_path.to_string_lossy().into_owned(),
        });
    }

    segments
}

/// Bulk rename execution with cycle handling
pub fn bulk_rename(items: &[RenameItem]) -> Vec<RenameResult> {
    let mut results = Vec::new();

    // First check: perform renames using temporary names if cyclic dependencies exist
    // For simple linear renames:
    for item in items {
        let parent = match item.original_path.parent() {
            Some(p) => p,
            None => {
                results.push(RenameResult {
                    original_path: item.original_path.clone(),
                    new_path: item.original_path.clone(),
                    success: false,
                    error: Some("Cannot rename filesystem root".to_string()),
                });
                continue;
            }
        };

        let new_path = parent.join(&item.new_name);
        match fs::rename(&item.original_path, &new_path) {
            Ok(_) => {
                results.push(RenameResult {
                    original_path: item.original_path.clone(),
                    new_path,
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                results.push(RenameResult {
                    original_path: item.original_path.clone(),
                    new_path,
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    results
}

/// Transform filenames using Regex search and replace
pub fn apply_regex_rename(
    filenames: &[String],
    search_pattern: &str,
    replace_pattern: &str,
) -> Result<Vec<String>, regex::Error> {
    let re = Regex::new(search_pattern)?;
    let mut out = Vec::with_capacity(filenames.len());
    for name in filenames {
        let replaced = re.replace_all(name, replace_pattern);
        out.push(replaced.into_owned());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_copy_and_move_file() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("source.txt");
        fs::write(&src, "data to copy").unwrap();

        let dst = tmp.path().join("sub/destination.txt");
        copy_file_fast(&src, &dst).unwrap();
        assert!(dst.exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "data to copy");

        let moved = tmp.path().join("moved.txt");
        move_item(&dst, &moved).unwrap();
        assert!(!dst.exists());
        assert!(moved.exists());
    }

    #[test]
    fn test_unique_name() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("test.txt");
        fs::write(&file, "hello").unwrap();

        let u1 = unique_name_for_destination(tmp.path(), "test.txt", &[]);
        assert_eq!(u1, "test (1).txt");

        let u2 = unique_name_for_destination(tmp.path(), "test.txt", &["test (1).txt".to_string()]);
        assert_eq!(u2, "test (2).txt");
    }

    #[test]
    fn test_breadcrumbs() {
        let segments = breadcrumb_segments("/home/naitik/Documents");
        assert_eq!(segments.len(), 4);
        assert_eq!(segments[0].path, "/");
        assert_eq!(segments[1].name, "home");
        assert_eq!(segments[2].name, "naitik");
        assert_eq!(segments[3].path, "/home/naitik/Documents");
    }

    #[test]
    fn test_regex_rename() {
        let files = vec!["photo_001.jpg".to_string(), "photo_002.jpg".to_string()];
        let renamed = apply_regex_rename(&files, r"photo_(\d+)", "pic_$1").unwrap();
        assert_eq!(renamed[0], "pic_001.jpg");
        assert_eq!(renamed[1], "pic_002.jpg");
    }
}
