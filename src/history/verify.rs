use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::history::manifest::{resolve, Dataset};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Verified,
    SizeMismatch { path: String },
    DigestMismatch { path: String },
    Missing { path: String },
}

#[derive(Debug, Default)]
pub struct VerifiedSet {
    verified: HashSet<(String, String)>,
}

impl VerifiedSet {
    pub fn contains(&self, dataset_id: &str, published_at: &str) -> bool {
        self.verified
            .contains(&(dataset_id.to_string(), published_at.to_string()))
    }
}

/// Sizes then digests every file. Remembered per (dataset_id, published_at) for
/// the session; a change in either re-verifies.
pub fn verify(dataset: &Dataset, root: &Path, cache: &mut VerifiedSet) -> Verdict {
    let key = (dataset.id.clone(), dataset.published_at.clone());
    if cache.verified.contains(&key) {
        return Verdict::Verified;
    }

    for file in &dataset.files {
        let path = match resolve(root, &file.path) {
            Ok(path) => path,
            Err(_) => {
                return Verdict::Missing {
                    path: file.path.clone(),
                };
            }
        };

        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => {
                return Verdict::Missing {
                    path: file.path.clone(),
                };
            }
        };

        if metadata.len() != file.size_bytes {
            return Verdict::SizeMismatch {
                path: file.path.clone(),
            };
        }

        let digest = match q_io::digest_file(&path, dataset.algorithm) {
            Ok(digest) => digest,
            Err(q_io::IoError::FileMissing { .. }) => {
                return Verdict::Missing {
                    path: file.path.clone(),
                };
            }
            Err(_) => {
                return Verdict::DigestMismatch {
                    path: file.path.clone(),
                };
            }
        };

        if digest != file.checksum {
            return Verdict::DigestMismatch {
                path: file.path.clone(),
            };
        }
    }

    cache.verified.insert(key);
    Verdict::Verified
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::manifest::{Dataset, ManifestFile};
    use q_io::DigestAlgorithm;
    use std::io::Write;

    fn write_bytes(path: &Path, bytes: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(path).unwrap();
        file.write_all(bytes).unwrap();
    }

    fn dataset(published_at: &str, files: Vec<ManifestFile>) -> Dataset {
        Dataset {
            id: "ds-1".to_string(),
            version: 1,
            published_at: published_at.to_string(),
            algorithm: DigestAlgorithm::Sha256,
            files,
            row_count: 1,
        }
    }

    #[test]
    fn correct_dataset_verifies() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let rel = "bars/part-0.parquet";
        let abs = root.join(rel);
        write_bytes(&abs, b"parquet-bytes");

        let digest = q_io::digest_file(&abs, DigestAlgorithm::Sha256).unwrap();
        let ds = dataset(
            "2026-01-01T00:00:00Z",
            vec![ManifestFile {
                path: rel.to_string(),
                size_bytes: abs.metadata().unwrap().len(),
                checksum: digest,
            }],
        );

        let mut cache = VerifiedSet::default();
        assert_eq!(verify(&ds, root, &mut cache), Verdict::Verified);
        assert!(cache.contains("ds-1", "2026-01-01T00:00:00Z"));
    }

    #[test]
    fn flipped_byte_fails_digest() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let rel = "bars/part-0.parquet";
        let abs = root.join(rel);
        write_bytes(&abs, b"parquet-bytes");

        let digest = q_io::digest_file(&abs, DigestAlgorithm::Sha256).unwrap();
        let mut flipped = b"parquet-bytes".to_vec();
        flipped[0] ^= 0x01;
        write_bytes(&abs, &flipped);

        let ds = dataset(
            "2026-01-01T00:00:00Z",
            vec![ManifestFile {
                path: rel.to_string(),
                size_bytes: abs.metadata().unwrap().len(),
                checksum: digest,
            }],
        );

        let verdict = verify(&ds, root, &mut VerifiedSet::default());
        assert_eq!(
            verdict,
            Verdict::DigestMismatch {
                path: rel.to_string()
            }
        );
    }

    #[test]
    fn truncated_file_fails_size() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let rel = "bars/part-0.parquet";
        let abs = root.join(rel);
        write_bytes(&abs, b"12345");

        let ds = dataset(
            "2026-01-01T00:00:00Z",
            vec![ManifestFile {
                path: rel.to_string(),
                size_bytes: 10,
                checksum: "deadbeef".to_string(),
            }],
        );

        assert_eq!(
            verify(&ds, root, &mut VerifiedSet::default()),
            Verdict::SizeMismatch {
                path: rel.to_string()
            }
        );
    }

    #[test]
    fn missing_file_fails() {
        let temp = tempfile::tempdir().unwrap();
        let ds = dataset(
            "2026-01-01T00:00:00Z",
            vec![ManifestFile {
                path: "bars/missing.parquet".to_string(),
                size_bytes: 1,
                checksum: "abc".to_string(),
            }],
        );
        assert_eq!(
            verify(&ds, temp.path(), &mut VerifiedSet::default()),
            Verdict::Missing {
                path: "bars/missing.parquet".to_string()
            }
        );
    }

    #[test]
    fn second_verify_uses_cache_even_if_file_corrupted() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let rel = "bars/part-0.parquet";
        let abs = root.join(rel);
        write_bytes(&abs, b"parquet-bytes");
        let digest = q_io::digest_file(&abs, DigestAlgorithm::Sha256).unwrap();
        let ds = dataset(
            "2026-01-01T00:00:00Z",
            vec![ManifestFile {
                path: rel.to_string(),
                size_bytes: abs.metadata().unwrap().len(),
                checksum: digest,
            }],
        );

        let mut cache = VerifiedSet::default();
        assert_eq!(verify(&ds, root, &mut cache), Verdict::Verified);

        write_bytes(&abs, b"corrupted");
        assert_eq!(verify(&ds, root, &mut cache), Verdict::Verified);
    }

    #[test]
    fn changed_published_at_reverifies() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let rel = "bars/part-0.parquet";
        let abs = root.join(rel);
        write_bytes(&abs, b"parquet-bytes");
        let digest = q_io::digest_file(&abs, DigestAlgorithm::Sha256).unwrap();
        let file = ManifestFile {
            path: rel.to_string(),
            size_bytes: abs.metadata().unwrap().len(),
            checksum: digest,
        };

        let mut cache = VerifiedSet::default();
        assert_eq!(
            verify(
                &dataset("2026-01-01T00:00:00Z", vec![file.clone()]),
                root,
                &mut cache
            ),
            Verdict::Verified
        );

        let mut flipped = b"parquet-bytes".to_vec();
        flipped[0] ^= 0x01;
        write_bytes(&abs, &flipped);
        let verdict = verify(
            &dataset("2026-01-02T00:00:00Z", vec![file]),
            root,
            &mut cache,
        );
        assert_eq!(
            verdict,
            Verdict::DigestMismatch {
                path: rel.to_string()
            }
        );
    }
}
