use qmetaobject::*;
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Default)]
pub struct BookmarkItem {
    pub name: String,
    pub path: String,
    pub icon: String,
}

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct BookmarkModel {
    _base: qt_base_class!(trait QAbstractListModel),

    pub count: qt_property!(i32; NOTIFY countChanged),
    pub countChanged: qt_signal!(),
    pub bookmarksChanged: qt_signal!(),

    pub addBookmark: qt_method!(fn(&mut self, path: QString)),
    pub insertBookmark: qt_method!(fn(&mut self, path: QString, index: i32)),
    pub removeBookmark: qt_method!(fn(&mut self, index: i32)),
    pub renameBookmark: qt_method!(fn(&mut self, index: i32, name: QString)),
    pub moveBookmark: qt_method!(fn(&mut self, from: i32, to: i32)),
    pub setBookmarks: qt_method!(fn(&mut self, paths: QVariant, names: QVariant)),
    pub getPath: qt_method!(fn(&self, index: i32) -> QString),
    pub getName: qt_method!(fn(&self, index: i32) -> QString),

    items: Vec<BookmarkItem>,
}

impl BookmarkModel {
    pub fn new() -> Self {
        let mut model = Self::default();
        let home = dirs::home_dir().unwrap_or_else(|| Path::new("/").to_path_buf());
        let mut items = Vec::new();
        items.push(BookmarkItem { name: "Home".into(), path: home.to_string_lossy().into(), icon: "folder-home".into() });
        for folder in &["Documents", "Downloads", "Pictures", "Music", "Videos"] {
            let p = home.join(folder);
            if p.exists() {
                items.push(BookmarkItem {
                    name: folder.to_string(),
                    path: p.to_string_lossy().into(),
                    icon: format!("folder-{}", folder.to_lowercase()),
                });
            }
        }
        model.items = items;
        model.count = model.items.len() as i32;
        model
    }

    pub fn addBookmark(&mut self, path: QString) {
        let p = path.to_string();
        let name = Path::new(&p).file_name().and_then(|n| n.to_str()).unwrap_or(&p).to_string();
        let row = self.items.len() as i32;
        (self as &mut dyn QAbstractListModel).begin_insert_rows(row, row);
        self.items.push(BookmarkItem { name, path: p, icon: "folder".into() });
        (self as &mut dyn QAbstractListModel).end_insert_rows();
        self.count = self.items.len() as i32;
        self.countChanged();
        self.bookmarksChanged();
    }

    pub fn insertBookmark(&mut self, path: QString, index: i32) {
        let p = path.to_string();
        let name = Path::new(&p).file_name().and_then(|n| n.to_str()).unwrap_or(&p).to_string();
        let idx = (index as usize).min(self.items.len());
        (self as &mut dyn QAbstractListModel).begin_insert_rows(idx as i32, idx as i32);
        self.items.insert(idx, BookmarkItem { name, path: p, icon: "folder".into() });
        (self as &mut dyn QAbstractListModel).end_insert_rows();
        self.count = self.items.len() as i32;
        self.countChanged();
        self.bookmarksChanged();
    }

    pub fn removeBookmark(&mut self, index: i32) {
        if index >= 0 && (index as usize) < self.items.len() {
            (self as &mut dyn QAbstractListModel).begin_remove_rows(index, index);
            self.items.remove(index as usize);
            (self as &mut dyn QAbstractListModel).end_remove_rows();
            self.count = self.items.len() as i32;
            self.countChanged();
            self.bookmarksChanged();
        }
    }

    pub fn renameBookmark(&mut self, index: i32, name: QString) {
        if index >= 0 && (index as usize) < self.items.len() {
            self.items[index as usize].name = name.to_string();
            let idx = (self as &mut dyn QAbstractListModel).row_index(index);
            (self as &mut dyn QAbstractListModel).data_changed(idx, idx);
            self.bookmarksChanged();
        }
    }

    pub fn moveBookmark(&mut self, from: i32, to: i32) {
        if from >= 0 && (from as usize) < self.items.len() && to >= 0 && (to as usize) < self.items.len() {
            let item = self.items.remove(from as usize);
            self.items.insert(to as usize, item);
            let from_idx = (self as &mut dyn QAbstractListModel).row_index(from.min(to));
            let to_idx = (self as &mut dyn QAbstractListModel).row_index(from.max(to));
            (self as &mut dyn QAbstractListModel).data_changed(from_idx, to_idx);
            self.bookmarksChanged();
        }
    }

    pub fn setBookmarks(&mut self, paths: QVariant, names: QVariant) {
        (self as &mut dyn QAbstractListModel).begin_reset_model();
        self.items.clear();
        let paths_list = <QVariantList as QMetaType>::from_qvariant(paths).unwrap_or_default();
        let names_map = <QVariantMap as QMetaType>::from_qvariant(names).unwrap_or_default();
        let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"));
        let home_str = home.to_string_lossy();

        for p_var in &paths_list {
            let mut p = <QString as QMetaType>::from_qvariant(p_var.clone()).unwrap_or_default().to_string();
            if p.starts_with("~/") {
                p = format!("{}{}", home_str, &p[1..]);
            } else if p == "~" {
                p = home_str.to_string();
            }
            let key = QString::from(p.as_str());
            let custom_name = if names_map.contains(key.clone()) {
                let v = <QString as QMetaType>::from_qvariant(names_map.value(key, QVariant::default()))
                    .unwrap_or_default()
                    .to_string();
                if v.trim().is_empty() { None } else { Some(v) }
            } else {
                None
            };
            let name = custom_name.unwrap_or_else(|| {
                Path::new(&p).file_name().and_then(|n| n.to_str()).unwrap_or(&p).to_string()
            });
            let icon = if p == home_str {
                "folder-home".to_string()
            } else {
                format!("folder-{}", name.to_lowercase())
            };
            self.items.push(BookmarkItem { name, path: p, icon });
        }
        (self as &mut dyn QAbstractListModel).end_reset_model();
        self.count = self.items.len() as i32;
        self.countChanged();
        self.bookmarksChanged();
    }

    pub fn getPath(&self, index: i32) -> QString {
        if index >= 0 && (index as usize) < self.items.len() {
            QString::from(self.items[index as usize].path.as_str())
        } else {
            QString::default()
        }
    }

    pub fn getName(&self, index: i32) -> QString {
        if index >= 0 && (index as usize) < self.items.len() {
            QString::from(self.items[index as usize].name.as_str())
        } else {
            QString::default()
        }
    }
}

impl QAbstractListModel for BookmarkModel {
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
            257 => QString::from(item.name.as_str()).to_qvariant(),
            258 => QString::from(item.path.as_str()).to_qvariant(),
            259 => QString::from(item.icon.as_str()).to_qvariant(),
            _ => QVariant::default(),
        }
    }

    fn role_names(&self) -> HashMap<i32, QByteArray> {
        let mut m = HashMap::new();
        m.insert(257, "name".into());
        m.insert(258, "path".into());
        m.insert(259, "iconName".into());
        m
    }
}
