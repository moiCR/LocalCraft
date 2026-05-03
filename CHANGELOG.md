## v0.2.2

### What's New

### Fixes

- Fixed the updater relaunch flow by registering the Tauri process plugin and granting the restart capability.
- Fixed the navbar updater action so it only appears when an update is actually pending.
- Fixed the navbar shared background indicator so it follows the active item after window resizing.
- Removed a duplicate updater plugin registration from the Tauri builder.
- Fixed the app binary version metadata so it no longer reports v0.1.1 when the release is published as v0.2.0.

### Notes

- Installed updates now relaunch LocalCraft automatically after installation.
- The updater fix still needs to be verified through a published release.
- Windows releases now use this changelog as the release body.
