use std::fs;
use std::io;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, SecondsFormat, Utc};
use quick_xml::events::BytesText;
use quick_xml::writer::Writer;

use crate::checksum::ChecksumAlgorithm;

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
}

pub(crate) fn iso8601(t: SystemTime) -> String {
    let dt: DateTime<Utc> = t.into();
    dt.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Element name for a hash choice, per the legacy MHL v1.1 XSD
/// (mediahashlist.org) -- XXH64 is written big-endian under the
/// `xxhash64be` tag, matching the "XXH64BE" convention this codebase
/// already uses (see checksum::StreamingHasher::finalize_hex).
fn hash_tag(algorithm: ChecksumAlgorithm) -> &'static str {
    match algorithm {
        ChecksumAlgorithm::Xxh64 => "xxhash64be",
        ChecksumAlgorithm::Md5 => "md5",
        ChecksumAlgorithm::Sha1 => "sha1",
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
pub fn render_mhl(entries: &[MhlFileEntry], started_at: SystemTime, finished_at: SystemTime) -> String {
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
                writer.create_element("hash").write_inner_content(|writer| {
                    write_text_element(writer, "file", &entry.relative_path)?;
                    write_text_element(writer, "size", &entry.size.to_string())?;
                    write_text_element(
                        writer,
                        "lastmodificationdate",
                        &iso8601(entry.modified),
                    )?;
                    match &entry.checksum {
                        Some(hash) => write_text_element(writer, hash_tag(entry.algorithm), hash)?,
                        None => write_text_element(writer, "null", "")?,
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
    let filename = format!("{}.mhl", dt.format("%Y%m%d_%H%M%S"));
    let path = destination.join(filename);
    fs::write(&path, render_mhl(entries, started_at, finished_at))?;
    Ok(Some(path))
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
        }
    }

    #[test]
    fn renders_the_required_top_level_structure() {
        let xml = render_mhl(&[sample_entry()], SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH);
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
        let xml = render_mhl(&[sample_entry()], SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH);
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
        let xml = render_mhl(&[sha1_entry], SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH);
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
    fn write_mhl_is_a_no_op_when_there_are_no_entries() {
        let dir = tempfile::tempdir().unwrap();
        let result = write_mhl(dir.path(), &[], SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH).unwrap();
        assert!(result.is_none());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn write_mhl_creates_a_real_file_at_the_destination_root() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_mhl(dir.path(), &[sample_entry()], SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH)
            .unwrap()
            .expect("entries were provided");

        assert_eq!(path.parent().unwrap(), dir.path());
        assert!(path.extension().and_then(|e| e.to_str()) == Some("mhl"));
        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("CLIP/C0001.MP4"));
    }
}
