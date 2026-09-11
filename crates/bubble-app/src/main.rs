use qmetaobject::*;

#[derive(QObject, Default)]
struct BubbleInfo {
    base: qt_base_class!(trait QObject),
    version: qt_property!(QString),
    backend: qt_property!(QString),
}

fn main() {
    println!("Initializing Bubble (Pure Rust Backend)...");
    let mut engine = QmlEngine::new();
    let info = QObjectBox::new(BubbleInfo {
        version: QString::from("0.6.1"),
        backend: QString::from("Pure Rust"),
        ..Default::default()
    });
    engine.set_object_property("bubbleInfo".into(), info.pinned());
    println!("Rust QML engine initialized successfully with context properties!");
}
