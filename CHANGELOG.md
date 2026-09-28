All notable changes to LocalCraft are documented in this file.

## v1.0.0 — GPUI migration

### Migration
- Replaced the Tauri/Vue application with a native Rust/GPUI desktop application and Tokio services.
- Rebuilt the app shell with a custom title bar, collapsible sidebar, and dark and light themes.

### Features
- Create and manage isolated Minecraft servers for Paper, Purpur, Fabric, Forge, and Vanilla.
- Configure Minecraft and Java versions, memory, port, and EULA acceptance during server creation.
- Automatically select the latest compatible server build; removed the manual build/loader selector.
- Start and stop servers, send console commands, and view streamed server output.
- Browse server files, create folders, rename and delete entries, edit text files, and upload files.
- Discover and install Java runtimes, including automatic installation when a server needs a runtime.
- Connect and manage the Playit agent, including its claim flow, status, and logs.
- Save preferences for theme, new server defaults, sidebar collapse, and console display.
- Check for signed updates at startup and install them from Settings or the title bar on Windows x86_64.
- Let existing Tauri installations upgrade to GPUI through the existing release channel.

### Improvements
- Stream software downloads and show progress. Verify SHA-256 when the provider publishes it; allow downloads without a provider checksum.
- Verify updater signatures and bind each signed installer to its declared version before installation.
- Add Windows NSIS packaging and a release workflow that publishes a Tauri-compatible update manifest.

### Not yet ported from Tauri
- Modrinth mod management, whitelist management, and system-tray actions are not available in the GPUI app yet.
