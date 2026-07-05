# Third-party notices

## FFmpeg / ffprobe

OffloadKit bundles prebuilt `ffmpeg`/`ffprobe` binaries (used for media thumbnail
generation and technical metadata probing in the Media Browser) as external
"sidecar" executables invoked as separate subprocesses -- their code is not
linked into OffloadKit's own binary.

- Source: https://github.com/eugeneware/ffmpeg-static (release `b6.1.1`),
  upstream project: https://ffmpeg.org
- Windows build: GPL-licensed (built with `--enable-gpl`, includes libx264/
  libx265). macOS builds (arm64 and x64): LGPL-only.
- Corresponding source for the exact bundled binaries is available from the
  release above; FFmpeg's own source is available at https://ffmpeg.org and
  https://github.com/FFmpeg/FFmpeg.
- License text is fetched alongside the binaries at build time (see
  `.github/workflows/build.yml`) and ships next to them.
