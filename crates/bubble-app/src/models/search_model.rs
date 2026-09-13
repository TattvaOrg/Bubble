use qmetaobject::*;
use std::collections::HashMap;

#[derive(Clone, Default)]
pub struct SearchResultItem {
    pub file_name: String,
    pub file_path: String,
    pub is_dir: bool,
    pub file_size_text: String,
    pub file_modified_text: String,
}

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct SearchResultsModel {
    _base: qt_base_class!(trait QAbstractListModel),
    pub items: Vec<SearchResultItem>,
}

impl QAbstractListModel for SearchResultsModel {
    fn row_count(&self) -> i32 {
        self.items.len() as i32
    }
    fn data(&self, index: QModelIndex, role: i32) -> QVariant {
        let r = index.row();
        if r < 0 || (r as usize) >= self.items.len() {
            return QVariant::default();
        }
        let it = &self.items[r as usize];
        match role {
            257 => QString::from(it.file_name.as_str()).to_qvariant(),
            258 => QString::from(it.file_path.as_str()).to_qvariant(),
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

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct SearchProxyModel {
    _base: qt_base_class!(trait QAbstractListModel),

    pub searchQuery: qt_property!(QString; NOTIFY searchChanged),
    pub fileTypeFilter: qt_property!(QString; NOTIFY searchChanged),
    pub dateFilter: qt_property!(QString; NOTIFY searchChanged),
    pub sizeFilter: qt_property!(QString; NOTIFY searchChanged),
    pub searchActive: qt_property!(bool; NOTIFY searchChanged),

    pub searchChanged: qt_signal!(),

    pub clearSearch: qt_method!(fn(&mut self)),
    pub rowCount: qt_method!(fn(&self) -> i32),

    items: Vec<SearchResultItem>,
}

impl SearchProxyModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clearSearch(&mut self) {
        self.searchQuery = QString::default();
        self.fileTypeFilter = QString::default();
        self.dateFilter = QString::default();
        self.sizeFilter = QString::default();
        self.searchActive = false;
        (self as &mut dyn QAbstractListModel).begin_reset_model();
        self.items.clear();
        (self as &mut dyn QAbstractListModel).end_reset_model();
        self.searchChanged();
    }

    pub fn rowCount(&self) -> i32 {
        self.items.len() as i32
    }
}

impl QAbstractListModel for SearchProxyModel {
    fn row_count(&self) -> i32 {
        self.items.len() as i32
    }

    fn data(&self, index: QModelIndex, role: i32) -> QVariant {
        let r = index.row();
        if r < 0 || (r as usize) >= self.items.len() {
            return QVariant::default();
        }
        let it = &self.items[r as usize];
        match role {
            257 => QString::from(it.file_name.as_str()).to_qvariant(),
            258 => QString::from(it.file_path.as_str()).to_qvariant(),
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

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct SearchService {
    _base: qt_base_class!(trait QObject),

    pub searchFinished: qt_signal!(),

    pub search: qt_method!(fn(&mut self, query: QString, path: QString)),
    pub cancel: qt_method!(fn(&mut self)),
}

impl SearchService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn search(&mut self, _query: QString, _path: QString) {
    }

    pub fn cancel(&mut self) {
    }
}
