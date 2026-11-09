//! Installed presentation-package verification before login or asset loading.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fmt, fs::File, io::Read, path::Path};
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Copy)]
pub struct Identity<'a> {
    pub release_id: &'a str,
    pub client_version: &'a str,
    pub channel: &'a str,
    pub game_commit: &'a str,
    pub target: &'a str,
}
impl Identity<'static> {
    pub fn compiled() -> Self {
        Self {
            release_id: crate::RELEASE_ID,
            client_version: env!("CARGO_PKG_VERSION"),
            channel: crate::RELEASE_ID
                .split_once('+')
                .and_then(|(_, tail)| tail.split('.').next())
                .unwrap_or("invalid"),
            game_commit: env!("OTERYN_GAME_COMMIT"),
            target: env!("OTERYN_BUILD_TARGET"),
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum Report {
    Development,
    Verified { packages: usize, bytes: u64 },
}
impl fmt::Display for Report {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Development => {
                out.write_str("Local development build; no installed packages manifest.")
            }
            Self::Verified { packages, bytes } => write!(
                out,
                "Verified {packages} presentation package(s), {bytes} bytes."
            ),
        }
    }
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("Missing or invalid manifest field: {key}"))
}
fn lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
/// All five release fields match this executable before any package is opened.
pub fn verify(directory: &Path, expected: Identity<'_>) -> Result<Report, String> {
    let root = directory
        .canonicalize()
        .map_err(|_| "Cannot locate the installed release directory")?;
    let manifest = root
        .join("packages.json")
        .canonicalize()
        .map_err(|_| "packages.json is missing or unreadable")?;
    if !manifest.starts_with(&root) {
        return Err("packages.json leaves the release directory".into());
    }
    let file = File::open(manifest).map_err(|_| "Cannot open packages.json")?;
    let mut json = String::new();
    file.take(MAX_MANIFEST_BYTES + 1)
        .read_to_string(&mut json)
        .map_err(|_| "Cannot read packages.json")?;
    if json.len() as u64 > MAX_MANIFEST_BYTES {
        return Err("packages.json exceeds 1 MiB".into());
    }
    let document: Value = serde_json::from_str(&json).map_err(|_| "Malformed packages.json")?;
    if text(&document, "schema")? != "oteryn.client.packages.v1" {
        return Err("Unknown packages manifest schema".into());
    }
    let release = &document["release"];
    for (key, expected) in [
        ("release_id", expected.release_id),
        ("client_version", expected.client_version),
        ("channel", expected.channel),
        ("game_commit", expected.game_commit),
        ("target", expected.target),
    ] {
        if text(release, key)? != expected {
            return Err(format!(
                "Package manifest does not match this executable: {key}"
            ));
        }
    }
    if !lower_hex(text(release, "game_commit")?, 40) {
        return Err("Invalid manifest Game commit".into());
    }
    let packages = document["packages"]
        .as_array()
        .filter(|entries| entries.len() <= 1024)
        .ok_or("Invalid packages list (maximum 1024)")?;
    let mut ids = HashSet::new();
    let mut bytes = 0u64;
    for package in packages {
        let id = text(package, "id")?;
        if id.is_empty()
            || id.len() > 128
            || !id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || !ids.insert(id)
        {
            return Err("Invalid or duplicate package id".into());
        }
        text(package, "version")?
            .parse::<semver::Version>()
            .map_err(|_| format!("Invalid package version: {id}"))?;
        let provenance = &package["provenance"];
        if !matches!(
            text(provenance, "source")?,
            "oteryn-original" | "oteryn-generated" | "licensed-redistributable"
        ) || text(provenance, "source_ref")?.trim().is_empty()
            || text(provenance, "source_ref")?.contains('\0')
        {
            return Err(format!("Invalid package provenance: {id}"));
        }
        let path = text(package, "path")?.replace('\\', "/");
        let parts: Vec<_> = path.split('/').collect();
        if parts.len() < 2
            || parts[0] != "packages"
            || parts.iter().any(|part| {
                part.is_empty()
                    || *part == "."
                    || *part == ".."
                    || part.contains(':')
                    || part.contains('\0')
            })
        {
            return Err(format!("Package path leaves packages/: {id}"));
        }
        let path = root
            .join(path)
            .canonicalize()
            .map_err(|_| format!("Package is missing: {id}"))?;
        if !path.starts_with(root.join("packages")) {
            return Err(format!("Package path leaves packages/: {id}"));
        }
        let size = package["size"]
            .as_u64()
            .ok_or_else(|| format!("Invalid package size: {id}"))?;
        let expected_hash = text(package, "sha256")?;
        if !lower_hex(expected_hash, 64) {
            return Err(format!("Invalid package SHA-256: {id}"));
        }
        let mut file = File::open(path).map_err(|_| format!("Cannot open package: {id}"))?;
        let metadata = file
            .metadata()
            .map_err(|_| format!("Cannot inspect package: {id}"))?;
        if !metadata.is_file() || metadata.len() != size {
            return Err(format!("Package size mismatch: {id}"));
        }
        let mut hash = Sha256::new();
        let mut buffer = [0; 64 * 1024];
        let mut read = 0u64;
        loop {
            let count = file
                .read(&mut buffer)
                .map_err(|_| format!("Cannot read package: {id}"))?;
            if count == 0 {
                break;
            }
            read = read
                .checked_add(count as u64)
                .ok_or("Package size overflow")?;
            hash.update(&buffer[..count]);
        }
        let actual_hash: String = hash
            .finalize()
            .iter()
            .flat_map(|byte| [byte >> 4, byte & 15])
            .map(|nibble| char::from(b"0123456789abcdef"[usize::from(nibble)]))
            .collect();
        if read != size || actual_hash != expected_hash {
            return Err(format!("Package integrity mismatch: {id}"));
        }
        bytes = bytes
            .checked_add(size)
            .ok_or("Package byte count overflow")?;
    }
    Ok(Report::Verified {
        packages: packages.len(),
        bytes,
    })
}

pub fn verify_current(require_manifest: bool) -> Result<Report, String> {
    let executable = std::env::current_exe().map_err(|_| "Cannot locate the running client")?;
    let directory = executable
        .parent()
        .ok_or("Cannot locate the release directory")?;
    if !require_manifest
        && crate::RELEASE_ID.contains(".local.")
        && !directory
            .join("packages.json")
            .try_exists()
            .map_err(|_| "Cannot inspect packages.json")?
    {
        return Ok(Report::Development);
    }
    verify(directory, Identity::compiled())
}

#[cfg(windows)]
pub fn show_repair_required(error: &str) {
    crate::about::show_information(&format!(
        "Installation verification failed: {error}\r\nClose the client and run the trusted Oteryn installer for this release to repair it. Automatic updates are unavailable. Obtain updates from your operator's trusted release channel."
    ));
}
