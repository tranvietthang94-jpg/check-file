use std::fs;
use std::io::Read;
use std::path::Path;

use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest as _, Sha512};
use xxhash_rust::xxh64::Xxh64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChecksumAlgorithm {
    Xxh64,
    Md5,
    Sha1,
    /// OffShoot's legacy "C4" option: a SHA-512 digest re-encoded as a C4 ID
    /// (`c4` + 88-digit base58), per the Avid/ASC C4 identifier spec
    /// (https://github.com/Avid-Technology/c4/blob/master/c4.md).
    C4,
}

impl Default for ChecksumAlgorithm {
    fn default() -> Self {
        ChecksumAlgorithm::Xxh64
    }
}

/// Incremental hasher so checksums can be computed inline while a file is
/// streamed for copying, instead of requiring a second read pass.
pub enum StreamingHasher {
    Xxh64(Xxh64),
    Md5(Md5),
    Sha1(Sha1),
    C4(Sha512),
}

impl StreamingHasher {
    pub fn new(algorithm: ChecksumAlgorithm) -> Self {
        match algorithm {
            ChecksumAlgorithm::Xxh64 => StreamingHasher::Xxh64(Xxh64::new(0)),
            ChecksumAlgorithm::Md5 => StreamingHasher::Md5(Md5::new()),
            ChecksumAlgorithm::Sha1 => StreamingHasher::Sha1(Sha1::new()),
            ChecksumAlgorithm::C4 => StreamingHasher::C4(Sha512::new()),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        match self {
            StreamingHasher::Xxh64(h) => h.update(data),
            StreamingHasher::Md5(h) => h.update(data),
            StreamingHasher::Sha1(h) => h.update(data),
            StreamingHasher::C4(h) => h.update(data),
        }
    }

    /// Hex digest for Xxh64/Md5/Sha1. XXH64 is formatted big-endian (the
    /// "XXH64BE" convention used by MHL tooling in this space) by printing
    /// the u64 as hex text. C4 instead formats its SHA-512 digest as a C4 ID
    /// (see [`c4_id_from_digest`]), which isn't hex but is still the
    /// canonical textual form for that algorithm.
    pub fn finalize_hex(self) -> String {
        match self {
            StreamingHasher::Xxh64(h) => format!("{:016x}", h.digest()),
            StreamingHasher::Md5(h) => hex::encode(h.finalize()),
            StreamingHasher::Sha1(h) => hex::encode(h.finalize()),
            StreamingHasher::C4(h) => c4_id_from_digest(&h.finalize()),
        }
    }
}

/// Encodes a digest (a SHA-512 output, 64 bytes) as a C4 identifier: the
/// 2-character prefix `"c4"` followed by the digest -- treated as a single
/// big-endian unsigned integer -- base58-encoded and left-padded with `'1'`
/// to a fixed 88 digits. Verified against the two official all-bytes-known
/// test vectors in the normative C4 spec (all-zero digest, and the decimal
/// value 123456789) -- see the tests below.
/// Spec: https://github.com/Avid-Technology/c4/blob/master/c4.md
fn c4_id_from_digest(digest: &[u8]) -> String {
    const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let mut buf = digest.to_vec();
    let mut digits = Vec::new();
    while buf.iter().any(|&b| b != 0) {
        let mut remainder: u32 = 0;
        for byte in buf.iter_mut() {
            let value = (remainder << 8) | (*byte as u32);
            *byte = (value / 58) as u8;
            remainder = value % 58;
        }
        digits.push(ALPHABET[remainder as usize]);
    }
    digits.reverse();
    let mut suffix = String::from_utf8(digits).expect("C4 alphabet is ASCII");
    while suffix.len() < 88 {
        suffix.insert(0, '1');
    }
    format!("c4{suffix}")
}

const HASH_BUFFER_SIZE: usize = 1024 * 1024;

/// Hashes a file from disk in one independent read pass. Used to verify a
/// destination file after copying (Source & Destination verification mode).
pub fn hash_file(path: &Path, algorithm: ChecksumAlgorithm) -> std::io::Result<String> {
    let (primary, _) = hash_file_dual(path, algorithm, None)?;
    Ok(primary)
}

/// Like [`hash_file`], but also feeds a second "legacy" algorithm off the
/// same read pass when `legacy_algorithm` is set -- OffShoot's "Also
/// generate legacy checksums" preference runs a second hash alongside the
/// primary one without re-reading the file a second time.
pub fn hash_file_dual(
    path: &Path,
    algorithm: ChecksumAlgorithm,
    legacy_algorithm: Option<ChecksumAlgorithm>,
) -> std::io::Result<(String, Option<String>)> {
    let mut file = fs::File::open(path)?;
    let mut hasher = StreamingHasher::new(algorithm);
    let mut legacy_hasher = legacy_algorithm.map(StreamingHasher::new);
    let mut buffer = vec![0u8; HASH_BUFFER_SIZE];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
        if let Some(h) = legacy_hasher.as_mut() {
            h.update(&buffer[..n]);
        }
    }
    Ok((hasher.finalize_hex(), legacy_hasher.map(|h| h.finalize_hex())))
}

/// Re-reads `path` and compares its checksum against `expected_hash`.
pub fn verify_file_hash(
    path: &Path,
    expected_hash: &str,
    algorithm: ChecksumAlgorithm,
) -> std::io::Result<bool> {
    let actual = hash_file(path, algorithm)?;
    Ok(actual.eq_ignore_ascii_case(expected_hash))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_matches_known_vector_for_empty_input() {
        let mut hasher = StreamingHasher::new(ChecksumAlgorithm::Md5);
        hasher.update(b"");
        assert_eq!(hasher.finalize_hex(), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn sha1_matches_known_vector_for_empty_input() {
        let mut hasher = StreamingHasher::new(ChecksumAlgorithm::Sha1);
        hasher.update(b"");
        assert_eq!(
            hasher.finalize_hex(),
            "da39a3ee5e6b4b0d3255bfef95601890afd80709"
        );
    }

    #[test]
    fn xxh64_is_deterministic_and_content_sensitive() {
        let mut a = StreamingHasher::new(ChecksumAlgorithm::Xxh64);
        a.update(b"offloadkit");
        let hash_a = a.finalize_hex();

        let mut b = StreamingHasher::new(ChecksumAlgorithm::Xxh64);
        b.update(b"offloadkit");
        let hash_b = b.finalize_hex();
        assert_eq!(hash_a, hash_b, "same input must hash identically");
        assert_eq!(hash_a.len(), 16, "xxh64 hex digest is 16 chars (64 bits)");

        let mut c = StreamingHasher::new(ChecksumAlgorithm::Xxh64);
        c.update(b"offloadkit!");
        assert_ne!(hash_a, c.finalize_hex(), "different input must hash differently");
    }

    #[test]
    fn streaming_update_in_chunks_matches_single_shot_update() {
        let data = b"the quick brown fox jumps over the lazy dog";

        let mut chunked = StreamingHasher::new(ChecksumAlgorithm::Xxh64);
        chunked.update(&data[..10]);
        chunked.update(&data[10..]);

        let mut single = StreamingHasher::new(ChecksumAlgorithm::Xxh64);
        single.update(data);

        assert_eq!(chunked.finalize_hex(), single.finalize_hex());
    }

    #[test]
    fn hash_file_dual_computes_both_algorithms_from_one_read_pass() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.bin");
        fs::write(&path, b"camera footage").unwrap();

        let (primary, legacy) =
            hash_file_dual(&path, ChecksumAlgorithm::Xxh64, Some(ChecksumAlgorithm::Sha1)).unwrap();

        assert_eq!(primary, hash_file(&path, ChecksumAlgorithm::Xxh64).unwrap());
        assert_eq!(legacy, Some(hash_file(&path, ChecksumAlgorithm::Sha1).unwrap()));
    }

    #[test]
    fn hash_file_dual_without_a_legacy_algorithm_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.bin");
        fs::write(&path, b"camera footage").unwrap();

        let (_, legacy) = hash_file_dual(&path, ChecksumAlgorithm::Xxh64, None).unwrap();
        assert_eq!(legacy, None);
    }

    #[test]
    fn c4_id_matches_official_all_zero_test_vector() {
        let digest = [0u8; 64];
        assert_eq!(c4_id_from_digest(&digest), format!("c4{}", "1".repeat(88)));
    }

    #[test]
    fn c4_id_matches_official_123456789_test_vector() {
        // 123456789 decimal == 0x075BCD15, placed in the last 4 bytes of a
        // 64-byte big-endian buffer (the other 60 bytes are zero).
        let mut digest = [0u8; 64];
        digest[60..].copy_from_slice(&[0x07, 0x5B, 0xCD, 0x15]);
        let expected = format!("c4{}{}", "1".repeat(83), "BukQL");
        assert_eq!(c4_id_from_digest(&digest), expected);
    }

    #[test]
    fn c4_hasher_is_deterministic_content_sensitive_and_well_formed() {
        let mut a = StreamingHasher::new(ChecksumAlgorithm::C4);
        a.update(b"offloadkit");
        let id_a = a.finalize_hex();

        let mut b = StreamingHasher::new(ChecksumAlgorithm::C4);
        b.update(b"offloadkit");
        assert_eq!(id_a, b.finalize_hex(), "same input must hash identically");

        let mut c = StreamingHasher::new(ChecksumAlgorithm::C4);
        c.update(b"offloadkit!");
        assert_ne!(id_a, c.finalize_hex(), "different input must hash differently");

        assert_eq!(id_a.len(), 90, "c4 + 88 base58 digits");
        assert!(id_a.starts_with("c4"));
        assert!(
            id_a[2..]
                .bytes()
                .all(|b| b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz".contains(&b)),
            "suffix must only use the C4 base58 alphabet"
        );
    }

    #[test]
    fn verify_file_hash_detects_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.bin");
        fs::write(&path, b"original content").unwrap();

        let good_hash = hash_file(&path, ChecksumAlgorithm::Xxh64).unwrap();
        assert!(verify_file_hash(&path, &good_hash, ChecksumAlgorithm::Xxh64).unwrap());

        fs::write(&path, b"tampered content").unwrap();
        assert!(!verify_file_hash(&path, &good_hash, ChecksumAlgorithm::Xxh64).unwrap());
    }
}
