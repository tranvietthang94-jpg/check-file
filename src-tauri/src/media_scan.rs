use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::metadata::{self, MediaKind, MediaMetadata};

pub const SCAN_ITEM_EVENT: &str = "media-scan-item";
pub const SCAN_COMPLETE_EVENT: &str = "media-scan-complete";

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MediaEntry {
    pub path: String,
    pub size: u64,
    pub kind: MediaKind,
    pub metadata: Option<MediaMetadata>,
    pub thumbnail_base64: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MediaScanItemPayload {
    pub scan_id: String,
    pub entry: MediaEntry,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MediaScanCompletePayload {
    pub scan_id: String,
    pub total: u64,
}

fn build_entry(path: &Path, relative: &Path, cache_dir: Option<&Path>) -> MediaEntry {
    let meta = fs::metadata(path).ok();
    let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
    let modified = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let kind = metadata::classify(path);

    let thumbnail_base64 = cache_dir.and_then(|dir| {
        metadata::get_or_create_thumbnail_base64(path, kind, size, modified, dir)
    });

    MediaEntry {
        path: relative.display().to_string(),
        size,
        kind,
        metadata: metadata::extract_metadata(path, kind),
        thumbnail_base64,
    }
}

/// Walks `folder` and emits one `media-scan-item` event per file as its
/// metadata/thumbnail become available, then a final `media-scan-complete`.
/// Runs on its own thread since ffprobe/ffmpeg subprocess calls per file
/// make this too slow to do inline with a command's return value.
pub fn start_media_scan<R: Runtime>(app_handle: AppHandle<R>, folder: PathBuf) -> String {
    let scan_id = Uuid::new_v4().to_string();
    let scan_id_thread = scan_id.clone();

    std::thread::spawn(move || {
        let cache_dir = app_handle
            .path()
            .app_cache_dir()
            .ok()
            .and_then(|base| metadata::thumbnail_cache_dir(&base).ok());

        let mut total = 0u64;
        for entry in WalkDir::new(&folder).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let relative = path.strip_prefix(&folder).unwrap_or(path);
            let media_entry = build_entry(path, relative, cache_dir.as_deref());

            total += 1;
            let _ = app_handle.emit(
                SCAN_ITEM_EVENT,
                MediaScanItemPayload {
                    scan_id: scan_id_thread.clone(),
                    entry: media_entry,
                },
            );
        }

        let _ = app_handle.emit(
            SCAN_COMPLETE_EVENT,
            MediaScanCompletePayload {
                scan_id: scan_id_thread,
                total,
            },
        );
    });

    scan_id
}
