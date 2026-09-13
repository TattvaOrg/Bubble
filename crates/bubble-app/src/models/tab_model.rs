use qmetaobject::*;
use std::collections::HashMap;
use std::path::Path;

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct TabModel {
    _base: qt_base_class!(trait QObject),

    pub currentPath: qt_property!(QString; NOTIFY currentPathChanged),
    pub title: qt_property!(QString; NOTIFY titleChanged),
    pub viewMode: qt_property!(QString; NOTIFY viewModeChanged),
    pub canGoBack: qt_property!(bool; NOTIFY historyChanged),
    pub canGoForward: qt_property!(bool; NOTIFY historyChanged),
    pub splitViewEnabled: qt_property!(bool; NOTIFY splitViewEnabledChanged),
    pub secondaryCurrentPath: qt_property!(QString; NOTIFY secondaryCurrentPathChanged),
    pub secondaryCanGoBack: qt_property!(bool; NOTIFY secondaryHistoryChanged),
    pub secondaryCanGoForward: qt_property!(bool; NOTIFY secondaryHistoryChanged),
    pub sortBy: qt_property!(QString; NOTIFY sortChanged),
    pub sortAscending: qt_property!(bool; NOTIFY sortChanged),

    pub currentPathChanged: qt_signal!(),
    pub titleChanged: qt_signal!(),
    pub viewModeChanged: qt_signal!(),
    pub historyChanged: qt_signal!(),
    pub splitViewEnabledChanged: qt_signal!(),
    pub secondaryCurrentPathChanged: qt_signal!(),
    pub secondaryHistoryChanged: qt_signal!(),
    pub sortChanged: qt_signal!(),

    pub navigateTo: qt_method!(fn(&mut self, path: QString)),
    pub navigateSecondaryTo: qt_method!(fn(&mut self, path: QString)),
    pub goBack: qt_method!(fn(&mut self)),
    pub goForward: qt_method!(fn(&mut self)),
    pub goUp: qt_method!(fn(&mut self)),
    pub secondaryGoBack: qt_method!(fn(&mut self)),
    pub secondaryGoForward: qt_method!(fn(&mut self)),
    pub secondaryGoUp: qt_method!(fn(&mut self)),
    pub resetSecondaryTo: qt_method!(fn(&mut self, path: QString)),

    back_stack: Vec<String>,
    forward_stack: Vec<String>,
    secondary_back_stack: Vec<String>,
    secondary_forward_stack: Vec<String>,
}

impl TabModel {
    pub fn new(initial_path: &str) -> Self {
        let p = initial_path.to_string();
        let name = Path::new(&p).file_name().and_then(|n| n.to_str()).unwrap_or(&p).to_string();

        Self {
            currentPath: QString::from(p.as_str()),
            title: QString::from(name.as_str()),
            viewMode: QString::from("grid"),
            canGoBack: false,
            canGoForward: false,
            splitViewEnabled: false,
            secondaryCurrentPath: QString::from(p.as_str()),
            secondaryCanGoBack: false,
            secondaryCanGoForward: false,
            sortBy: QString::from("name"),
            sortAscending: true,
            ..Default::default()
        }
    }

    pub fn navigateTo(&mut self, path: QString) {
        let new_p = path.to_string();
        let cur = self.currentPath.to_string();
        if !cur.is_empty() && cur != new_p {
            self.back_stack.push(cur);
            self.forward_stack.clear();
        }
        self.set_current_path_internal(new_p);
    }

    fn set_current_path_internal(&mut self, p: String) {
        let name = Path::new(&p).file_name().and_then(|n| n.to_str()).unwrap_or(&p).to_string();
        self.currentPath = QString::from(p.as_str());
        self.title = QString::from(name.as_str());
        self.canGoBack = !self.back_stack.is_empty();
        self.canGoForward = !self.forward_stack.is_empty();
        self.currentPathChanged();
        self.titleChanged();
        self.historyChanged();
    }

    pub fn navigateSecondaryTo(&mut self, path: QString) {
        let new_p = path.to_string();
        let cur = self.secondaryCurrentPath.to_string();
        if !cur.is_empty() && cur != new_p {
            self.secondary_back_stack.push(cur);
            self.secondary_forward_stack.clear();
        }
        self.secondaryCurrentPath = QString::from(new_p.as_str());
        self.secondaryCanGoBack = !self.secondary_back_stack.is_empty();
        self.secondaryCanGoForward = !self.secondary_forward_stack.is_empty();
        self.secondaryCurrentPathChanged();
        self.secondaryHistoryChanged();
    }

    pub fn goBack(&mut self) {
        if let Some(prev) = self.back_stack.pop() {
            let cur = self.currentPath.to_string();
            self.forward_stack.push(cur);
            self.set_current_path_internal(prev);
        }
    }

    pub fn goForward(&mut self) {
        if let Some(next) = self.forward_stack.pop() {
            let cur = self.currentPath.to_string();
            self.back_stack.push(cur);
            self.set_current_path_internal(next);
        }
    }

    pub fn goUp(&mut self) {
        let cur = self.currentPath.to_string();
        if let Some(parent) = Path::new(&cur).parent() {
            let p_str = parent.to_string_lossy().to_string();
            if !p_str.is_empty() && p_str != cur {
                self.navigateTo(QString::from(p_str.as_str()));
            }
        }
    }

    pub fn secondaryGoBack(&mut self) {
        if let Some(prev) = self.secondary_back_stack.pop() {
            let cur = self.secondaryCurrentPath.to_string();
            self.secondary_forward_stack.push(cur);
            self.secondaryCurrentPath = QString::from(prev.as_str());
            self.secondaryCanGoBack = !self.secondary_back_stack.is_empty();
            self.secondaryCanGoForward = !self.secondary_forward_stack.is_empty();
            self.secondaryCurrentPathChanged();
            self.secondaryHistoryChanged();
        }
    }

    pub fn secondaryGoForward(&mut self) {
        if let Some(next) = self.secondary_forward_stack.pop() {
            let cur = self.secondaryCurrentPath.to_string();
            self.secondary_back_stack.push(cur);
            self.secondaryCurrentPath = QString::from(next.as_str());
            self.secondaryCanGoBack = !self.secondary_back_stack.is_empty();
            self.secondaryCanGoForward = !self.secondary_forward_stack.is_empty();
            self.secondaryCurrentPathChanged();
            self.secondaryHistoryChanged();
        }
    }

    pub fn secondaryGoUp(&mut self) {
        let cur = self.secondaryCurrentPath.to_string();
        if let Some(parent) = Path::new(&cur).parent() {
            let p_str = parent.to_string_lossy().to_string();
            if !p_str.is_empty() && p_str != cur {
                self.navigateSecondaryTo(QString::from(p_str.as_str()));
            }
        }
    }

    pub fn resetSecondaryTo(&mut self, path: QString) {
        self.secondary_back_stack.clear();
        self.secondary_forward_stack.clear();
        self.secondaryCurrentPath = path;
        self.secondaryCanGoBack = false;
        self.secondaryCanGoForward = false;
        self.secondaryCurrentPathChanged();
        self.secondaryHistoryChanged();
    }
}

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct TabListModel {
    _base: qt_base_class!(trait QAbstractListModel),

    pub activeIndex: qt_property!(i32; NOTIFY activeIndexChanged),
    pub count: qt_property!(i32; NOTIFY countChanged),
    pub activeTab: qt_property!(QVariant; NOTIFY activeIndexChanged),

    pub activeIndexChanged: qt_signal!(),
    pub countChanged: qt_signal!(),
    pub lastTabClosed: qt_signal!(),
    pub sessionChanged: qt_signal!(),

    pub addTab: qt_method!(fn(&mut self)),
    pub openPath: qt_method!(fn(&mut self, path: QString)),
    pub moveTab: qt_method!(fn(&mut self, from: i32, to: i32)),
    pub closeTab: qt_method!(fn(&mut self, index: i32)),
    pub reopenClosedTab: qt_method!(fn(&mut self)),
    pub tabAt: qt_method!(fn(&self, index: i32) -> QVariant),

    tabs: Vec<QObjectBox<TabModel>>,
}

impl TabListModel {
    pub fn new(home_path: &str) -> Self {
        let first_tab = QObjectBox::new(TabModel::new(home_path));
        let mut list = Self {
            activeIndex: 0,
            count: 1,
            activeTab: QVariant::from(first_tab.pinned()),
            tabs: vec![first_tab],
            ..Default::default()
        };
        list.update_active();
        list
    }

    fn update_active(&mut self) {
        if self.tabs.is_empty() {
            self.activeTab = QVariant::default();
        } else {
            let idx = (self.activeIndex as usize).min(self.tabs.len() - 1);
            self.activeIndex = idx as i32;
            self.activeTab = QVariant::from(self.tabs[idx].pinned());
        }
        self.count = self.tabs.len() as i32;
        self.activeIndexChanged();
        self.countChanged();
    }

    pub fn addTab(&mut self) {
        let home = dirs::home_dir().unwrap_or_else(|| Path::new("/").to_path_buf());
        let new_tab = QObjectBox::new(TabModel::new(home.to_string_lossy().as_ref()));
        let row = self.tabs.len() as i32;
        (self as &mut dyn QAbstractListModel).begin_insert_rows(row, row);
        self.tabs.push(new_tab);
        (self as &mut dyn QAbstractListModel).end_insert_rows();
        self.activeIndex = row;
        self.update_active();
        self.sessionChanged();
    }

    pub fn openPath(&mut self, path: QString) {
        let p = path.to_string();
        for (i, t) in self.tabs.iter().enumerate() {
            if t.pinned().borrow().currentPath.to_string() == p {
                self.activeIndex = i as i32;
                self.update_active();
                return;
            }
        }
        let new_tab = QObjectBox::new(TabModel::new(&p));
        let row = self.tabs.len() as i32;
        (self as &mut dyn QAbstractListModel).begin_insert_rows(row, row);
        self.tabs.push(new_tab);
        (self as &mut dyn QAbstractListModel).end_insert_rows();
        self.activeIndex = row;
        self.update_active();
        self.sessionChanged();
    }

    pub fn moveTab(&mut self, from: i32, to: i32) {
        if from >= 0 && (from as usize) < self.tabs.len() && to >= 0 && (to as usize) < self.tabs.len() {
            let item = self.tabs.remove(from as usize);
            self.tabs.insert(to as usize, item);
            self.activeIndex = to;
            self.update_active();
            self.sessionChanged();
        }
    }

    pub fn closeTab(&mut self, index: i32) {
        if index >= 0 && (index as usize) < self.tabs.len() {
            (self as &mut dyn QAbstractListModel).begin_remove_rows(index, index);
            self.tabs.remove(index as usize);
            (self as &mut dyn QAbstractListModel).end_remove_rows();

            if self.tabs.is_empty() {
                self.lastTabClosed();
            } else {
                if self.activeIndex >= self.tabs.len() as i32 {
                    self.activeIndex = (self.tabs.len() - 1) as i32;
                }
                self.update_active();
                self.sessionChanged();
            }
        }
    }

    pub fn reopenClosedTab(&mut self) {
    }

    pub fn tabAt(&self, index: i32) -> QVariant {
        if index >= 0 && (index as usize) < self.tabs.len() {
            QVariant::from(self.tabs[index as usize].pinned())
        } else {
            QVariant::default()
        }
    }
}

impl QAbstractListModel for TabListModel {
    fn row_count(&self) -> i32 {
        self.tabs.len() as i32
    }

    fn data(&self, index: QModelIndex, role: i32) -> QVariant {
        let r = index.row();
        if r < 0 || (r as usize) >= self.tabs.len() {
            return QVariant::default();
        }
        let tab = self.tabs[r as usize].pinned();
        match role {
            257 => tab.borrow().title.to_qvariant(),
            258 => tab.borrow().currentPath.to_qvariant(),
            259 => QVariant::from(tab),
            _ => QVariant::default(),
        }
    }

    fn role_names(&self) -> HashMap<i32, QByteArray> {
        let mut m = HashMap::new();
        m.insert(257, "title".into());
        m.insert(258, "path".into());
        m.insert(259, "tabObject".into());
        m
    }
}
