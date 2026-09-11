use std::env;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use bubble_core::crypto::{self, KEY_SIZE};
use bubble_core::vault_db::VaultDatabase;

fn decode_base64(s: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut lookup = [0xFFu8; 256];
    for (i, &b) in TABLE.iter().enumerate() {
        lookup[b as usize] = i as u8;
    }

    let clean: Vec<u8> = s.bytes().filter(|&b| b != b'=' && !b.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity((clean.len() * 3) / 4);

    let chunks = clean.chunks_exact(4);
    let remainder = chunks.remainder();

    for chunk in chunks {
        let b0 = lookup[chunk[0] as usize];
        let b1 = lookup[chunk[1] as usize];
        let b2 = lookup[chunk[2] as usize];
        let b3 = lookup[chunk[3] as usize];
        if b0 == 0xFF || b1 == 0xFF || b2 == 0xFF || b3 == 0xFF {
            return None;
        }
        out.push((b0 << 2) | (b1 >> 4));
        out.push((b1 << 4) | (b2 >> 2));
        out.push((b2 << 6) | b3);
    }

    match remainder.len() {
        2 => {
            let b0 = lookup[remainder[0] as usize];
            let b1 = lookup[remainder[1] as usize];
            if b0 == 0xFF || b1 == 0xFF {
                return None;
            }
            out.push((b0 << 2) | (b1 >> 4));
        }
        3 => {
            let b0 = lookup[remainder[0] as usize];
            let b1 = lookup[remainder[1] as usize];
            let b2 = lookup[remainder[2] as usize];
            if b0 == 0xFF || b1 == 0xFF || b2 == 0xFF {
                return None;
            }
            out.push((b0 << 2) | (b1 >> 4));
            out.push((b1 << 4) | (b2 >> 2));
        }
        _ => {}
    }

    Some(out)
}

fn unlock_chattr(path: &Path) {
    let _ = Command::new("chattr").arg("-i").arg(path).status();
}

fn shred_and_remove(path: &Path) {
    if !path.exists() {
        return;
    }

    unlock_chattr(path);

    if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                shred_and_remove(&entry.path());
            }
        }
        let _ = std::fs::remove_dir(path);
    } else {
        println!("Shredding locked file: {}", path.display());
        let _ = crypto::shred_file(path);
    }
}

fn process_vault_db(db_path: &Path) {
    if !db_path.exists() {
        return;
    }

    println!("Processing vault at: {}", db_path.display());
    if let Ok(db) = VaultDatabase::open(db_path) {
        if let Ok(paths) = db.all_locked_paths() {
            for path_str in paths {
                shred_and_remove(Path::new(&path_str));
            }
        }
    }

    let _ = std::fs::remove_file(db_path);
    let wal = db_path.with_extension("db-wal");
    let shm = db_path.with_extension("db-shm");
    let _ = std::fs::remove_file(wal);
    let _ = std::fs::remove_file(shm);

    println!("Vault database destroyed: {}", db_path.display());
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Mode: Watch PID
    if let Some(idx) = args.iter().position(|a| a == "--watch") {
        if args.len() > idx + 3 {
            let file_path = &args[idx + 1];
            let pid: libc::pid_t = args[idx + 2].parse().unwrap_or(0);
            let key_b64 = &args[idx + 3];

            if let Some(key_bytes) = decode_base64(key_b64) {
                if key_bytes.len() == KEY_SIZE {
                    // Wait while pid is running
                    while pid > 0 && unsafe { libc::kill(pid, 0) } == 0 {
                        thread::sleep(Duration::from_millis(500));
                    }

                    // Wait for fuser to ensure file handle closed
                    for _ in 0..5 {
                        if let Ok(output) = Command::new("fuser").arg(file_path).output() {
                            if !output.stdout.is_empty() {
                                thread::sleep(Duration::from_millis(1000));
                                continue;
                            }
                        }
                        break;
                    }

                    let p = Path::new(file_path);
                    if p.exists() {
                        if let Ok(new_iv) = crypto::encrypt_file(p, &key_bytes) {
                            if let Some(home) = dirs::home_dir() {
                                let db_path = home.join(".config/bubble/vault.db");
                                if let Ok(db) = VaultDatabase::open(&db_path) {
                                    if let Ok(Some(mut entry)) = db.find_by_path(file_path) {
                                        entry.enc_iv = Some(new_iv.to_vec());
                                        let _ = db.update_entry(&entry);
                                        let _ = db.remove_session(entry.id);
                                    }
                                }
                            }
                        }

                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let _ = std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o000));
                            let _ = xattr::set(p, "user.bubble.locked", b"1");
                        }
                    }
                }
            }
            return;
        }
    }

    let all_users = args.iter().any(|a| a == "--all-users");

    if all_users {
        println!("Destroying Bubble vaults for all users...");
        if let Ok(entries) = std::fs::read_dir("/home") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let vault_db = path.join(".config/bubble/vault.db");
                    process_vault_db(&vault_db);
                }
            }
        }
        process_vault_db(Path::new("/root/.config/bubble/vault.db"));
    } else if let Some(home) = dirs::home_dir() {
        let vault_db = home.join(".config/bubble/vault.db");
        process_vault_db(&vault_db);
    }

    println!("Bubble vault cleanup complete.");
}
