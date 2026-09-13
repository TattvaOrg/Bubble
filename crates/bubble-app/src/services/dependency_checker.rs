use qmetaobject::*;

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct DependencyChecker {
    _base: qt_base_class!(trait QObject),

    pub hasMissingRequired: qt_property!(bool; NOTIFY dependenciesChanged),
    pub distroName: qt_property!(QString; NOTIFY dependenciesChanged),
    pub missingDependencies: qt_property!(QVariant; NOTIFY dependenciesChanged),

    pub dependenciesChanged: qt_signal!(),

    pub refresh: qt_method!(fn(&mut self)),
}

impl DependencyChecker {
    pub fn new() -> Self {
        let distro = std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|c| {
                for line in c.lines() {
                    if line.starts_with("PRETTY_NAME=") {
                        return Some(line.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string());
                    }
                }
                None
            })
            .unwrap_or_else(|| "Linux".to_string());

        Self {
            hasMissingRequired: false,
            distroName: QString::from(distro.as_str()),
            missingDependencies: QVariant::default(),
            ..Default::default()
        }
    }

    pub fn refresh(&mut self) {
        self.dependenciesChanged();
    }
}
