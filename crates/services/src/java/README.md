# Java runtimes

`JavaService::new()` creates the registry. `load().await` restores managed runtimes
from `LocalCraft/java/<major>/installation.json`; AppState calls it at startup.
`adoptium.rs` implements the `Java` provider trait, returning archive metadata with
Adoptium's published SHA256. `download.rs` streams the archive and validates its
size and digest. `extract.rs` runs on `tokio::task::spawn_blocking`, handles ZIP and
TAR.GZ, rejects escaping paths/links and restores Unix executable permissions.

All async service calls belong on the dedicated Tokio runtime. `watch` progress
retains the latest update and throttles downloaded-byte updates to 100 ms.

```rust,ignore
let service = JavaService::new();
let (progress, updates) = tokio::sync::watch::channel(JavaProgress::new(21));
let java = service.download(21, &progress).await?;
server.start(java.binary_path()).await?;
```

`install_for_instance(&server, &progress).await` reads the instance's `java_version`,
reuses or downloads that major version and atomically saves `java_path.txt` in the
instance directory. It returns `JavaInstallation`, whose executable path can also
be passed to `SoftwareService::download` for Forge installation. Java is shared
between instances. No Tauri handles or commands are required.

`delete(version).await` removes a managed runtime only when no saved instance
references that major version. Loading, installation and deletion are serialized
within the service. Extraction happens in a private staging directory; only a
complete runtime with its manifest becomes visible in the cache. Failed
operations remove their staging directory; interrupted operations may leave a
hidden staging directory which is ignored on reload.

Legacy directories without `installation.json` are reported as incomplete,
not silently trusted or migrated. Remove an unused legacy version before
reinstalling it. This service manages Adoptium JREs; it does not discover Java
installations elsewhere on the system.

API reference: https://github.com/adoptium/api.adoptium.net/blob/main/docs/cookbook.adoc
