use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone, Timelike};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

/// Whether a selective-copy filter list keeps only the matches, or keeps
/// everything except the matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectiveCopyMode {
    Exclude,
    Include,
}

/// Extension (leading `.`) or partial-filename patterns that decide which
/// files are copied. An empty pattern list always passes everything,
/// regardless of mode, so leaving this unconfigured never blocks a transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectiveCopyFilter {
    pub mode: SelectiveCopyMode,
    pub patterns: Vec<String>,
}

impl Default for SelectiveCopyFilter {
    fn default() -> Self {
        Self {
            mode: SelectiveCopyMode::Exclude,
            patterns: Vec::new(),
        }
    }
}

/// Whether the "shoot date" fed into `{YYYY}{MM}{DD}` etc. tokens tracks the
/// real system clock or a manually pinned date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DateOverrideMode {
    Automatic,
    Manual,
}

/// Lets a job's shoot date diverge from the computer's real clock -- e.g. to
/// keep offloading "yesterday's" overnight footage without having to change
/// the system date, or to give a whole multi-day festival dump one shoot date.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateTimeOverride {
    pub mode: DateOverrideMode,
    /// ISO `YYYY-MM-DD` date used verbatim (with the real time-of-day kept)
    /// when `mode` is `Manual`. Ignored in `Automatic` mode. An unparsable or
    /// absent value falls back to the real date rather than failing the
    /// transfer, matching `render_template`'s "degrade, don't error" policy.
    pub manual_date: Option<String>,
    /// Only meaningful in `Automatic` mode: treats times between midnight and
    /// 4am as still belonging to the previous calendar day, so footage from
    /// an overnight shoot keeps a consistent shoot-day date instead of
    /// rolling over right at midnight.
    pub rollover_at_4am: bool,
}

impl Default for DateTimeOverride {
    fn default() -> Self {
        Self {
            mode: DateOverrideMode::Automatic,
            manual_date: None,
            rollover_at_4am: false,
        }
    }
}

/// Resolves the real `now` against a `DateTimeOverride`, producing the
/// timestamp that should feed the plain (non-`File`/`Content`) date tokens.
pub fn effective_job_date(now: SystemTime, date_override: &DateTimeOverride) -> SystemTime {
    let now_local: DateTime<Local> = now.into();

    if date_override.mode == DateOverrideMode::Manual {
        if let Some(date_str) = &date_override.manual_date {
            if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                let naive = date.and_time(now_local.time());
                if let Some(local_dt) = Local.from_local_datetime(&naive).single() {
                    return local_dt.into();
                }
            }
        }
        return now;
    }

    if date_override.rollover_at_4am && now_local.hour() < 4 {
        let rolled_date = now_local.date_naive() - chrono::Duration::days(1);
        let naive = rolled_date.and_time(now_local.time());
        if let Some(local_dt) = Local.from_local_datetime(&naive).single() {
            return local_dt.into();
        }
    }
    now
}

/// Ignores a whole folder (e.g. an empty camera-generated bundle) by name,
/// but only when its total recursive size is at or below `max_size_bytes` --
/// a populated folder that happens to share the name is protected from
/// being silently skipped.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleIgnoreRule {
    pub name: String,
    pub max_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizeSettings {
    /// Renders the destination file name. `None` keeps the original file name
    /// (including extension) untouched.
    pub rename_template: Option<String>,
    /// Renders the destination subfolder path (segments separated by `/`).
    /// `None` preserves the source's original relative directory structure.
    pub folder_template: Option<String>,
    /// Leading-zero width for `{Counter}`. `{File Counter}` always uses 5.
    pub counter_padding: u8,
    pub selective_copy: SelectiveCopyFilter,
    pub bundle_ignore: Option<BundleIgnoreRule>,
    /// When true (the default), source folders that end up with no files to
    /// copy are never created at the destination. When false, they're
    /// mirrored as empty folders -- only meaningful while the original
    /// directory structure is otherwise preserved (no folder template, not flattened).
    pub ignore_empty_folders: bool,
    /// Copies every file directly into the destination root, discarding the
    /// source's directory hierarchy (unless `folder_template` places it back
    /// into folders of its own).
    pub flatten: bool,
    /// Extensions (leading `.` optional) excluded when finding the oldest
    /// file date for `{Content *}` tokens, e.g. sidecar XML/metadata files
    /// that would otherwise skew the shoot day.
    pub content_date_excluded_extensions: Vec<String>,
    /// Overrides the shoot date fed into the plain `{YYYY}{MM}{DD}` etc.
    /// tokens. `#[serde(default)]` so presets saved before this field existed
    /// still load, falling back to "follow the system clock".
    #[serde(default)]
    pub date_override: DateTimeOverride,
    /// User-defined custom tokens ("Elements") usable in the rename/folder
    /// templates, on top of the built-in ones. `#[serde(default)]` so
    /// presets saved before this field existed still load.
    #[serde(default)]
    pub elements: Vec<ElementDefinition>,
}

impl Default for OrganizeSettings {
    fn default() -> Self {
        Self {
            rename_template: None,
            folder_template: None,
            counter_padding: 3,
            selective_copy: SelectiveCopyFilter::default(),
            bundle_ignore: None,
            ignore_empty_folders: true,
            flatten: false,
            content_date_excluded_extensions: Vec::new(),
            date_override: DateTimeOverride::default(),
            elements: Vec::new(),
        }
    }
}

/// A user-defined custom token (e.g. `{Location}`, `{Project}`) with the
/// value to substitute for it in this job's rename/folder templates --
/// OffShoot calls these "Elements". Unlike the built-in tokens, both the
/// name and value are entered by the user rather than derived from the file
/// or job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementDefinition {
    /// Token name without braces, e.g. `"Location"` for `{Location}`.
    pub name: String,
    pub value: String,
}

/// Values available to the token engine while rendering one file's rename
/// and/or folder template.
pub struct TokenContext {
    pub source_name: String,
    pub job_started: SystemTime,
    pub counter: u32,
    pub counter_padding: u8,
    pub file_stem: String,
    pub file_extension: String,
    pub file_modified: SystemTime,
    /// Oldest modified time among the files being organized in this job
    /// (after excluded extensions), used by `{Content *}` tokens. Falls back
    /// to `job_started` when there's nothing to compute it from.
    pub content_oldest: Option<SystemTime>,
    /// User-defined custom tokens for this job. Same for every file.
    pub elements: Vec<ElementDefinition>,
}

fn pad(n: u32, width: u8) -> String {
    format!("{n:0width$}", width = width as usize)
}

fn date_tokens(prefix: &str, t: SystemTime) -> Vec<(String, String)> {
    let dt: DateTime<Local> = t.into();
    vec![
        (format!("{{{prefix}YYYY}}"), format!("{:04}", dt.year())),
        (format!("{{{prefix}YY}}"), format!("{:02}", dt.year() % 100)),
        (format!("{{{prefix}MM}}"), format!("{:02}", dt.month())),
        (format!("{{{prefix}DD}}"), format!("{:02}", dt.day())),
        (format!("{{{prefix}hh}}"), format!("{:02}", dt.hour())),
        (format!("{{{prefix}mm}}"), format!("{:02}", dt.minute())),
        (format!("{{{prefix}ss}}"), format!("{:02}", dt.second())),
    ]
}

/// Substitutes every known token in `template` with its rendered value.
/// Unknown `{...}` placeholders are left as-is rather than erroring, so a
/// typo degrades to a visibly wrong (but harmless) file name instead of
/// failing the whole transfer.
pub fn render_template(template: &str, ctx: &TokenContext) -> String {
    let mut tokens: Vec<(String, String)> = Vec::new();
    tokens.push(("{Source Name}".to_string(), ctx.source_name.clone()));
    tokens.push((
        "{Counter}".to_string(),
        pad(ctx.counter, ctx.counter_padding),
    ));
    tokens.push(("{Filename}".to_string(), ctx.file_stem.clone()));
    tokens.push(("{File Counter}".to_string(), pad(ctx.counter, 5)));
    tokens.push((
        "{File Extension}".to_string(),
        ctx.file_extension.clone(),
    ));
    tokens.extend(date_tokens("", ctx.job_started));
    tokens.extend(date_tokens("File ", ctx.file_modified));
    tokens.extend(date_tokens(
        "Content ",
        ctx.content_oldest.unwrap_or(ctx.job_started),
    ));
    for element in &ctx.elements {
        let name = element.name.trim();
        if name.is_empty() {
            continue;
        }
        tokens.push((format!("{{{name}}}"), element.value.clone()));
    }

    let mut rendered = template.to_string();
    for (token, value) in tokens {
        rendered = rendered.replace(&token, &value);
    }
    rendered
}

fn build_file_name(ext: &str, ctx: &TokenContext, template: Option<&str>, original_name: &str) -> String {
    let Some(template) = template else {
        return original_name.to_string();
    };
    let rendered = render_template(template, ctx);
    if ext.is_empty() || template.contains("{File Extension}") {
        rendered
    } else {
        format!("{rendered}.{ext}")
    }
}

/// Renders the final destination path (folder + file name) for one source
/// file, relative to the destination root. Falls back to the original
/// relative path unchanged when no organize options are configured, so this
/// is a no-op by default.
pub fn build_destination_path(relative: &Path, ctx: &TokenContext, settings: &OrganizeSettings) -> PathBuf {
    let original_name = relative
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let file_name = build_file_name(
        &ctx.file_extension,
        ctx,
        settings.rename_template.as_deref(),
        &original_name,
    );

    let folder = if settings.flatten {
        PathBuf::new()
    } else if let Some(folder_template) = &settings.folder_template {
        PathBuf::from(render_template(folder_template, ctx))
    } else {
        relative
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default()
    };

    folder.join(file_name)
}

/// Extension match (pattern starts with `.`) or case-insensitive partial
/// filename match otherwise.
fn matches_pattern(relative: &Path, pattern: &str) -> bool {
    let pattern = pattern.trim().to_lowercase();
    if pattern.is_empty() {
        return false;
    }
    let file_name = relative
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    match pattern.strip_prefix('.') {
        Some(ext) => file_name.ends_with(&format!(".{ext}")),
        None => file_name.contains(&pattern),
    }
}

pub fn passes_selective_filter(relative: &Path, filter: &SelectiveCopyFilter) -> bool {
    if filter.patterns.is_empty() {
        return true;
    }
    let matched = filter.patterns.iter().any(|p| matches_pattern(relative, p));
    match filter.mode {
        SelectiveCopyMode::Exclude => !matched,
        SelectiveCopyMode::Include => matched,
    }
}

fn dir_size(dir: &Path) -> u64 {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

/// Finds every directory under `source` matching the bundle's name whose
/// total recursive size is at or below the configured threshold. Callers
/// should exclude any file whose path starts with one of these.
pub fn find_ignored_bundle_dirs(source: &Path, rule: &BundleIgnoreRule) -> Vec<PathBuf> {
    let target = rule.name.trim().to_lowercase();
    if target.is_empty() {
        return Vec::new();
    }

    WalkDir::new(source)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|n| n.to_lowercase() == target)
                .unwrap_or(false)
        })
        .filter(|e| dir_size(e.path()) <= rule.max_size_bytes)
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// Oldest modified time among `files`, skipping any whose extension is in
/// `excluded_extensions` (leading `.` optional, case-insensitive).
pub fn compute_content_oldest_date(
    files: &[(PathBuf, SystemTime)],
    excluded_extensions: &[String],
) -> Option<SystemTime> {
    let excluded: Vec<String> = excluded_extensions
        .iter()
        .map(|e| e.trim_start_matches('.').to_lowercase())
        .collect();

    files
        .iter()
        .filter(|(path, _)| {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            !excluded.contains(&ext)
        })
        .map(|(_, modified)| *modified)
        .min()
}

/// Creates empty destination directories for every source directory that
/// isn't in `dirs_with_files` -- used to mirror empty folders when
/// `ignore_empty_folders` is disabled. Only meaningful when the destination
/// otherwise mirrors the source's original structure (no folder template,
/// not flattened); callers are responsible for only calling this then.
pub fn mirror_empty_source_dirs(
    source: &Path,
    destination: &Path,
    dirs_with_files: &HashSet<PathBuf>,
) -> io::Result<()> {
    for entry in WalkDir::new(source).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_dir() || entry.path() == source {
            continue;
        }
        let relative = match entry.path().strip_prefix(source) {
            Ok(r) => r.to_path_buf(),
            Err(_) => continue,
        };
        if dirs_with_files.contains(&relative) {
            continue;
        }
        fs::create_dir_all(destination.join(&relative))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::time::Duration;

    fn local_time(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> SystemTime {
        Local.with_ymd_and_hms(y, mo, d, h, mi, s).unwrap().into()
    }

    fn ctx(overrides: impl FnOnce(&mut TokenContext)) -> TokenContext {
        let mut ctx = TokenContext {
            source_name: "A-Cam".to_string(),
            job_started: local_time(2020, 9, 13, 12, 0, 0),
            counter: 1,
            counter_padding: 3,
            file_stem: "C0001".to_string(),
            file_extension: "MP4".to_string(),
            file_modified: local_time(2020, 9, 2, 8, 0, 0),
            content_oldest: None,
            elements: Vec::new(),
        };
        overrides(&mut ctx);
        ctx
    }

    #[test]
    fn default_settings_keep_the_original_relative_path_unchanged() {
        let relative = Path::new("CLIP").join("C0001.MP4");
        let path = build_destination_path(&relative, &ctx(|_| {}), &OrganizeSettings::default());
        assert_eq!(path, relative);
    }

    #[test]
    fn renders_source_name_and_padded_counter() {
        let rendered = render_template("{Source Name}_{Counter}", &ctx(|c| c.counter = 7));
        assert_eq!(rendered, "A-Cam_007");
    }

    #[test]
    fn file_counter_always_uses_five_digits_regardless_of_counter_padding() {
        let rendered = render_template("{File Counter}", &ctx(|c| {
            c.counter = 7;
            c.counter_padding = 2;
        }));
        assert_eq!(rendered, "00007");
    }

    #[test]
    fn custom_elements_render_alongside_built_in_tokens() {
        let rendered = render_template(
            "{Source Name}_{Location}_{Project}",
            &ctx(|c| {
                c.elements = vec![
                    ElementDefinition { name: "Location".to_string(), value: "Paris".to_string() },
                    ElementDefinition { name: "Project".to_string(), value: "Ad Shoot".to_string() },
                ];
            }),
        );
        assert_eq!(rendered, "A-Cam_Paris_Ad Shoot");
    }

    #[test]
    fn an_unfilled_element_renders_as_an_empty_string() {
        let rendered = render_template(
            "{Filename}-{Location}",
            &ctx(|c| {
                c.elements = vec![ElementDefinition { name: "Location".to_string(), value: String::new() }];
            }),
        );
        assert_eq!(rendered, "C0001-");
    }

    #[test]
    fn an_element_with_a_blank_name_is_ignored() {
        let rendered = render_template(
            "{Filename}",
            &ctx(|c| {
                c.elements = vec![ElementDefinition { name: "   ".to_string(), value: "x".to_string() }];
            }),
        );
        assert_eq!(rendered, "C0001");
    }

    #[test]
    fn job_started_and_file_modified_tokens_render_independently() {
        let rendered = render_template(
            "{YYYY}-{MM}-{DD}_shot-{File YYYY}-{File MM}-{File DD}",
            &ctx(|_| {}),
        );
        assert_eq!(rendered, "2020-09-13_shot-2020-09-02");
    }

    #[test]
    fn rename_template_auto_appends_original_extension() {
        let name = build_file_name("MP4", &ctx(|_| {}), Some("{Filename}_{Counter}"), "C0001.MP4");
        assert_eq!(name, "C0001_001.MP4");
    }

    #[test]
    fn rename_template_does_not_double_append_extension_when_token_used_explicitly() {
        let name = build_file_name(
            "MP4",
            &ctx(|_| {}),
            Some("{Filename}.{File Extension}"),
            "C0001.MP4",
        );
        assert_eq!(name, "C0001.MP4");
    }

    #[test]
    fn folder_template_sorts_by_file_extension() {
        let relative = Path::new("CLIP").join("C0001.MP4");
        let mut settings = OrganizeSettings::default();
        settings.folder_template = Some("{File Extension}".to_string());
        let path = build_destination_path(&relative, &ctx(|_| {}), &settings);
        assert_eq!(path, Path::new("MP4").join("C0001.MP4"));
    }

    #[test]
    fn folder_template_supports_nested_date_segments() {
        let relative = Path::new("CLIP").join("C0001.MP4");
        let mut settings = OrganizeSettings::default();
        settings.folder_template = Some("{File YYYY}/{File MM}/{File DD}".to_string());
        let path = build_destination_path(&relative, &ctx(|_| {}), &settings);
        assert_eq!(path, Path::new("2020").join("09").join("02").join("C0001.MP4"));
    }

    #[test]
    fn flatten_discards_original_subdirectories() {
        let relative = Path::new("CLIP").join("nested").join("C0001.MP4");
        let mut settings = OrganizeSettings::default();
        settings.flatten = true;
        let path = build_destination_path(&relative, &ctx(|_| {}), &settings);
        assert_eq!(path, Path::new("C0001.MP4"));
    }

    #[test]
    fn empty_pattern_list_passes_everything() {
        let filter = SelectiveCopyFilter::default();
        assert!(passes_selective_filter(Path::new("a.xml"), &filter));
    }

    #[test]
    fn exclude_mode_blocks_matching_extension() {
        let filter = SelectiveCopyFilter {
            mode: SelectiveCopyMode::Exclude,
            patterns: vec![".xml".to_string()],
        };
        assert!(!passes_selective_filter(Path::new("clip.XML"), &filter));
        assert!(passes_selective_filter(Path::new("clip.mp4"), &filter));
    }

    #[test]
    fn include_mode_keeps_only_matching_partial_name() {
        let filter = SelectiveCopyFilter {
            mode: SelectiveCopyMode::Include,
            patterns: vec!["proxy".to_string()],
        };
        assert!(passes_selective_filter(Path::new("clip_proxy.mov"), &filter));
        assert!(!passes_selective_filter(Path::new("clip.mov"), &filter));
    }

    #[test]
    fn bundle_below_threshold_is_ignored_but_populated_one_is_protected() {
        let dir = tempfile::tempdir().unwrap();
        let small_bundle = dir.path().join("PRIVATE");
        fs::create_dir_all(&small_bundle).unwrap();
        fs::write(small_bundle.join("stub.bin"), vec![0u8; 10]).unwrap();

        let populated = dir.path().join("keep").join("PRIVATE");
        fs::create_dir_all(&populated).unwrap();
        fs::write(populated.join("real.mp4"), vec![0u8; 10_000]).unwrap();

        let rule = BundleIgnoreRule {
            name: "PRIVATE".to_string(),
            max_size_bytes: 100,
        };
        let ignored = find_ignored_bundle_dirs(dir.path(), &rule);

        assert!(ignored.contains(&small_bundle));
        assert!(!ignored.contains(&populated), "a populated bundle must not be silently skipped");
    }

    #[test]
    fn content_oldest_date_excludes_configured_extensions() {
        let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let newer_sidecar = SystemTime::UNIX_EPOCH + Duration::from_secs(500); // older than `old`, but excluded
        let files = vec![
            (PathBuf::from("clip.mp4"), old),
            (PathBuf::from("clip.xml"), newer_sidecar),
        ];
        let oldest = compute_content_oldest_date(&files, &["xml".to_string()]);
        assert_eq!(oldest, Some(old));
    }

    #[test]
    fn automatic_mode_without_rollover_returns_now_unchanged() {
        let now = local_time(2020, 9, 13, 2, 30, 0);
        let result = effective_job_date(now, &DateTimeOverride::default());
        assert_eq!(result, now);
    }

    #[test]
    fn rollover_pushes_early_morning_times_back_to_the_previous_day() {
        let now = local_time(2020, 9, 13, 2, 30, 0); // 2:30am
        let date_override = DateTimeOverride {
            mode: DateOverrideMode::Automatic,
            manual_date: None,
            rollover_at_4am: true,
        };
        let result = effective_job_date(now, &date_override);
        let dt: DateTime<Local> = result.into();
        assert_eq!((dt.year(), dt.month(), dt.day()), (2020, 9, 12));
        assert_eq!((dt.hour(), dt.minute(), dt.second()), (2, 30, 0));
    }

    #[test]
    fn rollover_leaves_times_at_or_after_4am_on_the_real_day() {
        let now = local_time(2020, 9, 13, 4, 0, 0);
        let date_override = DateTimeOverride {
            mode: DateOverrideMode::Automatic,
            manual_date: None,
            rollover_at_4am: true,
        };
        let result = effective_job_date(now, &date_override);
        assert_eq!(result, now);
    }

    #[test]
    fn manual_mode_pins_the_date_but_keeps_the_real_time_of_day() {
        let now = local_time(2020, 9, 13, 14, 5, 30);
        let date_override = DateTimeOverride {
            mode: DateOverrideMode::Manual,
            manual_date: Some("2019-01-01".to_string()),
            rollover_at_4am: false,
        };
        let result = effective_job_date(now, &date_override);
        let dt: DateTime<Local> = result.into();
        assert_eq!((dt.year(), dt.month(), dt.day()), (2019, 1, 1));
        assert_eq!((dt.hour(), dt.minute(), dt.second()), (14, 5, 30));
    }

    #[test]
    fn manual_mode_with_unparsable_date_falls_back_to_now() {
        let now = local_time(2020, 9, 13, 14, 5, 30);
        let date_override = DateTimeOverride {
            mode: DateOverrideMode::Manual,
            manual_date: Some("not-a-date".to_string()),
            rollover_at_4am: false,
        };
        assert_eq!(effective_job_date(now, &date_override), now);
    }

    #[test]
    fn manual_mode_with_no_date_set_falls_back_to_now() {
        let now = local_time(2020, 9, 13, 14, 5, 30);
        let date_override = DateTimeOverride {
            mode: DateOverrideMode::Manual,
            manual_date: None,
            rollover_at_4am: false,
        };
        assert_eq!(effective_job_date(now, &date_override), now);
    }

    #[test]
    fn mirrors_only_directories_without_copied_files() {
        let source = tempfile::tempdir().unwrap();
        let dest = tempfile::tempdir().unwrap();
        fs::create_dir_all(source.path().join("has_files")).unwrap();
        fs::create_dir_all(source.path().join("empty_dir")).unwrap();

        let mut dirs_with_files = HashSet::new();
        dirs_with_files.insert(PathBuf::from("has_files"));

        mirror_empty_source_dirs(source.path(), dest.path(), &dirs_with_files).unwrap();

        assert!(!dest.path().join("has_files").exists());
        assert!(dest.path().join("empty_dir").exists());
    }
}
