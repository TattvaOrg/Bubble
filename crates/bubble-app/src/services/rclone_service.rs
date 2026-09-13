use qmetaobject::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct RcloneService {
    _base: qt_base_class!(trait QObject),

    pub rcloneAvailable: qt_property!(bool; NOTIFY rcloneAvailableChanged),
    pub activeMounts: qt_property!(QVariant; NOTIFY activeMountsChanged),

    pub rcloneAvailableChanged: qt_signal!(),
    pub activeMountsChanged: qt_signal!(),
    pub mountFinished: qt_signal!(remoteName: QString, success: bool, error: QString),
    pub unmountFinished: qt_signal!(remoteName: QString, success: bool),

    pub isRclonePath: qt_method!(fn(&self, path: QString) -> bool),
    pub isMounted: qt_method!(fn(&self, remote: QString) -> bool),
    pub isMounting: qt_method!(fn(&self, remote: QString) -> bool),
    pub isMountedForPath: qt_method!(fn(&self, path: QString) -> bool),
    pub mountRemote: qt_method!(fn(&mut self, remote: QString)),
    pub unmountRemote: qt_method!(fn(&mut self, remote: QString)),
    pub getMountPath: qt_method!(fn(&self, remote: QString) -> QString),
    pub getRemoteNameFromPath: qt_method!(fn(&self, path: QString) -> QString),

    mounts_base_dir: PathBuf,
    mounted_remotes: HashSet<String>,
    mounting_remotes: HashSet<String>,
}

impl RcloneService {
    pub fn new() -> Self {
        let has_rclone = Command::new("which")
            .arg("rclone")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let data_dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        let base_dir = data_dir.join("bubble").join("mounts");
        let _ = fs::create_dir_all(&base_dir);

        Self {
            rcloneAvailable: has_rclone,
            activeMounts: QVariantList::default().into(),
            mounts_base_dir: base_dir,
            ..Default::default()
        }
    }

    pub fn isRclonePath(&self, path: QString) -> bool {
        let p_str = path.to_string();
        let base_str = self.mounts_base_dir.to_string_lossy();
        p_str.starts_with(base_str.as_ref())
    }

    pub fn getRemoteNameFromPath(&self, path: QString) -> QString {
        if !self.isRclonePath(path.clone()) {
            return QString::default();
        }
        let p_str = path.to_string();
        let base_str = self.mounts_base_dir.to_string_lossy();
        let sub = p_str.trim_start_matches(base_str.as_ref()).trim_start_matches('/');
        let name = sub.split('/').next().unwrap_or("");
        QString::from(name)
    }

    pub fn getMountPath(&self, remote: QString) -> QString {
        let p = self.mounts_base_dir.join(remote.to_string());
        QString::from(p.to_string_lossy().as_ref())
    }

    pub fn isMounted(&self, remote: QString) -> bool {
        self.mounted_remotes.contains(&remote.to_string())
    }

    pub fn isMounting(&self, remote: QString) -> bool {
        self.mounting_remotes.contains(&remote.to_string())
    }

    pub fn isMountedForPath(&self, path: QString) -> bool {
        let remote = self.getRemoteNameFromPath(path);
        if remote.is_empty() {
            false
        } else {
            self.isMounted(remote)
        }
    }

    pub fn mountRemote(&mut self, remote: QString) {
        let r_str = remote.to_string();
        if !self.rcloneAvailable {
            self.mountFinished(remote, false, QString::from("rclone executable not found"));
            return;
        }

        if self.isMounted(remote.clone()) {
            self.mountFinished(remote, true, QString::default());
            return;
        }

        let mount_path = self.mounts_base_dir.join(&r_str);
        let _ = fs::create_dir_all(&mount_path);

        // Attempt background mount
        let r_str_clone = r_str.clone();
        let mount_path_str = mount_path.to_string_lossy().to_string();

        let _ = Command::new("fusermount")
            .arg("-u")
            .arg(&mount_path_str)
            .status();

        match Command::new("rclone")
            .arg("mount")
            .arg(format!("{}:", r_str_clone))
            .arg(&mount_path_str)
            .arg("--vfs-cache-mode")
            .arg("writes")
            .arg("--daemon")
            .status()
        {
            Ok(st) => {
                if st.success() {
                    self.mounted_remotes.insert(r_str.clone());
                    self.update_active_mounts();
                    self.mountFinished(remote, true, QString::default());
                } else {
                    self.mountFinished(remote, false, QString::from("rclone daemon exited with error"));
                }
            }
            Err(e) => {
                self.mountFinished(remote, false, QString::from(e.to_string().as_str()));
            }
        }
    }

    pub fn unmountRemote(&mut self, remote: QString) {
        let r_str = remote.to_string();
        let mount_path = self.mounts_base_dir.join(&r_str);
        let mount_path_str = mount_path.to_string_lossy().to_string();

        let res = Command::new("fusermount")
            .arg("-u")
            .arg(&mount_path_str)
            .status()
            .or_else(|_| Command::new("fusermount3").arg("-u").arg(&mount_path_str).status());

        let success = res.map(|s| s.success()).unwrap_or(false);
        self.mounted_remotes.remove(&r_str);
        self.update_active_mounts();
        self.unmountFinished(remote, success);
    }

    fn update_active_mounts(&mut self) {
        let mut list = QVariantList::default();
        for r in &self.mounted_remotes {
            list.push(QString::from(r.as_str()).to_qvariant());
        }
        self.activeMounts = list.into();
        self.activeMountsChanged();
    }
}
