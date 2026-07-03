use std::fs;
use std::io::Read;
use std::path::Path;

use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use xxhash_rust::xxh64::Xxh64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChecksumAlgorithm {
    Xxh64,
    Md5,
    Sha1,
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
}

impl StreamingHasher {
    pub fn new(algorithm: ChecksumAlgorithm) -> Self {
        match algorithm {
            ChecksumAlgorithm::Xxh64 => StreamingHasher::Xxh64(Xxh64::new(0)),
            ChecksumAlgorithm::Md5 => StreamingHasher::Md5(Md5::new()),
            ChecksumAlgorithm::Sha1 => StreamingHasher::Sha1(Sha1::new()),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        match self {
            StreamingHasher::Xxh64(h) => h.update(data),
            StreamingHasher::Md5(h) => h.update(data),
            StreamingHasher::Sha1(h) => h.update(data),
        }
    }

    /// Hex digest. XXH64 is formatted big-endian (the "XXH64BE" convention
    /// used by MHL tooling in this space) by printing the u64 as hex text.
    pub fn finalize_hex(self) -> String {
        match self {
            StreamingHasher::Xxh64(h) => format!("{:016x}", h.digest()),
            StreamingHasher::Md5(h) => hex::encode(h.finalize()),
            StreamingHasher::Sha1(h) => hex::encode(h.finalize()),
        }
    }
}

const HASH_BUFFER_SIZE: usize = 1024 * 1024;

/// Hashes a file from disk in one independent read pass. Used to verify a
/// destination file after copying (Source & Destination verification mode).
pub fn hash_file(path: &Path, algorithm: ChecksumAlgorithm) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = StreamingHasher::new(algorithm);
    let mut buffer = vec![0u8; HASH_BUFFER_SIZE];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hasher.finalize_hex())
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
