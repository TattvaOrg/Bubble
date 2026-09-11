cmake_minimum_required(VERSION 3.20)
project(bubble VERSION 0.6.1 LANGUAGES CXX)

find_package(PkgConfig REQUIRED)
pkg_check_modules(GIO REQUIRED IMPORTED_TARGET gio-2.0 gio-unix-2.0)
pkg_check_modules(ARGON2 REQUIRED IMPORTED_TARGET libargon2)
pkg_check_modules(SQLITE3 REQUIRED IMPORTED_TARGET sqlite3)
find_package(OpenSSL REQUIRED)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Qml Quick QuickControls2 DBus Svg Network Concurrent Sql)
find_package(KF6WindowSystem QUIET)

# Cargo build for Rust core library and helper binaries
add_custom_target(cargo_bubble
    COMMAND cargo build --offline --manifest-path "${CMAKE_SOURCE_DIR}/rust/Cargo.toml" --release
    WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}/rust"
    BYPRODUCTS
        "${CMAKE_SOURCE_DIR}/rust/target/release/libbubble_core.a"
        "${CMAKE_SOURCE_DIR}/rust/target/release/bubble-vault-helper"
        "${CMAKE_SOURCE_DIR}/rust/target/release/bubble-vault-destroy"
    COMMENT "Compiling Rust workspace (bubble-core, bubble-vault-helper, bubble-vault-destroy)..."
)

add_library(rust_bubble_core STATIC IMPORTED GLOBAL)
set_target_properties(rust_bubble_core PROPERTIES
    IMPORTED_LOCATION "${CMAKE_SOURCE_DIR}/rust/target/release/libbubble_core.a"
)
add_dependencies(rust_bubble_core cargo_bubble)

add_subdirectory(src)

# Install themes
install(DIRECTORY themes/ DESTINATION share/bubble/themes FILES_MATCHING PATTERN "*.toml")

# Install QML sources (Quill module + icons needed at runtime)
install(DIRECTORY src/qml/ DESTINATION share/bubble/src/qml)

# Install the generated Bubble QML module: qmldir + qml/ tree + qmltypes.
# The qmldir keeps Qt's `prefer :/Bubble/`, so at runtime the module resolves
# to the qmlcachegen-compiled qrc payload and nothing is recompiled on launch
# (a random AppImage mount path defeats the on-disk .qmlc cache every start).
# The on-disk tree is only the fallback main.cpp uses when the qrc load fails,
# as it did with Qt 6.7.3 NO_CACHEGEN in v0.4.14 ("SettingsPanel is not a
# type"); release builds force BUBBLE_ENABLE_QML_CACHEGEN=ON for that reason.
install(DIRECTORY ${CMAKE_BINARY_DIR}/src/Bubble/
    DESTINATION share/bubble/Bubble
    FILES_MATCHING
        PATTERN "*.qml"
        PATTERN "qmldir"
        PATTERN "*.qmltypes")


option(BUILD_TESTS "Build tests" ON)
if(BUILD_TESTS)
    enable_testing()
    find_package(Qt6 REQUIRED COMPONENTS Test)
    add_subdirectory(tests)
endif()
