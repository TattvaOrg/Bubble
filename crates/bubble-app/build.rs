fn main() {
    println!("cargo:rerun-if-changed=src/main.rs");
    let qt_include_path = std::env::var("DEP_QT_INCLUDE_PATH").unwrap();
    let mut config = cpp_build::Config::new();
    if let Ok(flags) = std::env::var("DEP_QT_COMPILE_FLAGS") {
        for f in flags.split_terminator(';') {
            config.flag(f);
        }
    }
    if let Ok(lib_path) = std::env::var("DEP_QT_LIBRARY_PATH") {
        println!("cargo:rustc-link-search=native={}", lib_path);
    }
    println!("cargo:rustc-link-lib=Qt6Svg");
    config.include(&qt_include_path);
    config.include(format!("{}/QtCore", qt_include_path));
    config.include(format!("{}/QtGui", qt_include_path));
    config.include(format!("{}/QtQml", qt_include_path));
    config.include(format!("{}/QtQuick", qt_include_path));
    config.include(format!("{}/QtSvg", qt_include_path));
    config.build("src/main.rs");
}
