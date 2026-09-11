use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrashEntry {
    pub name: String,
    pub files_path: PathBuf,
    pub original_path: PathBuf,
    pub deleted_at: String, // ISO-8601 string
}

pub fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(val) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..=i + 2]).unwrap_or(""), 16) {
                out.push(val);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

pub fn home_trash_root() -> PathBuf {
    if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        if !xdg_data.trim().is_empty() {
            return PathBuf::from(xdg_data).join("Trash");
        }
    }
    if let Some(home) = dirs::home_dir() {
        home.join(".local/share/Trash")
    } else {
        PathBuf::from("/tmp/Trash")
    }
}

pub fn looks_like_trash_root(root: &Path) -> bool {
    root.join("files").is_dir() && root.join("info").is_dir()
}

pub fn trash_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let home_root = home_trash_root();
    if looks_like_trash_root(&home_root) {
        roots.push(home_root);
    } else if let Some(home) = dirs::home_dir() {
        let alt = home.join(".local/share/Trash");
        if looks_like_trash_root(&alt) && !roots.contains(&alt) {
            roots.push(alt);
        }
    }

    // Check mounted volumes via /proc/mounts
    let uid = unsafe { libc::geteuid() };
    if let Ok(file) = File::open("/proc/mounts") {
        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let mount_point = Path::new(parts[1]);
                let c1 = mount_point.join(format!(".Trash-{}", uid));
                if looks_like_trash_root(&c1) && !roots.contains(&c1) {
                    roots.push(c1);
                }
                let c2 = mount_point.join(".Trash").join(uid.to_string());
                if looks_like_trash_root(&c2) && !roots.contains(&c2) {
                    roots.push(c2);
                }
            }
        }
    }

    roots
}

pub fn trash_root_for_entry(files_path: &Path) -> Option<PathBuf> {
    let parent = files_path.parent()?;
    if parent.file_name()? == "files" {
        let root = parent.parent()?;
        if looks_like_trash_root(root) {
            return Some(root.to_path_buf());
        }
    }
    None
}

pub fn volume_root_for(trash_root: &Path) -> Option<PathBuf> {
    let name = trash_root.file_name()?.to_string_lossy();
    if name.starts_with(".Trash-") {
        return trash_root.parent().map(|p| p.to_path_buf());
    }
    let parent = trash_root.parent()?;
    if parent.file_name()? == ".Trash" {
        return parent.parent().map(|p| p.to_path_buf());
    }
    None
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
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut entry = TrashEntry {
        name,
        files_path: files_path.to_path_buf(),
        original_path: files_path.to_path_buf(),
        deleted_at: String::new(),
    };

    if let Some(info_path) = info_path_for(files_path) {
        if let Ok(file) = File::open(&info_path) {
            let reader = BufReader::new(file);
            for line in reader.lines().map_while(Result::ok) {
                let line = line.trim();
                if let Some(path_str) = line.strip_prefix("Path=") {
                    let decoded = percent_decode(path_str);
                    let path = Path::new(&decoded);
                    if path.is_absolute() {
                        entry.original_path = path.to_path_buf();
                    } else if let Some(root) = trash_root_for_entry(files_path) {
                        if let Some(vol) = volume_root_for(&root) {
                            entry.original_path = vol.join(path);
                        }
                    }
                } else if let Some(date_str) = line.strip_prefix("DeletionDate=") {
                    entry.deleted_at = date_str.trim().to_string();
                }
            }
        }
    }

    entry
}

pub fn scan_trash() -> Vec<TrashEntry> {
    let mut entries = Vec::new();
    for root in trash_roots() {
        let files_dir = root.join("files");
        if let Ok(read_dir) = fs::read_dir(files_dir) {
            for dir_entry in read_dir.flatten() {
                entries.push(read_entry(&dir_entry.path()));
            }
        }
    }
    entries
}

pub fn remove_info(files_path: &Path) -> bool {
    if let Some(info_path) = info_path_for(files_path) {
        if info_path.exists() {
            return fs::remove_file(info_path).is_ok();
        }
    }
    false
}

pub fn iso_format_now() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let seconds_per_day = 86400;
    let days_since_epoch = (now / seconds_per_day) as i64;
    let day_seconds = (now % seconds_per_day) as u32;

    let hour = day_seconds / 3600;
    let minute = (day_seconds % 3600) / 60;
    let second = day_seconds % 60;

    let z = days_since_epoch + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}", year, m, d, hour, minute, second)
}

pub fn trash_item(path: &Path) -> io::Result<TrashEntry> {
    if !path.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "File does not exist"));
    }

    let canon_path = path.canonicalize()?;
    let root = home_trash_root();
    let files_dir = root.join("files");
    let info_dir = root.join("info");

    fs::create_dir_all(&files_dir)?;
    fs::create_dir_all(&info_dir)?;

    let base_name = canon_path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name"))?
        .to_string_lossy()
        .into_owned();

    let mut candidate_name = base_name.clone();
    let mut counter = 2;
    while files_dir.join(&candidate_name).exists() || info_dir.join(format!("{}.trashinfo", candidate_name)).exists() {
        let (stem, ext) = match base_name.rfind('.') {
            Some(dot) if dot > 0 => (&base_name[..dot], &base_name[dot..]),
            _ => (base_name.as_str(), ""),
        };
        candidate_name = format!("{} {}{}", stem, counter, ext);
        counter += 1;
    }

    let target_file = files_dir.join(&candidate_name);
    let target_info = info_dir.join(format!("{}.trashinfo", candidate_name));

    if let Err(_e) = fs::rename(&canon_path, &target_file) {
        if canon_path.is_dir() {
            copy_dir_recursive(&canon_path, &target_file)?;
            fs::remove_dir_all(&canon_path)?;
        } else {
            fs::copy(&canon_path, &target_file)?;
            fs::remove_file(&canon_path)?;
        }
    }

    let now_str = iso_format_now();
    let mut info_content = String::new();
    info_content.push_str("[Trash Info]\n");
    info_content.push_str(&format!("Path={}\n", percent_encode(&canon_path.to_string_lossy())));
    info_content.push_str(&format!("DeletionDate={}\n", now_str));

    fs::write(&target_info, info_content)?;

    Ok(TrashEntry {
        name: candidate_name,
        files_path: target_file,
        original_path: canon_path,
        deleted_at: now_str,
    })
}

pub fn restore_item(files_path: &Path) -> io::Result<PathBuf> {
    let entry = read_entry(files_path);
    if entry.original_path.as_os_str().is_empty() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "No original path metadata found"));
    }

    if let Some(parent) = entry.original_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut dest = entry.original_path.clone();
    if dest.exists() {
        let file_stem = dest.file_stem().and_then(|s| s.to_str()).unwrap_or("restored");
        let ext = dest.extension().and_then(|s| s.to_str()).unwrap_or("");
        let parent = dest.parent().unwrap_or_else(|| Path::new("/"));
        let mut idx = 1;
        loop {
            let candidate_name = if ext.is_empty() {
                format!("{} (restored {})", file_stem, idx)
            } else {
                format!("{} (restored {}).{}", file_stem, idx, ext)
            };
            let candidate = parent.join(candidate_name);
            if !candidate.exists() {
                dest = candidate;
                break;
            }
            idx += 1;
        }
    }

    if let Err(e) = fs::rename(files_path, &dest) {
        if e.raw_os_error() == Some(libc::EXDEV) {
            if files_path.is_dir() {
                copy_dir_recursive(files_path, &dest)?;
                let _ = fs::remove_dir_all(files_path);
            } else {
                fs::copy(files_path, &dest)?;
                let _ = fs::remove_file(files_path);
            }
        } else {
            return Err(e);
        }
    }
    remove_info(files_path);

    Ok(dest)
}

pub fn empty_trash() -> io::Result<usize> {
    let mut count = 0;
    for root in trash_roots() {
        let files_dir = root.join("files");
        let info_dir = root.join("info");

        if let Ok(read_dir) = fs::read_dir(&files_dir) {
            for entry in read_dir.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let _ = fs::remove_dir_all(&p);
                } else {
                    let _ = fs::remove_file(&p);
                }
                count += 1;
            }
        }

        if let Ok(read_dir) = fs::read_dir(&info_dir) {
            for entry in read_dir.flatten() {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    Ok(count)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_child = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_child)?;
        } else {
            fs::copy(entry.path(), &dest_child)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_percent_encode_decode() {
        let raw = "/home/user/My Documents/file #1.txt";
        let encoded = percent_encode(raw);
        assert_eq!(encoded, "/home/user/My%20Documents/file%20%231.txt");
        let decoded = percent_decode(&encoded);
        assert_eq!(decoded, raw);
    }

    #[test]
    fn test_trash_and_restore() {
        let tmp = tempdir().unwrap();
        let file_path = tmp.path().join("test_doc.txt");
        fs::write(&file_path, "Hello Trash").unwrap();

        let entry = trash_item(&file_path).unwrap();
        assert!(!file_path.exists());
        assert!(entry.files_path.exists());

        let read = read_entry(&entry.files_path);
        assert_eq!(read.name, entry.name);

        let restored = restore_item(&entry.files_path).unwrap();
        assert!(restored.exists());
        assert_eq!(fs::read_to_string(&restored).unwrap(), "Hello Trash");
    }
}
