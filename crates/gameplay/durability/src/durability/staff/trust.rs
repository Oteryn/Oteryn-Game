//! Restart-only trust. No trust value is read from PostgreSQL or a player.
use super::rules::{self, Activation, StaffRole};
use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};
use oteryn_protocol_oteryn::staff::{MAX_SIGNATURE_BYTES, MAX_SIGNED_BYTES, SignedEnvelope};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

pub const REQUEST_NAMESPACE: &str = "oteryn-ops-request-v1";
pub const RECEIPT_NAMESPACE: &str = "oteryn-ops-submission-receipt-v1";
const MAX_HISTORY_BYTES: u64 = 262_144;
static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaffConfig {
    pub environment: String,
    pub state_directory: PathBuf,
    pub activation_history_file: PathBuf,
    pub allowed_signers_sha256: String,
    pub newest_activation_sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    revisions: Vec<RevisionDocument>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RevisionDocument {
    allowed_signers: String,
    activation: String,
    signature: String,
    principal: String,
}
#[derive(Clone)]
struct Revision {
    roster: Vec<u8>,
    roster_sha256: String,
    activation: Activation,
    activation_sha256: String,
    signers: BTreeMap<String, (StaffRole, String)>,
}
pub struct StaffTrust {
    config: StaffConfig,
    revisions: Vec<Revision>,
}
impl std::fmt::Debug for StaffTrust {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StaffTrust").finish_non_exhaustive()
    }
}
#[derive(Debug)]
pub struct TrustError;
impl std::fmt::Display for TrustError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("staff trust or signed evidence refused")
    }
}
impl std::error::Error for TrustError {}

pub(super) fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
/// The typed grammar contains ASCII keys, ASCII strings and no JSON numbers.
/// RFC 8785 serialization of that subset equals serde_json's sorted Value encoding.
pub(super) fn canonical<T: Serialize>(value: &T) -> Result<Vec<u8>, TrustError> {
    // Explicit sorting also works if another workspace consumer enables serde_json's
    // preserve_order feature. The signed grammar has ASCII keys and string values only.
    fn sorted(value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let ordered: BTreeMap<_, _> = map
                    .into_iter()
                    .map(|(key, value)| (key, sorted(value)))
                    .collect();
                serde_json::Value::Object(ordered.into_iter().collect())
            }
            serde_json::Value::Array(values) => {
                serde_json::Value::Array(values.into_iter().map(sorted).collect())
            }
            scalar => scalar,
        }
    }
    serde_json::to_vec(&sorted(
        serde_json::to_value(value).map_err(|_| TrustError)?,
    ))
    .map_err(|_| TrustError)
}

pub(super) fn decode<T: serde::de::DeserializeOwned + Serialize>(
    bytes: &[u8],
) -> Result<T, TrustError> {
    if bytes.is_empty() || bytes.len() > MAX_SIGNED_BYTES || !bytes.is_ascii() {
        return Err(TrustError);
    }
    let value: T = serde_json::from_slice(bytes).map_err(|_| TrustError)?;
    if canonical(&value)? != bytes {
        return Err(TrustError);
    }
    Ok(value)
}
fn bounded_file(path: &Path) -> Result<Vec<u8>, TrustError> {
    let file = File::open(path).map_err(|_| TrustError)?;
    let mut bytes = Vec::new();
    file.take(MAX_HISTORY_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| TrustError)?;
    if bytes.len() as u64 > MAX_HISTORY_BYTES {
        return Err(TrustError);
    }
    Ok(bytes)
}
fn roster(bytes: &[u8]) -> Result<BTreeMap<String, (StaffRole, String)>, TrustError> {
    if bytes.len() > 32_768 {
        return Err(TrustError);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| TrustError)?;
    let mut roles = BTreeMap::new();
    let mut keys = BTreeMap::new();
    let mut fingerprints = BTreeSet::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<_> = line.split_ascii_whitespace().collect();
        if parts.first() == Some(&"#") {
            if parts.len() != 4 || parts[1] != "oteryn-staff-role" || !rules::atom(parts[2]) {
                return Err(TrustError);
            }
            let role = match parts[3] {
                "GM" => StaffRole::Gm,
                "SENIOR_GM" => StaffRole::SeniorGm,
                "ADMIN" => StaffRole::Admin,
                _ => return Err(TrustError),
            };
            if roles.insert(parts[2].to_owned(), role).is_some() {
                return Err(TrustError);
            }
        } else {
            // One principal and one Ed25519 key per line, with no wildcard or options.
            if parts.len() != 3 || !rules::atom(parts[0]) || parts[1] != "ssh-ed25519" {
                return Err(TrustError);
            }
            let key = base64::engine::general_purpose::STANDARD
                .decode(parts[2])
                .map_err(|_| TrustError)?;
            if key.len() != 51 || &key[..19] != b"\0\0\0\x0bssh-ed25519\0\0\0 " {
                return Err(TrustError);
            }
            let fingerprint = format!("SHA256:{}", STANDARD_NO_PAD.encode(Sha256::digest(&key)));
            if !fingerprints.insert(fingerprint.clone())
                || keys.insert(parts[0].to_owned(), fingerprint).is_some()
            {
                return Err(TrustError);
            }
        }
    }
    if keys.is_empty() || keys.len() > 32 || roles.len() != keys.len() {
        return Err(TrustError);
    }
    keys.into_iter()
        .map(|(p, k)| {
            let role = *roles.get(&p).ok_or(TrustError)?;
            Ok((p, (role, k)))
        })
        .collect()
}
struct Scratch {
    directory: PathBuf,
}
impl Scratch {
    fn new(parent: &Path) -> Result<Self, TrustError> {
        let id = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
        let directory = parent.join(format!("staff-verify-{}-{id}", std::process::id()));
        std::fs::create_dir(&directory).map_err(|_| TrustError)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
                .map_err(|_| TrustError)?;
        }
        Ok(Self { directory })
    }
    fn write(&self, name: &str, bytes: &[u8]) -> Result<PathBuf, TrustError> {
        let path = self.directory.join(name);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|_| TrustError)?;
        file.write_all(bytes).map_err(|_| TrustError)?;
        Ok(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}
fn verify(
    config: &StaffConfig,
    roster: &[u8],
    envelope: &SignedEnvelope,
    namespace: &str,
) -> Result<(), TrustError> {
    if envelope.canonical.is_empty()
        || envelope.canonical.len() > MAX_SIGNED_BYTES
        || envelope.signature.is_empty()
        || envelope.signature.len() > MAX_SIGNATURE_BYTES
        || !rules::atom(&envelope.principal)
    {
        return Err(TrustError);
    }
    let scratch = Scratch::new(&config.state_directory)?;
    let signers = scratch.write("allowed-signers", roster)?;
    let signature = scratch.write("signature", &envelope.signature)?;
    let input = scratch.write("message", &envelope.canonical)?;
    let mut child = Command::new("ssh-keygen")
        .args(["-Y", "verify", "-f"])
        .arg(signers)
        .args(["-I", &envelope.principal, "-n", namespace, "-s"])
        .arg(signature)
        .stdin(File::open(input).map_err(|_| TrustError)?)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| TrustError)?;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    Ok(())
                } else {
                    Err(TrustError)
                };
            }
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(2)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(TrustError);
            }
        }
    }
}
impl StaffTrust {
    pub fn load(config: StaffConfig) -> Result<Self, TrustError> {
        if !rules::atom(&config.environment)
            || !rules::hex(&config.allowed_signers_sha256, 32)
            || !rules::hex(&config.newest_activation_sha256, 32)
            || !config.state_directory.is_dir()
        {
            return Err(TrustError);
        }
        let document: Document =
            serde_json::from_slice(&bounded_file(&config.activation_history_file)?)
                .map_err(|_| TrustError)?;
        if document.revisions.is_empty() || document.revisions.len() > 16 {
            return Err(TrustError);
        }
        let mut revisions = Vec::new();
        let mut previous_seq = 0;
        let mut seen = BTreeSet::new();
        for entry in document.revisions {
            let bytes = entry.allowed_signers.into_bytes();
            let signers = roster(&bytes)?;
            if !signers.contains_key(&entry.principal) {
                return Err(TrustError);
            }
            let envelope = SignedEnvelope {
                principal: entry.principal,
                canonical: entry.activation.into_bytes(),
                signature: entry.signature.into_bytes(),
            };
            verify(&config, &bytes, &envelope, REQUEST_NAMESPACE)?;
            let activation: Activation = decode(&envelope.canonical)?;
            let seq = rules::decimal(&activation.control_seq).ok_or(TrustError)?;
            let roster_sha256 = digest(&bytes);
            let activation_sha256 = digest(&envelope.canonical);
            if activation.environment != config.environment
                || activation.roster_sha256 != roster_sha256
                || seq <= previous_seq
                || seq > i64::MAX as u64
                || !rules::hex(&activation.previous_link_sha256, 32)
                || !seen.insert(activation_sha256.clone())
            {
                return Err(TrustError);
            }
            // No unauthenticated bootstrap from a mid-chain DB head.
            if previous_seq == 0 && (seq != 1 || activation.previous_link_sha256 != "0".repeat(64))
            {
                return Err(TrustError);
            }
            previous_seq = seq;
            revisions.push(Revision {
                roster: bytes,
                roster_sha256,
                activation,
                activation_sha256,
                signers,
            });
        }
        let newest = revisions.last().ok_or(TrustError)?;
        if newest.roster_sha256 != config.allowed_signers_sha256
            || newest.activation_sha256 != config.newest_activation_sha256
        {
            return Err(TrustError);
        }
        Ok(Self { config, revisions })
    }
    pub(super) fn state_directory(&self) -> &Path {
        &self.config.state_directory
    }
    pub(super) fn environment(&self) -> &str {
        &self.config.environment
    }
    pub(super) fn activation(&self) -> &str {
        &self.config.newest_activation_sha256
    }
    pub(super) fn current_roster(&self) -> &str {
        &self.config.allowed_signers_sha256
    }
    pub(super) fn activation_at(&self, seq: u64) -> Option<(&str, &str)> {
        self.revisions
            .iter()
            .rev()
            .find(|r| rules::decimal(&r.activation.control_seq).is_some_and(|n| n <= seq))
            .map(|r| (r.activation_sha256.as_str(), r.roster_sha256.as_str()))
    }
    pub(super) fn authenticate_at(
        &self,
        e: &SignedEnvelope,
        namespace: &str,
        seq: u64,
    ) -> Result<(String, StaffRole, String), TrustError> {
        let revision = self
            .revisions
            .iter()
            .rev()
            .find(|r| rules::decimal(&r.activation.control_seq).is_some_and(|n| n <= seq))
            .ok_or(TrustError)?;
        let (role, fingerprint) = revision.signers.get(&e.principal).ok_or(TrustError)?;
        verify(&self.config, &revision.roster, e, namespace)?;
        Ok((revision.roster_sha256.clone(), *role, fingerprint.clone()))
    }
    /// Verify raw bytes before parsing anything as an authority-bearing request.
    /// A key may be retained in several revisions; signature validity alone must
    /// never select the newest revision for an older signed operation.
    pub(super) fn authenticate_any(
        &self,
        e: &SignedEnvelope,
        namespace: &str,
    ) -> Result<(), TrustError> {
        for revision in self.revisions.iter().rev() {
            if revision.signers.contains_key(&e.principal)
                && verify(&self.config, &revision.roster, e, namespace).is_ok()
            {
                return Ok(());
            }
        }
        Err(TrustError)
    }
}

#[cfg(test)]
#[path = "trust_tests.rs"]
mod tests;
