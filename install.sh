#!/usr/bin/env bash
# ==============================================================================
# Bubble Installation & Management Script
# ==============================================================================
# A fast, keyboard-friendly file manager for Wayland.
#
# Usage:
#   ./install.sh             # Install latest release binary to ~/.local (default)
#   ./install.sh --update    # Update existing installation to latest release
#   ./install.sh --uninstall # Remove Bubble and securely shred vault data
#   ./install.sh --system    # Install system-wide to /usr/local (requires sudo)
#   ./install.sh --build     # Build from source using Cargo and CMake
# ==============================================================================

set -euo pipefail

REPO_OWNER="TattvaOrg"
REPO_NAME="Bubble"
GITHUB_REPO="${REPO_OWNER}/${REPO_NAME}"

# Defaults
ACTION="install"
MODE="user"
CUSTOM_PREFIX=""
BUILD_FROM_SOURCE=0
BUILD_TYPE="Release"
AUTO_YES=0
FORCE_REBUILD=0
CHECK_DEPS=1

print_usage() {
    cat <<USAGE
Bubble Installer & Manager

Usage:
  ./install.sh [options]

Actions:
  (default)           Install Bubble binary from latest release
  --update            Check for and apply latest update
  --uninstall         Uninstall Bubble and securely clean up vault data

Options:
  --user              Install for current user only (~/.local) [default]
  --system            Install system-wide (/usr/local, requires sudo)
  --prefix <path>     Install to custom prefix path
  --from-source, -s   Build from source repository instead of downloading binary
  --build             Alias for --from-source
  --debug             Build in Debug mode instead of Release (source build only)
  --rebuild           Force clean build before installing (source build only)
  --no-deps           Skip dependency detection during source build
  -y, --yes           Non-interactive mode, answer yes to all prompts
  -h, --help          Show this help message

One-Liner Examples:
  curl -sSL https://raw.githubusercontent.com/${GITHUB_REPO}/main/install.sh | bash
  curl -sSL https://raw.githubusercontent.com/${GITHUB_REPO}/main/install.sh | bash -s -- --update
  curl -sSL https://raw.githubusercontent.com/${GITHUB_REPO}/main/install.sh | bash -s -- --uninstall
USAGE
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --install)
            ACTION="install"
            shift
            ;;
        --update)
            ACTION="update"
            shift
            ;;
        --uninstall)
            ACTION="uninstall"
            shift
            ;;
        --user)
            MODE="user"
            shift
            ;;
        --system)
            MODE="system"
            shift
            ;;
        --prefix)
            if [[ -z "${2:-}" ]]; then
                echo "Error: --prefix requires a directory path." >&2
                exit 1
            fi
            CUSTOM_PREFIX="$2"
            shift 2
            ;;
        --from-source|--build|-s)
            BUILD_FROM_SOURCE=1
            shift
            ;;
        --debug)
            BUILD_TYPE="Debug"
            BUILD_FROM_SOURCE=1
            shift
            ;;
        --rebuild)
            FORCE_REBUILD=1
            BUILD_FROM_SOURCE=1
            shift
            ;;
        --no-deps)
            CHECK_DEPS=0
            shift
            ;;
        -y|--yes)
            AUTO_YES=1
            shift
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            print_usage
            exit 1
            ;;
    esac
done

# Resolve prefix
if [[ -n "$CUSTOM_PREFIX" ]]; then
    PREFIX="$CUSTOM_PREFIX"
elif [[ "$MODE" == "system" ]]; then
    PREFIX="/usr/local"
else
    PREFIX="${HOME}/.local"
fi

if [[ "$MODE" == "system" && $EUID -ne 0 ]]; then
    echo "System-wide operations require root privileges. Please run with sudo or as root." >&2
    exit 1
fi

ARCH="$(uname -m)"
if [[ "$ARCH" != "x86_64" ]]; then
    echo "Notice: Precompiled binaries are currently provided for x86_64 Linux."
    echo "Switching to build from source for $ARCH..."
    BUILD_FROM_SOURCE=1
fi

# ==============================================================================
# Uninstallation
# ==============================================================================
if [[ "$ACTION" == "uninstall" ]]; then
    echo "=============================================="
    echo "              Uninstalling Bubble             "
    echo "=============================================="
    echo "Target prefix: $PREFIX"
    echo

    if [[ $AUTO_YES -eq 0 ]]; then
        read -r -p "Are you sure you want to uninstall Bubble from '$PREFIX'? [y/N] " confirm
        if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
            echo "Uninstall cancelled."
            exit 0
        fi
    fi

    # Vault destruction if vault files exist
    VAULT_DESTROY_BIN=""
    if [[ -x "$PREFIX/bin/bubble-vault-destroy" ]]; then
        VAULT_DESTROY_BIN="$PREFIX/bin/bubble-vault-destroy"
    elif command -v bubble-vault-destroy >/dev/null 2>&1; then
        VAULT_DESTROY_BIN="$(command -v bubble-vault-destroy)"
    fi

    if [[ -n "$VAULT_DESTROY_BIN" ]]; then
        echo "--> Checking for locked vault files..."
        if [[ $AUTO_YES -eq 1 ]]; then
            "$VAULT_DESTROY_BIN" || true
        else
            read -r -p "Do you want to securely shred locked vault files and remove the vault database? [y/N] " shred_confirm
            if [[ "$shred_confirm" =~ ^[Yy]$ ]]; then
                "$VAULT_DESTROY_BIN" || true
            else
                echo "Skipping vault shredding."
            fi
        fi
    fi

    echo "--> Removing installed files..."
    rm -f "$PREFIX/bin/bubble"
    rm -f "$PREFIX/bin/hyprfm"
    rm -f "$PREFIX/bin/bubble-vault-helper"
    rm -f "$PREFIX/bin/bubble-vault-destroy"
    rm -rf "$PREFIX/share/bubble"
    rm -f "$PREFIX/share/applications/io.github.soyeb_jim285.Bubble.desktop"
    rm -f "$PREFIX/share/applications/bubble.desktop"
    rm -f "$PREFIX/share/icons/hicolor/scalable/apps/io.github.soyeb_jim285.Bubble.svg"
    rm -f "$PREFIX/share/metainfo/io.github.soyeb_jim285.Bubble.metainfo.xml"
    rm -f "/usr/local/bin/bubble-vault-helper" 2>/dev/null || true

    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -t "$PREFIX/share/icons/hicolor" 2>/dev/null || true
    fi
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$PREFIX/share/applications" 2>/dev/null || true
    fi

    echo "Bubble has been cleanly uninstalled from $PREFIX."
    exit 0
fi

# ==============================================================================
# Helper: Fetch Release Info
# ==============================================================================
get_latest_release_info() {
    local api_url="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"
    if command -v curl >/dev/null 2>&1; then
        curl -sSL "$api_url"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "$api_url"
    else
        echo "Error: curl or wget is required to download binaries." >&2
        return 1
    fi
}

install_from_binary_tarball() {
    echo "--> Fetching latest release information from GitHub..."
    local release_json
    release_json="$(get_latest_release_info)" || return 1

    local tag_name
    tag_name="$(echo "$release_json" | grep -m1 '"tag_name":' | sed -E 's/.*"tag_name":[[:space:]]*"([^"]+)".*/\1/' || true)"

    if [[ -z "$tag_name" ]]; then
        echo "Warning: Unable to determine latest release tag from GitHub API."
        return 1
    fi

    local download_url
    download_url="$(echo "$release_json" | grep "browser_download_url.*Bubble-.*${ARCH}-linux\.tar\.gz" | cut -d : -f 2,3 | tr -d ' "')"

    if [[ -z "$download_url" ]]; then
        download_url="https://github.com/${GITHUB_REPO}/releases/download/${tag_name}/Bubble-${tag_name}-${ARCH}-linux.tar.gz"
    fi

    echo "--> Downloading precompiled release binary: ${tag_name} (${ARCH})..."
    local tmp_dir
    tmp_dir="$(mktemp -d /tmp/bubble-bin-XXXXXX)"

    local tarball="$tmp_dir/bubble.tar.gz"
    if command -v curl >/dev/null 2>&1; then
        if ! curl -sSL -f "$download_url" -o "$tarball"; then
            echo "Warning: Direct binary download failed from $download_url"
            rm -rf "$tmp_dir"
            return 1
        fi
    elif command -v wget >/dev/null 2>&1; then
        if ! wget -q "$download_url" -O "$tarball"; then
            echo "Warning: Direct binary download failed from $download_url"
            rm -rf "$tmp_dir"
            return 1
        fi
    fi

    echo "--> Extracting into '$PREFIX'..."
    mkdir -p "$PREFIX/bin" "$PREFIX/share/applications" "$PREFIX/share/icons/hicolor/scalable/apps" "$PREFIX/share/metainfo"
    tar -xzf "$tarball" -C "$PREFIX"
    rm -rf "$tmp_dir"

    # Set permissions on helper
    if [[ -x "$PREFIX/bin/bubble-vault-helper" ]]; then
        if [[ $EUID -eq 0 ]]; then
            chown root:root "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
            chmod 4755 "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
        elif command -v sudo >/dev/null 2>&1 && ( [[ $AUTO_YES -eq 1 ]] || sudo -n true 2>/dev/null ); then
            sudo chown root:root "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
            sudo chmod 4755 "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
        fi
    fi

    # Update databases
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -t "$PREFIX/share/icons/hicolor" 2>/dev/null || true
    fi
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$PREFIX/share/applications" 2>/dev/null || true
    fi

    echo
    echo "=============================================="
    echo "   Bubble has been installed successfully!    "
    echo "=============================================="
    echo " Installed version: $tag_name"
    echo " Main binary:       $PREFIX/bin/bubble"
    echo " Vault helper:      $PREFIX/bin/bubble-vault-helper"
    echo " Vault cleanup:     $PREFIX/bin/bubble-vault-destroy"
    echo " Desktop entry:     $PREFIX/share/applications/io.github.soyeb_jim285.Bubble.desktop"
    echo
    return 0
}

# ==============================================================================
# Update Action
# ==============================================================================
if [[ "$ACTION" == "update" ]]; then
    echo "=============================================="
    echo "              Updating Bubble                 "
    echo "=============================================="

    INSTALLED_VER="none"
    if command -v "$PREFIX/bin/bubble" >/dev/null 2>&1; then
        INSTALLED_VER="$("$PREFIX/bin/bubble" --version 2>/dev/null | head -n1 || echo 'unknown')"
    fi
    echo "Current installation: $INSTALLED_VER"

    if [[ $BUILD_FROM_SOURCE -eq 0 ]]; then
        if install_from_binary_tarball; then
            exit 0
        fi
        echo "Notice: Falling back to source update..."
    fi
fi

# ==============================================================================
# Installation (Binary first, then source fallback)
# ==============================================================================
if [[ $BUILD_FROM_SOURCE -eq 0 ]]; then
    if install_from_binary_tarball; then
        exit 0
    fi
    echo "Notice: Binary release installation unavailable. Proceeding with source build..."
fi

# ==============================================================================
# Source Build Flow
# ==============================================================================
CLEANUP_TMP=0
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ ! -f "$SCRIPT_DIR/CMakeLists.txt" || ! -d "$SCRIPT_DIR/src" ]]; then
    TMP_CLONE_DIR="$(mktemp -d /tmp/bubble-source-XXXXXX)"
    echo "--> Cloning Bubble repository into $TMP_CLONE_DIR..."
    git clone --depth 1 --recursive "https://github.com/${GITHUB_REPO}.git" "$TMP_CLONE_DIR"
    SCRIPT_DIR="$TMP_CLONE_DIR"
    CLEANUP_TMP=1
fi
cd "$SCRIPT_DIR"

# Ensure submodules
if [[ ! -f "$SCRIPT_DIR/src/qml/Quill/qmldir" || ! -f "$SCRIPT_DIR/src/qml/icons/qmldir" ]]; then
    echo "--> Initializing Git submodules..."
    git submodule update --init --recursive
fi

BUILD_DIR="${BUILD_DIR:-$SCRIPT_DIR/build}"
if [[ $FORCE_REBUILD -eq 1 && -d "$BUILD_DIR" ]]; then
    echo "--> Cleaning previous build..."
    rm -rf "$BUILD_DIR"
fi

echo "--> Compiling Rust workspace..."
cargo build --release --workspace

echo "--> Configuring CMake build..."
cmake -B "$BUILD_DIR" -S "$SCRIPT_DIR" -G Ninja \
    -DCMAKE_BUILD_TYPE="$BUILD_TYPE" \
    -DCMAKE_INSTALL_PREFIX="$PREFIX" \
    -DBUILD_TESTS=OFF \
    -DBUBBLE_DATA_DIR="$PREFIX/share/bubble"

echo "--> Building Bubble..."
cmake --build "$BUILD_DIR" --parallel

echo "--> Installing Bubble into '$PREFIX'..."
cmake --install "$BUILD_DIR" --prefix "$PREFIX"

if [[ -x "$PREFIX/bin/bubble-vault-helper" ]]; then
    if [[ $EUID -eq 0 ]]; then
        chown root:root "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
        chmod 4755 "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
    elif command -v sudo >/dev/null 2>&1 && ( [[ $AUTO_YES -eq 1 ]] || sudo -n true 2>/dev/null ); then
        sudo chown root:root "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
        sudo chmod 4755 "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
    fi
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$PREFIX/share/icons/hicolor" 2>/dev/null || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$PREFIX/share/applications" 2>/dev/null || true
fi

echo
echo "=============================================="
echo "    Bubble has been successfully installed!   "
echo "=============================================="
echo " Binary installed to: $PREFIX/bin/bubble"
echo " Vault cleanup:       $PREFIX/bin/bubble-vault-destroy"
echo " Vault helper:        $PREFIX/bin/bubble-vault-helper"
echo " Desktop file:        $PREFIX/share/applications/io.github.soyeb_jim285.Bubble.desktop"
echo " Icon:                $PREFIX/share/icons/hicolor/scalable/apps/io.github.soyeb_jim285.Bubble.svg"
echo

if [[ "$MODE" == "user" && ":$PATH:" != *":$PREFIX/bin:"* ]]; then
    echo "Note: '$PREFIX/bin' is not currently in your PATH."
    echo "Add this to your ~/.bashrc or ~/.zshrc:"
    echo "  export PATH=\"$PREFIX/bin:\$PATH\""
    echo
fi

if [[ $CLEANUP_TMP -eq 1 ]]; then
    rm -rf "$SCRIPT_DIR"
fi
