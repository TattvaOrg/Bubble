use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrashEntry {
    pub name: String,
    pub files_path: PathBuf,
    pub original_path: Option<PathBuf>,
    pub deleted_at: Option<DateTime<Utc>>,
}

fn uid_string() -> String {
    nix::unistd::geteuid().to_string()
}

fn xdg_data_trash_root() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME").map(|p| PathBuf::from(p).join("Trash"))
}

pub fn home_root() -> PathBuf {
    dirs::home_dir()
        .map(|home| home.join(".local/share/Trash"))
        .unwrap_or_else(|| PathBuf::from("/tmp/Trash"))
}

fn looks_like_trash_root(root: &Path) -> bool {
    root.join("files").is_dir() && root.join("info").is_dir()
}

fn trash_root_for_entry(files_path: &Path) -> Option<PathBuf> {
    let parent = files_path.parent()?;
    if parent.file_name()? != "files" {
        return None;
    }
    let root = parent.parent()?;
    if looks_like_trash_root(root) {
        Some(root.to_path_buf())
    } else {
        None
    }
}

fn volume_root_for(trash_root: &Path) -> Option<PathBuf> {
    let file_name = trash_root.file_name()?.to_string_lossy();
    if file_name.starts_with(".Trash-") {
        return trash_root.parent().map(|p| p.to_path_buf());
    }
    let parent = trash_root.parent()?;
    if parent.file_name()?.to_string_lossy() == ".Trash" {
        return parent.parent().map(|p| p.to_path_buf());
    }
    None
}

pub fn roots() -> Vec<PathBuf> {
    let mut found = Vec::new();

    // Home candidates: $XDG_DATA_HOME/Trash and ~/.local/share/Trash
    if let Some(xdg) = xdg_data_trash_root() {
        if looks_like_trash_root(&xdg) {
            found.push(xdg);
        }
    }
    let hr = home_root();
    if looks_like_trash_root(&hr) && !found.contains(&hr) {
        found.push(hr);
    }

    // Mount points from /proc/mounts
    let uid = uid_string();
    if let Ok(file) = File::open("/proc/mounts") {
        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let mount_point = Path::new(parts[1]);
                let cand1 = mount_point.join(format!(".Trash-{}", uid));
                if looks_like_trash_root(&cand1) && !found.contains(&cand1) {
                    found.push(cand1);
                }
                let cand2 = mount_point.join(".Trash").join(&uid);
                if looks_like_trash_root(&cand2) && !found.contains(&cand2) {
                    found.push(cand2);
                }
            }
        }
    }

    found
}

pub fn info_path_for(files_path: &Path) -> Option<PathBuf> {
    let root = trash_root_for_entry(files_path)?;
    let filename = files_path.file_name()?;
    let mut info_name = filename.to_os_string();
    info_name.push(".trashinfo");
    Some(root.join("info").join(info_name))
}

pub fn read_entry(files_path: &Path) -> TrashEntry {
    let name = files_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut entry = TrashEntry {
        name,
        files_path: files_path.to_path_buf(),
        original_path: None,
        deleted_at: None,
    };

    let info_path = match info_path_for(files_path) {
        Some(p) => p,
        None => return entry,
    };

    let file = match File::open(&info_path) {
        Ok(f) => f,
        Err(_) => return entry,
    };

    let reader = BufReader::new(file);
    for line in reader.lines().map_while(Result::ok) {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Path=") {
            let decoded = urlencoding_decode(rest);
            let p = Path::new(&decoded);
            if p.is_absolute() {
                entry.original_path = Some(p.to_path_buf());
            } else if let Some(trash_root) = trash_root_for_entry(files_path) {
                if let Some(volume) = volume_root_for(&trash_root) {
                    entry.original_path = Some(volume.join(p));
                }
            }
        } else if let Some(rest) = trimmed.strip_prefix("DeletionDate=") {
            if let Ok(dt) = DateTime::parse_from_rfc3339(rest) {
                entry.deleted_at = Some(dt.with_timezone(&Utc));
            } else if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(rest, "%Y-%m-%dT%H:%M:%S") {
                entry.deleted_at = Some(DateTime::from_naive_utc_and_offset(naive, Utc));
            }
        }
    }

    entry
}

fn urlencoding_decode(s: &str) -> String {
    // Percent decoding for XDG trash paths
    if let Ok(url) = Url::parse(&format!("file:///{}", s.trim_start_matches('/'))) {
        if let Ok(path) = url.to_file_path() {
            return path.to_string_lossy().to_string();
        }
    }
    // Fallback simple percent decoding
    percent_decode_str(s)
}

fn percent_decode_str(input: &str) -> String {
    let mut bytes = Vec::with_capacity(input.len());
    let mut chars = input.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let h1 = chars.next();
            let h2 = chars.next();
            if let (Some(c1), Some(c2)) = (h1, h2) {
                if let Ok(val) = u8::from_str_radix(&format!("{}{}", c1 as char, c2 as char), 16) {
                    bytes.push(val);
                    continue;
                }
            }
        }
        bytes.push(b);
    }
    String::from_utf8_lossy(&bytes).to_string()
}

pub fn scan() -> Vec<TrashEntry> {
    let mut entries = Vec::new();
    for root in roots() {
        let files_dir = root.join("files");
        if let Ok(read_dir) = fs::read_dir(files_dir) {
            for item in read_dir.flatten() {
                entries.push(read_entry(&item.path()));
            }
        }
    }
    entries
}

pub fn remove_info(files_path: &Path) -> bool {
    if let Some(info) = info_path_for(files_path) {
        if info.exists() {
            return fs::remove_file(info).is_ok();
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trash_entry_parsing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root = temp_dir.path();
        let files_dir = root.join("files");
        let info_dir = root.join("info");
        fs::create_dir_all(&files_dir).unwrap();
        fs::create_dir_all(&info_dir).unwrap();

        let file = files_dir.join("test.txt");
        fs::write(&file, "trashed data").unwrap();

        let info = info_dir.join("test.txt.trashinfo");
        fs::write(
            &info,
            "[Trash Info]\nPath=/home/user/test%20file.txt\nDeletionDate=2026-09-11T20:00:00\n",
        )
        .unwrap();

        assert!(looks_like_trash_root(root));
        let entry = read_entry(&file);
        assert_eq!(entry.name, "test.txt");
        assert_eq!(entry.files_path, file);
        assert_eq!(
            entry.original_path,
            Some(PathBuf::from("/home/user/test file.txt"))
        );
        assert!(entry.deleted_at.is_some());

        assert!(remove_info(&file));
        assert!(!info.exists());
    }
}
