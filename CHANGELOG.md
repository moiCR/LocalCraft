### What's New in v0.6.0

- Added the Options panel with app version, GitHub access, update checks and theme controls.
- Added System, Dark and Light theme support with saved preference.
- Added the release workflow for Windows and Linux AppImage builds.
- Added an optional updater build path for Windows and Linux AppImage releases.
- Added refreshed UI components, icons and layout updates across server, Java, file and mod management views.

### Fixes

- Fixed the Home tray menu item so it navigates back to the start page.
- Kept Arch/PKGBUILD builds free of updater permissions, updater artifacts and updater UI.
- Improved anchored modal behavior so source buttons keep their layout space while transitioning.
- Reduced animation and transition work in shared navigation and button surfaces.

### Notes

- The updater is only enabled for Windows and Linux AppImage release builds.
- Arch packaging should continue to use the default Tauri configuration without updater support.
