## v0.2.1

### What's New

- Added the Tauri updater plugin integration.
- Added an in-navbar update action that appears only when an update is available.
- Added update download and install handling with progress feedback.
- Added updater release artifact generation for app builds.

### Fixes

- Fixed the navbar shared background indicator so it follows the active item after window resizing.
- Removed a duplicate updater plugin registration from the Tauri builder.
- Fixed the app binary version metadata so it no longer reports v0.1.1 when the release is published as v0.2.0.

### Notes

- Installed updates require restarting LocalCraft to apply.
- Windows releases now use this changelog as the release body.
