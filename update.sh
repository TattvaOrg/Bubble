#!/usr/bin/env bash
# ==============================================================================
# Bubble Updater Script
# ==============================================================================
# Pulls the latest commits, updates submodules, checks dependencies,
# and rebuilds/reinstalls Bubble.
#
# Usage:
#   ./update.sh              # Update and reinstall with auto-detected prefix
#   ./update.sh --rebuild    # Force a full clean rebuild
#   ./update.sh --system     # Update and install system-wide (/usr/local)
#   ./update.sh --user       # Update and install for current user (~/.local)
#   ./update.sh --check      # Check if updates are available without applying
# ==============================================================================

set -euo pipefail

# Determine script location
if [[ -n "${BASH_SOURCE[0]:-}" && -f "${BASH_SOURCE[0]}" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
else
    SCRIPT_DIR="$(pwd)"
fi

# Handle execution via curl / outside git repo
if [[ ! -d "$SCRIPT_DIR/.git" || ! -f "$SCRIPT_DIR/CMakeLists.txt" ]]; then
    CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/bubble-source"
    echo "==> Bubble updater invoked outside source repo."
    if [[ -d "$CACHE_DIR/.git" ]]; then
        echo "==> Updating cached repository at $CACHE_DIR..."
        git -C "$CACHE_DIR" fetch --depth 1 origin main
        git -C "$CACHE_DIR" reset --hard origin/main
        git -C "$CACHE_DIR" submodule update --init --recursive
    else
        echo "==> Cloning Bubble into $CACHE_DIR..."
        mkdir -p "$(dirname "$CACHE_DIR")"
        git clone --depth 1 --recursive https://github.com/TattvaOrg/Bubble.git "$CACHE_DIR"
    fi
    exec bash "$CACHE_DIR/update.sh" "$@"
fi

cd "$SCRIPT_DIR"

# Flags
CHECK_ONLY=0
FORCE_UPDATE=0
PASSTHROUGH_ARGS=()

print_usage() {
    cat <<USAGE
Bubble Updater

Usage:
  ./update.sh [options]

Options:
  --check             Check for remote updates without building or installing
  --rebuild           Force clean build directory before updating
  --system            Force system-wide installation (/usr/local, requires sudo)
  --user              Force user-mode installation (~/.local)
  --no-deps           Skip dependency checking
  -y, --yes           Automatically install missing packages without prompting
  -f, --force         Rebuild and reinstall even if already on the latest commit
  -h, --help          Show this help message

Examples:
  ./update.sh                     # Update to latest version
  ./update.sh --check             # Check for new version only
  ./update.sh --rebuild           # Clean rebuild and reinstall
USAGE
}

for arg in "$@"; do
    case "$arg" in
        --check)
            CHECK_ONLY=1
            ;;
        -f|--force)
            FORCE_UPDATE=1
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        *)
            PASSTHROUGH_ARGS+=("$arg")
            ;;
    esac
done

echo "=============================================="
echo "               Bubble Updater                 "
echo "=============================================="

# Ensure git is available
if ! command -v git >/dev/null 2>&1; then
    echo "Error: 'git' is required to update Bubble." >&2
    exit 1
fi

# Fetch remote changes
echo "==> Fetching updates from remote repository..."
git fetch origin main

LOCAL_HASH="$(git rev-parse HEAD)"
REMOTE_HASH="$(git rev-parse origin/main)"
LOCAL_SHORT="$(git rev-parse --short HEAD)"
REMOTE_SHORT="$(git rev-parse --short origin/main)"

if [[ "$LOCAL_HASH" == "$REMOTE_HASH" ]]; then
    echo "==> Bubble is already on the latest version ($LOCAL_SHORT)."
    if [[ $CHECK_ONLY -eq 1 ]]; then
        exit 0
    fi
    if [[ $FORCE_UPDATE -eq 0 ]]; then
        echo
        echo "No new commits found. Use './update.sh --force' or './update.sh --rebuild' to force a reinstall."
        exit 0
    fi
else
    echo "==> New version available:"
    echo "    Current version : $LOCAL_SHORT"
    echo "    Latest version  : $REMOTE_SHORT"
    echo
    echo "Recent changes:"
    git log --oneline -n 5 "$LOCAL_HASH..$REMOTE_HASH"
    echo
fi

if [[ $CHECK_ONLY -eq 1 ]]; then
    exit 0
fi

# Check for local uncommitted changes
if ! git diff-index --quiet HEAD -- 2>/dev/null; then
    echo "==> Warning: You have uncommitted local changes in $SCRIPT_DIR."
    echo "Stashing uncommitted changes before update..."
    git stash push -m "bubble-autostash-before-update"
fi

echo "==> Updating source tree to $REMOTE_SHORT..."
git pull --rebase origin main

echo "==> Syncing git submodules..."
git submodule update --init --recursive

# Detect existing installation mode if not explicitly specified
HAS_MODE=0
for arg in "${PASSTHROUGH_ARGS[@]:-}"; do
    if [[ "$arg" == "--user" || "$arg" == "--system" || "$arg" == "--prefix" ]]; then
        HAS_MODE=1
        break
    fi
done

if [[ $HAS_MODE -eq 0 ]]; then
    if [[ -x "/usr/local/bin/bubble" && ! -x "$HOME/.local/bin/bubble" ]]; then
        echo "==> Detected existing system installation in /usr/local."
        PASSTHROUGH_ARGS+=("--system")
    else
        PASSTHROUGH_ARGS+=("--user")
    fi
fi

echo "==> Rebuilding and installing updated Bubble..."
./install.sh "${PASSTHROUGH_ARGS[@]:-}"

echo
echo "=============================================="
echo "    Bubble has been successfully updated!     "
echo "=============================================="
echo " Current commit: $(git rev-parse --short HEAD)"
echo " Enjoy using Bubble!"
