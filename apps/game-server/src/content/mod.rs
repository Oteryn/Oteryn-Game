//! Non-production VSL content evidence seam.
//!
//! The byte profile in this module exists only for deterministic first-slice
//! evidence. It is deliberately replaceable and is not the permanent Oteryn
//! World Project / World Bundle encoding.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

const EVIDENCE_MAGIC: &[u8; 8] = b"OTVSL001";
const EVIDENCE_PROFILE_VERSION: u32 = 1;
const EVIDENCE_DIGEST_BYTES: usize = 32;
const SECTION_TABLE_ENTRY_BYTES: usize = 4 + 4 + EVIDENCE_DIGEST_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentError {
    InvalidKey,
    ZeroRevision,
    DuplicateKey,
    DuplicateReference,
    MissingReference,
    ProjectionLeakage,
    InvalidLimits,
    ArtifactTooLarge,
    TooManySections,
    MalformedArtifact,
    IntegrityMismatch,
    ManifestMismatch,
}

impl Display for ContentError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidKey => "content key is invalid",
            Self::ZeroRevision => "revision must be non-zero",
            Self::DuplicateKey => "duplicate content key",
            Self::DuplicateReference => "duplicate content reference",
            Self::MissingReference => "content reference is missing",
            Self::ProjectionLeakage => "client-safe projection references server-only content",
            Self::InvalidLimits => "evidence limits are invalid",
            Self::ArtifactTooLarge => "evidence artifact exceeds caller limit",
            Self::TooManySections => "evidence artifact has too many sections",
            Self::MalformedArtifact => "evidence artifact is malformed",
            Self::IntegrityMismatch => "evidence artifact integrity mismatch",
            Self::ManifestMismatch => {
                "evidence artifact manifest does not match expected provenance"
            }
        })
    }
}

impl Error for ContentError {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentKey(String);

impl ContentKey {
    pub fn new(value: impl Into<String>) -> Result<Self, ContentError> {
        let value = value.into();
        let mut parts = value.split(':');
        let namespace = parts.next().unwrap_or_default();
        let name = parts.next().unwrap_or_default();
        if namespace.is_empty() || name.is_empty() || parts.next().is_some() {
            return Err(ContentError::InvalidKey);
        }
        if value.chars().any(char::is_control) {
            return Err(ContentError::InvalidKey);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Revision(u64);

impl Revision {
    pub fn new(value: u64) -> Result<Self, ContentError> {
        if value == 0 {
            return Err(ContentError::ZeroRevision);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    ClientSafe,
    ServerOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionClass {
    Server,
    ClientSafe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvidenceDigest([u8; EVIDENCE_DIGEST_BYTES]);

impl EvidenceDigest {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; EVIDENCE_DIGEST_BYTES]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; EVIDENCE_DIGEST_BYTES] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceManifest {
    content_revision: Revision,
    map_revision: Revision,
    ruleset_revision: Revision,
    world_policy_revision: Revision,
    compiler_identity: ContentKey,
    canonicalization_profile: ContentKey,
    content_lock_digest: EvidenceDigest,
    projection: ProjectionClass,
}

impl EvidenceManifest {
    #[must_use]
    pub const fn new(
        content_revision: Revision,
        map_revision: Revision,
        ruleset_revision: Revision,
        world_policy_revision: Revision,
        compiler_identity: ContentKey,
        canonicalization_profile: ContentKey,
        content_lock_digest: EvidenceDigest,
        projection: ProjectionClass,
    ) -> Self {
        Self {
            content_revision,
            map_revision,
            ruleset_revision,
            world_policy_revision,
            compiler_identity,
            canonicalization_profile,
            content_lock_digest,
            projection,
        }
    }

    #[must_use]
    pub const fn content_revision(&self) -> Revision {
        self.content_revision
    }

    #[must_use]
    pub const fn map_revision(&self) -> Revision {
        self.map_revision
    }

    #[must_use]
    pub const fn ruleset_revision(&self) -> Revision {
        self.ruleset_revision
    }

    #[must_use]
    pub const fn world_policy_revision(&self) -> Revision {
        self.world_policy_revision
    }

    #[must_use]
    pub const fn compiler_identity(&self) -> &ContentKey {
        &self.compiler_identity
    }

    #[must_use]
    pub const fn canonicalization_profile(&self) -> &ContentKey {
        &self.canonicalization_profile
    }

    #[must_use]
    pub const fn content_lock_digest(&self) -> EvidenceDigest {
        self.content_lock_digest
    }

    #[must_use]
    pub const fn projection(&self) -> ProjectionClass {
        self.projection
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Definition {
    key: ContentKey,
    visibility: Visibility,
    payload: Vec<u8>,
    references: Vec<ContentKey>,
}

impl Definition {
    #[must_use]
    pub fn new(
        key: ContentKey,
        visibility: Visibility,
        payload: Vec<u8>,
        references: Vec<ContentKey>,
    ) -> Self {
        Self {
            key,
            visibility,
            payload,
            references,
        }
    }

    #[must_use]
    pub const fn key(&self) -> &ContentKey {
        &self.key
    }

    #[must_use]
    pub const fn visibility(&self) -> Visibility {
        self.visibility
    }

    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    #[must_use]
    pub fn references(&self) -> &[ContentKey] {
        &self.references
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalGraph {
    entries: Vec<Definition>,
}

impl CanonicalGraph {
    pub fn compile(mut entries: Vec<Definition>) -> Result<Self, ContentError> {
        for entry in &mut entries {
            entry.references.sort();
            if entry.references.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(ContentError::DuplicateReference);
            }
        }
        entries.sort_by(|left, right| left.key.cmp(&right.key));
        if entries.windows(2).any(|pair| pair[0].key == pair[1].key) {
            return Err(ContentError::DuplicateKey);
        }

        let keys: BTreeSet<&ContentKey> = entries.iter().map(|entry| &entry.key).collect();
        for entry in &entries {
            if entry
                .references
                .iter()
                .any(|reference| !keys.contains(reference))
            {
                return Err(ContentError::MissingReference);
            }
        }
        Ok(Self { entries })
    }

    pub fn project(&self, projection: ProjectionClass) -> Result<Vec<Definition>, ContentError> {
        if projection == ProjectionClass::Server {
            return Ok(self.entries.clone());
        }

        let client_keys: BTreeSet<&ContentKey> = self
            .entries
            .iter()
            .filter(|entry| entry.visibility == Visibility::ClientSafe)
            .map(|entry| &entry.key)
            .collect();
        for entry in self
            .entries
            .iter()
            .filter(|entry| entry.visibility == Visibility::ClientSafe)
        {
            if entry
                .references
                .iter()
                .any(|reference| !client_keys.contains(reference))
            {
                return Err(ContentError::ProjectionLeakage);
            }
        }

        Ok(self
            .entries
            .iter()
            .filter(|entry| entry.visibility == Visibility::ClientSafe)
            .cloned()
            .collect())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvidenceLimits {
    max_artifact_bytes: usize,
    max_sections: usize,
}

impl EvidenceLimits {
    pub fn new(max_artifact_bytes: usize, max_sections: usize) -> Result<Self, ContentError> {
        if max_artifact_bytes == 0 || max_sections == 0 {
            return Err(ContentError::InvalidLimits);
        }
        Ok(Self {
            max_artifact_bytes,
            max_sections,
        })
    }
}

struct BoundedBytes {
    bytes: Vec<u8>,
    maximum: usize,
}

impl BoundedBytes {
    fn new(maximum: usize) -> Self {
        Self {
            bytes: Vec::new(),
            maximum,
        }
    }

    fn ensure_additional(&self, additional: usize) -> Result<(), ContentError> {
        let next = self
            .bytes
            .len()
            .checked_add(additional)
            .ok_or(ContentError::ArtifactTooLarge)?;
        if next > self.maximum {
            return Err(ContentError::ArtifactTooLarge);
        }
        Ok(())
    }

    fn push(&mut self, value: u8) -> Result<(), ContentError> {
        self.ensure_additional(1)?;
        self.bytes.push(value);
        Ok(())
    }

    fn extend(&mut self, value: &[u8]) -> Result<(), ContentError> {
        self.ensure_additional(value.len())?;
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    fn u32(&mut self, value: u32) -> Result<(), ContentError> {
        self.extend(&value.to_be_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), ContentError> {
        self.extend(&value.to_be_bytes())
    }

    fn length_prefixed(&mut self, value: &[u8]) -> Result<(), ContentError> {
        let length = u32::try_from(value.len()).map_err(|_| ContentError::ArtifactTooLarge)?;
        self.u32(length)?;
        self.extend(value)
    }

    fn len(&self) -> usize {
        self.bytes.len()
    }

    fn into_vec(self) -> Vec<u8> {
        self.bytes
    }
}

fn encode_manifest(
    writer: &mut BoundedBytes,
    manifest: &EvidenceManifest,
) -> Result<(), ContentError> {
    writer.u64(manifest.content_revision.get())?;
    writer.u64(manifest.map_revision.get())?;
    writer.u64(manifest.ruleset_revision.get())?;
    writer.u64(manifest.world_policy_revision.get())?;
    writer.length_prefixed(manifest.compiler_identity.as_str().as_bytes())?;
    writer.length_prefixed(manifest.canonicalization_profile.as_str().as_bytes())?;
    writer.extend(manifest.content_lock_digest.as_bytes())?;
    writer.push(match manifest.projection {
        ProjectionClass::Server => 1,
        ProjectionClass::ClientSafe => 2,
    })
}

fn encode_definition_section(
    entries: &[Definition],
    maximum: usize,
) -> Result<Vec<u8>, ContentError> {
    let mut writer = BoundedBytes::new(maximum);
    let entry_count = u32::try_from(entries.len()).map_err(|_| ContentError::ArtifactTooLarge)?;
    writer.u32(entry_count)?;
    for entry in entries {
        writer.push(match entry.visibility {
            Visibility::ClientSafe => 1,
            Visibility::ServerOnly => 2,
        })?;
        writer.length_prefixed(entry.key.as_str().as_bytes())?;
        writer.length_prefixed(&entry.payload)?;
        let reference_count =
            u32::try_from(entry.references.len()).map_err(|_| ContentError::ArtifactTooLarge)?;
        writer.u32(reference_count)?;
        for reference in &entry.references {
            writer.length_prefixed(reference.as_str().as_bytes())?;
        }
    }
    Ok(writer.into_vec())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceArtifact {
    manifest: EvidenceManifest,
    bytes: Vec<u8>,
}

impl EvidenceArtifact {
    pub fn compile(
        graph: &CanonicalGraph,
        manifest: &EvidenceManifest,
        limits: EvidenceLimits,
    ) -> Result<Self, ContentError> {
        if limits.max_sections < 1 {
            return Err(ContentError::TooManySections);
        }
        let projected = graph.project(manifest.projection)?;
        let section = encode_definition_section(&projected, limits.max_artifact_bytes)?;
        let section_digest = sha256(&section);

        let mut writer = BoundedBytes::new(limits.max_artifact_bytes);
        writer.extend(EVIDENCE_MAGIC)?;
        writer.u32(EVIDENCE_PROFILE_VERSION)?;
        encode_manifest(&mut writer, manifest)?;
        writer.u32(1)?;

        let payload_offset = writer
            .len()
            .checked_add(SECTION_TABLE_ENTRY_BYTES)
            .ok_or(ContentError::ArtifactTooLarge)?;
        let payload_offset =
            u32::try_from(payload_offset).map_err(|_| ContentError::ArtifactTooLarge)?;
        let payload_length =
            u32::try_from(section.len()).map_err(|_| ContentError::ArtifactTooLarge)?;
        writer.u32(payload_offset)?;
        writer.u32(payload_length)?;
        writer.extend(&section_digest)?;
        writer.extend(&section)?;

        let artifact_digest = sha256(&writer.bytes);
        writer.extend(&artifact_digest)?;

        Ok(Self {
            manifest: manifest.clone(),
            bytes: writer.into_vec(),
        })
    }

    #[must_use]
    pub fn manifest(&self) -> &EvidenceManifest {
        &self.manifest
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], ContentError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(ContentError::MalformedArtifact)?;
        let slice = self
            .bytes
            .get(self.offset..end)
            .ok_or(ContentError::MalformedArtifact)?;
        self.offset = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, ContentError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, ContentError> {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| ContentError::MalformedArtifact)?;
        Ok(u32::from_be_bytes(bytes))
    }

    fn u64(&mut self) -> Result<u64, ContentError> {
        let bytes: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| ContentError::MalformedArtifact)?;
        Ok(u64::from_be_bytes(bytes))
    }

    fn owned_bytes(&mut self) -> Result<Vec<u8>, ContentError> {
        let length = usize::try_from(self.u32()?).map_err(|_| ContentError::MalformedArtifact)?;
        Ok(self.take(length)?.to_vec())
    }

    fn string(&mut self) -> Result<String, ContentError> {
        String::from_utf8(self.owned_bytes()?).map_err(|_| ContentError::MalformedArtifact)
    }

    fn position(&self) -> usize {
        self.offset
    }

    fn is_finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

fn decode_manifest(cursor: &mut Cursor<'_>) -> Result<EvidenceManifest, ContentError> {
    let content_revision = Revision::new(cursor.u64()?)?;
    let map_revision = Revision::new(cursor.u64()?)?;
    let ruleset_revision = Revision::new(cursor.u64()?)?;
    let world_policy_revision = Revision::new(cursor.u64()?)?;
    let compiler_identity = ContentKey::new(cursor.string()?)?;
    let canonicalization_profile = ContentKey::new(cursor.string()?)?;
    let content_lock_digest: [u8; EVIDENCE_DIGEST_BYTES] = cursor
        .take(EVIDENCE_DIGEST_BYTES)?
        .try_into()
        .map_err(|_| ContentError::MalformedArtifact)?;
    let projection = match cursor.u8()? {
        1 => ProjectionClass::Server,
        2 => ProjectionClass::ClientSafe,
        _ => return Err(ContentError::MalformedArtifact),
    };
    Ok(EvidenceManifest::new(
        content_revision,
        map_revision,
        ruleset_revision,
        world_policy_revision,
        compiler_identity,
        canonicalization_profile,
        EvidenceDigest::from_bytes(content_lock_digest),
        projection,
    ))
}

fn decode_definition_section(section: &[u8]) -> Result<Vec<Definition>, ContentError> {
    let mut cursor = Cursor::new(section);
    let entry_count =
        usize::try_from(cursor.u32()?).map_err(|_| ContentError::MalformedArtifact)?;
    let mut entries = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        let visibility = match cursor.u8()? {
            1 => Visibility::ClientSafe,
            2 => Visibility::ServerOnly,
            _ => return Err(ContentError::MalformedArtifact),
        };
        let key = ContentKey::new(cursor.string()?)?;
        let payload = cursor.owned_bytes()?;
        let reference_count =
            usize::try_from(cursor.u32()?).map_err(|_| ContentError::MalformedArtifact)?;
        let mut references = Vec::with_capacity(reference_count);
        for _ in 0..reference_count {
            references.push(ContentKey::new(cursor.string()?)?);
        }
        entries.push(Definition::new(key, visibility, payload, references));
    }
    if !cursor.is_finished() {
        return Err(ContentError::MalformedArtifact);
    }
    Ok(entries)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedArtifact {
    manifest: EvidenceManifest,
    entries: Vec<Definition>,
    section_count: usize,
}

impl StagedArtifact {
    pub fn stage(
        bytes: &[u8],
        expected_manifest: &EvidenceManifest,
        limits: EvidenceLimits,
    ) -> Result<Self, ContentError> {
        if bytes.len() > limits.max_artifact_bytes {
            return Err(ContentError::ArtifactTooLarge);
        }
        if bytes.len() < EVIDENCE_DIGEST_BYTES {
            return Err(ContentError::MalformedArtifact);
        }

        let artifact_body_length = bytes.len() - EVIDENCE_DIGEST_BYTES;
        let (artifact_body, encoded_artifact_digest) = bytes.split_at(artifact_body_length);
        if sha256(artifact_body).as_slice() != encoded_artifact_digest {
            return Err(ContentError::IntegrityMismatch);
        }

        let mut cursor = Cursor::new(artifact_body);
        if cursor.take(EVIDENCE_MAGIC.len())? != EVIDENCE_MAGIC {
            return Err(ContentError::MalformedArtifact);
        }
        if cursor.u32()? != EVIDENCE_PROFILE_VERSION {
            return Err(ContentError::MalformedArtifact);
        }
        let manifest = decode_manifest(&mut cursor)?;
        if &manifest != expected_manifest {
            return Err(ContentError::ManifestMismatch);
        }

        let section_count =
            usize::try_from(cursor.u32()?).map_err(|_| ContentError::MalformedArtifact)?;
        if section_count > limits.max_sections {
            return Err(ContentError::TooManySections);
        }
        if section_count != 1 {
            return Err(ContentError::MalformedArtifact);
        }

        let section_offset =
            usize::try_from(cursor.u32()?).map_err(|_| ContentError::MalformedArtifact)?;
        let section_length =
            usize::try_from(cursor.u32()?).map_err(|_| ContentError::MalformedArtifact)?;
        let encoded_section_digest = cursor.take(EVIDENCE_DIGEST_BYTES)?;
        if section_offset != cursor.position() {
            return Err(ContentError::MalformedArtifact);
        }
        let section_end = section_offset
            .checked_add(section_length)
            .ok_or(ContentError::MalformedArtifact)?;
        if section_end != artifact_body.len() {
            return Err(ContentError::MalformedArtifact);
        }
        let section = artifact_body
            .get(section_offset..section_end)
            .ok_or(ContentError::MalformedArtifact)?;
        if sha256(section).as_slice() != encoded_section_digest {
            return Err(ContentError::IntegrityMismatch);
        }

        let graph = CanonicalGraph::compile(decode_definition_section(section)?)?;
        let entries = graph.project(manifest.projection)?;
        if entries != graph.entries {
            return Err(ContentError::ProjectionLeakage);
        }

        Ok(Self {
            manifest,
            entries,
            section_count,
        })
    }

    #[must_use]
    pub fn manifest(&self) -> &EvidenceManifest {
        &self.manifest
    }

    #[must_use]
    pub const fn section_count(&self) -> usize {
        self.section_count
    }

    #[must_use]
    pub fn activate(self) -> ActiveArtifact {
        ActiveArtifact {
            manifest: self.manifest,
            entries: self.entries,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveArtifact {
    manifest: EvidenceManifest,
    entries: Vec<Definition>,
}

impl ActiveArtifact {
    #[must_use]
    pub fn manifest(&self) -> &EvidenceManifest {
        &self.manifest
    }

    #[must_use]
    pub fn entries(&self) -> &[Definition] {
        &self.entries
    }
}

fn sha256(input: &[u8]) -> [u8; EVIDENCE_DIGEST_BYTES] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_length = (input.len() as u64).wrapping_mul(8);
    let mut padded = input.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_length.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut words = [0u32; 64];
        for (index, word) in words.iter_mut().take(16).enumerate() {
            let start = index * 4;
            *word = u32::from_be_bytes(
                chunk[start..start + 4]
                    .try_into()
                    .expect("SHA-256 chunk width is fixed"),
            );
        }
        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];

        for index in 0..64 {
            let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(sum1)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = sum0.wrapping_add(majority);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    let mut digest = [0u8; EVIDENCE_DIGEST_BYTES];
    for (index, value) in state.into_iter().enumerate() {
        digest[index * 4..index * 4 + 4].copy_from_slice(&value.to_be_bytes());
    }
    digest
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(value: &str) -> ContentKey {
        ContentKey::new(value).expect("valid test key")
    }

    fn revision(value: u64) -> Revision {
        Revision::new(value).expect("valid revision")
    }

    fn digest(byte: u8) -> EvidenceDigest {
        EvidenceDigest::from_bytes([byte; 32])
    }

    fn manifest(projection: ProjectionClass) -> EvidenceManifest {
        EvidenceManifest::new(
            revision(21),
            revision(22),
            revision(23),
            revision(24),
            key("oteryn:compiler"),
            key("oteryn:canonical-v1"),
            digest(0x5a),
            projection,
        )
    }

    fn limits() -> EvidenceLimits {
        EvidenceLimits::new(4096, 16).expect("valid limits")
    }

    fn definition(
        name: &str,
        visibility: Visibility,
        payload: &[u8],
        references: &[&str],
    ) -> Definition {
        Definition::new(
            key(name),
            visibility,
            payload.to_vec(),
            references.iter().map(|value| key(value)).collect(),
        )
    }

    #[test]
    fn compile_is_deterministic_across_input_order() {
        let a = definition("oteryn:tile", Visibility::ClientSafe, b"tile", &[]);
        let b = definition(
            "oteryn:rule",
            Visibility::ServerOnly,
            b"rule",
            &["oteryn:tile"],
        );
        let first = CanonicalGraph::compile(vec![a.clone(), b.clone()]).unwrap();
        let second = CanonicalGraph::compile(vec![b, a]).unwrap();
        assert_eq!(first, second);

        let manifest = manifest(ProjectionClass::Server);
        let first_artifact = EvidenceArtifact::compile(&first, &manifest, limits()).unwrap();
        let second_artifact = EvidenceArtifact::compile(&second, &manifest, limits()).unwrap();
        assert_eq!(first_artifact.bytes(), second_artifact.bytes());
    }

    #[test]
    fn duplicate_and_missing_references_fail_closed() {
        let item = definition("oteryn:item", Visibility::ClientSafe, b"item", &[]);
        assert_eq!(
            CanonicalGraph::compile(vec![item.clone(), item]).unwrap_err(),
            ContentError::DuplicateKey
        );
        let broken = definition(
            "oteryn:rule",
            Visibility::ServerOnly,
            b"rule",
            &["oteryn:missing"],
        );
        assert_eq!(
            CanonicalGraph::compile(vec![broken]).unwrap_err(),
            ContentError::MissingReference
        );
        let duplicate_reference = definition(
            "oteryn:rule",
            Visibility::ServerOnly,
            b"rule",
            &["oteryn:item", "oteryn:item"],
        );
        assert_eq!(
            CanonicalGraph::compile(vec![
                definition("oteryn:item", Visibility::ClientSafe, b"item", &[]),
                duplicate_reference,
            ])
            .unwrap_err(),
            ContentError::DuplicateReference
        );
    }

    #[test]
    fn client_projection_never_contains_server_only_payload() {
        let graph = CanonicalGraph::compile(vec![
            definition("oteryn:public", Visibility::ClientSafe, b"public", &[]),
            definition("oteryn:secret", Visibility::ServerOnly, b"secret", &[]),
        ])
        .unwrap();

        let client = graph.project(ProjectionClass::ClientSafe).unwrap();
        assert_eq!(client.len(), 1);
        assert_eq!(client[0].key().as_str(), "oteryn:public");
        assert!(!client.iter().any(|entry| entry.payload() == b"secret"));
    }

    #[test]
    fn client_projection_rejects_server_only_reference() {
        let graph = CanonicalGraph::compile(vec![
            definition(
                "oteryn:public",
                Visibility::ClientSafe,
                b"p",
                &["oteryn:secret"],
            ),
            definition("oteryn:secret", Visibility::ServerOnly, b"s", &[]),
        ])
        .unwrap();
        assert_eq!(
            graph.project(ProjectionClass::ClientSafe).unwrap_err(),
            ContentError::ProjectionLeakage,
        );
    }

    #[test]
    fn staging_rejects_corruption_oversize_truncation_and_manifest_mismatch() {
        let graph = CanonicalGraph::compile(vec![definition(
            "oteryn:public",
            Visibility::ClientSafe,
            b"public",
            &[],
        )])
        .unwrap();
        let expected = manifest(ProjectionClass::ClientSafe);
        let artifact = EvidenceArtifact::compile(&graph, &expected, limits()).unwrap();

        let mut corrupt = artifact.bytes().to_vec();
        corrupt[4] ^= 0x01;
        assert_eq!(
            StagedArtifact::stage(&corrupt, &expected, limits()).unwrap_err(),
            ContentError::IntegrityMismatch
        );

        let tiny = EvidenceLimits::new(8, 16).unwrap();
        assert_eq!(
            StagedArtifact::stage(artifact.bytes(), &expected, tiny).unwrap_err(),
            ContentError::ArtifactTooLarge
        );

        assert!(
            StagedArtifact::stage(
                &artifact.bytes()[..artifact.bytes().len() - 1],
                &expected,
                limits()
            )
            .is_err()
        );

        let incompatible = EvidenceManifest::new(
            revision(99),
            expected.map_revision(),
            expected.ruleset_revision(),
            expected.world_policy_revision(),
            expected.compiler_identity().clone(),
            expected.canonicalization_profile().clone(),
            expected.content_lock_digest(),
            expected.projection(),
        );
        assert_eq!(
            StagedArtifact::stage(artifact.bytes(), &incompatible, limits()).unwrap_err(),
            ContentError::ManifestMismatch
        );
    }

    #[test]
    fn evidence_manifest_round_trips_exact_provenance() {
        let graph = CanonicalGraph::compile(vec![definition(
            "oteryn:public",
            Visibility::ClientSafe,
            b"public",
            &[],
        )])
        .unwrap();
        let expected = manifest(ProjectionClass::ClientSafe);
        let artifact = EvidenceArtifact::compile(&graph, &expected, limits()).unwrap();
        assert_eq!(artifact.manifest(), &expected);
        let staged = StagedArtifact::stage(artifact.bytes(), &expected, limits()).unwrap();
        assert_eq!(staged.manifest(), &expected);
        assert_eq!(staged.section_count(), 1);

        let active = staged.activate();
        assert_eq!(active.manifest(), &expected);
        assert_eq!(active.entries().len(), 1);
        assert_eq!(active.entries()[0].key().as_str(), "oteryn:public");
    }

    #[test]
    fn evidence_sha256_matches_known_vector() {
        assert_eq!(
            sha256(b"abc"),
            [
                0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
                0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
                0xf2, 0x00, 0x15, 0xad,
            ],
        );
    }
}
