use qmetaobject::*;

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct DragHelper {
    _base: qt_base_class!(trait QObject),

    pub active: qt_property!(bool; NOTIFY dragStateChanged),
    pub activePaths: qt_property!(QVariant; NOTIFY dragStateChanged),

    pub dragStateChanged: qt_signal!(),
    pub dragStarted: qt_signal!(),
    pub dragFinished: qt_signal!(),

    pub startDrag: qt_method!(fn(&mut self, paths: QVariant, icon_name: QString, count: i32)),
}

impl DragHelper {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn startDrag(&mut self, paths: QVariant, _icon_name: QString, _count: i32) {
        self.active = true;
        self.activePaths = paths;
        self.dragStarted();
        self.dragStateChanged();

        self.active = false;
        self.activePaths = QVariant::default();
        self.dragStateChanged();
        self.dragFinished();
    }
}
