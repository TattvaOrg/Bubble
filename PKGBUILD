# Maintainer: Your Name <your@email.com>
pkgname=bubble-git
pkgver=r198.g53be041
pkgrel=1
pkgdesc="A lightweight Qt6/QML file manager for Hyprland"
arch=('x86_64' 'aarch64')
url="https://github.com/TattvaOrg/Bubble"
license=('MIT')
depends=(
    'glib2'
    'gvfs'
    'kwindowsystem'
    'qt6-base'
    'qt6-declarative'
    'qt6-svg'
    'qt6-wayland'
    'xdg-utils'
    'openssl'
    'argon2'
)
makedepends=(
    'cmake'
    'ninja'
    'git'
    'kwindowsystem'
    'qt6-base'
    'qt6-declarative'
    'qt6-svg'
)
optdepends=(
    'wl-clipboard: clipboard support via wl-copy and wl-paste'
    'fd: fast recursive search (falls back to a built-in walker)'
    'bat: syntax-highlighted text previews'
    'gvfs-smb: SMB/CIFS remote browsing support'
    'gvfs-mtp: Android phones (MTP) in the sidebar'
    'ffmpeg: video thumbnails and audio/video metadata (via ffprobe)'
    'poppler: PDF thumbnails, previews, and metadata (via pdftoppm/pdfinfo)'
    'perl-image-exiftool: EXIF metadata for images (via exiftool)'
    'udisks2: mount/unmount devices from sidebar'
)
provides=('bubble')
conflicts=('bubble')
source=(
    "${pkgname}::git+https://github.com/TattvaOrg/Bubble.git"
    "quill-icons::git+https://github.com/soyeb-jim285/quill-icons.git"
    "quill::git+https://github.com/soyeb-jim285/quill.git"
)
sha256sums=('SKIP' 'SKIP' 'SKIP')

pkgver() {
    cd "${pkgname}"
    printf "r%s.g%s" "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"
}

prepare() {
    cd "${pkgname}"
    git submodule init
    git config submodule.src/qml/icons.url "${srcdir}/quill-icons"
    git config submodule.src/qml/Quill.url "${srcdir}/quill"
    git -c protocol.file.allow=always submodule update
}

build() {
    cmake -B build -S "${pkgname}" -G Ninja \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX=/usr \
        -DBUILD_TESTS=OFF \
        -DBUBBLE_DATA_DIR=/usr/share/bubble
    cmake --build build --parallel
}

package() {
    # Install the compiled binary
    install -Dm755 "build/src/bubble" "${pkgdir}/usr/bin/bubble"

    # Install themes — loaded via applicationDirPath()/../themes → /usr/share/bubble/themes
    install -dm755 "${pkgdir}/usr/share/bubble/themes"
    install -Dm644 "${pkgname}/themes/"*.toml \
        -t "${pkgdir}/usr/share/bubble/themes/"

    # Install QML module metadata (needed for loadFromModule to find Bubble)
    install -Dm644 "build/src/Bubble/qmldir" \
        "${pkgdir}/usr/share/bubble/Bubble/qmldir"
    install -Dm644 "build/src/Bubble/bubble.qmltypes" \
        "${pkgdir}/usr/share/bubble/Bubble/bubble.qmltypes" 2>/dev/null || true

    # Install QML sources for Quill module
    install -dm755 "${pkgdir}/usr/share/bubble/src"
    cp -r "${pkgname}/src/qml" "${pkgdir}/usr/share/bubble/src/qml"

    # Install desktop entry, icon and AppStream metainfo
    install -Dm644 "${pkgname}/dist/io.github.tattvaorg.Bubble.desktop" \
        "${pkgdir}/usr/share/applications/io.github.tattvaorg.Bubble.desktop"
    install -Dm644 "${pkgname}/dist/io.github.tattvaorg.Bubble.svg" \
        "${pkgdir}/usr/share/icons/hicolor/scalable/apps/io.github.tattvaorg.Bubble.svg"
    install -Dm644 "${pkgname}/dist/io.github.tattvaorg.Bubble.metainfo.xml" \
        "${pkgdir}/usr/share/metainfo/io.github.tattvaorg.Bubble.metainfo.xml"

    # Install license
    install -Dm644 "${pkgname}/LICENSE" \
        "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE"
}
