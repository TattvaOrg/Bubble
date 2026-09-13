use qmetaobject::*;

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct MetadataExtractor {
    _base: qt_base_class!(trait QObject),

    pub extract: qt_method!(fn(&self, path: QString) -> QVariant),
    pub missingDepsHint: qt_method!(fn(&self, mime: QString) -> QString),
    pub refreshSupport: qt_method!(fn(&mut self)),
}

impl MetadataExtractor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn extract(&self, _path: QString) -> QVariant {
        QVariant::default()
    }

    pub fn missingDepsHint(&self, _mime: QString) -> QString {
        QString::default()
    }

    pub fn refreshSupport(&mut self) {
    }
}
