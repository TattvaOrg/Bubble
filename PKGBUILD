# Maintainer: Naitik Vadher
pkgname=bubble-git
pkgver=0.6.1.r200.gca5ce09
pkgrel=1
pkgdesc="A lightweight Qt6/QML file manager for Linux (Pure Rust)"
arch=('x86_64' 'aarch64')
url="https://github.com/Naitik-Vadher-4661/BUBBLE-RUST"
license=('MIT')
depends=(
    'glib2'
    'gvfs'
    'qt6-base'
    'qt6-declarative'
    'qt6-svg'
    'qt6-wayland'
    'xdg-utils'
    'openssl'
    'argon2'
    'sqlite'
)
makedepends=(
    'cargo'
    'rust'
    'ninja'
    'git'
    'pkgconf'
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
    'ffmpeg: video thumbnails and audio/video metadata'
    'poppler: PDF thumbnails, previews, and metadata'
    'perl-image-exiftool: EXIF metadata for images'
    'udisks2: mount/unmount devices from sidebar'
)
provides=('bubble' 'hyprfm')
conflicts=('bubble' 'hyprfm')
source=(
    "${pkgname}::git+https://github.com/Naitik-Vadher-4661/BUBBLE-RUST.git"
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
    cd "${pkgname}"
    cargo build --release
}

package() {
    cd "${pkgname}"

    # Install the compiled binaries
    install -Dm755 "target/release/bubble" "${pkgdir}/usr/bin/bubble"
    install -Dm755 "target/release/bubble-vault-helper" "${pkgdir}/usr/bin/bubble-vault-helper"
    install -Dm755 "target/release/bubble-vault-destroy" "${pkgdir}/usr/bin/bubble-vault-destroy"
    ln -s bubble "${pkgdir}/usr/bin/hyprfm"

    # Install themes
    install -dm755 "${pkgdir}/usr/share/bubble/themes"
    install -Dm644 themes/*.toml -t "${pkgdir}/usr/share/bubble/themes/"

    # Install QML sources
    install -dm755 "${pkgdir}/usr/share/bubble/src"
    cp -r "src/qml" "${pkgdir}/usr/share/bubble/src/qml"

    # Install desktop entry, icon and AppStream metainfo
    install -Dm644 "dist/io.github.soyeb_jim285.Bubble.desktop" \
        "${pkgdir}/usr/share/applications/io.github.soyeb_jim285.Bubble.desktop"
    install -Dm644 "dist/io.github.soyeb_jim285.Bubble.svg" \
        "${pkgdir}/usr/share/icons/hicolor/scalable/apps/io.github.soyeb_jim285.Bubble.svg"
    install -Dm644 "dist/io.github.soyeb_jim285.Bubble.metainfo.xml" \
        "${pkgdir}/usr/share/metainfo/io.github.soyeb_jim285.Bubble.metainfo.xml"
    install -Dm644 "dist/bubble-cleanup.hook" \
        "${pkgdir}/usr/share/libalpm/hooks/bubble-cleanup.hook" 2>/dev/null || true
    install -Dm644 "dist/org.bubble.vault.policy" \
        "${pkgdir}/usr/share/polkit-1/actions/org.bubble.vault.policy" 2>/dev/null || true

    # Install license
    install -Dm644 "LICENSE" "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE"
}
