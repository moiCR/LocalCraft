# LocalCraft (Local Minecraft Server Manager)

## Project Overview

- LocalCraft is a local, bare-metal Minecraft server manager and launcher built with Rust and GPUI.
- Purpose: Manage local JVM runtimes, configure server instances (Paper, Purpur, Fabric, Forge, Vanilla), monitor server resources, pipe interactive terminal I/O, and manage files without Docker or container overhead.
- Core philosophy: Low resource overhead, rock-solid process supervision, reactive and fluid UI, zero blocking operations on the main thread.

## Rust & Safety Standards

- Write idiomatic, robust Rust.
- **Strictly prohibit panics**: Never use `unwrap()`, `expect()`, or indexing that can panic in production code.
- Propagate all errors using `Result<T, E>` and `?`. Use custom error enums with `thiserror` or context with `anyhow`.
- Avoid unnecessary cloning, string reallocations, and large heap boxing.
- Functions must have a single, well-defined responsibility.
- Code style:
  - Run `cargo fmt` and keep `cargo clippy --workspace -- -D warnings` completely clean.
  - Do NOT write comments explaining "what" code does; write comments strictly explaining "why" or omit them.
  - If code outputs user-facing messages or internal log entries, always write them in English.

## Runtime Architecture & Task Separation

- **Runtime Isolation (GPUI vs. Tokio)**:
  - The UI runs on GPUI's event loop (`smol`/`async-task`).
  - Heavy I/O, Java process supervision, network downloads, file system indexing, and hardware polling MUST run inside a dedicated multi-threaded Tokio runtime.
  - Never execute blocking operations, child process piping, or disk file operations inside GPUI worker threads (`Worker-*`, `cx.spawn`).
- **Bridge Tokio to GPUI via Channels**:
  - Background Tokio workers must communicate state updates (server status, CPU/RAM stats, console lines, download progress) to GPUI views using bounded asynchronous channels (`tokio::sync::mpsc` or `broadcast`).
  - Ingest channel events inside GPUI using `cx.spawn` and update local models via `window.update()` or `entity.update()`.

## Subprocess & Java Management

- **Process Supervision**:
  - Spawn Minecraft server processes using `tokio::process::Command` configured with piped `stdin`, `stdout`, and `stderr`.
  - Never poll processes synchronously. Supervise child processes via asynchronous loops monitoring stdout/stderr streams.
  - Store child PIDs and process handles safely. Ensure orphan cleanup: if the application exits, trigger clean shutdown cascades on all managed child instances.
- **Graceful Server Shutdown Protocol**:
  - Always attempt graceful shutdown first: send `stop\n` (or compositor equivalent) to child `stdin`.
  - Wait for process termination with a configurable timeout (e.g., 30s).
  - Only escalate to `SIGTERM` and finally `SIGKILL` if the timeout expires without the JVM exiting.
- **Console & Log Streaming (Zero-Stutter Constraint)**:
  - High-volume server logs can emit thousands of lines per second during startup/crashes.
  - **Do NOT send raw single-line UI notifications per log line.**
  - Batch incoming console lines across a throttle window (e.g., flush to UI every 50ms–100ms).
  - Terminal buffers in memory MUST be circular / ring buffers capped at a strict maximum line count (e.g., 2,000–5,000 lines) to prevent unbounded memory growth.

## Java Runtime Acquisition & File Integrity

- **Non-Blocking Downloads & Extraction**:
  - Java runtime downloads (Adoptium/Zulu API) and server jar downloads must be executed via `reqwest` streaming on Tokio.
  - Stream progress chunks through channels to update UI progress bars without blocking.
  - Archive extraction (`tar.gz`, `zip`) must run on Tokio blocking thread pools (`tokio::task::spawn_blocking`).
  - Always verify SHA256 checksums before extracting or running downloaded binaries.
- **Cross-Platform Executable Permissions**:
  - When extracting Java runtimes on Linux/macOS, explicitly restore executable permissions (`chmod +x` / `std::os::unix::fs::PermissionsExt`) on `bin/java` and related binaries.
- **Atomic File Modifications**:
  - All modifications to configuration files (`server.properties`, `eula.txt`, `spigot.yml`, instance manifests) must be written atomically (write to a `.tmp` file, then rename/replace) to eliminate file corruption on crash or force-close.

## GPUI Rules & UI Performance

- **`render()` Must Be Pure & Declarative**:
  - `render()` is strictly for laying out elements.
  - PROHIBITED inside `render()`: reading disk files, writing configs, triggering server starts/stops, or spawning Tokio tasks.
  - All user actions must dispatch through callbacks (`on_click`, event handlers) that mutate application entities outside the render pass.
- **List & Console Reconciliation**:
  - In dynamic lists (instances, player lists, plugin tables, console rows), every iterated child element MUST have a unique, stable ID:
    ```rust
    .id(("console-line", line_index as u32))
    ```
  - For server console rendering, use virtualized / windowed rendering so only visible lines are converted into GPUI elements.
- **Resource Monitoring Throttling**:
  - Polling server CPU and memory usage (via `/proc` or `sysinfo`) must be rate-limited (interval >= 1.0s). Never sample hardware metrics per-frame.

## Codebase Structure

```
crates/
├── app/               Main GPUI application, window setup, routing, views, modal state
├── ui/                Shared GPUI design system (buttons, inputs, tables, tabs, modal dialogs)
├── core/              Core domain logic: Instance manifests, server types, configs, states
├── runner/            Process supervisor: JVM launcher, stdin/stdout stream pipes, graceful killer
├── java/              Java runtime manager: discovery, Adoptium API fetcher, download/unpack
└── assets/            Embedded icons (SVGs) and local presets
```

## Modular Views & Widgets

Do not cram entire pages or complex dashboards into single view files:
- Each page module must split its controls into sub-widgets (e.g., `views/instance_view.rs` uses `widgets/instance/console_widget.rs`, `widgets/instance/performance_widget.rs`, `widgets/instance/settings_widget.rs`).
- Global state (active instance list, system Java installations, global config) belongs in an `AppState` entity or singleton accessible via context.

## Notes & MCP Rules

- Consult `codebase-memory-mcp` before exploring repository architecture.
- Verify existing code before writing modifications.
- If MCP graph coverage is insufficient, inspect files directly and state the limitation.
- Use caveman skill.