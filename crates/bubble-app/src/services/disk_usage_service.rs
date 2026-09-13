use qmetaobject::*;

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct DiskUsageService {
    _base: qt_base_class!(trait QObject),

    pub requestFinished: qt_signal!(requestId: i32, result: QVariant),

    pub requestSize: qt_method!(fn(&self, paths: QVariant) -> i32),
    pub cancelRequest: qt_method!(fn(&self, id: i32)),
    pub invalidatePaths: qt_method!(fn(&self, paths: QVariant)),
    pub invalidatePath: qt_method!(fn(&self, path: QString)),
}

impl DiskUsageService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn requestSize(&self, _paths: QVariant) -> i32 {
        0
    }

    pub fn cancelRequest(&self, _id: i32) {
    }

    pub fn invalidatePaths(&self, _paths: QVariant) {
    }

    pub fn invalidatePath(&self, _path: QString) {
    }
}
