use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn find_repo_root(path: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .args([
            "--no-optional-locks",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.hooksPath=/dev/null",
            "rev-parse",
            "--show-toplevel",
        ])
        .current_dir(path)
        .output()
        .ok()?;

    if output.status.success() {
        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !root.is_empty() {
            return Some(PathBuf::from(root));
        }
    }
    None
}

pub fn query_git_status(repo_root: &Path) -> Option<(HashMap<PathBuf, String>, HashSet<PathBuf>)> {
    let output = Command::new("git")
        .args([
            "--no-optional-locks",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.hooksPath=/dev/null",
            "status",
            "--porcelain=v1",
            "-z",
            "--ignored=matching",
        ])
        .current_dir(repo_root)
        .output()
        .ok()?;

    if output.status.success() {
        Some(parse_porcelain_v1_z(repo_root, &output.stdout))
    } else {
        None
    }
}

pub fn parse_porcelain_v1_z(
    repo_root: &Path,
    output: &[u8],
) -> (HashMap<PathBuf, String>, HashSet<PathBuf>) {
    let mut status_cache = HashMap::new();
    let mut dirty_dirs = HashSet::new();

    if output.is_empty() {
        return (status_cache, dirty_dirs);
    }

    let entries: Vec<&[u8]> = output.split(|&b| b == 0).collect();
    let mut i = 0;
    while i < entries.len() {
        let entry = entries[i];
        if entry.len() < 4 {
            i += 1;
            continue;
        }

        let x = entry[0] as char;
        let y = entry[1] as char;
        let rel_path_raw = String::from_utf8_lossy(&entry[3..]).to_string();
        let rel_path = rel_path_raw.trim_end_matches('/');
        let abs_path = repo_root.join(rel_path);

        let status = if x == 'U' || y == 'U' || (x == 'A' && y == 'A') || (x == 'D' && y == 'D') {
            Some("conflicted")
        } else if x == 'R' {
            // Next entry is the old or new path for rename, skip it
            i += 1;
            Some("renamed")
        } else if y == 'M' {
            Some("modified")
        } else if y == 'D' {
            Some("deleted")
        } else if (x == 'M' || x == 'A') && y == ' ' {
            Some("staged")
        } else if x == 'D' && y == ' ' {
            Some("deleted")
        } else if x == '?' && y == '?' {
            Some("untracked")
        } else if x == '!' && y == '!' {
            Some("ignored")
        } else {
            None
        };

        if let Some(st) = status {
            status_cache.insert(abs_path.clone(), st.to_string());
            if st != "ignored" {
                // Mark parent directories dirty
                let mut curr = abs_path.parent();
                while let Some(parent) = curr {
                    if parent < repo_root || !parent.starts_with(repo_root) {
                        break;
                    }
                    dirty_dirs.insert(parent.to_path_buf());
                    if parent == repo_root {
                        break;
                    }
                    curr = parent.parent();
                }
            }
        }

        i += 1;
    }

    (status_cache, dirty_dirs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_porcelain_v1_z() {
        let root = Path::new("/repo");
        let mut raw = Vec::new();
        // M file1.txt\0
        raw.extend_from_slice(b" M file1.txt\0");
        // ?? untracked/test.txt\0
        raw.extend_from_slice(b"?? untracked/test.txt\0");
        // !! ignored/tmp.log\0
        raw.extend_from_slice(b"!! ignored/tmp.log\0");

        let (cache, dirty) = parse_porcelain_v1_z(root, &raw);
        assert_eq!(cache.get(&PathBuf::from("/repo/file1.txt")), Some(&"modified".to_string()));
        assert_eq!(cache.get(&PathBuf::from("/repo/untracked/test.txt")), Some(&"untracked".to_string()));
        assert_eq!(cache.get(&PathBuf::from("/repo/ignored/tmp.log")), Some(&"ignored".to_string()));

        assert!(dirty.contains(&PathBuf::from("/repo/untracked")));
        assert!(dirty.contains(&PathBuf::from("/repo")));
        assert!(!dirty.contains(&PathBuf::from("/repo/ignored")));
    }
}
