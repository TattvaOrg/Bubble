#!/usr/bin/env bash
# ==============================================================================
# Bubble Installation Script
# ==============================================================================
# A lightweight Qt6/QML file manager for Wayland.
#
# Usage:
#   ./install.sh             # Install for current user to ~/.local (default, no root needed)
#   ./install.sh --system    # Install system-wide to /usr/local (requires sudo)
#   ./install.sh --prefix /opt/bubble  # Install to a custom directory
#   ./install.sh --uninstall # Remove an existing installation
# ==============================================================================

set -euo pipefail

CLEANUP_TMP=0
if [[ -n "${BASH_SOURCE[0]:-}" && -f "${BASH_SOURCE[0]}" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
else
    SCRIPT_DIR="$(pwd)"
fi

if [[ ! -f "$SCRIPT_DIR/CMakeLists.txt" || ! -d "$SCRIPT_DIR/src" ]]; then
    TMP_CLONE_DIR="$(mktemp -d /tmp/bubble-install-XXXXXX)"
    echo "==> Fetching Bubble source repository to $TMP_CLONE_DIR..."
    git clone --depth 1 --recursive https://github.com/TattvaOrg/Bubble.git "$TMP_CLONE_DIR"
    SCRIPT_DIR="$TMP_CLONE_DIR"
    CLEANUP_TMP=1
fi
cd "$SCRIPT_DIR"

# Defaults
MODE="user"
CUSTOM_PREFIX=""
BUILD_DIR="${BUILD_DIR:-$SCRIPT_DIR/build}"
BUILD_TYPE="Release"
CHECK_DEPS=1
AUTO_YES=0
UNINSTALL=0
FORCE_REBUILD=0

print_usage() {
    cat <<USAGE
Bubble Installer

Usage:
  ./install.sh [options]

Options:
  --user              Install for current user only (~/.local) [default]
  --system            Install system-wide (/usr/local, requires sudo)
  --prefix <path>     Install to custom prefix path
  --build-dir <dir>   Specify build directory (default: ./build)
  --debug             Build in Debug mode instead of Release
  --rebuild           Force clean build before installing
  --no-deps           Skip dependency detection and package installation
  -y, --yes           Automatically install missing dependencies without prompting
  --uninstall         Uninstall Bubble from target prefix
  -h, --help          Show this help message

Examples:
  ./install.sh                    # Recommended: Installs to ~/.local
  sudo ./install.sh --system      # Installs to /usr/local
  ./install.sh --uninstall        # Removes from ~/.local
USAGE
}

while [[ $# -gt 0 ]]; do
    case "$1" in
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
        --build-dir)
            if [[ -z "${2:-}" ]]; then
                echo "Error: --build-dir requires a path." >&2
                exit 1
            fi
            BUILD_DIR="$2"
            shift 2
            ;;
        --debug)
            BUILD_TYPE="Debug"
            shift
            ;;
        --rebuild)
            FORCE_REBUILD=1
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
        --uninstall)
            UNINSTALL=1
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

# Determine target prefix
if [[ -n "$CUSTOM_PREFIX" ]]; then
    PREFIX="$CUSTOM_PREFIX"
elif [[ "$MODE" == "system" ]]; then
    PREFIX="/usr/local"
else
    PREFIX="${XDG_DATA_HOME:-$HOME/.local}"
fi

# Handle uninstall
if [[ $UNINSTALL -eq 1 ]]; then
    echo "==> Uninstalling Bubble from prefix: $PREFIX"

    # Securely shred and destroy all locked vault files before removal
    if command -v bubble-vault-destroy >/dev/null 2>&1; then
        echo "==> Securely shredding locked vault files..."
        bubble-vault-destroy || true
    elif [[ -x "$PREFIX/bin/bubble-vault-destroy" ]]; then
        echo "==> Securely shredding locked vault files..."
        "$PREFIX/bin/bubble-vault-destroy" || true
    fi

    rm -f "$PREFIX/bin/bubble"
    rm -f "$PREFIX/bin/bubble-vault-destroy"
    rm -f "$PREFIX/bin/bubble-vault-helper"
    rm -f "$PREFIX/bin/hyprfm"
    if [[ -f "/usr/local/bin/bubble-vault-helper" ]]; then
        if [[ $EUID -eq 0 ]]; then
            rm -f "/usr/local/bin/bubble-vault-helper"
        elif command -v sudo >/dev/null 2>&1; then
            sudo rm -f "/usr/local/bin/bubble-vault-helper" 2>/dev/null || true
        fi
    fi
    rm -rf "$PREFIX/share/bubble"
    rm -f "$PREFIX/share/applications/io.github.soyeb_jim285.Bubble.desktop"
    rm -f "$PREFIX/share/applications/bubble.desktop"
    rm -f "$PREFIX/share/icons/hicolor/scalable/apps/io.github.soyeb_jim285.Bubble.svg"
    rm -f "$PREFIX/share/metainfo/io.github.soyeb_jim285.Bubble.metainfo.xml"
    rm -f "$PREFIX/share/libalpm/hooks/bubble-cleanup.hook"
    rm -f "$PREFIX/share/polkit-1/actions/org.bubble.vault.policy"

    if [[ -f "/usr/share/polkit-1/actions/org.bubble.vault.policy" && $EUID -eq 0 ]]; then
        rm -f "/usr/share/polkit-1/actions/org.bubble.vault.policy"
    fi

    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -t "$PREFIX/share/icons/hicolor" 2>/dev/null || true
    fi
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$PREFIX/share/applications" 2>/dev/null || true
    fi

    echo "==> Bubble has been uninstalled successfully."
    exit 0
fi

echo "=============================================="
echo "          Bubble Installation Setup           "
echo "=============================================="
echo " Target Prefix : $PREFIX"
echo " Build Type    : $BUILD_TYPE"
echo " Build Dir     : $BUILD_DIR"
echo "=============================================="

# Check permissions for system install
if [[ "$MODE" == "system" || "$PREFIX" == /usr* || "$PREFIX" == /opt* ]]; then
    if [[ $EUID -ne 0 ]]; then
        echo "Error: Installing to '$PREFIX' requires root privileges." >&2
        echo "Please rerun with: sudo ./install.sh $@" >&2
        exit 1
    fi
fi

# ==============================================================================
# Dependency Checking and Auto-Installation
# ==============================================================================
detect_missing_dependencies() {
    local missing=()

    # Core build tools
    for tool in cmake git; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            missing+=("$tool")
        fi
    done

    # Ninja
    if ! command -v ninja >/dev/null 2>&1 && ! command -v ninja-build >/dev/null 2>&1; then
        missing+=("ninja")
    fi

    # Pkg-config
    if ! command -v pkg-config >/dev/null 2>&1 && ! command -v pkgconf >/dev/null 2>&1; then
        missing+=("pkg-config")
    fi

    # Compiler
    if ! command -v g++ >/dev/null 2>&1 && ! command -v clang++ >/dev/null 2>&1; then
        missing+=("c++-compiler")
    fi

    # Libraries check via pkg-config if available
    local PKG_CMD=""
    if command -v pkgconf >/dev/null 2>&1; then
        PKG_CMD="pkgconf"
    elif command -v pkg-config >/dev/null 2>&1; then
        PKG_CMD="pkg-config"
    fi

    if [[ -n "$PKG_CMD" ]]; then
        if ! "$PKG_CMD" --exists gio-2.0 gio-unix-2.0 2>/dev/null; then
            missing+=("gio-2.0")
        fi
        if ! "$PKG_CMD" --exists libargon2 2>/dev/null; then
            missing+=("libargon2")
        fi
        if ! "$PKG_CMD" --exists openssl 2>/dev/null; then
            missing+=("openssl")
        fi
    else
        missing+=("gio-2.0" "libargon2" "openssl")
    fi

    # Qt6 Core / Quick
    if ! cmake --find-package -DNAME=Qt6Core -DCOMPILER_ID=GNU -DLANGUAGE=CXX -DMODE=EXIST >/dev/null 2>&1 \
       && ! cmake --find-package -DNAME=Qt6Core -DCOMPILER_ID=Clang -DLANGUAGE=CXX -DMODE=EXIST >/dev/null 2>&1 \
       && ! command -v qmake6 >/dev/null 2>&1; then
        if [[ ! -d "/usr/lib/cmake/Qt6" && ! -d "/usr/lib64/cmake/Qt6" && ! -d "/usr/local/lib/cmake/Qt6" ]]; then
            missing+=("qt6")
        fi
    fi

    echo "${missing[@]:-}"
}

install_distro_dependencies() {
    local OS_ID=""
    local OS_LIKE=""
    if [[ -f /etc/os-release ]]; then
        # shellcheck disable=SC1091
        source /etc/os-release
        OS_ID="${ID:-}"
        OS_LIKE="${ID_LIKE:-}"
    fi

    local install_cmd=""
    local pkg_list=""

    if [[ "$OS_ID" =~ (arch|cachyos|manjaro|endeavouros|artix|garuda) || "$OS_LIKE" =~ arch ]]; then
        install_cmd="pacman -S --needed"
        pkg_list="cmake ninja git pkgconf gcc qt6-base qt6-declarative qt6-svg qt6-wayland glib2 xdg-utils openssl argon2 psmisc"
    elif [[ "$OS_ID" =~ (debian|ubuntu|linuxmint|pop|elementary|zorin|kali) || "$OS_LIKE" =~ (debian|ubuntu) ]]; then
        install_cmd="apt-get install -y"
        pkg_list="cmake ninja-build git pkg-config g++ qt6-base-dev qt6-declarative-dev libqt6svg6-dev qt6-wayland libglib2.0-dev xdg-utils libssl-dev libargon2-dev libqt6sql6-sqlite psmisc"
    elif [[ "$OS_ID" =~ (fedora|rhel|centos|rocky|alma) || "$OS_LIKE" =~ (fedora|rhel) ]]; then
        install_cmd="dnf install -y"
        pkg_list="cmake ninja-build git pkgconf-pkg-config gcc-c++ qt6-qtbase-devel qt6-qtdeclarative-devel qt6-qtsvg-devel qt6-qtwayland glib2-devel xdg-utils openssl-devel libargon2-devel qt6-qtbase-sqlite psmisc"
    elif [[ "$OS_ID" =~ opensuse || "$OS_LIKE" =~ (suse|opensuse) ]]; then
        install_cmd="zypper install -y"
        pkg_list="cmake ninja git pkgconf gcc-c++ qt6-base-devel qt6-declarative-devel libqt6svg6-devel libQt6WaylandClient6 glib2-devel xdg-utils libopenssl-devel libargon2-devel psmisc"
    elif [[ "$OS_ID" == "void" ]]; then
        install_cmd="xbps-install -S -y"
        pkg_list="cmake ninja git pkg-config gcc qt6-base-devel qt6-declarative-devel qt6-svg-devel qt6-wayland-devel glib-devel openssl-devel libargon2-devel psmisc"
    else
        echo "Warning: Could not automatically identify your Linux distribution ($OS_ID)." >&2
        return 1
    fi

    echo "==> Required packages for your distribution ($OS_ID):"
    echo "    $pkg_list"
    echo

    local do_install=0
    if [[ $AUTO_YES -eq 1 ]]; then
        do_install=1
    elif [[ -t 0 || -c /dev/tty ]]; then
        local reply=""
        if [[ -t 0 ]]; then
            read -r -p "==> Install missing packages automatically with sudo? [Y/n] " reply
        elif [[ -c /dev/tty ]]; then
            read -r -p "==> Install missing packages automatically with sudo? [Y/n] " reply </dev/tty
        fi
        if [[ -z "$reply" || "$reply" =~ ^[Yy]$ ]]; then
            do_install=1
        fi
    fi

    if [[ $do_install -eq 1 ]]; then
        echo "==> Installing dependencies..."
        if [[ $EUID -eq 0 ]]; then
            $install_cmd $pkg_list
        else
            sudo $install_cmd $pkg_list
        fi
        return 0
    else
        echo "==> Please install the required dependencies manually using:"
        if [[ $EUID -eq 0 ]]; then
            echo "    $install_cmd $pkg_list"
        else
            echo "    sudo $install_cmd $pkg_list"
        fi
        exit 1
    fi
}

if [[ $CHECK_DEPS -eq 1 ]]; then
    echo "==> Checking build dependencies..."
    MISSING_RAW="$(detect_missing_dependencies)"
    if [[ -n "$MISSING_RAW" ]]; then
        echo "==> Detected missing dependencies: $MISSING_RAW"
        install_distro_dependencies
    else
        echo "==> All required build dependencies are satisfied."
    fi
fi

# ==============================================================================
# Submodule Verification & Recovery
# ==============================================================================
echo "==> Verifying UI submodules (Quill & icons)..."
if [[ -d "$SCRIPT_DIR/.git" ]]; then
    git submodule update --init --recursive
fi

# Fallback in case submodules weren't cloned recursively or .git is missing
if [[ ! -f "$SCRIPT_DIR/src/qml/Quill/qmldir" ]]; then
    echo "==> Fetching Quill UI components..."
    rm -rf "$SCRIPT_DIR/src/qml/Quill"
    git clone --depth 1 https://github.com/soyeb-jim285/quill.git "$SCRIPT_DIR/src/qml/Quill"
fi

if [[ ! -f "$SCRIPT_DIR/src/qml/icons/qmldir" ]]; then
    echo "==> Fetching icon set components..."
    rm -rf "$SCRIPT_DIR/src/qml/icons"
    git clone --depth 1 https://github.com/soyeb-jim285/quill-icons.git "$SCRIPT_DIR/src/qml/icons"
fi

# ==============================================================================
# Configure & Build
# ==============================================================================
if [[ $FORCE_REBUILD -eq 1 && -d "$BUILD_DIR" ]]; then
    echo "==> Cleaning existing build directory..."
    rm -rf "$BUILD_DIR"
fi

echo "==> Configuring CMake..."
cmake -B "$BUILD_DIR" -S "$SCRIPT_DIR" -G Ninja \
    -DCMAKE_BUILD_TYPE="$BUILD_TYPE" \
    -DCMAKE_INSTALL_PREFIX="$PREFIX" \
    -DBUILD_TESTS=OFF \
    -DBUBBLE_DATA_DIR="$PREFIX/share/bubble"

echo "==> Building Bubble..."
cmake --build "$BUILD_DIR" --parallel

echo "==> Installing Bubble to '$PREFIX'..."
cmake --install "$BUILD_DIR" --prefix "$PREFIX"

# Additional integrations
mkdir -p "$PREFIX/bin" "$PREFIX/share/applications"

# Backward-compatibility symlink: hyprfm -> bubble
ln -sf bubble "$PREFIX/bin/hyprfm"
# Clean up old legacy bubble.desktop if present to prevent duplicate application menu entries
rm -f "$PREFIX/share/applications/bubble.desktop"

# Setuid permissions for bubble-vault-helper (kernel immutable attribute protection against sudo)
if [[ -x "$PREFIX/bin/bubble-vault-helper" ]]; then
    if [[ $EUID -eq 0 ]]; then
        chown root:root "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
        chmod 4755 "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
    elif command -v sudo >/dev/null 2>&1 && ( [[ $AUTO_YES -eq 1 ]] || sudo -n true 2>/dev/null ); then
        sudo chown root:root "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
        sudo chmod 4755 "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
    fi
fi

# If installing in user mode but sudo is available, install a setuid copy to /usr/local/bin
# so that kernel immutable attributes (+i) protect locked files from sudo/root operations
if [[ "$MODE" == "user" && -x "$PREFIX/bin/bubble-vault-helper" && ! -x "/usr/local/bin/bubble-vault-helper" ]]; then
    if [[ $EUID -eq 0 ]]; then
        install -m 4755 -o root -g root "$PREFIX/bin/bubble-vault-helper" /usr/local/bin/bubble-vault-helper 2>/dev/null || true
    elif command -v sudo >/dev/null 2>&1 && ( [[ $AUTO_YES -eq 1 ]] || sudo -n true 2>/dev/null ); then
        sudo install -m 4755 -o root -g root "$PREFIX/bin/bubble-vault-helper" /usr/local/bin/bubble-vault-helper 2>/dev/null || true
    fi
fi

# Polkit policy for system installations
if [[ "$MODE" == "system" || "$PREFIX" == /usr* ]]; then
    if [[ -d "/usr/share/polkit-1/actions" && $EUID -eq 0 ]]; then
        install -Dm644 "$SCRIPT_DIR/dist/org.bubble.vault.policy" "/usr/share/polkit-1/actions/org.bubble.vault.policy" 2>/dev/null || true
    fi
fi

# Update desktop and icon databases if available
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
echo " Vault cleanup binary: $PREFIX/bin/bubble-vault-destroy"
echo " Vault helper binary:  $PREFIX/bin/bubble-vault-helper"
echo " Legacy alias:        $PREFIX/bin/hyprfm"
echo " Desktop file:        $PREFIX/share/applications/io.github.soyeb_jim285.Bubble.desktop"
echo " Icon:                $PREFIX/share/icons/hicolor/scalable/apps/io.github.soyeb_jim285.Bubble.svg"
echo

# Path check for user mode
if [[ "$MODE" == "user" && ":$PATH:" != *":$PREFIX/bin:"* ]]; then
    echo "NOTE: '$PREFIX/bin' is not in your PATH."
    echo "To run 'bubble' from any terminal, add this to your ~/.bashrc or ~/.zshrc:"
    echo
    echo "  export PATH=\"$PREFIX/bin:\$PATH\""
    echo
fi

echo "You can now launch Bubble by running: bubble"

if [[ $CLEANUP_TMP -eq 1 ]]; then
    rm -rf "$SCRIPT_DIR"
fi
