# OffloadKit

**Local-first media offload and verification tool for filmmakers, DITs and video editors.**

OffloadKit helps copy camera media safely from a source drive to one or more backup destinations, verify the result with checksums, preserve an audit trail, and repair a verified file only after explicit confirmation.

> Personal, offline-first software inspired by professional offload workflows. It is not affiliated with or a copy of OffShoot. It does not include license/DRM, cloud accounts, telemetry or network transfer features.

## Current release

**v0.1.4 — Phase 13: Windows Explorer Workflows**

This release adds opt-in Windows Explorer actions for selecting endpoints and performing verified OffloadKit copies, plus fail-closed pathname revalidation immediately before sensitive copy mutations.

Download installers from the [Releases](https://github.com/tranvietthang94-jpg/check-file/releases) page.

## What it does

- Copy one source to multiple destinations in **Parallel** mode.
- Copy through destinations in **Cascade** mode.
- Verification modes: Transfer, Source, and Source + Destination.
- Checksums: XXH64, XXH3, XXH128, MD5, SHA-1 and C4.
- Duplicate detection and safe resume after an interrupted transfer.
- Move workflow that removes a source file only after successful verification.
- MHL generation and verification.
- Transfer logs and HTML reports.
- Recent source/destination folders and persistent preferences.
- Auto Source for removable drives matching a wildcard pattern.
- Auto Eject only after every expected job in a group completes cleanly.
- Broken Media Detection for zero-byte source files.
- Organize templates for folders, filenames, dates and counters.
- Media Browser with metadata and thumbnails when bundled FFmpeg/FFprobe is available.
- Windows Explorer integration (opt-in from Preferences):
  - Set selected files or folders as Source, or a folder as Destination.
  - Copy selections through the native Windows File Drop clipboard and Paste them into a selected folder with the current verification settings.
  - Preserve selected-path layout, deduplicate repeated paths and prune nested selections.
  - Reject filesystem links/reparse points, overlapping Source/Destination paths and destinations that fail a write probe.
- macOS Finder Quick Actions (opt-in from Preferences):
  - Install exactly four workflows in `~/Library/Services`: `OffloadKit Set Source.workflow`, `OffloadKit Set Destination.workflow`, `OffloadKit Copy.workflow` and `OffloadKit Paste.workflow`.
  - Use the native macOS file-URL pasteboard while preserving the same selected-path copy and verification pipeline as Windows.
  - Keep Paste in copy mode: neither source-removal option is enabled by a Finder request.
  - Require the production app at `/Applications/OffloadKit.app`; an app launched elsewhere reports guidance instead of installing workflows with a stale executable path.
  - Reject malformed actions, missing paths, links, overlapping endpoints and destinations that fail the write probe.
- Automatic Repair Planner:
  - Finds candidate copies from Source and sibling Destinations.
  - Accepts only a full checksum match against the MHL.
  - Rejects traversal paths, symlinks and Windows junction/reparse points.
  - Shows a repair plan before any mutation.
  - Requires explicit confirmation.
  - Keeps the replaced file as `.ofkit-corrupt` evidence.
  - Uses verified staging and re-verifies the repaired destination.

## Safety model

OffloadKit is designed to fail closed around destructive operations:

1. Source and destination path components are revalidated immediately before selected-file opens, staging operations, final placement and verified source removal.
2. Symlinks, junctions and Windows reparse points are rejected in protected paths.
3. Copies are staged before being moved into the final destination name.
4. Checksums are verified before a file is trusted or deleted from Source.
5. Repair never happens silently and never deletes the corrupt evidence automatically.
6. Transfer logs and MHL files use atomic writes.
7. Auto Eject is blocked by failed, missing, broken-media or move-delete-failed files.

These pathname checks and filesystem mutations are separate operating-system operations, so a small pathname time-of-check/time-of-use (TOCTOU) window remains on an actively changing filesystem. Revalidation narrows that exposure; it does not eliminate it or make pathname access atomic. No software can remove every risk from failing hardware or a hostile filesystem. Keep the original camera media until the verified backups have been reviewed.

## Verification status

Latest local verification for v0.1.4:

```text
Frontend build:                         PASS
Playwright E2E:                         34/34 PASS
Rust default suite:                     214 PASS, 1 ignored
Real Windows clipboard/copy smoke:      1/1 PASS
Clippy (all targets, warnings denied):  PASS
Rust formatting check:                  PASS
npm audit:                              0 vulnerabilities
Real-file repair smoke test:            PASS
```

The real-file repair smoke test used a copy of an Adobe Premiere Pro Auto-Save project, intentionally corrupted the copy, repaired it from the Auto-Save source, verified the repaired result, checked `.ofkit-corrupt` evidence, and removed only the temporary repair folder. The separately invoked Phase 13 smoke exercised the native Windows File Drop clipboard and a real selected-path copy; it preserved both Source files, copied only the selected files, and left its printed temporary smoke tree available for inspection.

## Platforms and installers

GitHub Actions builds:

- Windows x64: `.exe` and `.msi`
- macOS Intel: `.dmg` and app archive
- macOS Apple Silicon: `.dmg` and app archive

The application is currently intended for personal/local use. macOS signing and notarization may require additional release configuration on the maintainer's Apple account.

### Enable Finder Quick Actions on macOS

1. Move `OffloadKit.app` to `/Applications/OffloadKit.app` and launch it from there.
2. Open **Preferences > General** and enable **Finder Quick Actions**.
3. In Finder, select files or folders and use **Quick Actions** (or **Services**) to choose one of the four OffloadKit actions.

Disabling the preference removes only the four OffloadKit workflow bundles. It leaves other services untouched. These local builds are not claimed to be Apple-signed or notarized; macOS may therefore show its normal security confirmation before first launch.

## Development

### Requirements

- Node.js LTS
- Rust stable
- Tauri 2 prerequisites for your operating system
- Windows 11, macOS, or another supported desktop environment

### Install dependencies

```bash
npm ci
```

### Run in development

```bash
npm run tauri dev
```

### Run checks

```bash
npm run build
npm run test:e2e
cargo test --manifest-path src-tauri/Cargo.toml
cargo test phase13b_real_windows_file_drop_and_selected_copy_smoke --manifest-path src-tauri/Cargo.toml -- --ignored --nocapture
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
npm audit --audit-level=high
```

### Build an installer locally

```bash
npm run tauri build
```

The GitHub Actions workflow in `.github/workflows/build.yml` builds release installers for Windows and macOS when a release tag is pushed.

## Scope and exclusions

Included: local file copying, verification, reports, MHL interoperability, repair planning and local workflow automation.

Excluded by design: license or DRM bypass, cloud accounts, network transfer, telemetry, subscription logic and proprietary source code from other applications.

## Roadmap

Future work will be selected based on real-world use rather than implementing every professional feature at once. Possible candidates include stronger checkpoint workflows, deeper ASC MHL chain-of-custody support, and additional local automation.

## License

This repository does not currently declare a public open-source license. All rights are reserved unless a separate license file or repository notice says otherwise.
