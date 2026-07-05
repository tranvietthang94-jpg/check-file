use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::Local;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

use crate::metadata;
use crate::transfer_log::TransferLogEntry;

/// What the user fills in when building a Report from one or more Transfer
/// Logs -- mirrors OffShoot's Reports feature (logo/title/notes over a
/// Summary + per-transfer breakdown), scoped to Standard tier: no
/// Color-Awareness/EditReady stills, no Pro metadata-as-JSON export.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportRequest {
    /// Which `TransferLogEntry.job_id`s to include, in the order they should
    /// appear in the report.
    pub job_ids: Vec<String>,
    pub title: String,
    pub notes: String,
    /// A data: URL (e.g. `data:image/png;base64,...`) embedded verbatim into
    /// an `<img>` tag -- the caller is responsible for encoding it, so this
    /// module never needs to know an image format.
    pub logo_data_url: Option<String>,
    /// Re-extracts a thumbnail for every successfully-transferred clip
    /// still present at its destination. Off by default since it can be
    /// slow (spawns ffmpeg per file) for a large card.
    pub include_thumbnails: bool,
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}

fn format_timestamp(iso: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|_| iso.to_string())
}

struct Summary {
    sources: usize,
    destinations: usize,
    clips: u64,
    bytes: u64,
    failed: usize,
}

fn summarize(entries: &[TransferLogEntry]) -> Summary {
    let mut sources = HashSet::new();
    let mut destinations = HashSet::new();
    let mut clips = 0u64;
    let mut bytes = 0u64;
    let mut failed = 0usize;
    for e in entries {
        sources.insert(&e.source);
        destinations.insert(&e.destination);
        clips += e.files_copied;
        bytes += e.bytes_copied;
        failed += e.failed_files.len();
    }
    Summary {
        sources: sources.len(),
        destinations: destinations.len(),
        clips,
        bytes,
        failed,
    }
}

/// One thumbnail-able clip, resolved from a log entry's destination + a
/// relative path recorded as either verified or skipped.
struct ClipRef<'a> {
    source_name: &'a str,
    absolute: PathBuf,
    relative: &'a str,
}

fn clip_refs(entries: &[TransferLogEntry]) -> Vec<ClipRef<'_>> {
    let mut seen = HashSet::new();
    let mut clips = Vec::new();
    for entry in entries {
        let destination = Path::new(&entry.destination);
        let paths = entry
            .verified_files
            .iter()
            .map(|f| f.path.as_str())
            .chain(entry.skipped_files.iter().map(|f| f.path.as_str()));
        for relative in paths {
            let key = (entry.destination.as_str(), relative);
            if !seen.insert(key) {
                continue;
            }
            clips.push(ClipRef {
                source_name: &entry.source_name,
                absolute: destination.join(relative),
                relative,
            });
        }
    }
    clips
}

fn render_summary(summary: &Summary) -> String {
    let failed_stat = if summary.failed > 0 {
        format!(
            r#"<div class="stat stat-failed"><span class="stat-value">{}</span><span class="stat-label">Failed</span></div>"#,
            summary.failed
        )
    } else {
        String::new()
    };
    format!(
        r#"<section class="summary">
  <h2>Summary</h2>
  <div class="stat-grid">
    <div class="stat"><span class="stat-value">{sources}</span><span class="stat-label">Sources</span></div>
    <div class="stat"><span class="stat-value">{destinations}</span><span class="stat-label">Destinations</span></div>
    <div class="stat"><span class="stat-value">{clips}</span><span class="stat-label">Clips</span></div>
    <div class="stat"><span class="stat-value">{bytes}</span><span class="stat-label">Total Size</span></div>
    {failed_stat}
  </div>
</section>"#,
        sources = summary.sources,
        destinations = summary.destinations,
        clips = summary.clips,
        bytes = format_bytes(summary.bytes),
    )
}

fn render_transfers_table(entries: &[TransferLogEntry]) -> String {
    let rows: String = entries
        .iter()
        .map(|e| {
            format!(
                r#"<tr>
      <td>{source_name}<br><span class="muted">{source}</span></td>
      <td>{destination}</td>
      <td>{verification}</td>
      <td>{started}</td>
      <td>{finished}</td>
      <td>{files} file(s)<br><span class="muted">{bytes}</span></td>
      <td>{issues}</td>
    </tr>"#,
                source_name = escape_html(&e.source_name),
                source = escape_html(&e.source),
                destination = escape_html(&e.destination),
                verification = verification_label(e),
                started = format_timestamp(&e.started_at),
                finished = format_timestamp(&e.finished_at),
                files = e.files_copied,
                bytes = format_bytes(e.bytes_copied),
                issues = render_issue_summary(e),
            )
        })
        .collect();

    format!(
        r#"<section class="transfers">
  <h2>Transfers</h2>
  <table>
    <thead>
      <tr><th>Source</th><th>Destination</th><th>Verification</th><th>Started</th><th>Finished</th><th>Copied</th><th>Issues</th></tr>
    </thead>
    <tbody>
      {rows}
    </tbody>
  </table>
</section>"#
    )
}

fn verification_label(e: &TransferLogEntry) -> &'static str {
    match e.verification_mode {
        crate::copy_engine::VerificationMode::Transfer => "Size check only",
        crate::copy_engine::VerificationMode::Source => "Source hash",
        crate::copy_engine::VerificationMode::SourceAndDestination => "Source + destination hash",
    }
}

fn render_issue_summary(e: &TransferLogEntry) -> String {
    let mut parts = Vec::new();
    if !e.failed_files.is_empty() {
        parts.push(format!(r#"<span class="badge badge-failed">{} failed</span>"#, e.failed_files.len()));
    }
    if !e.skipped_files.is_empty() {
        parts.push(format!(r#"<span class="badge">{} skipped</span>"#, e.skipped_files.len()));
    }
    if !e.renamed_files.is_empty() {
        parts.push(format!(r#"<span class="badge">{} renamed</span>"#, e.renamed_files.len()));
    }
    if !e.deleted_source_files.is_empty() {
        parts.push(format!(r#"<span class="badge">{} moved</span>"#, e.deleted_source_files.len()));
    }
    if !e.move_delete_failed.is_empty() {
        parts.push(format!(
            r#"<span class="badge badge-failed">{} move-delete failed</span>"#,
            e.move_delete_failed.len()
        ));
    }
    if parts.is_empty() {
        r#"<span class="muted">None</span>"#.to_string()
    } else {
        parts.join(" ")
    }
}

/// A detailed per-file breakdown, but only for entries that have something
/// worth calling out -- a clean transfer doesn't need its own section.
fn render_files_section(entries: &[TransferLogEntry]) -> String {
    let sections: String = entries
        .iter()
        .filter(|e| {
            !e.failed_files.is_empty()
                || !e.skipped_files.is_empty()
                || !e.renamed_files.is_empty()
                || !e.move_delete_failed.is_empty()
        })
        .map(|e| {
            let failed: String = e
                .failed_files
                .iter()
                .map(|f| {
                    format!(
                        "<li><span class=\"badge badge-failed\">Failed</span> {} -- {}</li>",
                        escape_html(&f.path),
                        escape_html(&f.message)
                    )
                })
                .collect();
            let skipped: String = e
                .skipped_files
                .iter()
                .map(|f| format!("<li><span class=\"badge\">Skipped</span> {}</li>", escape_html(&f.path)))
                .collect();
            let renamed: String = e
                .renamed_files
                .iter()
                .map(|f| {
                    format!(
                        "<li><span class=\"badge\">Renamed</span> {} &rarr; {}</li>",
                        escape_html(&f.original_path),
                        escape_html(&f.renamed_to)
                    )
                })
                .collect();
            let move_delete_failed: String = e
                .move_delete_failed
                .iter()
                .map(|f| {
                    format!(
                        "<li><span class=\"badge badge-failed\">Move failed</span> {} -- {}</li>",
                        escape_html(&f.path),
                        escape_html(&f.message)
                    )
                })
                .collect();
            format!(
                "<h3>{}</h3><ul class=\"file-list\">{failed}{skipped}{renamed}{move_delete_failed}</ul>",
                escape_html(&e.source_name)
            )
        })
        .collect();

    if sections.is_empty() {
        String::new()
    } else {
        format!("<section class=\"files\"><h2>Files</h2>{sections}</section>")
    }
}

fn render_clips_section(entries: &[TransferLogEntry], cache_dir: Option<&Path>) -> String {
    let Some(cache_dir) = cache_dir else {
        return String::new();
    };
    let refs = clip_refs(entries);
    if refs.is_empty() {
        return String::new();
    }

    let cards: String = refs
        .iter()
        .filter_map(|clip| {
            let meta = fs::metadata(&clip.absolute).ok()?;
            let modified = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            let kind = metadata::classify(&clip.absolute);
            let thumbnail = metadata::get_or_create_thumbnail_base64(
                &clip.absolute,
                kind,
                meta.len(),
                modified,
                cache_dir,
            )?;
            let caption = metadata::extract_metadata(&clip.absolute, kind)
                .and_then(|m| match (m.width, m.height, m.duration_secs) {
                    (Some(w), Some(h), Some(d)) => Some(format!("{w}x{h} &middot; {d:.1}s")),
                    (Some(w), Some(h), None) => Some(format!("{w}x{h}")),
                    (None, None, Some(d)) => Some(format!("{d:.1}s")),
                    _ => None,
                })
                .unwrap_or_default();
            Some(format!(
                r#"<figure class="clip">
      <img src="data:image/jpeg;base64,{thumbnail}" alt="{alt}">
      <figcaption>{name}<br><span class="muted">{source} &middot; {caption}</span></figcaption>
    </figure>"#,
                alt = escape_html(clip.relative),
                name = escape_html(clip.relative),
                source = escape_html(clip.source_name),
            ))
        })
        .collect();

    if cards.is_empty() {
        String::new()
    } else {
        format!(r#"<section class="clips"><h2>Clips</h2><div class="clip-grid">{cards}</div></section>"#)
    }
}

const STYLE: &str = r#"
  body { font-family: -apple-system, Segoe UI, Helvetica, Arial, sans-serif; color: #1a1a1a; background: #fff; margin: 0; padding: 2rem; }
  .report { max-width: 960px; margin: 0 auto; }
  header { display: flex; align-items: center; gap: 1rem; border-bottom: 2px solid #1a1a1a; padding-bottom: 1rem; margin-bottom: 1.5rem; flex-wrap: wrap; }
  header img.logo { max-height: 64px; max-width: 200px; object-fit: contain; }
  header h1 { margin: 0; font-size: 1.5rem; }
  .generated { color: #666; font-size: 0.85rem; margin: 0.25rem 0 0; }
  .notes { white-space: pre-wrap; background: #f5f5f5; border-radius: 6px; padding: 0.75rem 1rem; margin: 1rem 0 0; font-size: 0.9rem; }
  h2 { font-size: 1.1rem; border-bottom: 1px solid #ddd; padding-bottom: 0.4rem; margin-top: 2rem; }
  .stat-grid { display: flex; flex-wrap: wrap; gap: 1rem; }
  .stat { background: #f5f5f5; border-radius: 8px; padding: 0.75rem 1.25rem; min-width: 100px; text-align: center; }
  .stat-value { display: block; font-size: 1.4rem; font-weight: 600; }
  .stat-label { display: block; font-size: 0.75rem; color: #666; text-transform: uppercase; letter-spacing: 0.03em; }
  .stat-failed .stat-value { color: #b3261e; }
  table { width: 100%; border-collapse: collapse; font-size: 0.85rem; }
  th, td { text-align: left; padding: 0.5rem 0.6rem; border-bottom: 1px solid #eee; vertical-align: top; }
  th { color: #666; font-weight: 600; font-size: 0.75rem; text-transform: uppercase; }
  .muted { color: #888; font-size: 0.8rem; }
  .badge { display: inline-block; background: #eee; border-radius: 4px; padding: 0.1rem 0.4rem; font-size: 0.75rem; margin-right: 0.25rem; }
  .badge-failed { background: #fbe9e7; color: #b3261e; }
  .file-list { list-style: none; padding: 0; margin: 0.5rem 0 1.5rem; font-size: 0.85rem; }
  .file-list li { padding: 0.15rem 0; }
  .clip-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 1rem; }
  .clip img { width: 100%; aspect-ratio: 16 / 9; object-fit: cover; border-radius: 6px; background: #eee; }
  .clip figcaption { font-size: 0.75rem; margin-top: 0.25rem; word-break: break-all; }
  @media print { body { padding: 0; } .clip-grid { grid-template-columns: repeat(3, 1fr); } }
"#;

/// Builds a self-contained HTML report (inline CSS, embedded base64 images)
/// from a set of Transfer Logs -- open it in any browser, or use the
/// browser's own Print > Save as PDF to get OffShoot's PDF export for free
/// without this app needing its own PDF renderer.
pub fn generate_report_html(
    entries: &[TransferLogEntry],
    request: &ReportRequest,
    thumbnail_cache_dir: Option<&Path>,
) -> String {
    let title = if request.title.trim().is_empty() {
        "Transfer Report".to_string()
    } else {
        escape_html(request.title.trim())
    };
    let logo = request
        .logo_data_url
        .as_deref()
        .map(|url| format!(r#"<img class="logo" src="{url}" alt="Logo">"#))
        .unwrap_or_default();
    let notes = if request.notes.trim().is_empty() {
        String::new()
    } else {
        format!(r#"<p class="notes">{}</p>"#, escape_html(request.notes.trim()))
    };
    let generated = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let summary = render_summary(&summarize(entries));
    let transfers = render_transfers_table(entries);
    let files = render_files_section(entries);
    let clips = if request.include_thumbnails {
        render_clips_section(entries, thumbnail_cache_dir)
    } else {
        String::new()
    };

    format!(
        r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>{title}</title>
<style>{STYLE}</style>
</head>
<body>
<div class="report">
  <header>
    {logo}
    <div>
      <h1>{title}</h1>
      <p class="generated">Generated {generated}</p>
    </div>
  </header>
  {notes}
  {summary}
  {transfers}
  {files}
  {clips}
</div>
</body>
</html>"#
    )
}

pub fn reports_dir<R: Runtime>(app_handle: &AppHandle<R>) -> io::Result<PathBuf> {
    let base = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    Ok(base.join("reports"))
}

/// Renders and writes the report to a timestamped file under the app's
/// reports directory, returning the path so the caller can open it.
pub fn save_report<R: Runtime>(
    app_handle: &AppHandle<R>,
    entries: &[TransferLogEntry],
    request: &ReportRequest,
) -> io::Result<PathBuf> {
    let dir = reports_dir(app_handle)?;
    fs::create_dir_all(&dir)?;

    let cache_dir = app_handle
        .path()
        .app_cache_dir()
        .ok()
        .and_then(|base| metadata::thumbnail_cache_dir(&base).ok());

    let html = generate_report_html(entries, request, cache_dir.as_deref());
    let file_name = format!("report_{}.html", Local::now().format("%Y%m%d_%H%M%S"));
    let path = dir.join(file_name);
    fs::write(&path, html)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checksum::ChecksumAlgorithm;
    use crate::copy_engine::{FailedFile, RenamedFile, SkippedFile, VerificationMode, VerifiedFile};

    fn sample_entry(job_id: &str, source: &str, destination: &str) -> TransferLogEntry {
        TransferLogEntry {
            job_id: job_id.to_string(),
            source_name: format!("{job_id}-Cam"),
            source: source.to_string(),
            destination: destination.to_string(),
            verification_mode: VerificationMode::SourceAndDestination,
            checksum_algorithm: ChecksumAlgorithm::Xxh64,
            started_at: "2026-07-04T20:00:00Z".to_string(),
            finished_at: "2026-07-04T20:05:00Z".to_string(),
            files_copied: 2,
            bytes_copied: 2_000_000,
            failed_files: Vec::new(),
            verified_files: vec![VerifiedFile {
                path: "C0001.MP4".to_string(),
                checksum: "abc".to_string(),
                algorithm: ChecksumAlgorithm::Xxh64,
                legacy_checksum: None, legacy_algorithm: None,
            }],
            skipped_files: Vec::new(),
            renamed_files: Vec::new(),
            deleted_source_files: Vec::new(),
            move_delete_failed: Vec::new(),
            broken_media_files: Vec::new(),
            mhl_path: None,
            cancelled: false,
        }
    }

    fn default_request() -> ReportRequest {
        ReportRequest {
            job_ids: vec!["job-1".to_string()],
            title: String::new(),
            notes: String::new(),
            logo_data_url: None,
            include_thumbnails: false,
        }
    }

    #[test]
    fn summary_aggregates_across_multiple_entries() {
        let entries = vec![
            sample_entry("job-1", "G:\\", "D:\\Offload1"),
            sample_entry("job-2", "G:\\", "D:\\Offload2"),
        ];
        let summary = summarize(&entries);
        assert_eq!(summary.sources, 1, "same source path counted once");
        assert_eq!(summary.destinations, 2);
        assert_eq!(summary.clips, 4);
        assert_eq!(summary.bytes, 4_000_000);
        assert_eq!(summary.failed, 0);
    }

    #[test]
    fn blank_title_falls_back_to_a_default() {
        let entries = vec![sample_entry("job-1", "G:\\", "D:\\Offload")];
        let html = generate_report_html(&entries, &default_request(), None);
        assert!(html.contains("Transfer Report"));
    }

    #[test]
    fn custom_title_and_notes_are_escaped_and_included() {
        let entries = vec![sample_entry("job-1", "G:\\", "D:\\Offload")];
        let mut request = default_request();
        request.title = "<Wedding> Day 1".to_string();
        request.notes = "Card A & B, all good".to_string();
        let html = generate_report_html(&entries, &request, None);
        assert!(html.contains("&lt;Wedding&gt; Day 1"));
        assert!(html.contains("Card A &amp; B, all good"));
        assert!(!html.contains("<Wedding>"));
    }

    #[test]
    fn failed_files_render_in_both_the_transfer_row_and_the_files_section() {
        let mut entry = sample_entry("job-1", "G:\\", "D:\\Offload");
        entry.failed_files.push(FailedFile {
            path: "C0002.MP4".to_string(),
            message: "I/O error".to_string(),
        });
        let html = generate_report_html(&[entry], &default_request(), None);
        assert!(html.contains("1 failed"));
        assert!(html.contains("C0002.MP4"));
        assert!(html.contains("I/O error"));
    }

    #[test]
    fn a_clean_transfer_has_no_files_section() {
        let entries = vec![sample_entry("job-1", "G:\\", "D:\\Offload")];
        let html = generate_report_html(&entries, &default_request(), None);
        assert!(!html.contains("class=\"files\""));
    }

    #[test]
    fn renamed_and_skipped_files_are_listed_in_the_files_section() {
        let mut entry = sample_entry("job-1", "G:\\", "D:\\Offload");
        entry.skipped_files.push(SkippedFile { path: "C0003.MP4".to_string() });
        entry.renamed_files.push(RenamedFile {
            original_path: "C0004.MP4".to_string(),
            renamed_to: "C0004_1.MP4".to_string(),
        });
        let html = generate_report_html(&[entry], &default_request(), None);
        assert!(html.contains("class=\"files\""));
        assert!(html.contains("C0003.MP4"));
        assert!(html.contains("C0004.MP4"));
        assert!(html.contains("C0004_1.MP4"));
    }

    #[test]
    fn moved_files_render_a_badge_but_no_files_section_entry() {
        let mut entry = sample_entry("job-1", "G:\\", "D:\\Offload");
        entry.deleted_source_files = vec!["C0001.MP4".to_string()];
        let html = generate_report_html(&[entry], &default_request(), None);
        assert!(html.contains("1 moved"));
        // A successful move isn't a problem worth its own Files-section row --
        // only a failure to delete is.
        assert!(!html.contains("class=\"files\""));
    }

    #[test]
    fn a_move_delete_failure_renders_in_both_the_transfer_row_and_the_files_section() {
        let mut entry = sample_entry("job-1", "G:\\", "D:\\Offload");
        entry.move_delete_failed.push(FailedFile {
            path: "C0005.MP4".to_string(),
            message: "access denied".to_string(),
        });
        let html = generate_report_html(&[entry], &default_request(), None);
        assert!(html.contains("1 move-delete failed"));
        assert!(html.contains("class=\"files\""));
        assert!(html.contains("C0005.MP4"));
        assert!(html.contains("access denied"));
    }

    #[test]
    fn clips_section_is_absent_without_a_cache_dir_even_if_requested() {
        let entries = vec![sample_entry("job-1", "G:\\", "D:\\Offload")];
        let mut request = default_request();
        request.include_thumbnails = true;
        let html = generate_report_html(&entries, &request, None);
        assert!(!html.contains("class=\"clips\""));
    }

    #[test]
    fn clip_refs_deduplicates_the_same_destination_path() {
        let mut a = sample_entry("job-1", "G:\\", "D:\\Offload");
        let mut b = sample_entry("job-2", "G:\\", "D:\\Offload");
        a.verified_files = vec![VerifiedFile {
            path: "C0001.MP4".to_string(),
            checksum: "x".to_string(),
            algorithm: ChecksumAlgorithm::Xxh64,
            legacy_checksum: None, legacy_algorithm: None,
        }];
        b.verified_files = a.verified_files.clone();
        let entries = [a, b];
        let refs = clip_refs(&entries);
        assert_eq!(refs.len(), 1, "same destination+path across two logs should only produce one clip");
    }
}
