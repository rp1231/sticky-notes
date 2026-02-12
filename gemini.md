# Project Context: Sticky Notes (Tauri v2)

## Configuration Research (docs.rs)

Based on documentation for `tauri` (v2.10.2) and `tauri-utils` (v2.8.2):

### tauri.conf.json Recommended Settings for Sticky Notes

To achieve the "sticky note" look and feel (borderless, always on top, tray-focused):

#### Window Configuration (`app.windows`)
- `decorations`: `false` - Removes window borders and title bar for a clean sticky note look.
- `transparent`: `true` - Allows for custom shapes or rounded corners (requires `macosPrivateApi` in `tauri` config for macOS).
- `alwaysOnTop`: `true` - Ensures notes stay above other windows.
- `skipTaskbar`: `true` - Prevents the window from cluttering the taskbar; use the tray for management.
- `resizable`: `true` - Allows users to adjust note size.
- `shadow`: `true` - Provides visual depth.

#### Tray Icon Configuration (`app.tray_icon`)
- `iconPath`: Path to the icon file (e.g., `"icons/icon.png"`).
- `showMenuOnLeftClick`: `true` - (V2 recommendation) Determines if the menu appears on left-click (Linux support varies).
- `id`: Optional unique identifier for the tray icon (defaults to `"main"`).

### Important Structs
- `tauri::Config`: Main configuration entry point.
- `tauri_utils::config::WindowConfig`: Detailed window properties.
- `tauri_utils::config::TrayIconConfig`: System tray properties.
