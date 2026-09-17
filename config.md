# Bubble Configuration & Ecosystem Guide

Comprehensive guide for configuring **Bubble**, customizing themes, and integrating with Wayland compositors (such as **Lniri / Niri**, **Hyprland**, and **KDE Plasma**) for an optical liquid-glass aesthetic.

---

## Table of Contents

1. [Configuration Overview](#configuration-overview)
2. [File Locations](#file-locations)
3. [Configuration Reference (`config.toml`)](#configuration-reference-configtoml)
   - [General (`[general]`)](#general-general)
   - [Sidebar (`[sidebar]`)](#sidebar-sidebar)
   - [Appearance (`[appearance]`)](#appearance-appearance)
   - [Animations (`[animations]`)](#animations-animations)
   - [Window Controls (`[window]`)](#window-controls-window)
   - [Detailed View Columns (`[list_view]`)](#detailed-view-columns-list_view)
   - [Miller Columns (`[miller_view]`)](#miller-columns-miller_view)
   - [Bookmarks (`[bookmarks]`)](#bookmarks-bookmarks)
   - [Custom Context Menu Actions (`[context_menu]`)](#custom-context-menu-actions-context_menu)
   - [Keyboard Shortcuts (`[shortcuts]`)](#keyboard-shortcuts-shortcuts)
4. [Liquid Glass Ecosystem & Compositor Integration](#liquid-glass-ecosystem--compositor-integration)
   - [Lniri / Niri (with kwin-glass)](#lniri--niri-with-kwin-glass)
   - [Hyprland](#hyprland)
   - [KDE Plasma / KWin](#kde-plasma--kwin)
5. [The Liquid Themes: `liquid-light` & `liquid-dark`](#the-liquid-themes-liquid-light--liquid-dark)
6. [Creating Custom Themes](#creating-custom-themes)

---

## Configuration Overview

Bubble is designed to be instantly configurable both via its graphical **Settings Panel** (`Ctrl+,` or the gear icon in the top-right toolbar) and directly via human-readable TOML files.

When Bubble is running, modifications saved in `config.toml` are automatically detected and reloaded live without needing to restart the application.

---

## File Locations

| Path | Purpose |
| :--- | :--- |
| `~/.config/bubble/config.toml` | User configuration file |
| `~/.config/bubble/config.toml.sample` | Auto-generated reference template containing all default keys and comments |
| `~/.config/bubble/themes/` | User-defined custom TOML themes |
| `/usr/share/bubble/themes/` | System-wide built-in themes |
| `~/.config/bubble/folder_sort.json` | Per-folder sort memory state |

> [!NOTE]
> When settings are updated from within Bubble's graphical UI, `config.toml` is written cleanly. The adjacent `config.toml.sample` is automatically updated on every launch to retain a complete, documented reference template.

---

## Configuration Reference (`config.toml`)

Below is a complete reference of all available tables and keys in `config.toml`.

### General (`[general]`)

```toml
[general]
# Active colour theme (theme name matching a TOML file without .toml).
# If omitted or empty, Bubble follows system light/dark preference.
theme = "liquid-light"

# Themes used when toggling Dark Mode in Settings or via shortcut.
light_theme = "liquid-light"
dark_theme = "liquid-dark"

# Icon theme for files and folders (directory name under /usr/share/icons or ~/.icons).
icon_theme = "Adwaita"

# UI font family. Empty string uses the system default UI font.
font_family = ""

# Default view for newly opened tabs: "grid" | "detailed" | "miller"
default_view = "grid"

# Show hidden (dot) files by default: true | false
show_hidden = false

# Right-clicking the breadcrumb address bar switches to text edit mode (Ctrl+L behavior)
right_click_to_edit_path = true

# Primary sort order: "name" | "size" | "modified" | "type"
sort_by = "name"
sort_ascending = true

# Store and remember individual sort preferences per folder
remember_sort_per_folder = true

# Check for required CLI helpers (gvfs, ffmpeg, bat, pdftoppm...) on startup
dependency_startup_check = true
```

### Sidebar (`[sidebar]`)

```toml
[sidebar]
# Sidebar placement: "left" | "right"
position = "left"

# Sidebar width in pixels
width = 200

# Initial sidebar visibility: true | false
visible = true

# Built-in quick access items to hide.
# Possible entries: "Home", "Recents", "Trash", "Network", "Pictures", "Downloads"
hidden_quick_access = []
```

### Appearance (`[appearance]`)

```toml
[appearance]
# Corner radii in pixels for components and cards
radius_small = 4
radius_medium = 8
radius_large = 14

# Master transparency toggle and level (0.0 = fully transparent, 1.0 = fully opaque)
transparency_enabled = true
transparency_level = 1.0
```

### Animations (`[animations]`)

```toml
[animations]
enabled = true

# Transition durations in milliseconds
anim_duration_fast = 100
anim_duration = 200
anim_duration_slow = 350

# Easing curves (Qt easing curve names):
# "Linear" | "InCubic" | "OutCubic" | "InOutCubic" | "OutBack" | "InOutQuad" | "OutQuad" | "OutExpo" | "InOutExpo" | "Bezier"
anim_curve_enter = "OutCubic"
anim_curve_exit = "InCubic"
anim_curve_transition = "Bezier"
```

### Window Controls (`[window]`)

```toml
[window]
# Render client-side minimize, maximize, and close buttons.
# When omitted, Bubble automatically detects if the compositor provides server-side decorations.
# show_controls = false

# Titlebar button ordering (":" separates left side from right side)
button_layout = ":minimize,maximize,close"
```

### Detailed View Columns (`[list_view]`)

```toml
[list_view]
# Column keys in display order ("name" is always present)
# Available: "name", "size", "modified", "type", "permissions", "owner", "group", "created", "accessed", "extension", "mime", "git", "symlink"
columns = ["name", "size", "modified", "type"]

# Column widths in pixels
column_widths = { size = 110, modified = 140, type = 80 }
```

### Miller Columns (`[miller_view]`)

```toml
[miller_view]
# Relative widths of parent and current columns (floating fraction between 0.12 and 0.70)
parent_fraction = 0.2
current_fraction = 0.5
```

### Bookmarks (`[bookmarks]`)

```toml
[bookmarks]
# Custom sidebar bookmark directory paths (defaults to standard XDG user folders)
paths = ["~/Documents", "~/Downloads", "~/Pictures", "~/Projects"]

# Custom bookmark display aliases
names = { "~/Projects" = "Work" }
```

### Custom Context Menu Actions (`[context_menu]`)

Add custom actions to the right-click menu for selected files and directories.

```toml
[[context_menu.actions]]
name = "Open in VS Code"
command = "code %f"
types = ["dir", "text/*"]

[[context_menu.actions]]
name = "Optimize PNG"
command = "oxipng -o 4 %f"
types = ["png"]
```

- `%f` or `%u`: Path or URL of the selected item.
- `types`: `"*"` (all files), `"dir"` (directories only), an extension (e.g. `"png"`), or a MIME type pattern (e.g. `"image/*"`).

### Keyboard Shortcuts (`[shortcuts]`)

Override any default key sequence using Qt shortcut syntax:

```toml
[shortcuts]
open = "Return"
back = "Alt+Left"
forward = "Alt+Right"
parent = "Alt+Up"
home = "Alt+Home"
refresh = "F5"
new_tab = "Ctrl+T"
new_window = "Ctrl+Alt+N"
close_tab = "Ctrl+W"
next_tab = "Ctrl+Tab"
previous_tab = "Ctrl+Shift+Tab"
reopen_tab = "Ctrl+Shift+T"
open_in_new_tab = "Ctrl+Return"
open_in_split = "Ctrl+Shift+Return"
copy = "Ctrl+C"
cut = "Ctrl+X"
paste = "Ctrl+V"
rename = "F2"
new_folder = "Ctrl+Shift+N"
new_file = "Ctrl+N"
trash = "Delete"
permanent_delete = "Shift+Delete"
toggle_hidden = "Ctrl+H"
toggle_transparency = "Ctrl+Shift+B"
quick_preview = "Space"
search = "Ctrl+F"
context_menu = "Shift+F10"
open_terminal = "Ctrl+Alt+T"
properties = "Alt+Return"
path_bar = "Ctrl+L"
vault_lock = "Ctrl+Shift+L"
toggle_sidebar = "F9"
split_view = "F3"
focus_next_pane = "F6"
focus_previous_pane = "Shift+F6"
focus_left_pane = "Ctrl+Alt+Left"
focus_right_pane = "Ctrl+Alt+Right"
grid_view = "Ctrl+1"
miller_view = "Ctrl+2"
detailed_view = "Ctrl+3"
select_all = "Ctrl+A"
undo = "Ctrl+Z"
redo = "Ctrl+Shift+Z"
settings = "Ctrl+,"
edit_config = "Ctrl+Shift+,"
keyboard_shortcuts = "Ctrl+?"
```

---

## Liquid Glass Ecosystem & Compositor Integration

Bubble features a dedicated **Liquid Glass Layout Engine** calibrated to match the optical aesthetics of modern frosted glass desktop environments (such as KDE Dolphin under Lniri's `kwin-glass` shader stack).

### Architectural Behavior:
- **Zero-Opacity Framing**: The sidebar, toolbar, and status bar have 0% background opacity (`opacity = 0.0`), eliminating harsh cutouts, divider lines, and inverse corner artifacts.
- **Floating Elevated Card Container**: The active file view sits in an elevated floating card with `14px` border radius, a `1px` subtle ambient glass rim border, and `30%` view opacity.
- **Dual-Pane Split View (`F3`)**: In split-view mode, Bubble renders twin floating frosted cards side-by-side with independent focus borders and clear visual separation.

### Lniri / Niri (with kwin-glass)

In `~/.config/niri/config.kdl`, add window rules for Bubble to activate hardware blur and rounded window clipping:

```kdl
window-rule {
    match app-id="io.github.tattvaorg.Bubble"
    match app-id="bubble"
    match app-id="Bubble"
    match title="Bubble"
    match title="Bubble Settings"

    clip-to-geometry true
    geometry-corner-radius 14
    blur true
}
```

Reload Lniri config dynamically:
```bash
niri msg action load-config-file
```

### Hyprland

In `~/.config/hypr/hyprland.conf`:

```ini
windowrulev2 = blur, class:^(bubble)$
windowrulev2 = rounding 14, class:^(bubble)$
windowrulev2 = noshadow, class:^(bubble)$
```

### KDE Plasma / KWin

In KDE Plasma, ensure **Background Contrast** and **Blur** desktop effects are enabled in *System Settings -> Workspace Behavior -> Desktop Effects*. Window rules can be added via *System Settings -> Window Management -> Window Rules* targeting `bubble` to enforce background blur.

---

## The Liquid Themes: `liquid-light` & `liquid-dark`

Bubble provides two official liquid themes pre-calibrated for optical glass environments:

1. **`liquid-light`**: High-key translucent glass theme with subtle specular borders and clean typography designed for light wallpapers.
2. **`liquid-dark`**: Deep ambient obsidian glass theme with luminous selections and soft rim highlights designed for dark and colorful wallpapers.

### Exact Optical Parameters:

```toml
[glass]
sidebar_opacity = 0.0
toolbar_opacity = 0.0
content_opacity = 0.30
noise_enabled = false
noise_opacity = 0.0
border_width = 1
```

Switch between them dynamically in `~/.config/bubble/config.toml` or using the Dark Mode switch in the Settings dialog (`Ctrl+,`).

---

## Creating Custom Themes

To create a custom theme, place a `.toml` file inside `~/.config/bubble/themes/<name>.toml`:

```toml
[theme]
name = "my-custom-glass"
author = "You"
description = "Custom Frosted Glass Theme"

[colors]
base = "#181825"
text = "#cdd6f4"
subtext0 = "#a6adc8"
surface0 = "#313244"
surface1 = "#45475a"
blue = "#89b4fa"
red = "#f38ba8"
green = "#a6e3a1"
yellow = "#f9e2af"
lavender = "#b4befe"

[glass]
# Set sidebar and toolbar to 0.0 to activate the floating card glass layout
sidebar_opacity = 0.0
toolbar_opacity = 0.0
content_opacity = 0.35
noise_enabled = false
noise_opacity = 0.0
border_color = "#ffffff"
border_opacity = 0.12
border_width = 1
```

When `sidebar_opacity` and `toolbar_opacity` are set to `0.0`, Bubble's QML engine automatically activates floating elevated card mode. If set to standard opacity (e.g. `0.9` or `1.0`), Bubble smoothly falls back to the classic unified panel layout.
