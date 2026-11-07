// Byte-for-byte copy of `src/content/production.rs`'s `ProductionKey`/`ProductionAtom`/
// `Sha256HexDigest`/`PackageManifestBinding`/`ContentLockEntry`/`ContentLockBinding` and their
// validation helpers (everything through `ContentLockBinding`'s struct definition; its
// `validate` method is dropped, since `reference_playable.rs` has its own
// `validate_content_lock` and never calls the one here). The real file also defines
// `load_reference_playable_artifact`/`stage_reference_playable`, which need `reference_artifact`
// — whose own `#[cfg(test)]` modules need `project`/`cw2_b1_import`/`static_cell_engine` in turn,
// none of which B3-2's PG test binary (`combat_pickup_postgres.rs`) has any other reason to
// compile. This slice is what `content_shim.rs`'s `production` module actually needs.
//
// Unused constants/imports from the source slice (record-format wire constants, the
// `FirstProductionLimits`-adjacent model types) are trimmed to avoid `-D warnings` dead-code
// noise; nothing here changes behavior versus the real file for the items it keeps.

use super::digest::sha256;
use super::model::ContentError;
use crate::foundation::WorldId;

pub const FIRST_PRODUCTION_MAX_KEY_BYTES: usize = 512;
pub const FIRST_PRODUCTION_MAX_ATOM_BYTES: usize = 512;
pub const FIRST_PRODUCTION_MAX_CONTENT_LOCK_ENTRIES: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirstProductionLimits;

impl FirstProductionLimits {
    pub const fn v1() -> Self {
        Self
    }

    fn check(
        self,
        resource: &'static str,
        actual: usize,
        limit: usize,
    ) -> Result<(), ContentError> {
        if actual > limit {
            return Err(ContentError::LimitExceeded {
                resource,
                actual,
                limit,
            });
        }
        Ok(())
    }
}

fn has_nonproduction_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let has_test_segment = lower
        .split([':', '.', '/', '_', '-'])
        .any(|segment| segment == "test");
    lower.contains("fixture")
        || lower.contains("synthetic")
        || lower.contains("evidence")
        || lower.contains("test-only")
        || has_test_segment
}

fn valid_key_bytes(value: &str) -> bool {
    !value.is_empty()
        && value.is_ascii()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'.' | b'_' | b'-' | b'/')
        })
}

fn validate_atom(
    field: &'static str,
    value: &str,
    max: usize,
    reject_nonproduction: bool,
) -> Result<(), ContentError> {
    FirstProductionLimits::v1().check(field, value.len(), max)?;
    if value.is_empty()
        || !value.is_ascii()
        || !value.bytes().all(|byte| byte.is_ascii_graphic())
        || (reject_nonproduction && has_nonproduction_marker(value))
    {
        return Err(ContentError::InvalidString(field));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProductionKey(String);

impl ProductionKey {
    pub fn new(value: &str) -> Result<Self, ContentError> {
        FirstProductionLimits::v1().check(
            "first-production key bytes",
            value.len(),
            FIRST_PRODUCTION_MAX_KEY_BYTES,
        )?;
        let namespaced = value
            .split_once(':')
            .is_some_and(|(namespace, local)| !namespace.is_empty() && !local.is_empty());
        if !namespaced || !valid_key_bytes(value) || has_nonproduction_marker(value) {
            return Err(ContentError::InvalidArtifact(
                "invalid first-production namespaced key",
            ));
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProductionAtom(String);

impl ProductionAtom {
    pub fn new(field: &'static str, value: &str) -> Result<Self, ContentError> {
        validate_atom(field, value, FIRST_PRODUCTION_MAX_ATOM_BYTES, true)?;
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Sha256HexDigest(String);

impl Sha256HexDigest {
    pub fn new(value: &str) -> Result<Self, ContentError> {
        FirstProductionLimits::v1().check(
            "first-production sha256 digest bytes",
            value.len(),
            64,
        )?;
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return Err(ContentError::InvalidArtifact(
                "sha256 digest must be exactly 64 lowercase hexadecimal ASCII bytes",
            ));
        }
        Ok(Self(value.to_owned()))
    }

    fn from_digest_bytes(bytes: [u8; 32]) -> Self {
        Self(hex_lower(&bytes))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn hex_lower(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(64);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

pub(crate) fn encode_world_id(world_id: WorldId) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(32);
    for byte in world_id.as_bytes() {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageManifestBinding {
    pub package_key: ProductionKey,
    pub package_revision: ProductionAtom,
    pub semantic_schema_version: ProductionAtom,
    pub licensing_metadata: ProductionAtom,
    pub source_manifest_digest: Sha256HexDigest,
}

impl PackageManifestBinding {
    pub fn new(
        package_key: ProductionKey,
        package_revision: ProductionAtom,
        semantic_schema_version: ProductionAtom,
        licensing_metadata: ProductionAtom,
        source_manifest_digest: Sha256HexDigest,
    ) -> Self {
        Self {
            package_key,
            package_revision,
            semantic_schema_version,
            licensing_metadata,
            source_manifest_digest,
        }
    }

    pub fn package_provenance_digest(&self) -> Result<Sha256HexDigest, ContentError> {
        Ok(Sha256HexDigest::from_digest_bytes(sha256(
            &self.provenance_preimage()?,
        )))
    }

    pub fn provenance_preimage(&self) -> Result<Vec<u8>, ContentError> {
        let mut preimage = Vec::new();
        append_provenance_field(&mut preimage, self.package_key.as_str())?;
        append_provenance_field(&mut preimage, self.package_revision.as_str())?;
        append_provenance_field(&mut preimage, self.semantic_schema_version.as_str())?;
        append_provenance_field(&mut preimage, self.licensing_metadata.as_str())?;
        append_provenance_field(&mut preimage, self.source_manifest_digest.as_str())?;
        Ok(preimage)
    }
}

fn append_provenance_field(bytes: &mut Vec<u8>, value: &str) -> Result<(), ContentError> {
    let length = u32::try_from(value.len()).map_err(|_| ContentError::LimitExceeded {
        resource: "first-production provenance field bytes",
        actual: value.len(),
        limit: usize::try_from(u32::MAX).unwrap_or(usize::MAX),
    })?;
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentLockEntry {
    pub package_key: ProductionKey,
    pub package_revision: ProductionAtom,
    pub package_provenance_digest: Sha256HexDigest,
    pub floating: bool,
    pub dependency: bool,
}

impl ContentLockEntry {
    pub fn exact(
        package_key: ProductionKey,
        package_revision: ProductionAtom,
        package_provenance_digest: Sha256HexDigest,
    ) -> Self {
        Self {
            package_key,
            package_revision,
            package_provenance_digest,
            floating: false,
            dependency: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentLockBinding {
    pub revision_digest_token: ProductionAtom,
    pub entries: Vec<ContentLockEntry>,
}
