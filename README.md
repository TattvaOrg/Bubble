<div align="center">

<img src="dist/io.github.soyeb_jim285.Bubble.svg" width="200" alt="Bubble Logo"/>

**A fast, modern Wayland file manager with Miller columns and a built-in Secure Vault.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Release](https://img.shields.io/github/v/release/TattvaOrg/Bubble?style=flat-square&color=emerald)](https://github.com/TattvaOrg/Bubble/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/TattvaOrg/Bubble/build.yml?style=flat-square)](https://github.com/TattvaOrg/Bubble/actions)
[![Platform](https://img.shields.io/badge/Platform-Wayland%20%7C%20Hyprland-purple?style=flat-square)](https://wayland.freedesktop.org/)


[Installation](#-installation) •
[Uninstallation](#4-uninstallation) •
[Secure Vault](#-secure-file--folder-vault) •
[Features](#-core-features) •
[Shortcuts](#-keyboard-shortcuts) •
[Theming](#-theming--customization) •
[Compositor & Glass](#-compositor-integration-hyprland-niri--lniri)

---

</div>

Bubble is a lightweight, responsive Qt6/QML desktop file manager engineered natively for Wayland and Hyprland. Combining **macOS Finder-style Miller columns**, buttery-smooth kinetic scrolling, and **AES-256 encrypted file vaults**, Bubble offers the speed of a keyboard-driven workflow with the polish of a modern desktop utility.

---



## Installation

### 1. Interactive One-Liner Install

Run the official installer and pick your preferred method (1 or 2):

```bash
curl -fsSL https://raw.githubusercontent.com/TattvaOrg/Bubble/main/install.sh | bash
```

### 2. Direct One-Liners

Choose directly between prebuilt AppImage or compiling from source:

#### Option 1: Prebuilt AppImage (Recommended)
Downloads the prebuilt binary, installs `bubble` to `~/.local/bin`, and configures desktop integration without needing build tools:
```bash
curl -fsSL https://raw.githubusercontent.com/TattvaOrg/Bubble/main/install.sh | bash -s -- 1
```

#### Option 2: Compile from Source
Fetches the source code, resolves dependencies, and builds natively with CMake & Ninja:
```bash
curl -fsSL https://raw.githubusercontent.com/TattvaOrg/Bubble/main/install.sh | bash -s -- 2
```

**Additional Installer Flags:**
- `1` / `--binary` / `--appimage` : Prebuilt binary *(default)*
- `2` / `--source` / `--build`    : Compile from source
- `--system`                      : Install system-wide to `/usr/local` *(requires sudo)*
- `--prefix <dir>`                : Install to a custom directory path
- `--tag <tag>`                   : Install a specific version tag *(e.g. `continuous`, `v0.6.1`)*
- `-y` / `--yes`                  : Automatically accept dependency installs without prompting



### 3. Complete Uninstallation & Wipeout

To completely wipe out Bubble, including all binaries, desktop integrations, configuration, cache, and cryptographically shred all locked vault files:

```bash
# One-liner remote uninstaller
curl -fsSL https://raw.githubusercontent.com/TattvaOrg/Bubble/main/uninstall.sh | bash

# Or run locally from cloned repository
./uninstall.sh
```

> [!WARNING]
> The uninstaller prompts once for confirmation (`Are you sure you want to completely wipe Bubble and all locked files? [y/N]`) and wipes everything clean. Pass `-y` to run non-interactively.

---

## Secure File & Folder Vault

Bubble includes a built-in cryptographic vault that lets you lock sensitive files and directories directly from the file manager with zero complex setup.
*Lock files instantly from context menus or with a single keyboard shortcut*

</div>

### Key Capabilities

- **One-Key Lock/Unlock (`Ctrl+Shift+L`)**: Select any file or folder and press `Ctrl+Shift+L` (or right-click → **Lock/Unlock**) to secure it with a password.
- **Authenticated Encryption**: Backed by **AES-256-GCM** authenticated encryption with **Argon2id** key derivation to prevent brute-force attacks.
- **Root & Tamper Prevention**: Locked items are placed in strict permission lockdown (`0000`) and protected by Linux kernel immutable flags (`+i`), preventing deletion, renaming, or modification even under `sudo`.
- **Intelligent Auto-Relocking**: When you unlock a file to view or edit it, Bubble monitors the active editing session in the background and automatically re-encrypts and locks it when you're finished.
- **Visual Status Indicators**: Locked files display a secure badge in both detailed and grid views.
- **Ghost-Lock & Corruption Protection**: Automatic integrity checks verify HMAC/GCM tags before decryption to guard against corruption or tampered data.

---

## Core Features

### Dynamic Views & Previews
- **Miller Columns (`Ctrl+2`)**: Seamless hierarchical navigation through directory trees with an automatic live preview pane.
- **Grid View (`Ctrl+1`)**: Responsive icon grid with adjustable column counts and kinetic mouse wheel zoom (`Ctrl+Scroll`).
- **Detailed List View (`Ctrl+3`)**: Sortable columns (Name, Size, Modified, Type, Permissions, Git status).
- **Quick Preview (`Space`)**: Instant full-screen modal preview for high-resolution images, video posters, PDFs, and syntax-highlighted code.
- **Dual-Pane Split (`F3`)**: Side-by-side independent navigation panes for rapid drag & drop and cross-folder management.

### Desktop & Wayland Integration
- **Compositor Blur**: Native Wayland surface transparency and background blur support on Hyprland and KDE Plasma.
- **Removable Media**: One-click mount and unmount of USB drives and partitions via UDisks2 and DBus.
- **Async File Transfers**: Non-blocking background copy, move, and trash operations with real-time transfer speed and ETA.
- **Git Badges**: Instant status indicators on modified, staged, and untracked files within repositories.

---

## ⌨️ Keyboard Shortcuts

Bubble is designed for efficient keyboard-first navigation:

### Navigation & Views
| Shortcut | Action |
|---|---|
| `Enter` / Double Click | Open file or directory |
| `Backspace` / `Alt+Up` | Go to parent directory |
| `Alt+Left` / `Alt+Right` | History Back / Forward |
| `Ctrl+L` | Focus address / path bar (inline edit) |
| `Ctrl+F` | Instant search filter |
| `Ctrl+1` / `Ctrl+2` / `Ctrl+3` | Switch to Grid / Miller Columns / Detailed View |
| `Space` | Quick Preview overlay (Images, Video, Code, PDF) |
| `F3` | Toggle Dual-Pane Split view |
| `F9` | Toggle Sidebar |
| `Ctrl+H` | Toggle hidden files (`.dotfiles`) |
| `Ctrl+Shift+B` | Toggle background transparency / blur |

### Tabs & Windows
| Shortcut | Action |
|---|---|
| `Ctrl+T` | Open new tab |
| `Ctrl+W` | Close current tab |
| `Ctrl+Shift+T` | Reopen last closed tab |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Cycle next / previous tab |
| `Alt+1` – `Alt+9` | Jump directly to tab 1–9 |
| `Ctrl+Alt+N` | Open new independent window |

### File Operations & Security
| Shortcut | Action |
|---|---|
| `Ctrl+Shift+L` | **Lock / Unlock file or folder in Vault** |
| `F2` | Inline file rename |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | Copy / Cut / Paste |
| `Delete` | Send to Trash |
| `Shift+Delete` | Permanently delete |
| `Ctrl+Shift+N` | Create new directory |
| `Ctrl+N` | Create new empty file |
| `Ctrl+Alt+T` | Open terminal at current directory |

---

## Theming & Customization

Bubble features a modular TOML-based theme engine with hot-reloading on save.

```
~/.config/bubble/
├── config.toml           # User preferences and keybindings
└── themes/               # Custom user themes (*.toml)
```

### Bundled Themes
- **Catppuccin**: Mocha (dark) & Latte (light)
- **Rose Pine**: Main (dark), Moon (dim), & Dawn (light)

### Custom Colors
Create your own theme file in `~/.config/bubble/themes/my-theme.toml`:

```toml
[colors]
base    = "#1e1e2e"
mantle  = "#181825"
surface = "#313244"
text    = "#cdd6f4"
accent  = "#89b4fa"
success = "#a6e3a1"
warning = "#f9e2af"
error   = "#f38ba8"
```

Switch themes inside the app via **Settings (`Ctrl+,`)** or set `theme = "my-theme"` in `~/.config/bubble/config.toml`. Changes apply immediately without restarting.

---

## Compositor Integration (Lniri)

Bubble features native support for Wayland compositor background effects including blur, contrast, and **Lniri's fluidmorphism / liquid-glass shaders** across the entire file manager window (sidebar, navigation toolbar, tabs, and file views).

### Lniri (Liquid Glass & Fluidmorphism)

To enable liquid glass in Lniri, add this window rule to `~/.config/niri/config.kdl`:

```kdl
window-rule {
    match app-id="Bubble"
    draw-border-with-background false
    background-effect {
        blur true
        xray true
        liquid-glass {
            liquidity 0.2
            refraction-strength 5.0
            power-factor 3.5
            refraction-power 1
            glow-weight 0.1
            edge-lighting 0.2
            saturation 1.1
            vibrancy 0.6
            adaptive-dim 0.1
            adaptive-boost 0.0
            physical-refraction 0.1
            lens-distortion 0.1
            fringing 0.6
        }
    }
}
```

> [!TIP]
> - Both `match app-id="Bubble"` and `match app-id="io.github.soyeb_jim285.Bubble"` match automatically. If your environment requires a specific custom Wayland `app-id`, specify `BUBBLE_APP_ID=<your-id> bubble`.
> - Transparency level and container transparency can be toggled and finely adjusted inside Bubble in **Settings (`Ctrl+,`)** under **Appearance**.

---

## Architecture

Bubble is architected into three distinct layers:

1. **Frontend (QML / QtQuick)**: Modern declarative UI with custom Lucide SVG vector iconography and fluid animations.
2. **Backend (C++20 & Qt6)**: High-performance `QAbstractItemModel` engines, multithreaded directory watchers, and background worker threads.
3. **Core Services**:
   - `CryptoEngine` & `VaultService`: AES-256-GCM authenticated encryption and session auto-locking.
   - `GioTransferWorker`: Asynchronous non-blocking file streaming via GIO.
   - `ThumbnailProvider`: Off-thread caching thumbnail generators for media and documents.

---

## Contributing

Contributions, bug reports, and suggestions are welcome!

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request
