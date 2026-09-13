use qmetaobject::*;
use std::process::Command;

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct PreviewService {
    _base: qt_base_class!(trait QObject),

    pub pdfPreviewAvailable: qt_property!(bool; NOTIFY previewSupportChanged),

    pub previewSupportChanged: qt_signal!(),
    pub supportChanged: qt_signal!(),
    pub previewReady: qt_signal!(requester: QString, path: QString, data: QVariant),

    pub requestPreview: qt_method!(fn(&mut self, requester: QString, path: QString, kind: QString, password: QString)),
    pub cancelPreview: qt_method!(fn(&mut self, requester: QString)),
    pub loadTextPreview: qt_method!(fn(&self, path: QString, maxBytes: i32, maxLines: i32) -> QVariant),
    pub loadDirectoryPreview: qt_method!(fn(&self, path: QString, maxEntries: i32) -> QVariant),
    pub loadArchivePreview: qt_method!(fn(&self, path: QString, maxEntries: i32, password: QString) -> QVariant),
    pub loadPdfPreview: qt_method!(fn(&self, path: QString) -> QVariant),
    pub loadFontPreview: qt_method!(fn(&self, path: QString) -> QVariant),
    pub localPreviewPath: qt_method!(fn(&self, path: QString) -> QString),
    pub refreshSupport: qt_method!(fn(&mut self)),
}

impl PreviewService {
    pub fn new() -> Self {
        let has_pdf = Command::new("which")
            .arg("pdftoppm")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        Self {
            pdfPreviewAvailable: has_pdf,
            ..Default::default()
        }
    }

    pub fn requestPreview(&mut self, requester: QString, path: QString, kind: QString, password: QString) {
        let mut data = QVariantMap::default();
        let k = kind.to_string();
        if k == "text" {
            data.insert(QString::from("text"), self.loadTextPreview(path.clone(), 131072, 400));
        } else if k == "directory" {
            data.insert(QString::from("directory"), self.loadDirectoryPreview(path.clone(), 40));
        } else if k == "archive" {
            data.insert(QString::from("archive"), self.loadArchivePreview(path.clone(), 200, password));
        } else if k == "pdf" {
            data.insert(QString::from("pdf"), self.loadPdfPreview(path.clone()));
        }

        self.previewReady(requester, path, QVariant::from(data));
    }

    pub fn cancelPreview(&mut self, _requester: QString) {}

    pub fn loadTextPreview(&self, path: QString, max_bytes: i32, max_lines: i32) -> QVariant {
        let mut map = QVariantMap::default();
        let path_str = path.to_string();
        if path_str.is_empty() {
            map.insert(QString::from("error"), QString::from("No file selected").to_qvariant());
            return QVariant::from(map);
        }

        let max_b = if max_bytes <= 0 { 131072 } else { max_bytes as usize };
        let max_l = if max_lines <= 0 { 400 } else { max_lines as usize };

        match std::fs::File::open(&path_str) {
            Ok(mut file) => {
                use std::io::Read;
                let mut buf = vec![0u8; max_b + 1];
                let n = file.read(&mut buf).unwrap_or(0);
                buf.truncate(n);

                let is_binary = buf.iter().take(4096).any(|&b| b == 0);
                let truncated = n > max_b;
                if truncated {
                    buf.truncate(max_b);
                }

                map.insert(QString::from("isBinary"), is_binary.to_qvariant());
                map.insert(QString::from("error"), QString::default().to_qvariant());

                if is_binary {
                    map.insert(
                        QString::from("content"),
                        QString::from("Binary file cannot be previewed").to_qvariant(),
                    );
                    map.insert(QString::from("truncated"), false.to_qvariant());
                } else {
                    let text = String::from_utf8_lossy(&buf);
                    let mut lines: Vec<&str> = text.lines().collect();
                    let line_truncated = lines.len() > max_l;
                    if line_truncated {
                        lines.truncate(max_l);
                    }
                    let content = lines.join("\n");
                    map.insert(QString::from("content"), QString::from(content.as_str()).to_qvariant());
                    map.insert(QString::from("truncated"), (truncated || line_truncated).to_qvariant());
                }
            }
            Err(e) => {
                map.insert(QString::from("error"), QString::from(e.to_string().as_str()).to_qvariant());
                map.insert(QString::from("content"), QString::default().to_qvariant());
                map.insert(QString::from("isBinary"), false.to_qvariant());
                map.insert(QString::from("truncated"), false.to_qvariant());
            }
        }
        QVariant::from(map)
    }

    pub fn loadDirectoryPreview(&self, path: QString, max_entries: i32) -> QVariant {
        let mut map = QVariantMap::default();
        let path_str = path.to_string();
        let max_e = if max_entries <= 0 { 40 } else { max_entries as usize };

        match std::fs::read_dir(&path_str) {
            Ok(entries) => {
                let mut list = QVariantList::default();
                let mut count = 0;
                let mut truncated = false;
                for entry in entries.flatten() {
                    count += 1;
                    if list.len() < max_e {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                        let display_name = if is_dir { format!("{}/", name) } else { name };
                        list.push(QString::from(display_name.as_str()).to_qvariant());
                    } else {
                        truncated = true;
                    }
                }
                map.insert(QString::from("entries"), QVariant::from(list));
                map.insert(QString::from("count"), (count as i32).to_qvariant());
                map.insert(QString::from("truncated"), truncated.to_qvariant());
                map.insert(QString::from("error"), QString::default().to_qvariant());
            }
            Err(e) => {
                map.insert(QString::from("entries"), QVariant::from(QVariantList::default()));
                map.insert(QString::from("count"), 0.to_qvariant());
                map.insert(QString::from("truncated"), false.to_qvariant());
                map.insert(QString::from("error"), QString::from(e.to_string().as_str()).to_qvariant());
            }
        }
        QVariant::from(map)
    }

    pub fn loadArchivePreview(&self, path: QString, max_entries: i32, _password: QString) -> QVariant {
        let mut map = QVariantMap::default();
        let path_str = path.to_string();
        let max_e = if max_entries <= 0 { 200 } else { max_entries as usize };

        let output = if path_str.ends_with(".zip") {
            Command::new("unzip").arg("-l").arg(&path_str).output()
        } else if path_str.ends_with(".tar")
            || path_str.ends_with(".tar.gz")
            || path_str.ends_with(".tgz")
            || path_str.ends_with(".tar.xz")
        {
            Command::new("tar").arg("-tf").arg(&path_str).output()
        } else if path_str.ends_with(".7z") {
            Command::new("7z").arg("l").arg(&path_str).output()
        } else {
            Command::new("tar").arg("-tf").arg(&path_str).output()
        };

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let lines: Vec<&str> = stdout.lines().collect();
                let mut list = QVariantList::default();
                let mut truncated = false;
                let count = lines.len();
                for line in lines.iter().take(max_e) {
                    list.push(QString::from(*line).to_qvariant());
                }
                if lines.len() > max_e {
                    truncated = true;
                }
                map.insert(QString::from("entries"), QVariant::from(list));
                map.insert(QString::from("count"), (count as i32).to_qvariant());
                map.insert(QString::from("truncated"), truncated.to_qvariant());
                map.insert(QString::from("error"), QString::default().to_qvariant());
            }
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                map.insert(QString::from("entries"), QVariant::from(QVariantList::default()));
                map.insert(QString::from("count"), 0.to_qvariant());
                map.insert(QString::from("truncated"), false.to_qvariant());
                map.insert(QString::from("error"), QString::from(err.as_str()).to_qvariant());
            }
            Err(e) => {
                map.insert(QString::from("entries"), QVariant::from(QVariantList::default()));
                map.insert(QString::from("count"), 0.to_qvariant());
                map.insert(QString::from("truncated"), false.to_qvariant());
                map.insert(QString::from("error"), QString::from(e.to_string().as_str()).to_qvariant());
            }
        }
        QVariant::from(map)
    }

    pub fn loadPdfPreview(&self, path: QString) -> QVariant {
        let mut map = QVariantMap::default();
        let path_str = path.to_string();
        map.insert(QString::from("localPath"), QString::from(path_str.as_str()).to_qvariant());

        let out = Command::new("pdfinfo").arg(&path_str).output();
        match out {
            Ok(o) if o.status.success() => {
                let text = String::from_utf8_lossy(&o.stdout);
                let pages = text
                    .lines()
                    .find(|l| l.starts_with("Pages:"))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(1);
                map.insert(QString::from("pageCount"), pages.to_qvariant());
                map.insert(QString::from("error"), QString::default().to_qvariant());
            }
            Ok(_) => {
                map.insert(QString::from("pageCount"), 0.to_qvariant());
                map.insert(QString::from("error"), QString::from("Unable to read PDF page count").to_qvariant());
            }
            Err(e) => {
                map.insert(QString::from("pageCount"), 0.to_qvariant());
                map.insert(QString::from("error"), QString::from(e.to_string().as_str()).to_qvariant());
            }
        }
        QVariant::from(map)
    }

    pub fn loadFontPreview(&self, path: QString) -> QVariant {
        let mut map = QVariantMap::default();
        let path_str = path.to_string();
        let p = std::path::Path::new(&path_str);
        if !p.is_file() {
            map.insert(QString::from("valid"), false.to_qvariant());
            map.insert(QString::from("error"), QString::from("Font file not found").to_qvariant());
            return QVariant::from(map);
        }

        let family = p
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        map.insert(QString::from("valid"), true.to_qvariant());
        map.insert(QString::from("family"), QString::from(family.as_str()).to_qvariant());
        map.insert(QString::from("styleName"), QString::from("Regular").to_qvariant());
        map.insert(QString::from("weight"), 400.to_qvariant());
        map.insert(QString::from("italic"), false.to_qvariant());
        map.insert(QString::from("error"), QString::default().to_qvariant());
        QVariant::from(map)
    }

    pub fn localPreviewPath(&self, path: QString) -> QString {
        path
    }

    pub fn refreshSupport(&mut self) {
        let has_pdf = Command::new("which")
            .arg("pdftoppm")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        self.pdfPreviewAvailable = has_pdf;
        self.previewSupportChanged();
        self.supportChanged();
    }
}
