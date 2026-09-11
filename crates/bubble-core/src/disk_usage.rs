use std::collections::HashSet;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InodeId {
    pub dev: u64,
    pub ino: u64,
}

#[derive(Debug, Clone, Default)]
pub struct DiskUsageResult {
    pub total_bytes: u64,
    pub unreadable_count: u32,
    pub cancelled: bool,
}

pub fn format_size(bytes: u64, verbose: bool) -> String {
    if bytes < 1024 {
        if verbose {
            format!("{} B ({} bytes)", bytes, bytes)
        } else {
            format!("{} B", bytes)
        }
    } else if bytes < 1024 * 1024 {
        let kb = bytes as f64 / 1024.0;
        let s = format!("{:.1} KB", kb);
        if verbose {
            format!("{} ({} bytes)", s, bytes)
        } else {
            s
        }
    } else if bytes < 1024 * 1024 * 1024 {
        let mb = bytes as f64 / (1024.0 * 1024.0);
        let s = format!("{:.1} MB", mb);
        if verbose {
            format!("{} ({} bytes)", s, bytes)
        } else {
            s
        }
    } else {
        let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let precision = if verbose { 2 } else { 1 };
        let s = format!("{:.1$} GB", gb, precision);
        if verbose {
            format!("{} ({} bytes)", s, bytes)
        } else {
            s
        }
    }
}

pub fn calculate_path_size(
    path: &Path,
    seen_inodes: &mut HashSet<InodeId>,
    cancelled: &Arc<AtomicBool>,
) -> DiskUsageResult {
    let mut total_bytes = 0u64;
    let mut unreadable_count = 0u32;

    if cancelled.load(Ordering::Relaxed) {
        return DiskUsageResult {
            total_bytes: 0,
            unreadable_count: 0,
            cancelled: true,
        };
    }

    let walker = WalkDir::new(path).same_file_system(false);

    for entry_result in walker {
        if cancelled.load(Ordering::Relaxed) {
            return DiskUsageResult {
                total_bytes,
                unreadable_count,
                cancelled: true,
            };
        }

        match entry_result {
            Ok(entry) => {
                // Read symlink metadata directly to avoid following broken links or infinite loops
                match entry.path().symlink_metadata() {
                    Ok(meta) => {
                        let id = InodeId {
                            dev: meta.dev(),
                            ino: meta.ino(),
                        };
                        // Deduplicate hard links
                        if seen_inodes.insert(id) {
                            if !meta.is_dir() {
                                total_bytes += meta.len();
                            }
                        }
                    }
                    Err(_) => {
                        unreadable_count += 1;
                    }
                }
            }
            Err(_) => {
                unreadable_count += 1;
            }
        }
    }

    DiskUsageResult {
        total_bytes,
        unreadable_count,
        cancelled: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500, false), "500 B");
        assert_eq!(format_size(500, true), "500 B (500 bytes)");
        assert_eq!(format_size(2048, false), "2.0 KB");
        assert_eq!(format_size(5 * 1024 * 1024, false), "5.0 MB");
        assert_eq!(format_size(2 * 1024 * 1024 * 1024, false), "2.0 GB");
    }

    #[test]
    fn test_calculate_size() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("sub/file2.txt");
        std::fs::create_dir_all(file2.parent().unwrap()).unwrap();

        std::fs::write(&file1, "12345").unwrap();
        std::fs::write(&file2, "1234567890").unwrap();

        let mut seen = HashSet::new();
        let cancelled = Arc::new(AtomicBool::new(false));

        let res = calculate_path_size(temp_dir.path(), &mut seen, &cancelled);
        assert!(!res.cancelled);
        assert_eq!(res.total_bytes, 15);
    }
}
