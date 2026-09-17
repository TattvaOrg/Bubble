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

cd "$SCRIPT_DIR"

ensure_source_tree() {
    if [[ ! -f "$SCRIPT_DIR/CMakeLists.txt" || ! -d "$SCRIPT_DIR/src" ]]; then
        TMP_CLONE_DIR="$(mktemp -d /tmp/bubble-install-XXXXXX)"
        echo "==> Fetching Bubble source repository (${BUBBLE_REPO}) to $TMP_CLONE_DIR..."
        git clone --depth 1 --recursive "https://github.com/${BUBBLE_REPO}.git" "$TMP_CLONE_DIR"
        SCRIPT_DIR="$TMP_CLONE_DIR"
        CLEANUP_TMP=1
        cd "$SCRIPT_DIR"
    fi
}

# Defaults
MODE="user"
CUSTOM_PREFIX=""
BUILD_DIR="${BUILD_DIR:-$SCRIPT_DIR/build}"
BUILD_TYPE="Release"
CHECK_DEPS=1
AUTO_YES=0
UNINSTALL=0
FORCE_REBUILD=0
INSTALL_METHOD=""
BUBBLE_REPO="${BUBBLE_REPO:-TattvaOrg/Bubble}"
TARGET_TAG=""

print_usage() {
    cat <<USAGE
Bubble Installer

Usage:
  ./install.sh [method] [options]

Installation Methods:
  1, --binary, --appimage  Install prebuilt binary (AppImage) [default]
  2, --source, --build     Build and install from source using CMake

Options:
  --repo <owner/repo> GitHub repository to download from (default: TattvaOrg/Bubble)
  --tag <tag>         Specific release tag to install (e.g. continuous, v0.6.1)
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
  ./install.sh                    # Interactive choice: AppImage or Source
  ./install.sh 1                  # Install prebuilt AppImage
  ./install.sh 2                  # Build and install from source
  ./install.sh --binary           # Install prebuilt AppImage
  ./install.sh --source           # Build and install from source
  sudo ./install.sh --system      # Installs to /usr/local
  ./install.sh --uninstall        # Removes from ~/.local
USAGE
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        1|--binary|--appimage)
            INSTALL_METHOD="binary"
            shift
            ;;
        2|--source|--build)
            INSTALL_METHOD="source"
            shift
            ;;
        --repo)
            if [[ -z "${2:-}" ]]; then
                echo "Error: --repo requires an owner/repo argument." >&2
                exit 1
            fi
            BUBBLE_REPO="$2"
            shift 2
            ;;
        --tag)
            if [[ -z "${2:-}" ]]; then
                echo "Error: --tag requires a version tag argument." >&2
                exit 1
            fi
            TARGET_TAG="$2"
            shift 2
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
        -f|--force|--rebuild)
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
    if [[ -f "$SCRIPT_DIR/uninstall.sh" ]]; then
        UNINSTALL_ARGS=()
        if [[ -n "$CUSTOM_PREFIX" ]]; then
            UNINSTALL_ARGS+=(--prefix "$CUSTOM_PREFIX")
        fi
        if [[ $AUTO_YES -eq 1 ]]; then
            UNINSTALL_ARGS+=(-y)
        fi
        exec bash "$SCRIPT_DIR/uninstall.sh" "${UNINSTALL_ARGS[@]}"
    fi

    echo "==> Uninstalling Bubble from prefix: $PREFIX"

    # Terminate running Bubble instances safely and completely
    kill_bubble_safely() {
        local my_pid="$$"
        local parent_pid="$PPID"
        local pids_to_kill=()

        local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/bubble"
        if command -v fuser >/dev/null 2>&1 && [[ -d "$config_dir" ]]; then
            for cfg_item in "$config_dir" "$config_dir"/*; do
                [[ -e "$cfg_item" ]] || continue
                while read -r fpid; do
                    [[ -z "$fpid" ]] && continue
                    [[ "$fpid" == "$my_pid" || "$fpid" == "$parent_pid" ]] && continue
                    pids_to_kill+=("$fpid")
                done < <(fuser "$cfg_item" 2>/dev/null | tr ' ' '\n' | grep -E '^[0-9]+$' || true)
            done
        fi

        for pdir in /proc/[0-9]*; do
            local pid="${pdir##*/}"
            [[ "$pid" == "$my_pid" || "$pid" == "$parent_pid" ]] && continue

            local cmdline=""
            [[ -r "/proc/$pid/cmdline" ]] && cmdline="$(tr '\0' ' ' < "/proc/$pid/cmdline" 2>/dev/null || true)"
            if [[ "$cmdline" == *"uninstall.sh"* || "$cmdline" == *"install.sh"* || "$cmdline" == *"antigravity"* || "$cmdline" == *"agy"* ]]; then
                continue
            fi

            local comm=""
            [[ -r "/proc/$pid/comm" ]] && comm="$(cat "/proc/$pid/comm" 2>/dev/null || true)"
            local exe=""
            [[ -L "/proc/$pid/exe" ]] && exe="$(readlink -f "/proc/$pid/exe" 2>/dev/null || true)"

            if [[ "$comm" =~ ^(bash|sh|zsh|sudo|curl|wget|python|python3|node)$ ]]; then
                if [[ "$exe" != */bin/bubble && "$exe" != */.mount_Bubble* && "$exe" != */.mount_bubble* ]]; then
                    continue
                fi
            fi

            local is_bubble=0
            if [[ "$comm" == "bubble" || "$comm" == "bubble-vault-helper" || "$comm" == "bubble-vault-destroy" || "$comm" == "hyprfm" ]]; then
                is_bubble=1
            elif [[ "$exe" == */bin/bubble || "$exe" == */build/*/bubble || "$exe" == */.mount_Bubble* || "$exe" == */.mount_bubble* || "$exe" == *Bubble*.AppImage* ]]; then
                is_bubble=1
            elif [[ "$comm" == "AppRun"* && ("$exe" == */.mount_* || "$cmdline" == *bubble* || "$cmdline" == *Bubble*) ]]; then
                is_bubble=1
            fi

            if [[ $is_bubble -eq 1 ]]; then
                pids_to_kill+=("$pid")
            fi
        done

        if [[ ${#pids_to_kill[@]} -gt 0 ]]; then
            local unique_pids=($(printf "%s\n" "${pids_to_kill[@]}" | sort -u))
            echo "==> Closing running Bubble application instances (PIDs: ${unique_pids[*]})..."
            kill -TERM "${unique_pids[@]}" 2>/dev/null || true
            sleep 0.8
            for p in "${unique_pids[@]}"; do
                if kill -0 "$p" 2>/dev/null; then
                    kill -9 "$p" 2>/dev/null || sudo kill -9 "$p" 2>/dev/null || true
                fi
            done
        fi
        pkill -9 -x bubble-vault-helper 2>/dev/null || true
        for mnt in /tmp/.mount_Bubble* /tmp/.mount_bubble*; do
            if [[ -d "$mnt" ]]; then
                fusermount -u "$mnt" 2>/dev/null || umount -l "$mnt" 2>/dev/null || true
            fi
        done
    }

    kill_bubble_safely

    # Securely unprotect, shred and destroy all locked vault files before removal
    LOCKED_ITEMS=()
    VAULT_DB="${XDG_CONFIG_HOME:-$HOME/.config}/bubble/vault.db"
    if [[ -f "$VAULT_DB" ]]; then
        if command -v sqlite3 >/dev/null 2>&1; then
            while IFS= read -r item_path; do
                [[ -n "$item_path" ]] && LOCKED_ITEMS+=("$item_path")
            done < <(sqlite3 "$VAULT_DB" "SELECT path FROM locked_items;" 2>/dev/null || true)
        fi
    fi
    for search_base in "$HOME" "$HOME/Desktop" "$HOME/Documents" "$HOME/Downloads"; do
        if [[ -d "$search_base" ]]; then
            while IFS= read -r found_xattr; do
                [[ -z "$found_xattr" ]] && continue
                [[ "$found_xattr" != /* ]] && found_xattr="/$found_xattr"
                LOCKED_ITEMS+=("$found_xattr")
            done < <(find "$search_base" -maxdepth 2 -exec getfattr -d -m "user.bubble.locked" {} + 2>/dev/null | grep "^# file: " | sed 's/^# file: //' || true)
        fi
    done
    if [[ ${#LOCKED_ITEMS[@]} -gt 0 ]]; then
        readarray -t LOCKED_ITEMS < <(printf "%s\n" "${LOCKED_ITEMS[@]}" | sort -u)
    fi

    if [[ ${#LOCKED_ITEMS[@]} -gt 0 ]]; then
        echo "==> Detected Locked Vault Files & Folders (will be shredded & wiped):"
        for item in "${LOCKED_ITEMS[@]}"; do
            [[ -d "$item" ]] && echo "  - $item (directory)" || echo "  - $item (file)"
        done
    fi

    echo "==> Cryptographically shredding and wiping all locked vault files..."
    for item in "${LOCKED_ITEMS[@]}"; do
        if [[ -e "$item" ]]; then
            if [[ -x "$PREFIX/bin/bubble-vault-helper" ]]; then
                "$PREFIX/bin/bubble-vault-helper" unprotect "$item" 2>/dev/null || true
            elif command -v bubble-vault-helper >/dev/null 2>&1; then
                bubble-vault-helper unprotect "$item" 2>/dev/null || true
            fi
            chattr -R -i "$item" 2>/dev/null || sudo chattr -R -i "$item" 2>/dev/null || true
            chmod -R 777 "$item" 2>/dev/null || sudo chmod -R 777 "$item" 2>/dev/null || true
        fi
    done

    if command -v bubble-vault-destroy >/dev/null 2>&1; then
        bubble-vault-destroy || true
    elif [[ -x "$PREFIX/bin/bubble-vault-destroy" ]]; then
        "$PREFIX/bin/bubble-vault-destroy" || true
    fi

    for item in "${LOCKED_ITEMS[@]}"; do
        if [[ -d "$item" ]]; then
            chmod -R 777 "$item" 2>/dev/null || sudo chmod -R 777 "$item" 2>/dev/null || true
            find "$item" -type f -exec shred -u -z {} + 2>/dev/null || true
            rm -rf "$item" 2>/dev/null || sudo rm -rf "$item" 2>/dev/null || true
        elif [[ -f "$item" ]]; then
            chmod 777 "$item" 2>/dev/null || sudo chmod 777 "$item" 2>/dev/null || true
            shred -u -z "$item" 2>/dev/null || rm -f "$item" 2>/dev/null || sudo rm -f "$item" 2>/dev/null || true
        fi
        if [[ -e "$item" ]]; then
            chattr -R -i "$item" 2>/dev/null || sudo chattr -R -i "$item" 2>/dev/null || true
            chmod -R 777 "$item" 2>/dev/null || sudo chmod -R 777 "$item" 2>/dev/null || true
            rm -rf "$item" 2>/dev/null || sudo rm -rf "$item" 2>/dev/null || true
        fi
    done

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
    rm -f "$PREFIX/share/applications/io.github.tattvaorg.Bubble.desktop"
    rm -f "$PREFIX/share/applications/io.github.soyeb_jim285.Bubble.desktop"
    rm -f "$PREFIX/share/applications/bubble.desktop"
    rm -f "$PREFIX/share/icons/hicolor/scalable/apps/io.github.tattvaorg.Bubble.svg"
    rm -f "$PREFIX/share/icons/hicolor/scalable/apps/io.github.soyeb_jim285.Bubble.svg"
    rm -f "$PREFIX/share/metainfo/io.github.tattvaorg.Bubble.metainfo.xml"
    rm -f "$PREFIX/share/metainfo/io.github.soyeb_jim285.Bubble.metainfo.xml"
    rm -f "$PREFIX/share/libalpm/hooks/bubble-cleanup.hook"
    rm -f "$PREFIX/share/polkit-1/actions/org.bubble.vault.policy"

    if [[ -f "/usr/share/polkit-1/actions/org.bubble.vault.policy" && $EUID -eq 0 ]]; then
        rm -f "/usr/share/polkit-1/actions/org.bubble.vault.policy"
    fi

    # Wipe configuration, cache, and state
    rm -rf "${XDG_CONFIG_HOME:-$HOME/.config}/bubble"
    rm -rf "${XDG_CACHE_HOME:-$HOME/.cache}/bubble"
    rm -rf "${XDG_STATE_HOME:-$HOME/.local/state}/bubble"

    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -t "$PREFIX/share/icons/hicolor" 2>/dev/null || true
    fi
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$PREFIX/share/applications" 2>/dev/null || true
    fi

    echo "==> Bubble has been uninstalled successfully."
    exit 0
fi

# Prompt for installation method if not specified via CLI
if [[ -z "$INSTALL_METHOD" ]]; then
    if [[ $AUTO_YES -eq 1 || ( ! -t 0 && ! -c /dev/tty ) ]]; then
        INSTALL_METHOD="binary"
    else
        echo "=============================================="
        echo "               Bubble Installer               "
        echo "=============================================="
        echo "Choose an installation method:"
        echo "  1) Prebuilt AppImage (Recommended: instant download, no build tools)"
        echo "  2) Compile from source (Clone git repo, build via CMake & Ninja)"
        echo
        choice=""
        if [[ -t 0 ]]; then
            read -r -p "Enter choice [1-2] (default: 1): " choice || choice=""
        elif [[ -c /dev/tty ]]; then
            read -r -p "Enter choice [1-2] (default: 1): " choice </dev/tty || choice=""
        fi
        if [[ "$choice" == "2" || "$choice" == "source" ]]; then
            INSTALL_METHOD="source"
        else
            INSTALL_METHOD="binary"
        fi
        echo
    fi
fi

echo "=============================================="
echo "          Bubble Installation Setup           "
echo "=============================================="
echo " Method        : $INSTALL_METHOD"
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
# Prebuilt Binary Installation (AppImage)
# ==============================================================================
install_prebuilt_binary() {
    local arch
    arch="$(uname -m)"
    if [[ "$arch" != "x86_64" ]]; then
        echo "==> Prebuilt binary is only available for x86_64 (detected: $arch)."
        return 1
    fi

    local repos_to_check=()
    if [[ -n "${BUBBLE_REPO:-}" ]]; then
        repos_to_check+=("$BUBBLE_REPO")
    fi
    local origin_repo=""
    if command -v git >/dev/null 2>&1; then
        origin_repo="$(git remote get-url origin 2>/dev/null | sed -E 's#^.*github\.com[:/]([^/]+/[^/.]+)(\.git)?$#\1#' || true)"
    fi
    if [[ -n "$origin_repo" && "$origin_repo" != "${BUBBLE_REPO:-}" ]]; then
        repos_to_check+=("$origin_repo")
    fi
    for fallback in "TattvaOrg/Bubble"; do
        if [[ ! " ${repos_to_check[*]} " =~ " ${fallback} " ]]; then
            repos_to_check+=("$fallback")
        fi
    done

    local tmp_appimage
    tmp_appimage="$(mktemp /tmp/bubble-bin-XXXXXX.AppImage)"
    local success=0
    local download_url=""
    local matched_repo=""

    for repo in "${repos_to_check[@]}"; do
        echo "==> Searching for prebuilt Bubble AppImage on ${repo}..."
        local candidate_urls=()
        if [[ -n "$TARGET_TAG" ]]; then
            candidate_urls+=(
                "https://github.com/${repo}/releases/download/${TARGET_TAG}/Bubble-${TARGET_TAG}-x86_64.AppImage"
                "https://github.com/${repo}/releases/download/${TARGET_TAG}/Bubble-x86_64.AppImage"
            )
        else
            candidate_urls+=(
                "https://github.com/${repo}/releases/latest/download/Bubble-x86_64.AppImage"
                "https://github.com/${repo}/releases/download/continuous/Bubble-continuous-x86_64.AppImage"
                "https://github.com/${repo}/releases/download/continuous/Bubble-x86_64.AppImage"
            )
        fi

        for url in "${candidate_urls[@]}"; do
            echo "--> Checking $url..."
            if command -v curl >/dev/null 2>&1; then
                if curl -f -sSL -L "$url" -o "$tmp_appimage" 2>/dev/null; then
                    if [[ -s "$tmp_appimage" ]] && head -c 4 "$tmp_appimage" | grep -q "ELF"; then
                        download_url="$url"
                        matched_repo="$repo"
                        success=1
                        break 2
                    fi
                fi
            elif command -v wget >/dev/null 2>&1; then
                if wget -q -O "$tmp_appimage" "$url" 2>/dev/null; then
                    if [[ -s "$tmp_appimage" ]] && head -c 4 "$tmp_appimage" | grep -q "ELF"; then
                        download_url="$url"
                        matched_repo="$repo"
                        success=1
                        break 2
                    fi
                fi
            fi
        done

        local api_url="https://api.github.com/repos/${repo}/releases"
        local found_url=""
        if command -v curl >/dev/null 2>&1; then
            found_url=$(curl -sSL -H "Accept: application/vnd.github.v3+json" "$api_url" 2>/dev/null \
                | grep -E -o 'https://github.com/[^"]*Bubble[^"]*\.AppImage' | head -n 1 || true)
        elif command -v wget >/dev/null 2>&1; then
            found_url=$(wget -qO- "$api_url" 2>/dev/null \
                | grep -E -o 'https://github.com/[^"]*Bubble[^"]*\.AppImage' | head -n 1 || true)
        fi

        if [[ -n "$found_url" ]]; then
            echo "--> Found release asset: $found_url"
            if command -v curl >/dev/null 2>&1 && curl -f -sSL -L "$found_url" -o "$tmp_appimage" 2>/dev/null; then
                if [[ -s "$tmp_appimage" ]] && head -c 4 "$tmp_appimage" | grep -q "ELF"; then
                    download_url="$found_url"
                    matched_repo="$repo"
                    success=1
                    break
                fi
            elif command -v wget >/dev/null 2>&1 && wget -q -O "$tmp_appimage" "$found_url" 2>/dev/null; then
                if [[ -s "$tmp_appimage" ]] && head -c 4 "$tmp_appimage" | grep -q "ELF"; then
                    download_url="$found_url"
                    matched_repo="$repo"
                    success=1
                    break
                fi
            fi
        fi
    done

    if [[ $success -eq 0 || ! -s "$tmp_appimage" ]]; then
        rm -f "$tmp_appimage"
        echo "==> No prebuilt binary found on candidate repositories (${repos_to_check[*]})."
        return 1
    fi

    # Verify binary format (ELF magic number)
    if ! head -c 4 "$tmp_appimage" | grep -q "ELF"; then
        echo "==> Downloaded file is not a valid ELF executable."
        rm -f "$tmp_appimage"
        return 1
    fi

    echo "==> Successfully downloaded prebuilt binary from: $download_url"
    echo "==> Installing Bubble AppImage to '$PREFIX'..."

    mkdir -p "$PREFIX/bin" "$PREFIX/share/applications" "$PREFIX/share/icons/hicolor/scalable/apps"

    install -m 755 "$tmp_appimage" "$PREFIX/bin/bubble"
    rm -f "$tmp_appimage"
    rm -f "$PREFIX/bin/hyprfm"
    rm -f "$PREFIX/share/applications/bubble.desktop"

    # Install desktop entry and icon
    if [[ -f "$SCRIPT_DIR/dist/io.github.tattvaorg.Bubble.desktop" ]]; then
        install -m 644 "$SCRIPT_DIR/dist/io.github.tattvaorg.Bubble.desktop" "$PREFIX/share/applications/"
    elif command -v curl >/dev/null 2>&1; then
        curl -sSL -f "https://raw.githubusercontent.com/${BUBBLE_REPO}/main/dist/io.github.tattvaorg.Bubble.desktop" \
            -o "$PREFIX/share/applications/io.github.tattvaorg.Bubble.desktop" 2>/dev/null || true
    fi

    if [[ -f "$SCRIPT_DIR/dist/io.github.tattvaorg.Bubble.svg" ]]; then
        install -m 644 "$SCRIPT_DIR/dist/io.github.tattvaorg.Bubble.svg" "$PREFIX/share/icons/hicolor/scalable/apps/"
    elif command -v curl >/dev/null 2>&1; then
        curl -sSL -f "https://raw.githubusercontent.com/${BUBBLE_REPO}/main/dist/io.github.tattvaorg.Bubble.svg" \
            -o "$PREFIX/share/icons/hicolor/scalable/apps/io.github.tattvaorg.Bubble.svg" 2>/dev/null || true
    fi

    # Extract vault helpers if present inside AppImage
    local extract_tmp
    extract_tmp="$(mktemp -d /tmp/bubble-extract-XXXXXX)"
    if (cd "$extract_tmp" && "$PREFIX/bin/bubble" --appimage-extract "usr/bin/bubble-vault-*" >/dev/null 2>&1); then
        if [[ -f "$extract_tmp/squashfs-root/usr/bin/bubble-vault-destroy" ]]; then
            install -m 755 "$extract_tmp/squashfs-root/usr/bin/bubble-vault-destroy" "$PREFIX/bin/bubble-vault-destroy" 2>/dev/null || true
        fi
        if [[ -f "$extract_tmp/squashfs-root/usr/bin/bubble-vault-helper" ]]; then
            install -m 755 "$extract_tmp/squashfs-root/usr/bin/bubble-vault-helper" "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
        fi
    fi
    rm -rf "$extract_tmp"

    # Setuid permissions for bubble-vault-helper if installed
    if [[ -x "$PREFIX/bin/bubble-vault-helper" ]]; then
        if [[ $EUID -eq 0 ]]; then
            chown root:root "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
            chmod 4755 "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
        elif command -v sudo >/dev/null 2>&1 && sudo -n true 2>/dev/null; then
            sudo chown root:root "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
            sudo chmod 4755 "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
        fi
    fi

    # Update desktop and icon databases
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
    echo " Mode:                Prebuilt Binary (AppImage)"
    echo " Binary installed to: $PREFIX/bin/bubble"
    echo " Desktop file:        $PREFIX/share/applications/io.github.tattvaorg.Bubble.desktop"
    echo " Icon:                $PREFIX/share/icons/hicolor/scalable/apps/io.github.tattvaorg.Bubble.svg"
    echo

    if [[ "$MODE" == "user" && ":$PATH:" != *":$PREFIX/bin:"* ]]; then
        echo "NOTE: '$PREFIX/bin' is not in your PATH."
        echo "To run 'bubble' from any terminal, add this to your ~/.bashrc or ~/.zshrc:"
        echo
        echo "  export PATH=\"$PREFIX/bin:\$PATH\""
        echo
    fi

    echo "You can now launch Bubble by running: bubble"
    return 0
}

# Run prebuilt binary installation if requested
if [[ "$INSTALL_METHOD" == "binary" ]]; then
    if install_prebuilt_binary; then
        if [[ $CLEANUP_TMP -eq 1 ]]; then
            rm -rf "$SCRIPT_DIR"
        fi
        exit 0
    fi
    echo "==> Falling back to building from source..."
    INSTALL_METHOD="source"
fi

# Ensure source repository is available for compilation
ensure_source_tree

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

# Clean up any legacy hyprfm symlink/binary
rm -f "$PREFIX/bin/hyprfm"
# Clean up old legacy bubble.desktop if present to prevent duplicate application menu entries
rm -f "$PREFIX/share/applications/bubble.desktop"

# Setuid permissions for bubble-vault-helper (kernel immutable attribute protection against sudo)
if [[ -x "$PREFIX/bin/bubble-vault-helper" ]]; then
    if [[ $EUID -eq 0 ]]; then
        chown root:root "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
        chmod 4755 "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
    elif command -v sudo >/dev/null 2>&1 && sudo -n true 2>/dev/null; then
        sudo chown root:root "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
        sudo chmod 4755 "$PREFIX/bin/bubble-vault-helper" 2>/dev/null || true
    fi
fi

# If installing in user mode but sudo is available, install a setuid copy to /usr/local/bin
# so that kernel immutable attributes (+i) protect locked files from sudo/root operations
if [[ "$MODE" == "user" && -x "$PREFIX/bin/bubble-vault-helper" && ! -x "/usr/local/bin/bubble-vault-helper" ]]; then
    if [[ $EUID -eq 0 ]]; then
        install -m 4755 -o root -g root "$PREFIX/bin/bubble-vault-helper" /usr/local/bin/bubble-vault-helper 2>/dev/null || true
    elif command -v sudo >/dev/null 2>&1 && sudo -n true 2>/dev/null; then
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
echo " Desktop file:        $PREFIX/share/applications/io.github.tattvaorg.Bubble.desktop"
echo " Icon:                $PREFIX/share/icons/hicolor/scalable/apps/io.github.tattvaorg.Bubble.svg"
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
