use crate::error::AssetError;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The 15.30 `appearances` file is 5,017,996 bytes.
pub const MAX_APPEARANCES_BYTES: u64 = 8 * 1024 * 1024;
/// `catalog-content.json` is 1,042,224 bytes.
pub const MAX_CATALOG_BYTES: u64 = 2 * 1024 * 1024;
/// A compressed sprite sheet.
pub const MAX_COMPRESSED_SHEET_BYTES: u64 = 2 * 1024 * 1024;
/// A decompressed sprite sheet: a 384x384 RGBA bitmap plus header slack.
pub const MAX_DECOMPRESSED_SHEET_BYTES: usize = 384 * 384 * 4 + 65_536;
/// The 15.30 manifest lists 6,249 files.
pub const MAX_MANIFEST_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Deserialize)]
struct RawManifest {
    files: Vec<RawManifestFile>,
}

#[derive(Deserialize)]
struct RawManifestFile {
    name: String,
    sha256: String,
}

/// An asset directory plus the manifest that pins every file in it.
#[derive(Debug, Clone)]
pub struct AssetStore {
    dir: PathBuf,
    manifest: HashMap<String, String>,
}

impl AssetStore {
    /// Opens `dir` against the manifest file at `manifest_path`.
    pub fn open(dir: &Path, manifest_path: &Path) -> Result<Self, AssetError> {
        let label = manifest_path.display().to_string();
        let bytes = read_bounded(manifest_path, MAX_MANIFEST_BYTES, &label)?;
        let raw: RawManifest =
            serde_json::from_slice(&bytes).map_err(|error| AssetError::Malformed {
                file: label.clone(),
                reason: error.to_string(),
            })?;
        let manifest = raw
            .files
            .into_iter()
            .map(|file| (file.name, file.sha256.to_ascii_lowercase()))
            .collect();
        Ok(Self {
            dir: dir.to_path_buf(),
            manifest,
        })
    }

    /// Reads `name` from the asset directory and checks it against its manifest `sha256`.
    /// The hash token inside a file name is never the check.
    pub fn read_verified(&self, name: &str, max_bytes: u64) -> Result<Vec<u8>, AssetError> {
        if !is_plain_file_name(name) {
            return Err(AssetError::UnsafeFileName {
                file: name.to_owned(),
            });
        }
        let expected = self
            .manifest
            .get(name)
            .ok_or_else(|| AssetError::NotInManifest {
                file: name.to_owned(),
            })?;
        let bytes = read_bounded(&self.dir.join(name), max_bytes, name)?;
        if &sha256_hex(&bytes) != expected {
            return Err(AssetError::HashMismatch {
                file: name.to_owned(),
            });
        }
        Ok(bytes)
    }
}

pub(crate) fn is_plain_file_name(name: &str) -> bool {
    !(name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0'))
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn read_bounded(path: &Path, max_bytes: u64, label: &str) -> Result<Vec<u8>, AssetError> {
    let io_error = |error: std::io::Error| AssetError::Io {
        file: label.to_owned(),
        message: error.to_string(),
    };
    let file = File::open(path).map_err(io_error)?;
    let mut bytes = Vec::new();
    // One byte past the bound tells an oversized file from one that exactly fits.
    file.take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() as u64 > max_bytes {
        return Err(AssetError::TooLarge {
            file: label.to_owned(),
            limit: max_bytes,
        });
    }
    Ok(bytes)
}
