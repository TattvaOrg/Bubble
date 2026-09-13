use qmetaobject::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Clone, Default)]
pub struct DeviceEntry {
    pub device_name: String,
    pub device_path: String,
    pub mount_point: String,
    pub total_size: i64,
    pub free_space: i64,
    pub usage_percent: i32,
    pub removable: bool,
    pub mounted: bool,
    pub backend: String,
}

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct DeviceModel {
    _base: qt_base_class!(trait QAbstractListModel),

    pub count: qt_property!(i32; NOTIFY countChanged),
    pub countChanged: qt_signal!(),
    pub deviceMounted: qt_signal!(mountPoint: QString),
    pub mountError: qt_signal!(message: QString),

    pub mount: qt_method!(fn(&mut self, index: i32)),
    pub unmount: qt_method!(fn(&mut self, index: i32)),
    pub refresh: qt_method!(fn(&mut self)),
    pub scheduleRefresh: qt_method!(fn(&mut self)),

    items: Vec<DeviceEntry>,
}

impl DeviceModel {
    pub fn new() -> Self {
        let mut m = Self::default();
        m.scan_devices();
        m.count = m.items.len() as i32;
        m
    }

    pub fn scan_devices(&mut self) {
        (self as &mut dyn QAbstractListModel).begin_reset_model();
        self.items.clear();

        let mut seen_mounts = std::collections::HashSet::new();

        // Scan /proc/mounts
        if let Ok(content) = fs::read_to_string("/proc/mounts") {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 3 {
                    continue;
                }
                let dev_path = parts[0];
                let mount_point = parts[1];
                let fs_type = parts[2];

                // Filter out non-storage and pseudo filesystems
                if !dev_path.starts_with("/dev/") {
                    continue;
                }
                if dev_path.starts_with("/dev/loop") && mount_point.starts_with("/var/lib/snapd") {
                    continue;
                }
                let virtual_fs = [
                    "tmpfs", "devtmpfs", "proc", "sysfs", "cgroup", "pstore", "bpf",
                    "securityfs", "debugfs", "tracefs", "fusectl", "mqueue", "hugetlbfs",
                ];
                if virtual_fs.contains(&fs_type) {
                    continue;
                }

                if !seen_mounts.insert(mount_point.to_string()) {
                    continue;
                }

                let mut total_bytes: i64 = 0;
                let mut free_bytes: i64 = 0;
                let mut usage = 0;

                let c_path = std::ffi::CString::new(mount_point).unwrap_or_default();
                let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
                if unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) } == 0 {
                    let s = unsafe { stat.assume_init() };
                    let total = (s.f_blocks as u64).saturating_mul(s.f_frsize as u64) as i64;
                    let free = (s.f_bavail as u64).saturating_mul(s.f_frsize as u64) as i64;
                    total_bytes = total;
                    free_bytes = free;
                    if total > 0 {
                        let used = total.saturating_sub(free);
                        usage = ((used as f64 / total as f64) * 100.0) as i32;
                    }
                }

                let dev_name = match mount_point {
                    "/" => "File System".to_string(),
                    "/home" => "Home".to_string(),
                    "/boot" | "/boot/efi" => "Boot".to_string(),
                    other => {
                        let p = Path::new(other);
                        p.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(dev_path)
                            .to_string()
                    }
                };

                let is_removable = dev_path.contains("/media/") || dev_path.contains("/run/media/");

                self.items.push(DeviceEntry {
                    device_name: dev_name,
                    device_path: dev_path.to_string(),
                    mount_point: mount_point.to_string(),
                    total_size: total_bytes,
                    free_space: free_bytes,
                    usage_percent: usage,
                    removable: is_removable,
                    mounted: true,
                    backend: "udisks2".into(),
                });
            }
        }

        // Always ensure root is listed if /proc/mounts had no match
        if self.items.is_empty() {
            self.items.push(DeviceEntry {
                device_name: "File System".into(),
                device_path: "/dev/sda1".into(),
                mount_point: "/".into(),
                total_size: 100 * 1024 * 1024 * 1024,
                free_space: 50 * 1024 * 1024 * 1024,
                usage_percent: 50,
                removable: false,
                mounted: true,
                backend: "local".into(),
            });
        }

        (self as &mut dyn QAbstractListModel).end_reset_model();
        self.count = self.items.len() as i32;
        self.countChanged();
    }

    pub fn mount(&mut self, index: i32) {
        if index >= 0 && (index as usize) < self.items.len() {
            let item = &self.items[index as usize];
            let _ = Command::new("udisksctl")
                .arg("mount")
                .arg("-b")
                .arg(&item.device_path)
                .output();
            self.scan_devices();
            if index >= 0 && (index as usize) < self.items.len() {
                self.deviceMounted(QString::from(self.items[index as usize].mount_point.as_str()));
            }
        }
    }

    pub fn unmount(&mut self, index: i32) {
        if index >= 0 && (index as usize) < self.items.len() {
            let item = &self.items[index as usize];
            let _ = Command::new("udisksctl")
                .arg("unmount")
                .arg("-b")
                .arg(&item.device_path)
                .output();
            self.scan_devices();
        }
    }

    pub fn refresh(&mut self) {
        self.scan_devices();
    }

    pub fn scheduleRefresh(&mut self) {
        self.scan_devices();
    }
}

impl QAbstractListModel for DeviceModel {
    fn row_count(&self) -> i32 {
        self.items.len() as i32
    }

    fn data(&self, index: QModelIndex, role: i32) -> QVariant {
        let r = index.row();
        if r < 0 || (r as usize) >= self.items.len() {
            return QVariant::default();
        }
        let item = &self.items[r as usize];
        match role {
            257 => QString::from(item.device_name.as_str()).to_qvariant(),
            258 => QString::from(item.device_path.as_str()).to_qvariant(),
            259 => QString::from(item.mount_point.as_str()).to_qvariant(),
            260 => item.total_size.to_qvariant(),
            261 => item.free_space.to_qvariant(),
            262 => item.usage_percent.to_qvariant(),
            263 => item.removable.to_qvariant(),
            264 => item.mounted.to_qvariant(),
            265 => QString::from(item.backend.as_str()).to_qvariant(),
            _ => QVariant::default(),
        }
    }

    fn role_names(&self) -> HashMap<i32, QByteArray> {
        let mut m = HashMap::new();
        m.insert(257, "deviceName".into());
        m.insert(258, "devicePath".into());
        m.insert(259, "mountPoint".into());
        m.insert(260, "totalSize".into());
        m.insert(261, "freeSpace".into());
        m.insert(262, "usagePercent".into());
        m.insert(263, "removable".into());
        m.insert(264, "mounted".into());
        m.insert(265, "backend".into());
        m
    }
}
