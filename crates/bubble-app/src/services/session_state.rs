use qmetaobject::*;

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct SessionState {
    _base: qt_base_class!(trait QObject),

    pub gridColumns: qt_property!(i32; NOTIFY gridColumnsChanged),
    pub rowHeightDetailed: qt_property!(i32; NOTIFY rowHeightDetailedChanged),
    pub rowHeightMiller: qt_property!(i32; NOTIFY rowHeightMillerChanged),

    pub gridColumnsChanged: qt_signal!(),
    pub rowHeightDetailedChanged: qt_signal!(),
    pub rowHeightMillerChanged: qt_signal!(),
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            gridColumns: 6,
            rowHeightDetailed: 28,
            rowHeightMiller: 28,
            ..Default::default()
        }
    }
}
