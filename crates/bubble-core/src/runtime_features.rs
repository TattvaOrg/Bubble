
pub fn which(executable: &str) -> bool {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let full_path = dir.join(executable);
            if full_path.is_file() {
                // In Unix, check if executable
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(meta) = full_path.metadata() {
                        if meta.permissions().mode() & 0o111 != 0 {
                            return true;
                        }
                    }
                }
                #[cfg(not(unix))]
                return true;
            }
        }
    }
    false
}

pub fn is_ffmpeg_available() -> bool {
    which("ffmpeg")
}

pub fn is_bat_available() -> bool {
    which("bat") || which("batcat")
}

pub fn is_wl_clipboard_available() -> bool {
    which("wl-copy") && which("wl-paste")
}

pub fn is_git_available() -> bool {
    which("git")
}

pub fn is_fd_available() -> bool {
    which("fd") || which("fdfind")
}

pub fn is_integrated_window_controls_desktop() -> bool {
    let xdg_desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_lowercase();
    let session = std::env::var("DESKTOP_SESSION").unwrap_or_default().to_lowercase();
    let combined = format!("{xdg_desktop};{session}");

    combined.contains("gnome")
        || combined.contains("ubuntu")
        || combined.contains("unity")
        || combined.contains("plasma")
        || combined.contains("kde")
}

pub fn install_hint(feature: &str) -> &'static str {
    match feature {
        "videoPreview" => "Install ffmpeg to enable video poster previews.",
        "pdfPreview" => "Install poppler-utils (pdftoppm) to enable PDF previews.",
        "remoteAccess" => "Install gvfs to browse remote filesystems through Connect to Server.",
        "smbRemoteAccess" => "Install gvfs-smb to browse SMB/CIFS shares.",
        "deviceMount" => "Install udisks2 to mount and unmount devices from the sidebar.",
        "clipboardImage" => "Install wl-clipboard to paste images and copy paths through Wayland.",
        "textHighlight" => "Install bat for syntax-highlighted text previews.",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_which_lookup() {
        // sh or ls should always exist on linux
        assert!(which("sh") || which("bash"));
        assert!(!which("nonexistent_binary_xyz_123"));
    }

    #[test]
    fn test_install_hints() {
        assert!(!install_hint("videoPreview").is_empty());
        assert_eq!(install_hint("unknown"), "");
    }
}
