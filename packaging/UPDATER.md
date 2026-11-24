# Release and updater workflows

The `release` workflow publishes one release in `moiCR/LocalCraft` from the full `CHANGELOG.md`. It then calls the separate `Windows` and `Linux` workflows. Each platform workflow also supports a manual run against an existing release tag, so a fix from the selected branch can replace that platform's assets without changing the release notes.

Configure these repository secrets in `moiCR/LocalCraft`:

- `TAURI_SIGNING_PRIVATE_KEY`: the existing private key paired with the public key embedded in the updater. Keep this key so installed Tauri versions can verify the GPUI release.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: set this only when the private key has a password.

GitHub's `GITHUB_TOKEN` publishes assets and releases in this repository. No separate release token is needed.

Windows builds the signed x64 NSIS installer. Linux builds signed x64 AppImage, Debian, and RPM packages. The Linux updater replaces AppImages atomically. Debian and RPM updates use `pkexec` with the system package manager. If authentication is canceled or installation fails, LocalCraft stays open and shows the release download page.

## Verification & Manifest Format

- Each release publishes `latest.json` with `version`, `notes`, `pub_date`, and per-platform entries (`url`, `signature`, and `size` in bytes).
- Downloads stream atomically into a private `.part` file with live byte progress and enforce both `size` (against `Content-Length` and final byte count, up to 1 GiB) and streaming minisign verification before renaming.
- Both raw minisign and base64-encoded `.sig` payloads are decoded and checked against the trusted comment (`version:<version>` / `app-version:<version>` and matching `file:<filename>` when present) to prevent rollback or package-swap attacks.

Release tags must match the version in `crates/app/Cargo.toml`, such as `1.1.0` and `v1.1.0`. For a manual platform run, choose the source branch in GitHub Actions and enter the existing release tag. The branch must keep the same app version as that tag.
