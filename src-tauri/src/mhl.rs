use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, SecondsFormat, Utc};
use quick_xml::events::{BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use serde::Serialize;

use crate::checksum::{self, ChecksumAlgorithm};
use crate::dedup::mtimes_close;

/// One transferred file's record for the MHL. `checksum` is `None` when the
/// job ran in Transfer verification mode (no hash was computed) -- the
/// legacy MHL v1.1 schema has a `<null/>` hash choice for exactly this
/// size-only case, so a file is still recorded rather than silently omitted.
#[derive(Clone)]
pub struct MhlFileEntry {
    pub relative_path: String,
    pub size: u64,
    pub modified: SystemTime,
    pub checksum: Option<String>,
    pub algorithm: ChecksumAlgorithm,
    pub hashed_at: SystemTime,
    /// OffShoot's "Also generate legacy checksums" -- a second hash-choice
    /// element written alongside the primary one in the same `<hash>`
    /// block, for interop with tooling that expects an older algorithm.
    pub legacy_checksum: Option<String>,
    pub legacy_algorithm: Option<ChecksumAlgorithm>,
}

pub(crate) fn iso8601(t: SystemTime) -> String {
    let dt: DateTime<Utc> = t.into();
    dt.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Element name for a hash choice, per the legacy MHL v1.1 XSD
/// (mediahashlist.org) -- XXH64 is written big-endian under the
/// `xxhash64be` tag, matching the "XXH64BE" convention this codebase
/// already uses (see checksum::StreamingHasher::finalize_hex). C4 predates
/// the legacy XSD and has no enumerated tag there; `"c4"` follows the same
/// lowercase-algorithm-name convention as the others -- not verified against
/// a real OffShoot-written MHL sample, since none was available to check.
fn hash_tag(algorithm: ChecksumAlgorithm) -> &'static str {
    match algorithm {
        ChecksumAlgorithm::Xxh64 => "xxhash64be",
        ChecksumAlgorithm::Xxh3 => "xxh3",
        ChecksumAlgorithm::Xxh128 => "xxh128",
        ChecksumAlgorithm::Md5 => "md5",
        ChecksumAlgorithm::Sha1 => "sha1",
        ChecksumAlgorithm::C4 => "c4",
    }
}

fn current_username() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn current_hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-host".to_string())
}

fn write_text_element(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    name: &str,
    text: &str,
) -> io::Result<()> {
    writer
        .create_element(name)
        .write_text_content(BytesText::new(text))?;
    Ok(())
}

/// Renders a legacy MHL v1.1 document (https://mediahashlist.org) for one
/// completed transfer -- the format OffShoot itself defaults to outside its
/// Pro/ASC-MHL tier, and the simpler of the two since it's a single flat
/// file with no chain-of-custody bookkeeping.
pub fn render_mhl(
    entries: &[MhlFileEntry],
    started_at: SystemTime,
    finished_at: SystemTime,
) -> String {
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

    writer
        .create_element("hashlist")
        .with_attribute(("version", "1.1"))
        .write_inner_content(|writer| {
            writer
                .create_element("creatorinfo")
                .write_inner_content(|writer| {
                    write_text_element(writer, "username", &current_username())?;
                    write_text_element(writer, "hostname", &current_hostname())?;
                    write_text_element(
                        writer,
                        "tool",
                        &format!("OffloadKit {}", env!("CARGO_PKG_VERSION")),
                    )?;
                    write_text_element(writer, "startdate", &iso8601(started_at))?;
                    write_text_element(writer, "finishdate", &iso8601(finished_at))?;
                    Ok(())
                })?;

            for entry in entries {
                writer
                    .create_element("hash")
                    .write_inner_content(|writer| {
                        write_text_element(writer, "file", &entry.relative_path)?;
                        write_text_element(writer, "size", &entry.size.to_string())?;
                        write_text_element(
                            writer,
                            "lastmodificationdate",
                            &iso8601(entry.modified),
                        )?;
                        match &entry.checksum {
                            Some(hash) => {
                                write_text_element(writer, hash_tag(entry.algorithm), hash)?
                            }
                            None => write_text_element(writer, "null", "")?,
                        }
                        if let (Some(hash), Some(algorithm)) =
                            (&entry.legacy_checksum, entry.legacy_algorithm)
                        {
                            write_text_element(writer, hash_tag(algorithm), hash)?;
                        }
                        write_text_element(writer, "hashdate", &iso8601(entry.hashed_at))?;
                        Ok(())
                    })?;
            }
            Ok(())
        })
        .expect("writing XML to an in-memory buffer cannot fail");

    let bytes = writer.into_inner().into_inner();
    let body = String::from_utf8(bytes).expect("quick-xml always writes valid UTF-8");
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{body}\n")
}

/// OffShoot's "Also create an MHL for each file" -- one small single-entry
/// MHL per copied file, written next to that file (the common per-clip
/// sidecar convention), in addition to the one combined MHL [`write_mhl`]
/// already writes at the destination root. Best-effort per file: one
/// unwritable entry (e.g. an unreadable parent dir) doesn't stop the rest.
pub fn write_per_file_mhls(
    destination: &Path,
    entries: &[MhlFileEntry],
    started_at: SystemTime,
    finished_at: SystemTime,
) -> Vec<PathBuf> {
    let mut written = Vec::new();
    for entry in entries {
        let file_dest = destination.join(&entry.relative_path);
        let (Some(parent), Some(file_name)) = (file_dest.parent(), file_dest.file_name()) else {
            continue;
        };
        let mhl_path = parent.join(format!("{}.mhl", file_name.to_string_lossy()));
        let xml = render_mhl(std::slice::from_ref(entry), started_at, finished_at);
        if crate::atomic_file::write_atomic(&mhl_path, xml.as_bytes()).is_ok() {
            written.push(mhl_path);
        }
    }
    written
}

/// Writes the MHL for one completed transfer to the destination root,
/// returning the path written. A no-op when there's nothing to record, e.g.
/// a transfer that copied zero files (skips-only re-run of an already
/// offloaded card).
pub fn write_mhl(
    destination: &Path,
    entries: &[MhlFileEntry],
    started_at: SystemTime,
    finished_at: SystemTime,
) -> io::Result<Option<PathBuf>> {
    if entries.is_empty() {
        return Ok(None);
    }
    let dt: DateTime<Utc> = started_at.into();
    let stem = dt.format("%Y%m%d_%H%M%S").to_string();
    let mut path = destination.join(format!("{stem}.mhl"));
    let mut counter = 2;
    while path.exists() {
        path = destination.join(format!("{stem}_{counter}.mhl"));
        counter += 1;
    }
    crate::atomic_file::write_atomic(
        &path,
        render_mhl(entries, started_at, finished_at).as_bytes(),
    )?;
    Ok(Some(path))
}

/// One `<hash>` record read back out of an MHL document. `checksum`/`algorithm`
/// are `None` for a `<null/>` choice (a Transfer-mode entry) or for a hash tag
/// this build doesn't recognize (e.g. `sha256`/`c4` written by another tool) --
/// such an entry is still returned so its file/size can be checked, just
/// without a checksum to reuse or re-verify.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedMhlEntry {
    pub relative_path: String,
    pub size: u64,
    pub modified: SystemTime,
    pub checksum: Option<String>,
    pub algorithm: Option<ChecksumAlgorithm>,
}

fn algorithm_for_hash_tag(tag: &str) -> Option<ChecksumAlgorithm> {
    match tag {
        "xxhash64be" | "xxh64" => Some(ChecksumAlgorithm::Xxh64),
        "xxh3" => Some(ChecksumAlgorithm::Xxh3),
        "xxh128" => Some(ChecksumAlgorithm::Xxh128),
        "md5" => Some(ChecksumAlgorithm::Md5),
        "sha1" => Some(ChecksumAlgorithm::Sha1),
        _ => None,
    }
}

fn parse_iso8601(s: &str) -> Option<SystemTime> {
    DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.into())
}

/// Parses a legacy MHL v1.1 document (as written by [`render_mhl`], or by any
/// other tool using the same `mediahashlist.org` schema) back into its file
/// entries. Elements this build doesn't recognize are skipped rather than
/// failing the whole document, matching this codebase's graceful-degradation
/// stance on external/legacy formats.
pub fn parse_mhl(xml: &str) -> quick_xml::Result<Vec<ParsedMhlEntry>> {
    // Trimming is deliberately left off: indentation whitespace between
    // sibling elements is already skipped below (`current_tag` is `None`
    // there), and a leaf element's own text is never itself surrounded by
    // whitespace this writer adds -- but trimming *would* corrupt a value
    // containing an escaped entity, since quick-xml 0.41 splits text at each
    // entity boundary and trims every fragment independently (e.g. the
    // spaces around `&amp;` in `"A &amp; B"` would each get trimmed away).
    let mut reader = Reader::from_str(xml);

    let mut entries = Vec::new();
    let mut in_hash = false;
    let mut current_tag: Option<String> = None;
    let mut current_path_attributes: Option<(u64, SystemTime)> = None;
    // Text content accumulates across `Text` and `GeneralRef` events -- since
    // quick-xml 0.41 splits an entity reference like `&lt;` out of the
    // surrounding text into its own event, a single element's content can
    // arrive as several events (e.g. "A ", then a ref for `&amp;`, then " B ").
    let mut current_text = String::new();
    let mut file: Option<String> = None;
    let mut size: Option<u64> = None;
    let mut modified: Option<SystemTime> = None;
    let mut checksum: Option<String> = None;
    let mut algorithm: Option<ChecksumAlgorithm> = None;

    loop {
        match reader.read_event()? {
            Event::Eof => break,
            Event::Start(e) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if name == "hash" {
                    in_hash = true;
                    file = None;
                    size = None;
                    modified = None;
                    checksum = None;
                    algorithm = None;
                } else if in_hash {
                    current_tag = Some(name.clone());
                    current_text.clear();
                    if name == "path" {
                        let mut path_size = None;
                        let mut path_modified = None;
                        for attribute in e.attributes().flatten() {
                            match attribute.key.as_ref() {
                                b"size" => {
                                    path_size =
                                        String::from_utf8_lossy(&attribute.value).parse().ok()
                                }
                                b"lastmodificationdate" => {
                                    path_modified =
                                        parse_iso8601(&String::from_utf8_lossy(&attribute.value))
                                }
                                _ => {}
                            }
                        }
                        current_path_attributes = path_size.zip(path_modified);
                    }
                }
            }
            Event::Text(t) => {
                if current_tag.is_some() {
                    current_text.push_str(&t.decode()?);
                }
            }
            Event::GeneralRef(r) => {
                if current_tag.is_some() {
                    let resolved = match r.resolve_char_ref()? {
                        Some(ch) => Some(ch),
                        None => match r.decode()?.as_ref() {
                            "amp" => Some('&'),
                            "lt" => Some('<'),
                            "gt" => Some('>'),
                            "quot" => Some('"'),
                            "apos" => Some('\''),
                            _ => None,
                        },
                    };
                    if let Some(ch) = resolved {
                        current_text.push(ch);
                    }
                }
            }
            Event::End(e) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if name == "hash" {
                    in_hash = false;
                    if let (Some(relative_path), Some(size), Some(modified)) =
                        (file.take(), size.take(), modified.take())
                    {
                        entries.push(ParsedMhlEntry {
                            relative_path,
                            size,
                            modified,
                            checksum: checksum.take(),
                            algorithm: algorithm.take(),
                        });
                    }
                } else if in_hash {
                    match name.as_str() {
                        "file" => file = Some(std::mem::take(&mut current_text)),
                        "path" => {
                            file = Some(std::mem::take(&mut current_text));
                            if let Some((path_size, path_modified)) = current_path_attributes.take()
                            {
                                size = Some(path_size);
                                modified = Some(path_modified);
                            }
                        }
                        "size" => size = current_text.parse().ok(),
                        "lastmodificationdate" => modified = parse_iso8601(&current_text),
                        other => {
                            if let Some(algo) = algorithm_for_hash_tag(other) {
                                algorithm = Some(algo);
                                checksum = Some(std::mem::take(&mut current_text));
                            }
                        }
                    }
                }
                current_tag = None;
                current_text.clear();
            }
            _ => {}
        }
    }

    Ok(entries)
}

/// Scans `source`'s top level (the same place [`write_mhl`] itself writes to)
/// for `.mhl` files and indexes their entries by relative path, so a copy can
/// reuse an already-known-good checksum instead of hashing the source again.
/// Best-effort: a source with no MHL, or one that fails to parse, simply
/// yields an empty index rather than blocking the copy.
pub fn load_source_mhl_index(source: &Path) -> HashMap<PathBuf, ParsedMhlEntry> {
    let mut index = HashMap::new();
    let Ok(read_dir) = fs::read_dir(source) else {
        return index;
    };
    for entry in read_dir.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("mhl"))
            != Some(true)
        {
            continue;
        }
        let Ok(xml) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(entries) = parse_mhl(&xml) else {
            continue;
        };
        for parsed in entries {
            index.insert(PathBuf::from(&parsed.relative_path), parsed);
        }
    }
    index
}

/// Looks up a source-relative path in an MHL index and returns its checksum
/// only if the live file's size and modified time still match what the MHL
/// recorded (otherwise the file has changed since the MHL was written, so its
/// old checksum can't be trusted) and the checksum uses the algorithm this
/// job actually wants (a stored MD5 can't stand in for a requested XXH64).
pub fn reusable_checksum(
    index: &HashMap<PathBuf, ParsedMhlEntry>,
    relative: &Path,
    size: u64,
    modified: SystemTime,
    wanted_algorithm: ChecksumAlgorithm,
) -> Option<String> {
    let entry = index.get(relative)?;
    if entry.size != size || !mtimes_close(entry.modified, modified) {
        return None;
    }
    if entry.algorithm != Some(wanted_algorithm) {
        return None;
    }
    entry.checksum.clone()
}

/// Per-file outcome of checking an MHL's recorded entry against the real
/// file on disk, without running a transfer.
#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MhlEntryStatus {
    /// Checksum re-read from disk matches what the MHL recorded.
    Verified,
    /// Checksum re-read from disk differs -- the file has changed or been corrupted.
    Mismatch,
    /// No file exists at the recorded relative path anymore.
    Missing,
    /// The file exists but its size no longer matches the MHL (implies content changed).
    SizeMismatch,
    /// The MHL recorded this file with a `<null/>` hash (a Transfer-mode entry),
    /// so only its existence and size could be checked, not its content.
    NoChecksumRecorded,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MhlEntryResult {
    pub relative_path: String,
    pub status: MhlEntryStatus,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MhlVerifyReport {
    pub mhl_path: String,
    pub results: Vec<MhlEntryResult>,
}

/// Verifies one `.mhl` file's entries against the real files on disk, rooted
/// at the MHL's own parent directory (matching where [`write_mhl`] always
/// writes it -- the destination root the MHL describes).
pub fn verify_mhl_file(mhl_path: &Path) -> io::Result<MhlVerifyReport> {
    let xml = fs::read_to_string(mhl_path)?;
    let entries = parse_mhl(&xml)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
    let root = mhl_path.parent().unwrap_or_else(|| Path::new(""));

    let results = entries
        .into_iter()
        .map(|entry| {
            let status = verify_entry_against_disk(&entry, root);
            MhlEntryResult {
                relative_path: entry.relative_path,
                status,
            }
        })
        .collect();

    Ok(MhlVerifyReport {
        mhl_path: mhl_path.display().to_string(),
        results,
    })
}

fn is_safe_relative_path(path: &Path) -> bool {
    !path.is_absolute()
        && path.components().all(|component| {
            matches!(
                component,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
}

fn verify_entry_against_disk(entry: &ParsedMhlEntry, root: &Path) -> MhlEntryStatus {
    let relative = Path::new(&entry.relative_path);
    if !is_safe_relative_path(relative) {
        return MhlEntryStatus::Missing;
    }
    let absolute = match crate::path_safety::safe_destination(root, relative) {
        Ok(path) => path,
        Err(_) => return MhlEntryStatus::Missing,
    };
    if crate::path_safety::revalidate_destination(root, &absolute).is_err() {
        return MhlEntryStatus::Missing;
    }
    let Ok(meta) = fs::metadata(&absolute) else {
        return MhlEntryStatus::Missing;
    };
    if meta.len() != entry.size {
        return MhlEntryStatus::SizeMismatch;
    }
    match (&entry.checksum, entry.algorithm) {
        (Some(expected), Some(algorithm)) => match checksum::hash_file(&absolute, algorithm) {
            Ok(actual) if actual.eq_ignore_ascii_case(expected) => MhlEntryStatus::Verified,
            _ => MhlEntryStatus::Mismatch,
        },
        _ => MhlEntryStatus::NoChecksumRecorded,
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RepairCandidate {
    pub root: String,
    pub path: String,
    pub checksum: String,
    pub algorithm: ChecksumAlgorithm,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RepairPlan {
    pub mhl_path: String,
    pub relative_path: String,
    pub expected_checksum: String,
    pub algorithm: ChecksumAlgorithm,
    pub candidates: Vec<RepairCandidate>,
}

pub fn plan_mhl_repair(
    mhl_path: &Path,
    relative_path: &Path,
    candidate_roots: &[PathBuf],
) -> io::Result<RepairPlan> {
    if !is_safe_relative_path(relative_path) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "repair path must stay inside its roots",
        ));
    }
    let xml = fs::read_to_string(mhl_path)?;
    let entries = parse_mhl(&xml)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
    let entry = entries
        .into_iter()
        .find(|entry| Path::new(&entry.relative_path) == relative_path)
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "file is not recorded in the MHL")
        })?;
    let algorithm = entry
        .algorithm
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "MHL entry has no checksum"))?;
    let expected_checksum = entry
        .checksum
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "MHL entry has no checksum"))?;
    let destination = mhl_path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(relative_path);
    let mut seen = std::collections::HashSet::new();
    let mut candidates = Vec::new();

    for root in candidate_roots {
        let key = root.to_string_lossy().to_lowercase();
        if !seen.insert(key) {
            continue;
        }
        let candidate = match crate::path_safety::safe_destination(root, relative_path) {
            Ok(path) => path,
            Err(_) => continue,
        };
        if candidate == destination {
            continue;
        }
        let Ok(meta) = fs::metadata(&candidate) else {
            continue;
        };
        if meta.len() != entry.size {
            continue;
        }
        if crate::path_safety::revalidate_destination(root, &candidate).is_err() {
            continue;
        }
        let Ok(actual) = checksum::hash_file(&candidate, algorithm) else {
            continue;
        };
        if actual.eq_ignore_ascii_case(&expected_checksum) {
            candidates.push(RepairCandidate {
                root: root.display().to_string(),
                path: candidate.display().to_string(),
                checksum: actual,
                algorithm,
            });
        }
    }

    Ok(RepairPlan {
        mhl_path: mhl_path.display().to_string(),
        relative_path: relative_path.display().to_string(),
        expected_checksum,
        algorithm,
        candidates,
    })
}

pub fn repair_mhl_entry(
    destination_root: &Path,
    relative_path: &Path,
    source_root: &Path,
    algorithm: ChecksumAlgorithm,
    expected_checksum: &str,
    approved: bool,
) -> io::Result<()> {
    if !approved {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "repair requires explicit approval",
        ));
    }
    if !is_safe_relative_path(relative_path) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "repair path must stay inside its roots",
        ));
    }

    let source = crate::path_safety::safe_destination(source_root, relative_path)?;
    crate::path_safety::revalidate_destination(source_root, &source)?;
    let destination = crate::path_safety::safe_destination(destination_root, relative_path)?;
    let actual = checksum::hash_file(&source, algorithm)?;
    if !actual.eq_ignore_ascii_case(expected_checksum) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "repair source checksum does not match the MHL",
        ));
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
        crate::path_safety::revalidate_destination(destination_root, parent)?;
    }
    let mut staging_name = destination.file_name().unwrap_or_default().to_os_string();
    staging_name.push(".ofkit-repair");
    let staging = destination.with_file_name(staging_name);
    let result = (|| {
        fs::copy(&source, &staging)?;
        let copied = checksum::hash_file(&staging, algorithm)?;
        if !copied.eq_ignore_ascii_case(expected_checksum) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "repaired staging file failed verification",
            ));
        }
        let mut evidence = destination.with_file_name({
            let mut name = destination.file_name().unwrap_or_default().to_os_string();
            name.push(".ofkit-corrupt");
            name
        });
        let mut counter = 2;
        while evidence.exists() {
            let mut name = destination.file_name().unwrap_or_default().to_os_string();
            name.push(format!(".ofkit-corrupt.{counter}"));
            evidence = destination.with_file_name(name);
            counter += 1;
        }
        crate::path_safety::revalidate_destination(destination_root, &destination)?;
        let had_destination = destination.exists();
        if had_destination {
            fs::rename(&destination, &evidence)?;
        }
        crate::path_safety::revalidate_destination(destination_root, &destination)?;
        if let Err(err) = crate::atomic_file::replace_file(&staging, &destination) {
            if had_destination {
                if let Err(restore_err) = fs::rename(&evidence, &destination) {
                    return Err(io::Error::new(
                        restore_err.kind(),
                        format!(
                            "repair install failed: {err}; restoring corrupt original also failed: {restore_err}; evidence remains at {}",
                            evidence.display()
                        ),
                    ));
                }
            }
            return Err(err);
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(staging);
    }
    result
}

pub fn repair_mhl_entry_from_report(
    mhl_path: &Path,
    relative_path: &Path,
    source_root: &Path,
    approved: bool,
) -> io::Result<MhlVerifyReport> {
    let xml = fs::read_to_string(mhl_path)?;
    let entries = parse_mhl(&xml)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
    let entry = entries
        .into_iter()
        .find(|entry| Path::new(&entry.relative_path) == relative_path)
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "file is not recorded in the MHL")
        })?;
    let algorithm = entry
        .algorithm
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "MHL entry has no checksum"))?;
    let expected = entry
        .checksum
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "MHL entry has no checksum"))?;
    let destination_root = mhl_path.parent().unwrap_or_else(|| Path::new(""));
    repair_mhl_entry(
        destination_root,
        relative_path,
        source_root,
        algorithm,
        &expected,
        approved,
    )?;
    verify_mhl_file(mhl_path)
}

/// Finds every `.mhl` file directly inside `folder` and verifies each one
/// independently -- the "verify all MHLs on this drive/folder" batch action.
pub fn verify_mhls_in_folder(folder: &Path) -> io::Result<Vec<MhlVerifyReport>> {
    let mut reports = Vec::new();
    for entry in fs::read_dir(folder)?.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("mhl"))
            == Some(true)
        {
            reports.push(verify_mhl_file(&path)?);
        }
    }
    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn sample_entry() -> MhlFileEntry {
        MhlFileEntry {
            relative_path: "CLIP/C0001.MP4".to_string(),
            size: 1234,
            modified: SystemTime::UNIX_EPOCH + Duration::from_secs(1_600_000_000),
            checksum: Some("0ea03b369a463d9d".to_string()),
            algorithm: ChecksumAlgorithm::Xxh64,
            hashed_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1_600_000_100),
            legacy_checksum: None,
            legacy_algorithm: None,
        }
    }

    #[test]
    fn plan_repair_returns_only_checksum_verified_candidates_without_mutating_destination() {
        let destination = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        fs::write(destination.path().join("clip.mov"), b"broken").unwrap();
        fs::write(source.path().join("clip.mov"), b"good footage").unwrap();
        let expected =
            checksum::hash_file(&source.path().join("clip.mov"), ChecksumAlgorithm::Xxh64).unwrap();
        let entry = MhlFileEntry {
            relative_path: "clip.mov".to_string(),
            size: 12,
            modified: SystemTime::UNIX_EPOCH,
            checksum: Some(expected.clone()),
            algorithm: ChecksumAlgorithm::Xxh64,
            hashed_at: SystemTime::UNIX_EPOCH,
            legacy_checksum: None,
            legacy_algorithm: None,
        };
        let mhl_path = write_mhl(
            destination.path(),
            &[entry],
            SystemTime::UNIX_EPOCH,
            SystemTime::UNIX_EPOCH,
        )
        .unwrap()
        .unwrap();

        let plan = plan_mhl_repair(
            &mhl_path,
            Path::new("clip.mov"),
            &[source.path().to_path_buf()],
        )
        .unwrap();

        assert_eq!(plan.candidates.len(), 1);
        assert_eq!(plan.candidates[0].checksum, expected);
        assert_eq!(
            fs::read(destination.path().join("clip.mov")).unwrap(),
            b"broken"
        );
    }

    #[test]
    fn plan_repair_rejects_same_size_wrong_checksum_and_unsafe_paths() {
        let destination = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        fs::write(source.path().join("clip.mov"), b"wrong footage").unwrap();
        let entry = MhlFileEntry {
            relative_path: "clip.mov".to_string(),
            size: 13,
            modified: SystemTime::UNIX_EPOCH,
            checksum: Some("0000000000000000".to_string()),
            algorithm: ChecksumAlgorithm::Xxh64,
            hashed_at: SystemTime::UNIX_EPOCH,
            legacy_checksum: None,
            legacy_algorithm: None,
        };
        let mhl_path = write_mhl(
            destination.path(),
            &[entry],
            SystemTime::UNIX_EPOCH,
            SystemTime::UNIX_EPOCH,
        )
        .unwrap()
        .unwrap();

        assert!(plan_mhl_repair(
            &mhl_path,
            Path::new("clip.mov"),
            &[source.path().to_path_buf()],
        )
        .unwrap()
        .candidates
        .is_empty());
        assert!(plan_mhl_repair(
            &mhl_path,
            Path::new("../clip.mov"),
            &[source.path().to_path_buf()],
        )
        .is_err());
    }

    #[test]
    fn parses_asc_mhl_v2_xxh128_fixture_shape() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<hashlist version="2.0" xmlns="urn:ASC:MHL:v2.0">
  <hashes><hash>
    <path size="5" lastmodificationdate="2020-01-15T13:00:00Z">clip.mov</path>
    <xxh128 action="original" hashdate="2020-01-16T09:15:00Z">99aa06d3014798d86001c324468d497f</xxh128>
  </hash></hashes>
</hashlist>"#;
        let entries = parse_mhl(xml).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].relative_path, "clip.mov");
        assert_eq!(entries[0].size, 5);
        assert_eq!(entries[0].algorithm, Some(ChecksumAlgorithm::Xxh128));
    }

    #[test]
    fn renders_the_required_top_level_structure() {
        let xml = render_mhl(
            &[sample_entry()],
            SystemTime::UNIX_EPOCH,
            SystemTime::UNIX_EPOCH,
        );
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<hashlist version=\"1.1\">"));
        assert!(xml.contains("<creatorinfo>"));
        assert!(xml.contains("<username>"));
        assert!(xml.contains("<hostname>"));
        assert!(xml.contains("<startdate>"));
        assert!(xml.contains("<finishdate>"));
    }

    #[test]
    fn xxh64_entries_use_the_xxhash64be_tag() {
        let xml = render_mhl(
            &[sample_entry()],
            SystemTime::UNIX_EPOCH,
            SystemTime::UNIX_EPOCH,
        );
        assert!(xml.contains("<xxhash64be>0ea03b369a463d9d</xxhash64be>"));
    }

    #[test]
    fn md5_and_sha1_entries_use_their_own_tags() {
        let mut md5_entry = sample_entry();
        md5_entry.algorithm = ChecksumAlgorithm::Md5;
        md5_entry.checksum = Some("d41d8cd98f00b204e9800998ecf8427e".to_string());
        let xml = render_mhl(&[md5_entry], SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH);
        assert!(xml.contains("<md5>d41d8cd98f00b204e9800998ecf8427e</md5>"));

        let mut sha1_entry = sample_entry();
        sha1_entry.algorithm = ChecksumAlgorithm::Sha1;
        sha1_entry.checksum = Some("da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string());
        let xml = render_mhl(
            &[sha1_entry],
            SystemTime::UNIX_EPOCH,
            SystemTime::UNIX_EPOCH,
        );
        assert!(xml.contains("<sha1>da39a3ee5e6b4b0d3255bfef95601890afd80709</sha1>"));
    }

    #[test]
    fn transfer_mode_entries_with_no_hash_use_the_null_choice() {
        let mut entry = sample_entry();
        entry.checksum = None;
        let xml = render_mhl(&[entry], SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH);
        assert!(xml.contains("<null></null>") || xml.contains("<null/>"));
        assert!(!xml.contains("xxhash64be"));
    }

    #[test]
    fn special_characters_in_file_paths_are_escaped() {
        let mut entry = sample_entry();
        entry.relative_path = "A & B <clip>.MP4".to_string();
        let xml = render_mhl(&[entry], SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH);
        assert!(xml.contains("A &amp; B &lt;clip&gt;.MP4"));
        assert!(!xml.contains("A & B <clip>.MP4"));
    }

    #[test]
    fn write_mhl_leaves_no_temporary_document() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_mhl(
            dir.path(),
            &[sample_entry()],
            SystemTime::UNIX_EPOCH,
            SystemTime::UNIX_EPOCH,
        )
        .unwrap()
        .unwrap();

        assert!(path.is_file());
        assert!(!path
            .with_file_name(format!(
                "{}.tmp",
                path.file_name().unwrap().to_string_lossy()
            ))
            .exists());
    }

    #[test]
    fn write_mhl_is_a_no_op_when_there_are_no_entries() {
        let dir = tempfile::tempdir().unwrap();
        let result = write_mhl(
            dir.path(),
            &[],
            SystemTime::UNIX_EPOCH,
            SystemTime::UNIX_EPOCH,
        )
        .unwrap();
        assert!(result.is_none());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn two_mhls_started_in_the_same_second_get_distinct_names() {
        let dst_dir = tempfile::tempdir().unwrap();
        let started = SystemTime::UNIX_EPOCH;
        let first = write_mhl(dst_dir.path(), &[sample_entry()], started, started)
            .unwrap()
            .unwrap();
        let second = write_mhl(dst_dir.path(), &[sample_entry()], started, started)
            .unwrap()
            .unwrap();

        assert_ne!(first, second);
        assert!(first.exists());
        assert!(second.exists());
    }

    #[test]
    fn write_mhl_creates_a_real_file_at_the_destination_root() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_mhl(
            dir.path(),
            &[sample_entry()],
            SystemTime::UNIX_EPOCH,
            SystemTime::UNIX_EPOCH,
        )
        .unwrap()
        .expect("entries were provided");

        assert_eq!(path.parent().unwrap(), dir.path());
        assert!(path.extension().and_then(|e| e.to_str()) == Some("mhl"));
        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("CLIP/C0001.MP4"));
    }

    #[test]
    fn write_per_file_mhls_writes_one_sidecar_mhl_next_to_each_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("CLIP")).unwrap();

        let written = write_per_file_mhls(
            dir.path(),
            &[sample_entry()],
            SystemTime::UNIX_EPOCH,
            SystemTime::UNIX_EPOCH,
        );

        assert_eq!(written.len(), 1);
        assert_eq!(written[0], dir.path().join("CLIP").join("C0001.MP4.mhl"));
        let contents = fs::read_to_string(&written[0]).unwrap();
        assert!(contents.contains("CLIP/C0001.MP4"));
    }

    #[test]
    fn parse_mhl_round_trips_a_rendered_document() {
        let started = SystemTime::UNIX_EPOCH + Duration::from_secs(1_600_000_000);
        let xml = render_mhl(&[sample_entry()], started, started);

        let parsed = parse_mhl(&xml).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].relative_path, "CLIP/C0001.MP4");
        assert_eq!(parsed[0].size, 1234);
        assert_eq!(parsed[0].modified, sample_entry().modified);
        assert_eq!(parsed[0].checksum.as_deref(), Some("0ea03b369a463d9d"));
        assert_eq!(parsed[0].algorithm, Some(ChecksumAlgorithm::Xxh64));
    }

    #[test]
    fn parse_mhl_round_trips_escaped_special_characters() {
        let mut entry = sample_entry();
        entry.relative_path = "A & B <clip>.MP4".to_string();
        let xml = render_mhl(&[entry], SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH);

        let parsed = parse_mhl(&xml).unwrap();
        assert_eq!(parsed[0].relative_path, "A & B <clip>.MP4");
    }

    #[test]
    fn parse_mhl_treats_a_null_hash_as_no_reusable_checksum() {
        let mut entry = sample_entry();
        entry.checksum = None;
        let xml = render_mhl(&[entry], SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH);

        let parsed = parse_mhl(&xml).unwrap();
        assert_eq!(parsed[0].checksum, None);
        assert_eq!(parsed[0].algorithm, None);
    }

    #[test]
    fn load_source_mhl_index_indexes_files_from_every_mhl_at_the_source_root() {
        let dir = tempfile::tempdir().unwrap();
        let xml = render_mhl(
            &[sample_entry()],
            SystemTime::UNIX_EPOCH,
            SystemTime::UNIX_EPOCH,
        );
        fs::write(dir.path().join("card.mhl"), xml).unwrap();
        // A non-MHL file in the same folder must not confuse the scan.
        fs::write(dir.path().join("notes.txt"), "hello").unwrap();

        let index = load_source_mhl_index(dir.path());
        assert_eq!(index.len(), 1);
        assert!(index.contains_key(&PathBuf::from("CLIP/C0001.MP4")));
    }

    #[test]
    fn reusable_checksum_returns_none_when_size_or_mtime_moved_on() {
        let dir = tempfile::tempdir().unwrap();
        let xml = render_mhl(
            &[sample_entry()],
            SystemTime::UNIX_EPOCH,
            SystemTime::UNIX_EPOCH,
        );
        fs::write(dir.path().join("card.mhl"), xml).unwrap();
        let index = load_source_mhl_index(dir.path());
        let relative = PathBuf::from("CLIP/C0001.MP4");
        let recorded = sample_entry();

        assert_eq!(
            reusable_checksum(
                &index,
                &relative,
                recorded.size,
                recorded.modified,
                ChecksumAlgorithm::Xxh64
            ),
            Some("0ea03b369a463d9d".to_string()),
            "matching size/mtime/algorithm should reuse the recorded checksum"
        );
        assert_eq!(
            reusable_checksum(
                &index,
                &relative,
                recorded.size + 1,
                recorded.modified,
                ChecksumAlgorithm::Xxh64
            ),
            None,
            "a size change means the file isn't the one the MHL described"
        );
        assert_eq!(
            reusable_checksum(
                &index,
                &relative,
                recorded.size,
                recorded.modified,
                ChecksumAlgorithm::Md5
            ),
            None,
            "an MHL checksum in a different algorithm can't stand in for the requested one"
        );
    }

    #[test]
    fn malformed_mhl_paths_are_not_joined_outside_the_mhl_root() {
        let sandbox = tempfile::tempdir().unwrap();
        let root = sandbox.path().join("mhl-root");
        fs::create_dir(&root).unwrap();
        let outside = sandbox.path().join("outside.bin");
        fs::write(&outside, b"outside").unwrap();
        let entry = ParsedMhlEntry {
            relative_path: "../outside.bin".to_string(),
            size: 7,
            modified: SystemTime::UNIX_EPOCH,
            checksum: Some(checksum::hash_file(&outside, ChecksumAlgorithm::Xxh64).unwrap()),
            algorithm: Some(ChecksumAlgorithm::Xxh64),
        };

        assert_eq!(
            verify_entry_against_disk(&entry, &root),
            MhlEntryStatus::Missing
        );
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn verify_treats_a_file_through_a_directory_link_as_missing() {
        let sandbox = tempfile::tempdir().unwrap();
        let root = sandbox.path().join("mhl-root");
        let outside = sandbox.path().join("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("clip.mov"), b"outside bytes").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, root.join("linked")).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&outside, root.join("linked")).unwrap();
        let entry = ParsedMhlEntry {
            relative_path: "linked/clip.mov".to_string(),
            size: 13,
            modified: SystemTime::UNIX_EPOCH,
            checksum: Some(
                checksum::hash_file(&outside.join("clip.mov"), ChecksumAlgorithm::Xxh64).unwrap(),
            ),
            algorithm: Some(ChecksumAlgorithm::Xxh64),
        };

        assert_eq!(
            verify_entry_against_disk(&entry, &root),
            MhlEntryStatus::Missing
        );
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn repair_rejects_a_source_path_through_a_directory_link() {
        let sandbox = tempfile::tempdir().unwrap();
        let destination = sandbox.path().join("destination");
        let outside = sandbox.path().join("outside");
        let source = sandbox.path().join("source");
        fs::create_dir_all(&destination).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::create_dir_all(&source).unwrap();
        fs::write(outside.join("clip.mov"), b"outside source bytes").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, source.join("linked")).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&outside, source.join("linked")).unwrap();
        let expected =
            checksum::hash_file(&outside.join("clip.mov"), ChecksumAlgorithm::Xxh64).unwrap();

        assert!(repair_mhl_entry(
            &destination,
            Path::new("linked/clip.mov"),
            &source,
            ChecksumAlgorithm::Xxh64,
            &expected,
            true,
        )
        .is_err());
        assert!(!destination.join("linked/clip.mov").exists());
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn repair_rejects_a_destination_path_through_a_directory_link() {
        let sandbox = tempfile::tempdir().unwrap();
        let destination = sandbox.path().join("destination");
        let outside = sandbox.path().join("outside");
        let source = sandbox.path().join("source");
        fs::create_dir_all(&destination).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::create_dir_all(source.join("linked")).unwrap();
        fs::write(source.join("linked/clip.mov"), b"known good bytes").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, destination.join("linked")).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&outside, destination.join("linked")).unwrap();

        let expected =
            checksum::hash_file(&source.join("linked/clip.mov"), ChecksumAlgorithm::Xxh64).unwrap();
        assert!(repair_mhl_entry(
            &destination,
            Path::new("linked/clip.mov"),
            &source,
            ChecksumAlgorithm::Xxh64,
            &expected,
            true,
        )
        .is_err());
        assert!(!outside.join("clip.mov").exists());
    }

    #[test]
    fn repair_preserves_previous_evidence_with_a_unique_name() {
        let destination = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        fs::write(destination.path().join("clip.mov"), b"current bad bytes").unwrap();
        fs::write(
            destination.path().join("clip.mov.ofkit-corrupt"),
            b"older bad bytes",
        )
        .unwrap();
        fs::write(source.path().join("clip.mov"), b"known good bytes").unwrap();
        let expected =
            checksum::hash_file(&source.path().join("clip.mov"), ChecksumAlgorithm::Xxh64).unwrap();

        repair_mhl_entry(
            destination.path(),
            Path::new("clip.mov"),
            source.path(),
            ChecksumAlgorithm::Xxh64,
            &expected,
            true,
        )
        .unwrap();

        assert_eq!(
            fs::read(destination.path().join("clip.mov.ofkit-corrupt")).unwrap(),
            b"older bad bytes"
        );
        assert_eq!(
            fs::read(destination.path().join("clip.mov.ofkit-corrupt.2")).unwrap(),
            b"current bad bytes"
        );
    }

    #[test]
    fn repair_replaces_a_corrupt_destination_from_an_explicit_good_source() {
        let destination = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        fs::write(destination.path().join("clip.mov"), b"bad bytes").unwrap();
        fs::write(source.path().join("clip.mov"), b"known good bytes").unwrap();
        let expected =
            checksum::hash_file(&source.path().join("clip.mov"), ChecksumAlgorithm::Xxh64).unwrap();

        repair_mhl_entry(
            destination.path(),
            Path::new("clip.mov"),
            source.path(),
            ChecksumAlgorithm::Xxh64,
            &expected,
            true,
        )
        .unwrap();

        assert_eq!(
            fs::read(destination.path().join("clip.mov")).unwrap(),
            b"known good bytes"
        );
        assert_eq!(
            fs::read(destination.path().join("clip.mov.ofkit-corrupt")).unwrap(),
            b"bad bytes"
        );
    }

    #[test]
    fn repair_requires_explicit_approval_before_replacing_a_file() {
        let destination = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        fs::write(destination.path().join("clip.mov"), b"bad bytes").unwrap();
        fs::write(source.path().join("clip.mov"), b"known good bytes").unwrap();
        let expected =
            checksum::hash_file(&source.path().join("clip.mov"), ChecksumAlgorithm::Xxh64).unwrap();

        assert!(repair_mhl_entry(
            destination.path(),
            Path::new("clip.mov"),
            source.path(),
            ChecksumAlgorithm::Xxh64,
            &expected,
            false,
        )
        .is_err());
        assert_eq!(
            fs::read(destination.path().join("clip.mov")).unwrap(),
            b"bad bytes"
        );
    }

    #[test]
    fn real_repair_smoke_uses_auto_save_source_and_restores_a_corrupt_copy() {
        let source_root = PathBuf::from(r"F:\Adobe Premiere Pro Auto-Save");
        let source_name = "AD--9d8890ef-c5ad-f899-d794-64788ed9c875-2026-05-29_14-11-38.prproj";
        let source = source_root.join(source_name);
        assert!(
            source.is_file(),
            "missing real Auto-Save source: {source:?}"
        );
        let destination_root = source_root.join("_OffloadKit_Repair_Smoke");
        if destination_root.exists() {
            fs::remove_dir_all(&destination_root).unwrap();
        }
        fs::create_dir_all(&destination_root).unwrap();
        let destination = destination_root.join(source_name);
        fs::copy(&source, &destination).unwrap();
        fs::write(&destination, b"INTENTIONALLY CORRUPTED TEST COPY").unwrap();
        let metadata = fs::metadata(&source).unwrap();
        let expected = checksum::hash_file(&source, ChecksumAlgorithm::Sha1).unwrap();
        let entry = MhlFileEntry {
            relative_path: source_name.to_string(),
            size: metadata.len(),
            modified: metadata.modified().unwrap(),
            checksum: Some(expected),
            algorithm: ChecksumAlgorithm::Sha1,
            hashed_at: SystemTime::now(),
            legacy_checksum: None,
            legacy_algorithm: None,
        };
        let mhl_path = write_mhl(
            &destination_root,
            &[entry],
            SystemTime::now(),
            SystemTime::now(),
        )
        .unwrap()
        .expect("real repair smoke MHL should be written");
        let report =
            repair_mhl_entry_from_report(&mhl_path, Path::new(source_name), &source_root, true)
                .unwrap();
        assert_eq!(fs::read(&destination).unwrap(), fs::read(&source).unwrap());
        assert!(destination
            .with_file_name(format!("{source_name}.ofkit-corrupt"))
            .is_file());
        assert_eq!(report.results[0].status, MhlEntryStatus::Verified);
        fs::remove_dir_all(&destination_root).unwrap();
    }

    #[test]
    fn verify_mhl_file_reports_verified_for_an_untouched_copy() {
        let dst_dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dst_dir.path().join("CLIP")).unwrap();
        let file_path = dst_dir.path().join("CLIP/C0001.MP4");
        fs::write(&file_path, b"camera footage").unwrap();
        let meta = fs::metadata(&file_path).unwrap();

        let entry = MhlFileEntry {
            relative_path: "CLIP/C0001.MP4".to_string(),
            size: meta.len(),
            modified: meta.modified().unwrap(),
            checksum: Some(checksum::hash_file(&file_path, ChecksumAlgorithm::Xxh64).unwrap()),
            algorithm: ChecksumAlgorithm::Xxh64,
            hashed_at: SystemTime::now(),
            legacy_checksum: None,
            legacy_algorithm: None,
        };
        let mhl_path = write_mhl(
            dst_dir.path(),
            &[entry],
            SystemTime::now(),
            SystemTime::now(),
        )
        .unwrap()
        .unwrap();

        let report = verify_mhl_file(&mhl_path).unwrap();
        assert_eq!(report.results.len(), 1);
        assert_eq!(report.results[0].status, MhlEntryStatus::Verified);
    }

    #[test]
    fn verify_mhl_file_detects_corruption_and_missing_files() {
        let dst_dir = tempfile::tempdir().unwrap();
        let good_path = dst_dir.path().join("good.bin");
        let corrupted_path = dst_dir.path().join("corrupted.bin");
        fs::write(&good_path, b"good content").unwrap();
        fs::write(&corrupted_path, b"original content").unwrap();

        let entries = vec![
            MhlFileEntry {
                relative_path: "good.bin".to_string(),
                size: fs::metadata(&good_path).unwrap().len(),
                modified: fs::metadata(&good_path).unwrap().modified().unwrap(),
                checksum: Some(checksum::hash_file(&good_path, ChecksumAlgorithm::Xxh64).unwrap()),
                algorithm: ChecksumAlgorithm::Xxh64,
                hashed_at: SystemTime::now(),
                legacy_checksum: None,
                legacy_algorithm: None,
            },
            MhlFileEntry {
                relative_path: "corrupted.bin".to_string(),
                size: fs::metadata(&corrupted_path).unwrap().len(),
                modified: fs::metadata(&corrupted_path).unwrap().modified().unwrap(),
                checksum: Some("deadbeefdeadbeef".to_string()),
                algorithm: ChecksumAlgorithm::Xxh64,
                hashed_at: SystemTime::now(),
                legacy_checksum: None,
                legacy_algorithm: None,
            },
            MhlFileEntry {
                relative_path: "missing.bin".to_string(),
                size: 1,
                modified: SystemTime::now(),
                checksum: Some("0000000000000000".to_string()),
                algorithm: ChecksumAlgorithm::Xxh64,
                hashed_at: SystemTime::now(),
                legacy_checksum: None,
                legacy_algorithm: None,
            },
        ];
        let mhl_path = write_mhl(
            dst_dir.path(),
            &entries,
            SystemTime::now(),
            SystemTime::now(),
        )
        .unwrap()
        .unwrap();

        let report = verify_mhl_file(&mhl_path).unwrap();
        let status = |name: &str| {
            report
                .results
                .iter()
                .find(|r| r.relative_path == name)
                .unwrap()
                .status
        };
        assert_eq!(status("good.bin"), MhlEntryStatus::Verified);
        assert_eq!(status("corrupted.bin"), MhlEntryStatus::Mismatch);
        assert_eq!(status("missing.bin"), MhlEntryStatus::Missing);
    }

    #[test]
    fn verify_mhls_in_folder_verifies_every_mhl_at_the_top_level() {
        let dst_dir = tempfile::tempdir().unwrap();
        fs::write(dst_dir.path().join("a.bin"), b"a").unwrap();
        fs::write(dst_dir.path().join("b.bin"), b"b").unwrap();

        let entry_for = |name: &str| {
            let path = dst_dir.path().join(name);
            MhlFileEntry {
                relative_path: name.to_string(),
                size: fs::metadata(&path).unwrap().len(),
                modified: fs::metadata(&path).unwrap().modified().unwrap(),
                checksum: Some(checksum::hash_file(&path, ChecksumAlgorithm::Xxh64).unwrap()),
                algorithm: ChecksumAlgorithm::Xxh64,
                hashed_at: SystemTime::now(),
                legacy_checksum: None,
                legacy_algorithm: None,
            }
        };
        let first_path = write_mhl(
            dst_dir.path(),
            &[entry_for("a.bin")],
            SystemTime::now(),
            SystemTime::now(),
        )
        .unwrap()
        .unwrap();
        // Filenames are timestamp-based to the second -- rename the first one
        // out of the way so a second MHL written moments later (e.g. from a
        // Resume that only covers what was added afterward) can't collide
        // with and overwrite it.
        fs::rename(&first_path, dst_dir.path().join("first.mhl")).unwrap();
        write_mhl(
            dst_dir.path(),
            &[entry_for("b.bin")],
            SystemTime::now(),
            SystemTime::now(),
        )
        .unwrap();

        let reports = verify_mhls_in_folder(dst_dir.path()).unwrap();
        assert_eq!(reports.len(), 2);
        for report in &reports {
            assert_eq!(report.results[0].status, MhlEntryStatus::Verified);
        }
    }
}
