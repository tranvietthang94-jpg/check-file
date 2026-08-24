# OffloadKit

**Local-first media offload and verification tool for filmmakers, DITs and video editors.**

OffloadKit helps copy camera media safely from a source drive to one or more backup destinations, verify the result with checksums, preserve an audit trail, and repair a verified file only after explicit confirmation.

> Personal, offline-first software inspired by professional offload workflows. It is not affiliated with or a copy of OffShoot. It does not include license/DRM, cloud accounts, telemetry or network transfer features.

## Current release

**v0.1.3 — Phase 12: Automatic Repair Planner**

This release includes the Phase 12 safety hardening and a real-file repair smoke test using Adobe Premiere Pro Auto-Save media.

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

1. Source and destination paths are validated before filesystem operations.
2. Symlinks, junctions and Windows reparse points are rejected in protected paths.
3. Copies are staged before being moved into the final destination name.
4. Checksums are verified before a file is trusted or deleted from Source.
5. Repair never happens silently and never deletes the corrupt evidence automatically.
6. Transfer logs and MHL files use atomic writes.
7. Auto Eject is blocked by failed, missing, broken-media or move-delete-failed files.

No software can remove every risk from failing hardware or an actively changing filesystem. Keep the original camera media until the verified backups have been reviewed.

## Verification status

Latest local verification for v0.1.3:

```text
Frontend build:   PASS
Playwright E2E:   25/25 PASS
Rust tests:       176/176 PASS
npm audit:        0 vulnerabilities
Real-file repair smoke test: PASS
```

The real-file smoke test used a copy of an Adobe Premiere Pro Auto-Save project, intentionally corrupted the copy, repaired it from the Auto-Save source, verified the repaired result, checked `.ofkit-corrupt` evidence, and removed only the temporary smoke-test folder.

## Platforms and installers

GitHub Actions builds:

- Windows x64: `.exe` and `.msi`
- macOS Intel: `.dmg` and app archive
- macOS Apple Silicon: `.dmg` and app archive

The application is currently intended for personal/local use. macOS signing and notarization may require additional release configuration on the maintainer's Apple account.

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
