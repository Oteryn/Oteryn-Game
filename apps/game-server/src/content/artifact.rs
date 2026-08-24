use super::digest::sha256;
use super::model::*;
use std::collections::BTreeSet;

const MAGIC: [u8; 8] = *b"OTVSL01\0";
const HEADER_LEN: usize = 24;
const SECTION_ENTRY_LEN: usize = 48;
const TRAILER_LEN: usize = 32;
const SECTION_FLAG_CRITICAL: u16 = 0x0001;
const SECTION_MANIFEST: u16 = 1;
const SECTION_BODY: u16 = 2;
const MANIFEST_FIELD_COUNT: u32 = 15;

pub(crate) const RECORD_REGION: u8 = 1;
pub(crate) const RECORD_AREA: u8 = 2;
pub(crate) const RECORD_TERRAIN: u8 = 3;
pub(crate) const RECORD_CELL: u8 = 4;
pub(crate) const RECORD_RELOCATION: u8 = 5;
pub(crate) const RECORD_BEHAVIOR: u8 = 6;
pub(crate) const RECORD_PRESENTATION: u8 = 7;
pub(crate) const RECORD_CREATURE: u8 = 8;
pub(crate) const RECORD_SPAWN: u8 = 9;
pub(crate) const RECORD_FORMULA: u8 = 10;
pub(crate) const RECORD_EFFECT: u8 = 11;
pub(crate) const RECORD_ABILITY: u8 = 12;
pub(crate) const RECORD_ITEM: u8 = 13;
pub(crate) const RECORD_LOOT_TABLE: u8 = 14;
pub(crate) const RECORD_LOOT_ENTRY: u8 = 15;
pub(crate) const RECORD_XP: u8 = 16;
pub(crate) const RECORD_RNG_CONTEXT: u8 = 17;
pub(crate) const RECORD_RNG_PURPOSE: u8 = 18;
pub(crate) const RECORD_CLIENT_CREATURE: u8 = 108;
pub(crate) const RECORD_CLIENT_ABILITY: u8 = 112;
pub(crate) const RECORD_CLIENT_ITEM: u8 = 113;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionClass {
    ServerAuthoritative,
    ClientSafe,
}

impl ProjectionClass {
    const fn as_byte(self) -> u8 {
        match self {
            Self::ServerAuthoritative => 1,
            Self::ClientSafe => 2,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::ServerAuthoritative => "server-authoritative",
            Self::ClientSafe => "client-safe",
        }
    }

    fn from_byte(value: u8) -> Result<Self, ContentError> {
        match value {
            1 => Ok(Self::ServerAuthoritative),
            2 => Ok(Self::ClientSafe),
            _ => Err(ContentError::InvalidArtifact("unknown projection class")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvidenceRecord {
    pub(crate) kind: u8,
    pub(crate) fields: Vec<String>,
}

impl EvidenceRecord {
    pub(crate) fn new(kind: u8, fields: Vec<String>) -> Self {
        Self { kind, fields }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactMetadata {
    package_key: PackageKey,
    package_revision: RevisionToken,
    world_id: WorldId,
    revisions: RevisionSet,
    projection: ProjectionClass,
}

impl ArtifactMetadata {
    pub(crate) fn from_graph(graph: &CanonicalGraph, projection: ProjectionClass) -> Self {
        Self {
            package_key: graph.package_key().clone(),
            package_revision: graph.package_revision().clone(),
            world_id: graph.world_id().clone(),
            revisions: graph.revisions().clone(),
            projection,
        }
    }

    fn same_revision_identity(&self, other: &Self) -> bool {
        self.package_key == other.package_key
            && self.package_revision == other.package_revision
            && self.world_id == other.world_id
            && self.revisions == other.revisions
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactExpectation {
    pub package_key: PackageKey,
    pub package_revision: RevisionToken,
    pub world_id: WorldId,
    pub content_revision: RevisionToken,
    pub map_revision: RevisionToken,
    pub ruleset_revision: RevisionToken,
    pub world_policy_revision: RevisionToken,
    pub compiler_revision: RevisionToken,
    pub canonicalization_revision: RevisionToken,
    pub content_lock_revision: RevisionToken,
    pub provenance_revision: RevisionToken,
    pub sim_profile_revision: RevisionToken,
    pub fixture_profile_revision: RevisionToken,
}

impl ArtifactExpectation {
    pub(crate) fn from_graph(graph: &CanonicalGraph) -> Self {
        let revisions = graph.revisions();
        Self {
            package_key: graph.package_key().clone(),
            package_revision: graph.package_revision().clone(),
            world_id: graph.world_id().clone(),
            content_revision: revisions.content.clone(),
            map_revision: revisions.map.clone(),
            ruleset_revision: revisions.ruleset.clone(),
            world_policy_revision: revisions.world_policy.clone(),
            compiler_revision: revisions.compiler.clone(),
            canonicalization_revision: revisions.canonicalization.clone(),
            content_lock_revision: revisions.content_lock.clone(),
            provenance_revision: revisions.provenance.clone(),
            sim_profile_revision: revisions.sim_profile.clone(),
            fixture_profile_revision: revisions.fixture_profile.clone(),
        }
    }

    fn verify(&self, metadata: &ArtifactMetadata) -> Result<(), ContentError> {
        verify_field(
            self.package_key.as_str(),
            metadata.package_key.as_str(),
            "package key",
        )?;
        verify_field(
            self.package_revision.as_str(),
            metadata.package_revision.as_str(),
            "package revision",
        )?;
        verify_field(
            self.world_id.as_str(),
            metadata.world_id.as_str(),
            "world id",
        )?;
        verify_field(
            self.content_revision.as_str(),
            metadata.revisions.content.as_str(),
            "content revision",
        )?;
        verify_field(
            self.map_revision.as_str(),
            metadata.revisions.map.as_str(),
            "map revision",
        )?;
        verify_field(
            self.ruleset_revision.as_str(),
            metadata.revisions.ruleset.as_str(),
            "ruleset revision",
        )?;
        verify_field(
            self.world_policy_revision.as_str(),
            metadata.revisions.world_policy.as_str(),
            "world policy revision",
        )?;
        verify_field(
            self.compiler_revision.as_str(),
            metadata.revisions.compiler.as_str(),
            "compiler revision",
        )?;
        verify_field(
            self.canonicalization_revision.as_str(),
            metadata.revisions.canonicalization.as_str(),
            "canonicalization revision",
        )?;
        verify_field(
            self.content_lock_revision.as_str(),
            metadata.revisions.content_lock.as_str(),
            "content lock revision",
        )?;
        verify_field(
            self.provenance_revision.as_str(),
            metadata.revisions.provenance.as_str(),
            "provenance revision",
        )?;
        verify_field(
            self.sim_profile_revision.as_str(),
            metadata.revisions.sim_profile.as_str(),
            "simulation profile revision",
        )?;
        verify_field(
            self.fixture_profile_revision.as_str(),
            metadata.revisions.fixture_profile.as_str(),
            "fixture profile revision",
        )
    }
}

fn verify_field(expected: &str, actual: &str, field: &'static str) -> Result<(), ContentError> {
    if expected != actual {
        return Err(ContentError::RevisionMismatch(field));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EncodedArtifact {
    pub(crate) bytes: Vec<u8>,
    pub(crate) digest: [u8; 32],
}

pub(crate) fn encode_artifact(
    metadata: &ArtifactMetadata,
    records: &[EvidenceRecord],
    limits: &EvidenceLimits,
) -> Result<EncodedArtifact, ContentError> {
    limits.check("sections", 2, limits.max_sections())?;
    limits.check("records", records.len(), limits.max_records())?;
    let manifest = encode_manifest(metadata, limits)?;
    let body = encode_body(records, limits)?;
    limits.check("section bytes", manifest.len(), limits.max_section_bytes())?;
    limits.check("section bytes", body.len(), limits.max_section_bytes())?;

    let table_len = SECTION_ENTRY_LEN
        .checked_mul(2)
        .ok_or(ContentError::InvalidSectionBounds)?;
    let manifest_offset = HEADER_LEN
        .checked_add(table_len)
        .ok_or(ContentError::InvalidSectionBounds)?;
    let body_offset = manifest_offset
        .checked_add(manifest.len())
        .ok_or(ContentError::InvalidSectionBounds)?;
    let payload_end = body_offset
        .checked_add(body.len())
        .ok_or(ContentError::InvalidSectionBounds)?;
    let total_len = payload_end
        .checked_add(TRAILER_LEN)
        .ok_or(ContentError::InvalidSectionBounds)?;
    limits.check("artifact bytes", total_len, limits.max_artifact_bytes())?;

    let mut bytes = Vec::with_capacity(total_len);
    bytes.extend_from_slice(&MAGIC);
    put_u16(&mut bytes, EVIDENCE_PROFILE_VERSION);
    put_u16(&mut bytes, 0);
    bytes.push(metadata.projection.as_byte());
    bytes.push(0);
    put_u16(&mut bytes, 2);
    put_u32(&mut bytes, to_u32(HEADER_LEN)?);
    put_u32(&mut bytes, to_u32(payload_end)?);

    encode_section_entry(
        &mut bytes,
        SECTION_MANIFEST,
        manifest_offset,
        manifest.len(),
        MANIFEST_FIELD_COUNT,
        sha256(&manifest),
    )?;
    encode_section_entry(
        &mut bytes,
        SECTION_BODY,
        body_offset,
        body.len(),
        to_u32(records.len())?,
        sha256(&body),
    )?;
    bytes.extend_from_slice(&manifest);
    bytes.extend_from_slice(&body);
    let digest = sha256(&bytes);
    bytes.extend_from_slice(&digest);
    Ok(EncodedArtifact { bytes, digest })
}

fn encode_manifest(
    metadata: &ArtifactMetadata,
    limits: &EvidenceLimits,
) -> Result<Vec<u8>, ContentError> {
    let revisions = &metadata.revisions;
    let fields = [
        EVIDENCE_PROFILE_ID,
        metadata.package_key.as_str(),
        metadata.package_revision.as_str(),
        metadata.world_id.as_str(),
        revisions.content.as_str(),
        revisions.map.as_str(),
        revisions.ruleset.as_str(),
        revisions.world_policy.as_str(),
        revisions.compiler.as_str(),
        revisions.canonicalization.as_str(),
        revisions.content_lock.as_str(),
        revisions.provenance.as_str(),
        revisions.sim_profile.as_str(),
        revisions.fixture_profile.as_str(),
        metadata.projection.as_str(),
    ];
    let mut bytes = Vec::new();
    for field in fields {
        put_string(&mut bytes, field, limits)?;
    }
    Ok(bytes)
}

fn encode_body(
    records: &[EvidenceRecord],
    limits: &EvidenceLimits,
) -> Result<Vec<u8>, ContentError> {
    let mut bytes = Vec::new();
    put_u32(&mut bytes, to_u32(records.len())?);
    for record in records {
        let expected = expected_field_count(record.kind).ok_or(ContentError::InvalidArtifact(
            "compiler emitted unknown record kind",
        ))?;
        if record.fields.len() != usize::from(expected) {
            return Err(ContentError::InvalidArtifact(
                "compiler emitted invalid record shape",
            ));
        }
        let mut encoded = Vec::new();
        encoded.push(record.kind);
        encoded.push(expected);
        for field in &record.fields {
            put_string(&mut encoded, field, limits)?;
        }
        limits.check("record bytes", encoded.len(), limits.max_record_bytes())?;
        let next_len = bytes
            .len()
            .checked_add(4)
            .and_then(|value| value.checked_add(encoded.len()))
            .ok_or(ContentError::InvalidSectionBounds)?;
        limits.check("section bytes", next_len, limits.max_section_bytes())?;
        put_u32(&mut bytes, to_u32(encoded.len())?);
        bytes.extend_from_slice(&encoded);
    }
    Ok(bytes)
}

fn put_string(
    bytes: &mut Vec<u8>,
    value: &str,
    limits: &EvidenceLimits,
) -> Result<(), ContentError> {
    limits.check("string bytes", value.len(), limits.max_string_bytes())?;
    if value.is_empty() || !value.is_ascii() || !value.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(ContentError::InvalidString("artifact field"));
    }
    let next_len = bytes
        .len()
        .checked_add(2)
        .and_then(|current| current.checked_add(value.len()))
        .ok_or(ContentError::InvalidSectionBounds)?;
    limits.check("section bytes", next_len, limits.max_section_bytes())?;
    let length = u16::try_from(value.len()).map_err(|_| ContentError::LimitExceeded {
        resource: "string bytes",
        actual: value.len(),
        limit: usize::from(u16::MAX),
    })?;
    put_u16(bytes, length);
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn encode_section_entry(
    bytes: &mut Vec<u8>,
    kind: u16,
    offset: usize,
    length: usize,
    item_count: u32,
    digest: [u8; 32],
) -> Result<(), ContentError> {
    put_u16(bytes, kind);
    put_u16(bytes, SECTION_FLAG_CRITICAL);
    put_u32(bytes, to_u32(offset)?);
    put_u32(bytes, to_u32(length)?);
    put_u32(bytes, item_count);
    bytes.extend_from_slice(&digest);
    Ok(())
}

fn to_u32(value: usize) -> Result<u32, ContentError> {
    u32::try_from(value).map_err(|_| ContentError::InvalidSectionBounds)
}

fn put_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn put_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SectionEntry {
    kind: u16,
    flags: u16,
    offset: usize,
    length: usize,
    item_count: usize,
    digest: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedRecord {
    kind: u8,
    fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedArtifact {
    metadata: ArtifactMetadata,
    artifact_digest: [u8; 32],
    body_digest: [u8; 32],
}

impl StagedArtifact {
    pub fn stage(bytes: &[u8], limits: &EvidenceLimits) -> Result<Self, ContentError> {
        limits.check("artifact bytes", bytes.len(), limits.max_artifact_bytes())?;
        if bytes.len() < HEADER_LEN + TRAILER_LEN {
            return Err(ContentError::Truncated);
        }
        if bytes.get(..8) != Some(MAGIC.as_slice()) {
            return Err(ContentError::InvalidMagic);
        }
        let profile_version = read_u16_at(bytes, 8)?;
        if profile_version != EVIDENCE_PROFILE_VERSION {
            return Err(ContentError::UnsupportedProfile(profile_version));
        }
        let header_flags = read_u16_at(bytes, 10)?;
        if header_flags != 0 {
            return Err(ContentError::UnknownCriticalFlags(header_flags));
        }
        let projection =
            ProjectionClass::from_byte(*bytes.get(12).ok_or(ContentError::Truncated)?)?;
        if *bytes.get(13).ok_or(ContentError::Truncated)? != 0 {
            return Err(ContentError::InvalidArtifact(
                "reserved header byte is nonzero",
            ));
        }
        let section_count = usize::from(read_u16_at(bytes, 14)?);
        limits.check("sections", section_count, limits.max_sections())?;
        if section_count == 0 {
            return Err(ContentError::InvalidArtifact("artifact has no sections"));
        }
        let table_offset = usize::try_from(read_u32_at(bytes, 16)?)
            .map_err(|_| ContentError::InvalidSectionBounds)?;
        if table_offset != HEADER_LEN {
            return Err(ContentError::InvalidSectionBounds);
        }
        let payload_end = usize::try_from(read_u32_at(bytes, 20)?)
            .map_err(|_| ContentError::InvalidSectionBounds)?;
        let table_bytes = SECTION_ENTRY_LEN
            .checked_mul(section_count)
            .ok_or(ContentError::InvalidSectionBounds)?;
        let table_end = table_offset
            .checked_add(table_bytes)
            .ok_or(ContentError::InvalidSectionBounds)?;
        if table_end > payload_end {
            return Err(ContentError::InvalidSectionBounds);
        }
        let expected_total = payload_end
            .checked_add(TRAILER_LEN)
            .ok_or(ContentError::InvalidSectionBounds)?;
        if expected_total != bytes.len() {
            return Err(ContentError::Truncated);
        }

        let expected_digest = array32(bytes.get(payload_end..).ok_or(ContentError::Truncated)?)?;
        let actual_digest = sha256(bytes.get(..payload_end).ok_or(ContentError::Truncated)?);
        if expected_digest != actual_digest {
            return Err(ContentError::IntegrityMismatch("artifact"));
        }

        let entries = parse_section_entries(bytes, section_count, table_offset, limits)?;
        validate_section_ranges(&entries, table_end, payload_end, limits)?;
        let manifest_entry = unique_known_section(&entries, SECTION_MANIFEST)?;
        let body_entry = unique_known_section(&entries, SECTION_BODY)?;
        if manifest_entry.item_count
            != usize::try_from(MANIFEST_FIELD_COUNT)
                .map_err(|_| ContentError::InvalidArtifact("manifest count conversion"))?
        {
            return Err(ContentError::InvalidArtifact(
                "manifest field count mismatch",
            ));
        }
        limits.check("records", body_entry.item_count, limits.max_records())?;

        let manifest_bytes = section_bytes(bytes, manifest_entry)?;
        let body_bytes = section_bytes(bytes, body_entry)?;
        verify_section_digest(manifest_bytes, manifest_entry)?;
        verify_section_digest(body_bytes, body_entry)?;
        let metadata = parse_manifest(manifest_bytes, projection, limits)?;
        let records = parse_body(body_bytes, body_entry.item_count, limits)?;
        validate_record_semantics(&records, projection, limits)?;

        Ok(Self {
            metadata,
            artifact_digest: actual_digest,
            body_digest: body_entry.digest,
        })
    }
}

fn parse_section_entries(
    bytes: &[u8],
    count: usize,
    table_offset: usize,
    limits: &EvidenceLimits,
) -> Result<Vec<SectionEntry>, ContentError> {
    let mut entries = Vec::with_capacity(count);
    for index in 0..count {
        let start = table_offset
            .checked_add(
                SECTION_ENTRY_LEN
                    .checked_mul(index)
                    .ok_or(ContentError::InvalidSectionBounds)?,
            )
            .ok_or(ContentError::InvalidSectionBounds)?;
        let kind = read_u16_at(bytes, start)?;
        let flags = read_u16_at(bytes, start + 2)?;
        if flags & !SECTION_FLAG_CRITICAL != 0 {
            return Err(ContentError::UnknownCriticalFlags(flags));
        }
        if !matches!(kind, SECTION_MANIFEST | SECTION_BODY) && flags & SECTION_FLAG_CRITICAL != 0 {
            return Err(ContentError::UnknownCriticalSection(kind));
        }
        let offset = usize::try_from(read_u32_at(bytes, start + 4)?)
            .map_err(|_| ContentError::InvalidSectionBounds)?;
        let length = usize::try_from(read_u32_at(bytes, start + 8)?)
            .map_err(|_| ContentError::InvalidSectionBounds)?;
        limits.check("section bytes", length, limits.max_section_bytes())?;
        let item_count = usize::try_from(read_u32_at(bytes, start + 12)?)
            .map_err(|_| ContentError::InvalidSectionBounds)?;
        let digest = array32(
            bytes
                .get(start + 16..start + SECTION_ENTRY_LEN)
                .ok_or(ContentError::Truncated)?,
        )?;
        entries.push(SectionEntry {
            kind,
            flags,
            offset,
            length,
            item_count,
            digest,
        });
    }
    Ok(entries)
}

fn validate_section_ranges(
    entries: &[SectionEntry],
    table_end: usize,
    payload_end: usize,
    limits: &EvidenceLimits,
) -> Result<(), ContentError> {
    let mut ranges = Vec::with_capacity(entries.len());
    for entry in entries {
        let end = entry
            .offset
            .checked_add(entry.length)
            .ok_or(ContentError::InvalidSectionBounds)?;
        if entry.offset < table_end || end > payload_end {
            return Err(ContentError::InvalidSectionBounds);
        }
        limits.check("section bytes", entry.length, limits.max_section_bytes())?;
        ranges.push((entry.offset, end));
    }
    ranges.sort_unstable_by_key(|range| range.0);
    for pair in ranges.windows(2) {
        if pair[0].1 > pair[1].0 {
            return Err(ContentError::InvalidSectionBounds);
        }
    }
    Ok(())
}

fn unique_known_section(
    entries: &[SectionEntry],
    kind: u16,
) -> Result<&SectionEntry, ContentError> {
    let mut matches = entries.iter().filter(|entry| entry.kind == kind);
    let first = matches
        .next()
        .ok_or(ContentError::InvalidArtifact("required section missing"))?;
    if matches.next().is_some() {
        return Err(ContentError::InvalidArtifact("duplicate required section"));
    }
    if first.flags & SECTION_FLAG_CRITICAL == 0 {
        return Err(ContentError::InvalidArtifact(
            "required section is not critical",
        ));
    }
    Ok(first)
}

fn section_bytes<'a>(bytes: &'a [u8], entry: &SectionEntry) -> Result<&'a [u8], ContentError> {
    let end = entry
        .offset
        .checked_add(entry.length)
        .ok_or(ContentError::InvalidSectionBounds)?;
    bytes
        .get(entry.offset..end)
        .ok_or(ContentError::InvalidSectionBounds)
}

fn verify_section_digest(bytes: &[u8], entry: &SectionEntry) -> Result<(), ContentError> {
    if sha256(bytes) != entry.digest {
        return Err(ContentError::IntegrityMismatch("section"));
    }
    Ok(())
}

fn parse_manifest(
    bytes: &[u8],
    projection: ProjectionClass,
    limits: &EvidenceLimits,
) -> Result<ArtifactMetadata, ContentError> {
    let mut reader = SliceReader::new(bytes);
    let profile_id = reader.read_string(limits)?;
    if profile_id != EVIDENCE_PROFILE_ID {
        return Err(ContentError::InvalidArtifact(
            "unexpected evidence profile id",
        ));
    }
    let package_key_value = reader.read_string(limits)?;
    let package_key = PackageKey::new(&package_key_value, limits)?;
    let package_revision = RevisionToken::new(&reader.read_string(limits)?, limits)?;
    let world_id = WorldId::new(&reader.read_string(limits)?, limits)?;
    let revisions = RevisionSet {
        content: RevisionToken::new(&reader.read_string(limits)?, limits)?,
        map: RevisionToken::new(&reader.read_string(limits)?, limits)?,
        ruleset: RevisionToken::new(&reader.read_string(limits)?, limits)?,
        world_policy: RevisionToken::new(&reader.read_string(limits)?, limits)?,
        compiler: RevisionToken::new(&reader.read_string(limits)?, limits)?,
        canonicalization: RevisionToken::new(&reader.read_string(limits)?, limits)?,
        content_lock: RevisionToken::new(&reader.read_string(limits)?, limits)?,
        provenance: RevisionToken::new(&reader.read_string(limits)?, limits)?,
        sim_profile: RevisionToken::new(&reader.read_string(limits)?, limits)?,
        fixture_profile: RevisionToken::new(&reader.read_string(limits)?, limits)?,
    };
    let manifest_projection = reader.read_string(limits)?;
    if manifest_projection != projection.as_str() {
        return Err(ContentError::InvalidArtifact(
            "manifest/header projection mismatch",
        ));
    }
    reader.ensure_end()?;
    Ok(ArtifactMetadata {
        package_key,
        package_revision,
        world_id,
        revisions,
        projection,
    })
}

fn parse_body(
    bytes: &[u8],
    expected_count: usize,
    limits: &EvidenceLimits,
) -> Result<Vec<ParsedRecord>, ContentError> {
    let mut reader = SliceReader::new(bytes);
    let record_count = usize::try_from(reader.read_u32()?)
        .map_err(|_| ContentError::InvalidArtifact("record count conversion"))?;
    limits.check("records", record_count, limits.max_records())?;
    if record_count != expected_count {
        return Err(ContentError::InvalidArtifact("record count mismatch"));
    }
    let mut records = Vec::with_capacity(record_count);
    for _ in 0..record_count {
        let record_len = usize::try_from(reader.read_u32()?)
            .map_err(|_| ContentError::InvalidArtifact("record length conversion"))?;
        limits.check("record bytes", record_len, limits.max_record_bytes())?;
        let raw = reader.take(record_len)?;
        let mut record_reader = SliceReader::new(raw);
        let kind = record_reader.read_u8()?;
        let declared_fields = record_reader.read_u8()?;
        let expected_fields = expected_field_count(kind)
            .ok_or(ContentError::InvalidArtifact("unknown record kind"))?;
        if declared_fields != expected_fields {
            return Err(ContentError::InvalidArtifact("record field count mismatch"));
        }
        let mut fields = Vec::with_capacity(usize::from(expected_fields));
        for _ in 0..expected_fields {
            fields.push(record_reader.read_string(limits)?);
        }
        record_reader.ensure_end()?;
        records.push(ParsedRecord { kind, fields });
    }
    reader.ensure_end()?;
    Ok(records)
}

fn expected_field_count(kind: u8) -> Option<u8> {
    match kind {
        RECORD_REGION | RECORD_AREA | RECORD_TERRAIN | RECORD_LOOT_TABLE | RECORD_RNG_PURPOSE => {
            Some(1)
        }
        RECORD_BEHAVIOR
        | RECORD_PRESENTATION
        | RECORD_FORMULA
        | RECORD_RNG_CONTEXT
        | RECORD_CLIENT_CREATURE
        | RECORD_CLIENT_ABILITY
        | RECORD_CLIENT_ITEM => Some(2),
        RECORD_RELOCATION | RECORD_EFFECT | RECORD_ABILITY | RECORD_ITEM | RECORD_XP => Some(3),
        RECORD_CREATURE => Some(4),
        RECORD_LOOT_ENTRY => Some(5),
        RECORD_CELL | RECORD_SPAWN => Some(8),
        _ => None,
    }
}

fn validate_record_semantics(
    records: &[ParsedRecord],
    projection: ProjectionClass,
    limits: &EvidenceLimits,
) -> Result<(), ContentError> {
    let mut definitions = BTreeSet::new();
    let mut references = Vec::new();
    let mut has_presentation = false;
    let mut has_creature = false;
    let mut has_ability = false;
    let mut has_item = false;
    let mut has_cell = false;
    let mut has_spawn = false;
    let mut has_loot = false;

    for record in records {
        match record.kind {
            RECORD_REGION | RECORD_AREA | RECORD_TERRAIN | RECORD_CELL | RECORD_RELOCATION
            | RECORD_BEHAVIOR | RECORD_PRESENTATION | RECORD_CREATURE | RECORD_SPAWN
            | RECORD_FORMULA | RECORD_EFFECT | RECORD_ABILITY | RECORD_ITEM | RECORD_LOOT_TABLE
            | RECORD_XP | RECORD_RNG_PURPOSE => {
                insert_definition(&mut definitions, &record.fields[0], limits)?;
            }
            RECORD_LOOT_ENTRY => insert_definition(&mut definitions, &record.fields[1], limits)?,
            RECORD_CLIENT_CREATURE | RECORD_CLIENT_ABILITY | RECORD_CLIENT_ITEM => {
                insert_definition(&mut definitions, &record.fields[0], limits)?;
            }
            RECORD_RNG_CONTEXT => {}
            _ => return Err(ContentError::InvalidArtifact("unsupported record kind")),
        }

        match record.kind {
            RECORD_CELL => {
                has_cell = true;
                references.extend([
                    record.fields[1].clone(),
                    record.fields[2].clone(),
                    record.fields[3].clone(),
                ]);
                parse_i32(&record.fields[4])?;
                parse_i32(&record.fields[5])?;
                parse_i16(&record.fields[6])?;
                if !matches!(record.fields[7].as_str(), "walkable" | "blocked") {
                    return Err(ContentError::InvalidArtifact("invalid collision class"));
                }
            }
            RECORD_RELOCATION => {
                references.extend([record.fields[1].clone(), record.fields[2].clone()]);
            }
            RECORD_PRESENTATION => {
                has_presentation = true;
                if !record.fields[1].starts_with("synthetic://") {
                    return Err(ContentError::InvalidArtifact(
                        "non-synthetic VSL presentation token",
                    ));
                }
            }
            RECORD_CREATURE => {
                has_creature = true;
                references.extend([record.fields[1].clone(), record.fields[2].clone()]);
                parse_positive_u32(&record.fields[3])?;
            }
            RECORD_SPAWN => {
                has_spawn = true;
                references.extend([
                    record.fields[1].clone(),
                    record.fields[2].clone(),
                    record.fields[3].clone(),
                ]);
                parse_positive_u16(&record.fields[4])?;
                if !matches!(
                    record.fields[5].as_str(),
                    "EPHEMERAL_SCOPE_RESET"
                        | "CHECKPOINTED_RUNTIME_CONTINUITY"
                        | "DURABLE_EVENT_OCCURRENCE"
                ) {
                    return Err(ContentError::InvalidArtifact(
                        "invalid spawn recovery class",
                    ));
                }
                if !matches!(
                    record.fields[6].as_str(),
                    "CHANNEL_LOCAL_REPEATABLE"
                        | "CHANNEL_LOCAL_SHARED_ELIGIBILITY"
                        | "WORLD_SCOPED_UNIQUE"
                        | "EXPLICIT_EVENT_POLICY_REQUIRED"
                ) {
                    return Err(ContentError::InvalidArtifact(
                        "invalid channel multiplicity class",
                    ));
                }
                if !matches!(
                    record.fields[7].as_str(),
                    "CHARACTER_WORLD" | "ACCOUNT_WORLD" | "WORLD"
                ) {
                    return Err(ContentError::InvalidArtifact("invalid eligibility scope"));
                }
            }
            RECORD_FORMULA => {
                if record.fields[1] != "true" {
                    return Err(ContentError::InvalidArtifact(
                        "VSL formula profile is not fixture-only",
                    ));
                }
            }
            RECORD_EFFECT => {
                references.push(record.fields[2].clone());
                if record.fields[1] != "damage" {
                    return Err(ContentError::InvalidArtifact(
                        "unsupported VSL effect family",
                    ));
                }
            }
            RECORD_ABILITY => {
                has_ability = true;
                references.extend([record.fields[1].clone(), record.fields[2].clone()]);
            }
            RECORD_ITEM => {
                has_item = true;
                references.push(record.fields[1].clone());
                if record.fields[2] != "true" {
                    return Err(ContentError::InvalidArtifact(
                        "VSL item is not materializable",
                    ));
                }
            }
            RECORD_LOOT_TABLE => has_loot = true,
            RECORD_LOOT_ENTRY => {
                references.extend([
                    record.fields[0].clone(),
                    record.fields[2].clone(),
                    record.fields[3].clone(),
                ]);
                parse_positive_u32(&record.fields[4])?;
            }
            RECORD_XP => {
                references.push(record.fields[1].clone());
                parse_positive_u32(&record.fields[2])?;
            }
            RECORD_CLIENT_CREATURE | RECORD_CLIENT_ABILITY | RECORD_CLIENT_ITEM => {
                references.push(record.fields[1].clone());
                match record.kind {
                    RECORD_CLIENT_CREATURE => has_creature = true,
                    RECORD_CLIENT_ABILITY => has_ability = true,
                    RECORD_CLIENT_ITEM => has_item = true,
                    _ => {}
                }
            }
            RECORD_RNG_CONTEXT | RECORD_RNG_PURPOSE | RECORD_REGION | RECORD_AREA
            | RECORD_TERRAIN | RECORD_BEHAVIOR => {}
            _ => {
                return Err(ContentError::InvalidArtifact(
                    "unsupported record semantics",
                ));
            }
        }
    }

    for target in references {
        ContentKey::new(&target, limits)?;
        if !definitions.contains(&target) {
            return Err(ContentError::MissingReference {
                owner: "evidence artifact".to_owned(),
                target,
            });
        }
    }

    match projection {
        ProjectionClass::ServerAuthoritative => {
            if !(has_cell
                && has_spawn
                && has_creature
                && has_ability
                && has_item
                && has_loot
                && has_presentation)
            {
                return Err(ContentError::InvalidArtifact(
                    "server projection is incomplete",
                ));
            }
        }
        ProjectionClass::ClientSafe => {
            if !(has_presentation && has_creature && has_ability && has_item) {
                return Err(ContentError::InvalidArtifact(
                    "client-safe projection is incomplete",
                ));
            }
            if records.iter().any(|record| {
                !matches!(
                    record.kind,
                    RECORD_PRESENTATION
                        | RECORD_CLIENT_CREATURE
                        | RECORD_CLIENT_ABILITY
                        | RECORD_CLIENT_ITEM
                )
            }) {
                return Err(ContentError::InvalidArtifact(
                    "server-only record leaked into client projection",
                ));
            }
        }
    }
    Ok(())
}

fn insert_definition(
    definitions: &mut BTreeSet<String>,
    key: &str,
    limits: &EvidenceLimits,
) -> Result<(), ContentError> {
    ContentKey::new(key, limits)?;
    if !definitions.insert(key.to_owned()) {
        return Err(ContentError::DuplicateKey(key.to_owned()));
    }
    Ok(())
}

fn parse_i32(value: &str) -> Result<i32, ContentError> {
    value
        .parse::<i32>()
        .map_err(|_| ContentError::InvalidArtifact("invalid i32 field"))
}

fn parse_i16(value: &str) -> Result<i16, ContentError> {
    value
        .parse::<i16>()
        .map_err(|_| ContentError::InvalidArtifact("invalid i16 field"))
}

fn parse_positive_u32(value: &str) -> Result<u32, ContentError> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| ContentError::InvalidArtifact("invalid u32 field"))?;
    if parsed == 0 {
        return Err(ContentError::InvalidArtifact("zero value is invalid"));
    }
    Ok(parsed)
}

fn parse_positive_u16(value: &str) -> Result<u16, ContentError> {
    let parsed = value
        .parse::<u16>()
        .map_err(|_| ContentError::InvalidArtifact("invalid u16 field"))?;
    if parsed == 0 {
        return Err(ContentError::InvalidArtifact("zero value is invalid"));
    }
    Ok(parsed)
}

struct SliceReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> SliceReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], ContentError> {
        let end = self
            .position
            .checked_add(length)
            .ok_or(ContentError::InvalidSectionBounds)?;
        let result = self
            .bytes
            .get(self.position..end)
            .ok_or(ContentError::Truncated)?;
        self.position = end;
        Ok(result)
    }

    fn read_u8(&mut self) -> Result<u8, ContentError> {
        Ok(*self.take(1)?.first().ok_or(ContentError::Truncated)?)
    }

    fn read_u16(&mut self) -> Result<u16, ContentError> {
        let raw = self.take(2)?;
        Ok(u16::from_be_bytes([raw[0], raw[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, ContentError> {
        let raw = self.take(4)?;
        Ok(u32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]))
    }

    fn read_string(&mut self, limits: &EvidenceLimits) -> Result<String, ContentError> {
        let length = usize::from(self.read_u16()?);
        limits.check("string bytes", length, limits.max_string_bytes())?;
        let raw = self.take(length)?;
        if raw.is_empty() || !raw.iter().all(|byte| byte.is_ascii_graphic()) {
            return Err(ContentError::InvalidString("artifact field"));
        }
        let value =
            std::str::from_utf8(raw).map_err(|_| ContentError::InvalidString("artifact field"))?;
        Ok(value.to_owned())
    }

    fn ensure_end(&self) -> Result<(), ContentError> {
        if self.position != self.bytes.len() {
            return Err(ContentError::InvalidArtifact("trailing bytes"));
        }
        Ok(())
    }
}

fn read_u16_at(bytes: &[u8], offset: usize) -> Result<u16, ContentError> {
    let end = offset
        .checked_add(2)
        .ok_or(ContentError::InvalidSectionBounds)?;
    let raw = bytes.get(offset..end).ok_or(ContentError::Truncated)?;
    Ok(u16::from_be_bytes([raw[0], raw[1]]))
}

fn read_u32_at(bytes: &[u8], offset: usize) -> Result<u32, ContentError> {
    let end = offset
        .checked_add(4)
        .ok_or(ContentError::InvalidSectionBounds)?;
    let raw = bytes.get(offset..end).ok_or(ContentError::Truncated)?;
    Ok(u32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn array32(bytes: &[u8]) -> Result<[u8; 32], ContentError> {
    <[u8; 32]>::try_from(bytes).map_err(|_| ContentError::Truncated)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedContentPair {
    server: StagedArtifact,
    client: StagedArtifact,
}

impl StagedContentPair {
    pub fn stage(
        server_bytes: &[u8],
        client_bytes: &[u8],
        limits: &EvidenceLimits,
    ) -> Result<Self, ContentError> {
        let server = StagedArtifact::stage(server_bytes, limits)?;
        let client = StagedArtifact::stage(client_bytes, limits)?;
        if server.metadata.projection != ProjectionClass::ServerAuthoritative {
            return Err(ContentError::PairMismatch(
                "server artifact has wrong projection",
            ));
        }
        if client.metadata.projection != ProjectionClass::ClientSafe {
            return Err(ContentError::PairMismatch(
                "client artifact has wrong projection",
            ));
        }
        if !server.metadata.same_revision_identity(&client.metadata) {
            return Err(ContentError::PairMismatch(
                "semantic revision identity differs",
            ));
        }
        Ok(Self { server, client })
    }

    pub fn verify_expected(&self, expectation: &ArtifactExpectation) -> Result<(), ContentError> {
        expectation.verify(&self.server.metadata)?;
        expectation.verify(&self.client.metadata)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(test)]
pub struct ActiveContent {
    content_revision: RevisionToken,
    server_artifact_digest: [u8; 32],
    client_artifact_digest: [u8; 32],
    server_body_digest: [u8; 32],
    client_body_digest: [u8; 32],
}

#[cfg(test)]
impl ActiveContent {
    pub fn content_revision(&self) -> &str {
        self.content_revision.as_str()
    }
}

#[derive(Debug, Default)]
#[cfg(test)]
pub struct ActivationSlot {
    active: Option<ActiveContent>,
}

#[cfg(test)]
impl ActivationSlot {
    pub const fn new() -> Self {
        Self { active: None }
    }

    pub fn active(&self) -> Option<&ActiveContent> {
        self.active.as_ref()
    }

    pub fn stage_and_activate(
        &mut self,
        server_bytes: &[u8],
        client_bytes: &[u8],
        expectation: &ArtifactExpectation,
        limits: &EvidenceLimits,
    ) -> Result<&ActiveContent, ContentError> {
        let staged = StagedContentPair::stage(server_bytes, client_bytes, limits)?;
        staged.verify_expected(expectation)?;
        let next = ActiveContent {
            content_revision: staged.server.metadata.revisions.content.clone(),
            server_artifact_digest: staged.server.artifact_digest,
            client_artifact_digest: staged.client.artifact_digest,
            server_body_digest: staged.server.body_digest,
            client_body_digest: staged.client.body_digest,
        };
        self.active = Some(next);
        self.active.as_ref().ok_or(ContentError::InvalidArtifact(
            "activation publication failed",
        ))
    }
}

#[cfg(test)]
pub(crate) fn test_reseal_unknown_critical_section(bytes: &[u8]) -> Result<Vec<u8>, ContentError> {
    let mut changed = bytes.to_vec();
    let second_entry = HEADER_LEN
        .checked_add(SECTION_ENTRY_LEN)
        .ok_or(ContentError::InvalidSectionBounds)?;
    let kind_range = second_entry..second_entry + 2;
    changed
        .get_mut(kind_range)
        .ok_or(ContentError::Truncated)?
        .copy_from_slice(&0x7ffe_u16.to_be_bytes());
    reseal_for_test(&mut changed)?;
    Ok(changed)
}

#[cfg(test)]
pub(crate) fn test_reseal_body_offset(bytes: &[u8], offset: u32) -> Result<Vec<u8>, ContentError> {
    let mut changed = bytes.to_vec();
    let second_entry = HEADER_LEN
        .checked_add(SECTION_ENTRY_LEN)
        .ok_or(ContentError::InvalidSectionBounds)?;
    let offset_start = second_entry
        .checked_add(4)
        .ok_or(ContentError::InvalidSectionBounds)?;
    changed
        .get_mut(offset_start..offset_start + 4)
        .ok_or(ContentError::Truncated)?
        .copy_from_slice(&offset.to_be_bytes());
    reseal_for_test(&mut changed)?;
    Ok(changed)
}

#[cfg(test)]
fn reseal_for_test(bytes: &mut [u8]) -> Result<(), ContentError> {
    if bytes.len() < TRAILER_LEN {
        return Err(ContentError::Truncated);
    }
    let payload_end = bytes.len() - TRAILER_LEN;
    let digest = sha256(&bytes[..payload_end]);
    bytes[payload_end..].copy_from_slice(&digest);
    Ok(())
}
