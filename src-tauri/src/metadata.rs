use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::SystemTime;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Serialize};

/// Coarse classification used to decide which extraction strategy applies.
/// Deliberately extension-based rather than sniffing file contents -- camera
/// cards don't lie about their own file extensions, and content sniffing
/// would add real complexity for no practical benefit here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaKind {
    Video,
    Audio,
    Photo,
    Other,
}

pub fn classify(path: &Path) -> MediaKind {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "mp4" | "mov" | "mxf" | "avi" | "m4v" | "mts" | "m2ts" | "braw" => MediaKind::Video,
        "wav" | "mp3" | "aac" | "flac" | "m4a" => MediaKind::Audio,
        "jpg" | "jpeg" | "png" | "heic" | "heif" | "tif" | "tiff" | "arw" | "cr2" | "cr3"
        | "nef" | "raf" | "dng" | "orf" | "rw2" => MediaKind::Photo,
        _ => MediaKind::Other,
    }
}

/// Technical metadata for one file. Every field is optional because which
/// ones apply depends on `MediaKind`, and any single field can fail to parse
/// without the rest -- a corrupt duration shouldn't hide a readable codec.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaMetadata {
    pub codec: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<f64>,
    pub duration_secs: Option<f64>,
    pub timecode: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub camera_model: Option<String>,
    pub lens: Option<String>,
    pub iso: Option<String>,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub focal_length: Option<String>,
}

impl MediaMetadata {
    fn is_empty(&self) -> bool {
        self.codec.is_none()
            && self.width.is_none()
            && self.height.is_none()
            && self.frame_rate.is_none()
            && self.duration_secs.is_none()
            && self.timecode.is_none()
            && self.sample_rate.is_none()
            && self.channels.is_none()
            && self.camera_model.is_none()
            && self.lens.is_none()
            && self.iso.is_none()
            && self.aperture.is_none()
            && self.shutter_speed.is_none()
            && self.focal_length.is_none()
    }
}

/// The directory a bundled sidecar binary would live in -- mirrors
/// `tauri_plugin_shell`'s own `relative_command_path` (verified against that
/// crate's real source rather than assumed, since this project doesn't
/// otherwise depend on it just for this one path computation): next to
/// `current_exe()`, adjusted up one level out of `deps/` when running under
/// `cargo test`.
fn sidecar_dir() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    if exe_dir.ends_with("deps") {
        Some(exe_dir.parent().unwrap_or(&exe_dir).to_path_buf())
    } else {
        Some(exe_dir)
    }
}

/// The path a bundled `<name>` sidecar (see `externalBin` in
/// `tauri.conf.json`) would be installed at, if the file actually exists
/// there -- a packaged build ships one, a plain `cargo build`/`cargo test`
/// run generally doesn't.
fn sidecar_path(name: &str) -> Option<PathBuf> {
    let mut path = sidecar_dir()?.join(name);
    if cfg!(windows) {
        path.set_extension("exe");
    }
    path.exists().then_some(path)
}

fn tool_responds(program: &Path) -> bool {
    Command::new(program)
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Resolves the command to invoke for `name` ("ffmpeg"/"ffprobe"): prefers
/// the bundled sidecar next to the running executable (so a packaged
/// install works with no external dependency), falling back to a bare PATH
/// lookup so a dev machine with ffmpeg already installed -- but no sidecar
/// fetched -- keeps working exactly as before.
fn resolve_tool(name: &str) -> Option<PathBuf> {
    if let Some(sidecar) = sidecar_path(name) {
        if tool_responds(&sidecar) {
            return Some(sidecar);
        }
    }
    let bare = PathBuf::from(name);
    tool_responds(&bare).then_some(bare)
}

fn ffprobe_path() -> Option<PathBuf> {
    static RESOLVED: OnceLock<Option<PathBuf>> = OnceLock::new();
    RESOLVED.get_or_init(|| resolve_tool("ffprobe")).clone()
}

fn ffmpeg_path() -> Option<PathBuf> {
    static RESOLVED: OnceLock<Option<PathBuf>> = OnceLock::new();
    RESOLVED.get_or_init(|| resolve_tool("ffmpeg")).clone()
}

#[derive(Deserialize)]
struct FfprobeOutput {
    #[serde(default)]
    streams: Vec<FfprobeStream>,
    format: Option<FfprobeFormat>,
}

#[derive(Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    sample_rate: Option<String>,
    channels: Option<u32>,
    duration: Option<String>,
    #[serde(default)]
    tags: HashMap<String, String>,
}

#[derive(Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}

fn parse_frame_rate(raw: &str) -> Option<f64> {
    let (num, den) = raw.split_once('/')?;
    let num: f64 = num.parse().ok()?;
    let den: f64 = den.parse().ok()?;
    (den != 0.0).then_some(num / den)
}

/// Probes a video/audio file with `ffprobe`. Returns `None` if ffprobe isn't
/// installed, the file can't be parsed, or nothing useful came back --
/// callers treat that as "no metadata available", not an error worth
/// blocking on (this mirrors the copy engine's own approach to bad files).
pub fn probe_with_ffprobe(path: &Path) -> Option<MediaMetadata> {
    let ffprobe = ffprobe_path()?;
    let output = Command::new(ffprobe)
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let parsed: FfprobeOutput = serde_json::from_slice(&output.stdout).ok()?;

    let mut meta = MediaMetadata {
        duration_secs: parsed
            .format
            .as_ref()
            .and_then(|f| f.duration.as_deref())
            .and_then(|d| d.parse().ok()),
        ..Default::default()
    };

    for stream in &parsed.streams {
        match stream.codec_type.as_deref() {
            Some("video") => {
                meta.codec = stream.codec_name.clone();
                meta.width = stream.width;
                meta.height = stream.height;
                meta.frame_rate = stream.r_frame_rate.as_deref().and_then(parse_frame_rate);
                meta.timecode = stream.tags.get("timecode").cloned();
                if meta.duration_secs.is_none() {
                    meta.duration_secs = stream.duration.as_deref().and_then(|d| d.parse().ok());
                }
            }
            Some("audio") => {
                meta.sample_rate = stream.sample_rate.as_deref().and_then(|s| s.parse().ok());
                meta.channels = stream.channels;
                if meta.codec.is_none() {
                    meta.codec = stream.codec_name.clone();
                }
                if meta.duration_secs.is_none() {
                    meta.duration_secs = stream.duration.as_deref().and_then(|d| d.parse().ok());
                }
            }
            _ => {}
        }
    }

    (!meta.is_empty()).then_some(meta)
}

fn exif_field_string(exif: &exif::Exif, tag: exif::Tag) -> Option<String> {
    exif.get_field(tag, exif::In::PRIMARY)
        .map(|f| f.display_value().to_string())
}

/// Reads EXIF tags for a photo. Works for JPEG and most camera RAW formats,
/// since formats like ARW/NEF/DNG/CR2 are themselves TIFF-based containers
/// that a spec-compliant EXIF reader can parse tags out of even without
/// understanding the proprietary raw pixel data.
pub fn read_exif_metadata(path: &Path) -> Option<MediaMetadata> {
    let file = fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;

    let meta = MediaMetadata {
        camera_model: exif_field_string(&exif, exif::Tag::Model),
        lens: exif_field_string(&exif, exif::Tag::LensModel),
        iso: exif_field_string(&exif, exif::Tag::PhotographicSensitivity),
        aperture: exif_field_string(&exif, exif::Tag::FNumber),
        shutter_speed: exif_field_string(&exif, exif::Tag::ExposureTime),
        focal_length: exif_field_string(&exif, exif::Tag::FocalLength),
        ..Default::default()
    };

    (!meta.is_empty()).then_some(meta)
}

/// Extracts technical metadata for one file, dispatching by kind. Never
/// fails the caller -- an unparsable file just yields `None`.
pub fn extract_metadata(path: &Path, kind: MediaKind) -> Option<MediaMetadata> {
    match kind {
        MediaKind::Video | MediaKind::Audio => probe_with_ffprobe(path),
        MediaKind::Photo => read_exif_metadata(path),
        MediaKind::Other => None,
    }
}

/// Generates a small JPEG thumbnail via ffmpeg: one frame ~1s in for video,
/// a straight decode+scale for a still image. Camera RAW formats (ARW/CR2/
/// NEF/...) commonly aren't decodable by stock ffmpeg builds, so this can
/// legitimately fail for those -- the caller treats a `false` return as
/// "no thumbnail available", not an error.
fn generate_thumbnail(path: &Path, kind: MediaKind, out_path: &Path) -> bool {
    let Some(ffmpeg) = ffmpeg_path() else {
        return false;
    };
    let mut cmd = Command::new(ffmpeg);
    cmd.arg("-y");
    if kind == MediaKind::Video {
        cmd.args(["-ss", "1"]);
    }
    cmd.arg("-i").arg(path);
    cmd.args(["-frames:v", "1", "-vf", "scale=320:-2", "-q:v", "4"]);
    cmd.arg(out_path);
    cmd.stdout(Stdio::null()).stderr(Stdio::null());
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

/// Cheap identity for the thumbnail cache -- (path, size, modified) rather
/// than a content checksum, since hashing a multi-GB source file just to
/// key a 320px preview would defeat the point of caching.
fn cache_key(path: &Path, size: u64, modified: SystemTime) -> String {
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    size.hash(&mut hasher);
    modified.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub fn thumbnail_cache_dir(app_cache_dir: &Path) -> std::io::Result<PathBuf> {
    let dir = app_cache_dir.join("thumbnails");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Returns a base64-encoded JPEG thumbnail for the file, generating and
/// caching it on first request. `None` if thumbnails aren't supported for
/// this kind, ffmpeg is unavailable, or generation failed.
pub fn get_or_create_thumbnail_base64(
    path: &Path,
    kind: MediaKind,
    size: u64,
    modified: SystemTime,
    cache_dir: &Path,
) -> Option<String> {
    if !matches!(kind, MediaKind::Video | MediaKind::Photo) {
        return None;
    }
    let cache_path = cache_dir.join(format!("{}.jpg", cache_key(path, size, modified)));
    if !cache_path.exists() && !generate_thumbnail(path, kind, &cache_path) {
        return None;
    }
    let bytes = fs::read(&cache_path).ok()?;
    Some(BASE64.encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_camera_extensions() {
        assert_eq!(classify(Path::new("C0001.MP4")), MediaKind::Video);
        assert_eq!(classify(Path::new("clip.mov")), MediaKind::Video);
        assert_eq!(classify(Path::new("track.wav")), MediaKind::Audio);
        assert_eq!(classify(Path::new("photo.ARW")), MediaKind::Photo);
        assert_eq!(classify(Path::new("thumb.JPG")), MediaKind::Photo);
        assert_eq!(classify(Path::new("sidecar.xml")), MediaKind::Other);
        assert_eq!(classify(Path::new("no_extension")), MediaKind::Other);
    }

    #[test]
    fn frame_rate_parses_fractional_ffprobe_format() {
        assert_eq!(parse_frame_rate("30000/1001"), Some(30000.0 / 1001.0));
        assert_eq!(parse_frame_rate("25/1"), Some(25.0));
        assert_eq!(parse_frame_rate("25/0"), None);
        assert_eq!(parse_frame_rate("not-a-fraction"), None);
    }

    #[test]
    fn cache_key_is_stable_for_identical_inputs_and_differs_otherwise() {
        let path = Path::new("C:/card/clip.mp4");
        let t = SystemTime::UNIX_EPOCH;
        let a = cache_key(path, 100, t);
        let b = cache_key(path, 100, t);
        assert_eq!(a, b);

        let different_size = cache_key(path, 200, t);
        assert_ne!(a, different_size);
    }

    #[test]
    fn missing_ffprobe_binary_returns_none_without_panicking() {
        // On this dev machine ffprobe is installed, so this exercises the
        // "unreadable file" branch instead -- either way, must not panic
        // and must degrade to None rather than blocking a copy.
        let result = probe_with_ffprobe(Path::new("Z:\\definitely\\missing.mp4"));
        assert!(result.is_none());
    }

    #[test]
    fn unreadable_photo_returns_none_without_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not_really_a_photo.jpg");
        fs::write(&path, b"this is not valid image data").unwrap();
        assert!(read_exif_metadata(&path).is_none());
    }

    #[test]
    fn thumbnail_cache_dir_is_created_on_disk() {
        let base = tempfile::tempdir().unwrap();
        let dir = thumbnail_cache_dir(base.path()).unwrap();
        assert!(dir.exists());
        assert!(dir.ends_with("thumbnails"));
    }
}
