# Server software

`SoftwareService::new()` creates the service. Providers resolve a Minecraft version
into `JarDownload` metadata; the shared downloader streams the artifact into a
unique temporary file, checks its SHA256 and size, then replaces the destination.
Paper, Vanilla and Fabric produce `server.jar`. Forge downloads and runs its
installer with an explicitly selected Java executable and records the installed
version for the instance launcher.

Call async methods on the dedicated Tokio runtime. Progress uses a `watch` channel
so a slow GPUI consumer receives the latest update without buffering every chunk.
Download byte updates are throttled to 100 ms. The result of `download` is the
final success/error signal; progress is also updated to `Complete` or `Failed`.

```rust,ignore
let (progress, mut updates) = tokio::sync::watch::channel(DownloadProgress {
    instance_id: server.id.clone(),
    stage: DownloadStage::Resolving,
    downloaded: 0,
    total: None,
});

// Paper publishes the SHA256 in its build metadata.
software_service.download(&server, None, None, &progress).await?;

// Forge needs Java to run the installer.
software_service.download(&server, Some(java.binary_path()), None, &progress).await?;
```

The project requires a trusted SHA256 before installation. Vanilla's manifest
publishes SHA1, and Fabric's generated launcher endpoint does not publish SHA256.
For those providers, first resolve with `get_jar`, obtain a trusted SHA256 for that
exact URL, then pass it as `expected_sha256` to `download`. Forge uses its Maven
`.sha256` sidecar when available and otherwise requires the same explicit hash.
A digest calculated from an unverified download is not a trusted expected hash.
If the resolved build changes, verification fails instead of installing it.

Downloads hold the instance lifecycle lock, preventing start or concurrent
installation while replacing files. Running instances cannot be updated.
Forge's installer may modify its libraries before failing; a failed installer is
reported as an error and does not record a new installed version.

Provider references:
- https://docs.papermc.io/misc/downloads-service/
- https://piston-meta.mojang.com/mc/game/version_manifest_v2.json
- https://github.com/FabricMC/fabric-meta
- https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json

Playit and Tauri are not part of this service.
