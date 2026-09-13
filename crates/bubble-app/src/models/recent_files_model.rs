use qmetaobject::*;
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Default)]
pub struct RecentItem {
    pub file_name: String,
    pub file_path: String,
}

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct RecentFilesModel {
    _base: qt_base_class!(trait QAbstractListModel),

    pub count: qt_property!(i32; NOTIFY countChanged),
    pub countChanged: qt_signal!(),

    pub addRecent: qt_method!(fn(&mut self, path: QString)),

    items: Vec<RecentItem>,
}

impl RecentFilesModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn addRecent(&mut self, path: QString) {
        let p = path.to_string();
        let name = Path::new(&p).file_name().and_then(|n| n.to_str()).unwrap_or(&p).to_string();
        (self as &mut dyn QAbstractListModel).begin_insert_rows(0, 0);
        self.items.insert(0, RecentItem { file_name: name, file_path: p });
        (self as &mut dyn QAbstractListModel).end_insert_rows();
        self.count = self.items.len() as i32;
        self.countChanged();
    }
}

impl QAbstractListModel for RecentFilesModel {
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
            257 => QString::from(item.file_name.as_str()).to_qvariant(),
            258 => QString::from(item.file_path.as_str()).to_qvariant(),
            _ => QVariant::default(),
        }
    }

    fn role_names(&self) -> HashMap<i32, QByteArray> {
        let mut m = HashMap::new();
        m.insert(257, "fileName".into());
        m.insert(258, "filePath".into());
        m
    }
}
