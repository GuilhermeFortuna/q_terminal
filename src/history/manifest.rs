use std::path::{Component, Path, PathBuf};

use q_io::DigestAlgorithm;

use crate::history::catalog::DatasetManifest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestFile {
    pub path: String,
    pub size_bytes: u64,
    pub checksum: String,
}

/// A published manifest, destructured and validated out of the vendored type.
#[derive(Debug, Clone, PartialEq)]
pub struct Dataset {
    pub id: String,
    pub version: i64,
    pub published_at: String,
    pub algorithm: DigestAlgorithm,
    pub files: Vec<ManifestFile>,
    pub row_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    NotPublished,
    BadPath { path: String },
    UnsupportedDigest { name: String },
    Shape { field: &'static str },
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::NotPublished => write!(f, "dataset is not published"),
            ManifestError::BadPath { path } => write!(f, "refused manifest path `{path}`"),
            ManifestError::UnsupportedDigest { name } => {
                write!(f, "unsupported digest algorithm `{name}`")
            }
            ManifestError::Shape { field } => {
                write!(f, "manifest missing required field `{field}`")
            }
        }
    }
}

impl std::error::Error for ManifestError {}

fn parse_file_entry(value: &serde_json::Value) -> Result<ManifestFile, ManifestError> {
    let path = value
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or(ManifestError::Shape {
            field: "files.path",
        })?;
    let size_bytes = value
        .get("size_bytes")
        .and_then(|v| v.as_i64())
        .filter(|v| *v >= 0)
        .ok_or(ManifestError::Shape {
            field: "files.size_bytes",
        })?;
    let checksum = value
        .get("checksum")
        .and_then(|v| v.as_str())
        .ok_or(ManifestError::Shape {
            field: "files.checksum",
        })?;
    Ok(ManifestFile {
        path: path.to_string(),
        size_bytes: size_bytes as u64,
        checksum: checksum.to_string(),
    })
}

pub fn dataset_from_manifest(manifest: &DatasetManifest) -> Result<Dataset, ManifestError> {
    if manifest.state != "published" {
        return Err(ManifestError::NotPublished);
    }

    if manifest.dataset_id.is_empty() {
        return Err(ManifestError::Shape {
            field: "dataset_id",
        });
    }
    if manifest.published_at.is_empty() {
        return Err(ManifestError::Shape {
            field: "published_at",
        });
    }
    if manifest.checksum_algorithm.is_empty() {
        return Err(ManifestError::Shape {
            field: "checksum_algorithm",
        });
    }
    if manifest.row_count < 0 {
        return Err(ManifestError::Shape { field: "row_count" });
    }

    let algorithm = match DigestAlgorithm::parse(&manifest.checksum_algorithm) {
        Ok(algorithm) => algorithm,
        Err(q_io::IoError::UnsupportedDigest { algorithm }) => {
            return Err(ManifestError::UnsupportedDigest { name: algorithm });
        }
        Err(_) => {
            return Err(ManifestError::Shape {
                field: "checksum_algorithm",
            });
        }
    };

    let mut files = Vec::with_capacity(manifest.files.len());
    for entry in &manifest.files {
        files.push(parse_file_entry(entry)?);
    }

    Ok(Dataset {
        id: manifest.dataset_id.clone(),
        version: manifest.version,
        published_at: manifest.published_at.clone(),
        algorithm,
        files,
        row_count: manifest.row_count as usize,
    })
}

/// Picks the current dataset: highest `version` with `state == "published"`.
pub fn select_current(datasets: &[DatasetManifest]) -> Result<Option<Dataset>, ManifestError> {
    let mut best: Option<Dataset> = None;
    for manifest in datasets {
        if manifest.state != "published" {
            continue;
        }
        let candidate = dataset_from_manifest(manifest)?;
        match &best {
            None => best = Some(candidate),
            Some(current) if candidate.version > current.version => best = Some(candidate),
            _ => {}
        }
    }
    Ok(best)
}

fn path_has_parent_segment(path: &str) -> bool {
    Path::new(path)
        .components()
        .any(|component| component == Component::ParentDir)
}

fn path_stays_under_root(root: &Path, candidate: &Path) -> bool {
    let root_parts: Vec<_> = root
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect();
    let candidate_parts: Vec<_> = candidate
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect();
    if candidate_parts.len() < root_parts.len() {
        return false;
    }
    candidate_parts[..root_parts.len()] == root_parts[..]
}

/// Joins a lake-relative path onto the catalog's root, refusing an absolute
/// path, a parent segment, or any result outside the root.
pub fn resolve(root: &Path, path: &str) -> Result<PathBuf, ManifestError> {
    if path.starts_with('/') || path.starts_with('\\') || path_has_parent_segment(path) {
        return Err(ManifestError::BadPath {
            path: path.to_string(),
        });
    }

    let joined = root.join(path);
    if !path_stays_under_root(root, &joined) {
        return Err(ManifestError::BadPath {
            path: path.to_string(),
        });
    }

    Ok(joined)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn manifest(
        version: i64,
        state: &str,
        algorithm: &str,
        files: Vec<serde_json::Value>,
    ) -> DatasetManifest {
        DatasetManifest {
            arrow_schema: json!({}),
            checksum_algorithm: algorithm.to_string(),
            dataset_id: format!("ds-{version}"),
            files,
            published_at: format!("2026-01-0{version}T00:00:00Z"),
            row_count: 100,
            state: state.to_string(),
            subject: json!({"kind": "bars", "symbol": "WINFUT", "timeframe": "M1"}),
            supersedes: None,
            time_range: json!({}),
            tombstone: None,
            version,
        }
    }

    fn sample_file(path: &str) -> serde_json::Value {
        json!({
            "path": path,
            "size_bytes": 128,
            "checksum": "abc"
        })
    }

    #[test]
    fn select_highest_published_version() {
        let datasets = vec![
            manifest(1, "published", "sha256", vec![sample_file("a.parquet")]),
            manifest(2, "published", "sha256", vec![sample_file("b.parquet")]),
            manifest(3, "tombstoned", "sha256", vec![sample_file("c.parquet")]),
        ];
        let selected = select_current(&datasets).unwrap().unwrap();
        assert_eq!(selected.version, 2);
        assert_eq!(selected.id, "ds-2");
    }

    #[test]
    fn select_all_tombstoned_returns_none() {
        let datasets = vec![
            manifest(1, "tombstoned", "sha256", vec![sample_file("a.parquet")]),
            manifest(2, "deleted", "sha256", vec![sample_file("b.parquet")]),
        ];
        assert!(select_current(&datasets).unwrap().is_none());
    }

    #[test]
    fn select_skips_publishing_state() {
        let datasets = vec![
            manifest(1, "publishing", "sha256", vec![sample_file("a.parquet")]),
            manifest(2, "published", "sha256", vec![sample_file("b.parquet")]),
        ];
        let selected = select_current(&datasets).unwrap().unwrap();
        assert_eq!(selected.version, 2);
    }

    #[test]
    fn missing_required_field_reports_shape() {
        let mut bad = manifest(1, "published", "sha256", vec![sample_file("a.parquet")]);
        bad.dataset_id = String::new();
        let err = select_current(&[bad]).unwrap_err();
        assert_eq!(
            err,
            ManifestError::Shape {
                field: "dataset_id"
            }
        );
    }

    #[test]
    fn unsupported_digest_algorithm_is_reported() {
        let datasets = vec![manifest(
            1,
            "published",
            "blake3",
            vec![sample_file("a.parquet")],
        )];
        let err = select_current(&datasets).unwrap_err();
        assert_eq!(
            err,
            ManifestError::UnsupportedDigest {
                name: "blake3".to_string()
            }
        );
    }

    #[test]
    fn resolve_refuses_absolute_parent_and_escape() {
        let root = Path::new("/lake/root");
        for path in ["/etc/passwd", "../../etc/passwd", "a/../../b"] {
            let err = resolve(root, path).unwrap_err();
            assert_eq!(
                err,
                ManifestError::BadPath {
                    path: path.to_string()
                }
            );
        }
    }

    #[test]
    fn resolve_accepts_valid_relative_path() {
        let root = Path::new("/lake/root");
        let resolved = resolve(root, "bars/WINFUT/M1/part-0.parquet").unwrap();
        assert_eq!(
            resolved,
            PathBuf::from("/lake/root/bars/WINFUT/M1/part-0.parquet")
        );
    }
}
