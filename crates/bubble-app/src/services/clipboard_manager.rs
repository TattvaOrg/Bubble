use qmetaobject::*;
use std::process::Command;

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct ClipboardManager {
    _base: qt_base_class!(trait QObject),

    pub hasContent: qt_property!(bool; NOTIFY clipboardChanged),
    pub isCut: qt_property!(bool; NOTIFY clipboardChanged),
    pub paths: qt_property!(QVariant; NOTIFY clipboardChanged),

    pub clipboardChanged: qt_signal!(),

    pub copy: qt_method!(fn(&mut self, paths: QVariant)),
    pub cut: qt_method!(fn(&mut self, paths: QVariant)),
    pub clear: qt_method!(fn(&mut self)),
    pub copyText: qt_method!(fn(&self, text: QString)),
    pub contains: qt_method!(fn(&self, path: QString) -> bool),

    raw_paths: Vec<String>,
}

impl ClipboardManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn copy(&mut self, paths: QVariant) {
        self.isCut = false;
        self.hasContent = true;
        self.paths = paths;
        self.clipboardChanged();
    }

    pub fn cut(&mut self, paths: QVariant) {
        self.isCut = true;
        self.hasContent = true;
        self.paths = paths;
        self.clipboardChanged();
    }

    pub fn clear(&mut self) {
        self.isCut = false;
        self.hasContent = false;
        self.raw_paths.clear();
        self.paths = QVariant::default();
        self.clipboardChanged();
    }

    pub fn copyText(&self, text: QString) {
        let t = text.to_string();
        let _ = Command::new("wl-copy").arg(&t).spawn();
    }

    pub fn contains(&self, path: QString) -> bool {
        let p = path.to_string();
        self.raw_paths.contains(&p)
    }
}
