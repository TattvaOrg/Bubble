#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-v0.1.0}"
VERSION="${VERSION#v}"
TARBALL_NAME="Bubble-v${VERSION}-x86_64-linux.tar.gz"
DIST_DIR="$(mktemp -d /tmp/bubble-dist-XXXXXX)"

mkdir -p "$DIST_DIR/bin"
mkdir -p "$DIST_DIR/share/bubble/themes"
mkdir -p "$DIST_DIR/share/bubble/src/qml"
mkdir -p "$DIST_DIR/share/applications"
mkdir -p "$DIST_DIR/share/icons/hicolor/scalable/apps"
mkdir -p "$DIST_DIR/share/metainfo"
mkdir -p "$DIST_DIR/share/polkit-1/actions"

# Copy binaries
cp -p build/src/bubble "$DIST_DIR/bin/"
cp -p target/release/bubble-vault-helper "$DIST_DIR/bin/"
cp -p target/release/bubble-vault-destroy "$DIST_DIR/bin/"
ln -sf bubble "$DIST_DIR/bin/hyprfm"

# Copy assets
cp -p themes/*.toml "$DIST_DIR/share/bubble/themes/"
cp -rp src/qml/* "$DIST_DIR/share/bubble/src/qml/"
cp -p dist/io.github.soyeb_jim285.Bubble.desktop "$DIST_DIR/share/applications/"
cp -p dist/io.github.soyeb_jim285.Bubble.svg "$DIST_DIR/share/icons/hicolor/scalable/apps/"
cp -p dist/io.github.soyeb_jim285.Bubble.metainfo.xml "$DIST_DIR/share/metainfo/"
cp -p dist/org.bubble.vault.policy "$DIST_DIR/share/polkit-1/actions/"

# Create tarball
tar -czf "$TARBALL_NAME" -C "$DIST_DIR" .
rm -rf "$DIST_DIR"

echo "Created release tarball: $TARBALL_NAME"
