use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

use crate::runtime_features::which;

pub fn find_files(
    root_path: &Path,
    pattern: &str,
    show_hidden: bool,
    max_results: usize,
    cancelled: &Arc<AtomicBool>,
) -> Vec<PathBuf> {
    if which("fd") || which("fdfind") {
        find_files_fd(root_path, pattern, show_hidden, max_results, cancelled)
    } else {
        find_files_fallback(root_path, pattern, show_hidden, max_results, cancelled)
    }
}

pub fn find_files_fd(
    root_path: &Path,
    pattern: &str,
    show_hidden: bool,
    max_results: usize,
    cancelled: &Arc<AtomicBool>,
) -> Vec<PathBuf> {
    let fd_bin = if which("fd") { "fd" } else { "fdfind" };
    let mut cmd = Command::new(fd_bin);
    cmd.arg("--color").arg("never");

    if show_hidden {
        cmd.arg("--hidden");
    }

    cmd.arg(pattern);
    cmd.arg(root_path);

    let output = match cmd.output() {
        Ok(o) => o,
        Err(_) => return find_files_fallback(root_path, pattern, show_hidden, max_results, cancelled),
    };

    let mut results = Vec::new();
    let out_str = String::from_utf8_lossy(&output.stdout);
    for line in out_str.lines() {
        if cancelled.load(Ordering::Relaxed) || results.len() >= max_results {
            break;
        }
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            results.push(PathBuf::from(trimmed));
        }
    }
    results
}

pub fn find_files_fallback(
    root_path: &Path,
    pattern: &str,
    show_hidden: bool,
    max_results: usize,
    cancelled: &Arc<AtomicBool>,
) -> Vec<PathBuf> {
    let mut results = Vec::new();
    let query_lower = pattern.to_lowercase();

    let mut walker = WalkDir::new(root_path).into_iter();
    while let Some(entry_res) = walker.next() {
        if cancelled.load(Ordering::Relaxed) || results.len() >= max_results {
            break;
        }

        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };

        let file_name = entry.file_name().to_string_lossy();
        if !show_hidden && file_name.starts_with('.') && entry.depth() > 0 {
            if entry.file_type().is_dir() {
                walker.skip_current_dir();
            }
            continue;
        }

        if file_name.to_lowercase().contains(&query_lower) {
            results.push(entry.into_path());
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_files_fallback() {
        let temp_dir = tempfile::tempdir().unwrap();
        let f1 = temp_dir.path().join("match_one.txt");
        let f2 = temp_dir.path().join("ignored.txt");
        let f3 = temp_dir.path().join(".hidden_match.txt");
        std::fs::write(&f1, "1").unwrap();
        std::fs::write(&f2, "2").unwrap();
        std::fs::write(&f3, "3").unwrap();

        let cancelled = Arc::new(AtomicBool::new(false));
        let results = find_files_fallback(temp_dir.path(), "match", false, 100, &cancelled);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], f1);

        let results_hidden = find_files_fallback(temp_dir.path(), "match", true, 100, &cancelled);
        assert_eq!(results_hidden.len(), 2);
    }
}
