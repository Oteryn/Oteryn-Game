use super::digest::sha256;
use super::model::{
    CollisionClass, ContentError, EffectFamily, EligibilityScope, MultiplicityClass,
    SpawnRecoveryClass,
};
use crate::domain::WorldId;
use std::collections::BTreeSet;

pub const FIRST_PRODUCTION_PROFILE_ID: &str = "FIRST_PRODUCTION_CONTENT_PROFILE/v1";
pub const FIRST_PRODUCTION_ARTIFACT_PROFILE_ID: &str = "OTERYN_FIRST_PRODUCTION_BOOTSTRAP/v1";
pub const FIRST_PRODUCTION_CAPABILITY_PROFILE: &str = "content:first-production-v1";

pub const FIRST_PRODUCTION_MAX_MANIFEST_FIELDS: usize = 20;
pub const FIRST_PRODUCTION_MAX_MANIFEST_BYTES: usize = 9_384;
pub const FIRST_PRODUCTION_MAX_SERVER_ARTIFACT_BYTES: usize = 4_304_614;
pub const FIRST_PRODUCTION_MAX_CLIENT_ARTIFACT_BYTES: usize = 34_248;
pub const FIRST_PRODUCTION_MAX_GENERATION_PAIR_BYTES: usize = 4_338_862;
pub const FIRST_PRODUCTION_MAX_SECTION_BYTES: usize = 4_295_078;
pub const FIRST_PRODUCTION_MAX_RECORD_BYTES: usize = 4_114;
pub const FIRST_PRODUCTION_MAX_KEY_BYTES: usize = 512;
pub const FIRST_PRODUCTION_MAX_ATOM_BYTES: usize = 512;
pub const FIRST_PRODUCTION_MAX_DEFINITIONS: usize = 1_042;
pub const FIRST_PRODUCTION_MAX_REFERENCES: usize = 3_087;
pub const FIRST_PRODUCTION_MAX_SERVER_RECORDS: usize = 1_043;
pub const FIRST_PRODUCTION_MAX_CLIENT_RECORDS: usize = 6;
pub const FIRST_PRODUCTION_MAX_CELLS: usize = 1_024;
pub const FIRST_PRODUCTION_MAX_X_SPAN: usize = 32;
pub const FIRST_PRODUCTION_MAX_Y_SPAN: usize = 32;
pub const FIRST_PRODUCTION_MAX_FLOORS: usize = 1;
pub const FIRST_PRODUCTION_MAX_DECODED_FIELDS: usize = 8_432;
pub const FIRST_PRODUCTION_MAX_SPAWN_POPULATION: usize = 1;
pub const FIRST_PRODUCTION_MAX_SCOPE_POPULATION: usize = 1;
pub const FIRST_PRODUCTION_MAX_CONTENT_LOCK_ENTRIES: usize = 1;

const MAGIC: [u8; 8] = *b"OTFPC01\0";
const PROFILE_VERSION: u16 = 1;
const HEADER_LEN: usize = 24;
const SECTION_ENTRY_LEN: usize = 48;
const TRAILER_LEN: usize = 32;
const SECTION_FLAG_CRITICAL: u16 = 0x0001;
const SECTION_MANIFEST: u16 = 1;
const SECTION_BODY: u16 = 2;

const RECORD_REGION: u8 = 1;
const RECORD_AREA: u8 = 2;
const RECORD_TERRAIN: u8 = 3;
const RECORD_CELL: u8 = 4;
const RECORD_RELOCATION: u8 = 5;
const RECORD_BEHAVIOR: u8 = 6;
const RECORD_PRESENTATION: u8 = 7;
const RECORD_CREATURE: u8 = 8;
const RECORD_SPAWN: u8 = 9;
const RECORD_FORMULA: u8 = 10;
const RECORD_EFFECT: u8 = 11;
const RECORD_ABILITY: u8 = 12;
const RECORD_ITEM: u8 = 13;
const RECORD_LOOT_TABLE: u8 = 14;
const RECORD_LOOT_ENTRY: u8 = 15;
const RECORD_XP: u8 = 16;
const RECORD_RNG_CONTEXT: u8 = 17;
const RECORD_RNG_PURPOSE: u8 = 18;
const RECORD_CLIENT_CREATURE: u8 = 108;
const RECORD_CLIENT_ABILITY: u8 = 112;
const RECORD_CLIENT_ITEM: u8 = 113;

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

    fn check_exact(
        self,
        resource: &'static str,
        actual: usize,
        expected: usize,
    ) -> Result<(), ContentError> {
        if actual > expected {
            return Err(ContentError::LimitExceeded {
                resource,
                actual,
                limit: expected,
            });
        }
        if actual < expected {
            return Err(ContentError::InvalidArtifact(resource));
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

    fn from_artifact(field: &'static str, value: String) -> Result<Self, ContentError> {
        validate_atom(field, &value, FIRST_PRODUCTION_MAX_ATOM_BYTES, true)?;
        Ok(Self(value))
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

fn encode_world_id(world_id: WorldId) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(32);
    for byte in world_id.as_bytes() {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn parse_world_id(value: &str) -> Result<WorldId, ContentError> {
    if value.len() != 32 {
        return Err(ContentError::InvalidArtifact(
            "first-production WorldId must be 32 lowercase hexadecimal UUIDv7 bytes",
        ));
    }
    let mut bytes = [0_u8; 16];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = decode_lower_hex(pair[0]).ok_or(ContentError::InvalidArtifact(
            "first-production WorldId must be lowercase hexadecimal",
        ))?;
        let low = decode_lower_hex(pair[1]).ok_or(ContentError::InvalidArtifact(
            "first-production WorldId must be lowercase hexadecimal",
        ))?;
        bytes[index] = (high << 4) | low;
    }
    WorldId::from_bytes(bytes)
        .map_err(|_| ContentError::InvalidArtifact("first-production WorldId must be UUIDv7"))
}

fn decode_lower_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
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

impl ContentLockBinding {
    fn validate(&self, package: &PackageManifestBinding) -> Result<(), ContentError> {
        FirstProductionLimits::v1().check_exact(
            "first-production Content Lock must contain exactly one entry",
            self.entries.len(),
            FIRST_PRODUCTION_MAX_CONTENT_LOCK_ENTRIES,
        )?;
        let entry = self
            .entries
            .first()
            .ok_or(ContentError::InvalidArtifact("Content Lock entry missing"))?;
        if entry.floating || entry.dependency {
            return Err(ContentError::InvalidArtifact(
                "floating or dependency Content Lock entry is unavailable in v1",
            ));
        }
        let expected_digest = package.package_provenance_digest()?;
        if entry.package_key != package.package_key
            || entry.package_revision != package.package_revision
            || entry.package_provenance_digest != expected_digest
        {
            return Err(ContentError::InvalidArtifact(
                "Content Lock does not bind exact package revision and provenance",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionRevisionSet {
    pub content: ProductionAtom,
    pub map: ProductionAtom,
    pub ruleset: ProductionAtom,
    pub world_policy: ProductionAtom,
    pub compiler: ProductionAtom,
    pub canonicalization: ProductionAtom,
    pub sim_profile: ProductionAtom,
    pub profile_revision: ProductionAtom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurableMigrationClass {
    CompatibleNoMigration,
    MigrationRequired,
}

impl DurableMigrationClass {
    fn as_str(self) -> &'static str {
        match self {
            Self::CompatibleNoMigration => "COMPATIBLE_NO_MIGRATION",
            Self::MigrationRequired => "MIGRATION_REQUIRED",
        }
    }

    fn from_str(value: &str) -> Result<Self, ContentError> {
        match value {
            "COMPATIBLE_NO_MIGRATION" => Ok(Self::CompatibleNoMigration),
            _ => Err(ContentError::InvalidArtifact(
                "migration-bearing content is unavailable in first-production v1",
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstProductionCompileTarget {
    OrdinaryRelease,
    NonProductionEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionRegion {
    pub key: ProductionKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionArea {
    pub key: ProductionKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionTerrain {
    pub key: ProductionKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionCell {
    pub key: ProductionKey,
    pub region_key: ProductionKey,
    pub area_key: ProductionKey,
    pub terrain_key: ProductionKey,
    pub x: i32,
    pub y: i32,
    pub z: i16,
    pub collision: CollisionClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionRelocation {
    pub key: ProductionKey,
    pub from_cell: ProductionKey,
    pub to_cell: ProductionKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionBehavior {
    pub key: ProductionKey,
    pub policy_revision: ProductionAtom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionPresentation {
    pub key: ProductionKey,
    pub metadata_token: ProductionAtom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionCreature {
    pub key: ProductionKey,
    pub behavior_key: ProductionKey,
    pub presentation_key: ProductionKey,
    pub policy_revision: ProductionAtom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionSpawn {
    pub key: ProductionKey,
    pub creature_key: ProductionKey,
    pub behavior_key: ProductionKey,
    pub cell_key: ProductionKey,
    pub population_limit: u16,
    pub recovery: SpawnRecoveryClass,
    pub multiplicity: MultiplicityClass,
    pub eligibility_scope: EligibilityScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionFormulaProfile {
    pub key: ProductionKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionEffect {
    pub key: ProductionKey,
    pub family: EffectFamily,
    pub formula_profile_key: ProductionKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionAbility {
    pub key: ProductionKey,
    pub effect_key: ProductionKey,
    pub presentation_key: ProductionKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionItem {
    pub key: ProductionKey,
    pub presentation_key: ProductionKey,
    pub materializable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionLootEntry {
    pub key: ProductionKey,
    pub item_key: ProductionKey,
    pub rng_purpose_key: ProductionKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionLootTable {
    pub key: ProductionKey,
    pub entries: Vec<FirstProductionLootEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionXpDefinition {
    pub key: ProductionKey,
    pub formula_profile_key: ProductionKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionRngContext {
    pub profile_revision: ProductionAtom,
    pub purpose_keys: Vec<ProductionKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionContentSource {
    pub package_manifest: PackageManifestBinding,
    pub content_lock: ContentLockBinding,
    pub world_id: WorldId,
    pub revisions: FirstProductionRevisionSet,
    pub capability_profile: ProductionAtom,
    pub migration_class: DurableMigrationClass,
    pub regions: Vec<FirstProductionRegion>,
    pub areas: Vec<FirstProductionArea>,
    pub terrains: Vec<FirstProductionTerrain>,
    pub cells: Vec<FirstProductionCell>,
    pub relocations: Vec<FirstProductionRelocation>,
    pub behaviors: Vec<FirstProductionBehavior>,
    pub presentations: Vec<FirstProductionPresentation>,
    pub creatures: Vec<FirstProductionCreature>,
    pub spawns: Vec<FirstProductionSpawn>,
    pub formula_profiles: Vec<FirstProductionFormulaProfile>,
    pub effects: Vec<FirstProductionEffect>,
    pub abilities: Vec<FirstProductionAbility>,
    pub items: Vec<FirstProductionItem>,
    pub loot_tables: Vec<FirstProductionLootTable>,
    pub xp_definitions: Vec<FirstProductionXpDefinition>,
    pub rng: FirstProductionRngContext,
}

impl FirstProductionContentSource {
    #[cfg(test)]
    pub(crate) fn reverse_enumeration_for_test(&mut self) {
        self.regions.reverse();
        self.areas.reverse();
        self.terrains.reverse();
        self.cells.reverse();
        self.relocations.reverse();
        self.behaviors.reverse();
        self.presentations.reverse();
        self.creatures.reverse();
        self.spawns.reverse();
        self.formula_profiles.reverse();
        self.effects.reverse();
        self.abilities.reverse();
        self.items.reverse();
        self.loot_tables.reverse();
        self.xp_definitions.reverse();
        self.rng.purpose_keys.reverse();
        for table in &mut self.loot_tables {
            table.entries.reverse();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalFirstProductionSource {
    source: FirstProductionContentSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProductionRecord {
    kind: u8,
    fields: Vec<String>,
}

impl ProductionRecord {
    fn new(kind: u8, fields: Vec<String>) -> Self {
        Self { kind, fields }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProductionProjection {
    ServerAuthoritative,
    ClientSafe,
}

impl ProductionProjection {
    fn as_byte(self) -> u8 {
        match self {
            Self::ServerAuthoritative => 1,
            Self::ClientSafe => 2,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::ServerAuthoritative => "server-authoritative",
            Self::ClientSafe => "client-safe",
        }
    }

    fn from_byte(value: u8) -> Result<Self, ContentError> {
        match value {
            1 => Ok(Self::ServerAuthoritative),
            2 => Ok(Self::ClientSafe),
            _ => Err(ContentError::InvalidArtifact(
                "unknown first-production projection class",
            )),
        }
    }

    fn artifact_limit(self) -> usize {
        match self {
            Self::ServerAuthoritative => FIRST_PRODUCTION_MAX_SERVER_ARTIFACT_BYTES,
            Self::ClientSafe => FIRST_PRODUCTION_MAX_CLIENT_ARTIFACT_BYTES,
        }
    }

    fn record_limit(self) -> usize {
        match self {
            Self::ServerAuthoritative => FIRST_PRODUCTION_MAX_SERVER_RECORDS,
            Self::ClientSafe => FIRST_PRODUCTION_MAX_CLIENT_RECORDS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProductionArtifactMetadata {
    package_key: ProductionKey,
    package_revision: ProductionAtom,
    semantic_schema_version: ProductionAtom,
    licensing_metadata: ProductionAtom,
    source_manifest_digest: Sha256HexDigest,
    world_id: WorldId,
    revisions: FirstProductionRevisionSet,
    content_lock_token: ProductionAtom,
    package_provenance_digest: Sha256HexDigest,
    projection: ProductionProjection,
    migration_class: DurableMigrationClass,
    capability_profile: ProductionAtom,
}

impl ProductionArtifactMetadata {
    fn from_source(
        source: &FirstProductionContentSource,
        projection: ProductionProjection,
    ) -> Result<Self, ContentError> {
        Ok(Self {
            package_key: source.package_manifest.package_key.clone(),
            package_revision: source.package_manifest.package_revision.clone(),
            semantic_schema_version: source.package_manifest.semantic_schema_version.clone(),
            licensing_metadata: source.package_manifest.licensing_metadata.clone(),
            source_manifest_digest: source.package_manifest.source_manifest_digest.clone(),
            world_id: source.world_id,
            revisions: source.revisions.clone(),
            content_lock_token: source.content_lock.revision_digest_token.clone(),
            package_provenance_digest: source.package_manifest.package_provenance_digest()?,
            projection,
            migration_class: source.migration_class,
            capability_profile: source.capability_profile.clone(),
        })
    }

    fn same_generation_identity(&self, other: &Self) -> bool {
        self.package_key == other.package_key
            && self.package_revision == other.package_revision
            && self.semantic_schema_version == other.semantic_schema_version
            && self.licensing_metadata == other.licensing_metadata
            && self.source_manifest_digest == other.source_manifest_digest
            && self.world_id == other.world_id
            && self.revisions == other.revisions
            && self.content_lock_token == other.content_lock_token
            && self.package_provenance_digest == other.package_provenance_digest
            && self.migration_class == other.migration_class
            && self.capability_profile == other.capability_profile
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstProductionExpectation {
    metadata: ProductionArtifactMetadata,
    server_artifact_digest: [u8; 32],
    client_artifact_digest: [u8; 32],
}

impl FirstProductionExpectation {
    fn from_source(
        source: &FirstProductionContentSource,
        server_artifact_digest: [u8; 32],
        client_artifact_digest: [u8; 32],
    ) -> Result<Self, ContentError> {
        Ok(Self {
            metadata: ProductionArtifactMetadata::from_source(
                source,
                ProductionProjection::ServerAuthoritative,
            )?,
            server_artifact_digest,
            client_artifact_digest,
        })
    }

    pub fn from_generation_identity(identity: &GenerationIdentity) -> Self {
        Self {
            metadata: ProductionArtifactMetadata {
                package_key: identity.package_key.clone(),
                package_revision: identity.package_revision.clone(),
                semantic_schema_version: identity.semantic_schema_version.clone(),
                licensing_metadata: identity.licensing_metadata.clone(),
                source_manifest_digest: identity.source_manifest_digest.clone(),
                world_id: identity.world_id,
                revisions: identity.revisions.clone(),
                content_lock_token: identity.content_lock_token.clone(),
                package_provenance_digest: identity.package_provenance_digest.clone(),
                projection: ProductionProjection::ServerAuthoritative,
                migration_class: identity.migration_class,
                capability_profile: identity.capability_profile.clone(),
            },
            server_artifact_digest: identity.server_artifact_digest,
            client_artifact_digest: identity.client_artifact_digest,
        }
    }

    pub fn package_provenance_digest(&self) -> &str {
        self.metadata.package_provenance_digest.as_str()
    }

    pub fn content_revision(&self) -> &str {
        self.metadata.revisions.content.as_str()
    }

    pub fn package_revision(&self) -> &str {
        self.metadata.package_revision.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledFirstProductionContent {
    pub server_artifact: Vec<u8>,
    pub client_artifact: Vec<u8>,
    server_digest: [u8; 32],
    client_digest: [u8; 32],
    expectation: FirstProductionExpectation,
}

impl CompiledFirstProductionContent {
    pub fn server_digest(&self) -> [u8; 32] {
        self.server_digest
    }

    pub fn client_digest(&self) -> [u8; 32] {
        self.client_digest
    }

    pub fn expectation(&self) -> &FirstProductionExpectation {
        &self.expectation
    }
}

pub fn compile_first_production(
    source: &FirstProductionContentSource,
    target: FirstProductionCompileTarget,
) -> Result<CompiledFirstProductionContent, ContentError> {
    if target != FirstProductionCompileTarget::OrdinaryRelease {
        return Err(ContentError::FixtureOnlyReleaseRejected);
    }
    let canonical = canonicalize_first_production(source)?;
    let server_records = server_records(&canonical.source)?;
    let client_records = client_records(&canonical.source);
    let server_metadata = ProductionArtifactMetadata::from_source(
        &canonical.source,
        ProductionProjection::ServerAuthoritative,
    )?;
    let client_metadata = ProductionArtifactMetadata::from_source(
        &canonical.source,
        ProductionProjection::ClientSafe,
    )?;

    let server = encode_artifact(&server_metadata, &server_records)?;
    let client = encode_artifact(&client_metadata, &client_records)?;
    check_pair_lengths(server.bytes.len(), client.bytes.len())?;

    Ok(CompiledFirstProductionContent {
        server_artifact: server.bytes,
        client_artifact: client.bytes,
        server_digest: server.digest,
        client_digest: client.digest,
        expectation: FirstProductionExpectation::from_source(
            &canonical.source,
            server.digest,
            client.digest,
        )?,
    })
}

fn canonicalize_first_production(
    source: &FirstProductionContentSource,
) -> Result<CanonicalFirstProductionSource, ContentError> {
    validate_source_shape(source)?;
    validate_source_semantics(source)?;
    let mut canonical = source.clone();
    canonical.regions.sort_by(|a, b| a.key.cmp(&b.key));
    canonical.areas.sort_by(|a, b| a.key.cmp(&b.key));
    canonical.terrains.sort_by(|a, b| a.key.cmp(&b.key));
    canonical.cells.sort_by(|a, b| a.key.cmp(&b.key));
    canonical.relocations.sort_by(|a, b| a.key.cmp(&b.key));
    canonical.behaviors.sort_by(|a, b| a.key.cmp(&b.key));
    canonical.presentations.sort_by(|a, b| a.key.cmp(&b.key));
    canonical.creatures.sort_by(|a, b| a.key.cmp(&b.key));
    canonical.spawns.sort_by(|a, b| a.key.cmp(&b.key));
    canonical.formula_profiles.sort_by(|a, b| a.key.cmp(&b.key));
    canonical.effects.sort_by(|a, b| a.key.cmp(&b.key));
    canonical.abilities.sort_by(|a, b| a.key.cmp(&b.key));
    canonical.items.sort_by(|a, b| a.key.cmp(&b.key));
    canonical.loot_tables.sort_by(|a, b| a.key.cmp(&b.key));
    for table in &mut canonical.loot_tables {
        table.entries.sort_by(|a, b| a.key.cmp(&b.key));
    }
    canonical.xp_definitions.sort_by(|a, b| a.key.cmp(&b.key));
    canonical.rng.purpose_keys.sort();
    Ok(CanonicalFirstProductionSource { source: canonical })
}

fn validate_source_shape(source: &FirstProductionContentSource) -> Result<(), ContentError> {
    let limits = FirstProductionLimits::v1();
    if source.capability_profile.as_str() != FIRST_PRODUCTION_CAPABILITY_PROFILE {
        return Err(ContentError::InvalidArtifact(
            "first-production required capability profile mismatch",
        ));
    }
    if source.revisions.profile_revision.as_str() != FIRST_PRODUCTION_PROFILE_ID {
        return Err(ContentError::InvalidArtifact(
            "first-production profile revision mismatch",
        ));
    }
    if source.migration_class != DurableMigrationClass::CompatibleNoMigration {
        return Err(ContentError::InvalidArtifact(
            "migration-bearing content is unavailable in first-production v1",
        ));
    }
    source.content_lock.validate(&source.package_manifest)?;

    limits.check_exact(
        "first-production regions must equal 1",
        source.regions.len(),
        1,
    )?;
    limits.check_exact("first-production areas must equal 1", source.areas.len(), 1)?;
    limits.check_exact(
        "first-production terrains must equal 1",
        source.terrains.len(),
        1,
    )?;
    if source.cells.len() < 3 {
        return Err(ContentError::InvalidArtifact(
            "first-production cells must contain at least 3 entries",
        ));
    }
    limits.check(
        "first-production cells",
        source.cells.len(),
        FIRST_PRODUCTION_MAX_CELLS,
    )?;
    limits.check_exact(
        "first-production relocations must equal 1",
        source.relocations.len(),
        1,
    )?;
    limits.check_exact(
        "first-production behaviors must equal 1",
        source.behaviors.len(),
        1,
    )?;
    limits.check_exact(
        "first-production presentations must equal 3",
        source.presentations.len(),
        3,
    )?;
    limits.check_exact(
        "first-production creatures must equal 1",
        source.creatures.len(),
        1,
    )?;
    limits.check_exact(
        "first-production spawns must equal 1",
        source.spawns.len(),
        1,
    )?;
    limits.check_exact(
        "first-production formula profiles must equal 1",
        source.formula_profiles.len(),
        1,
    )?;
    limits.check_exact(
        "first-production effects must equal 1",
        source.effects.len(),
        1,
    )?;
    limits.check_exact(
        "first-production abilities must equal 1",
        source.abilities.len(),
        1,
    )?;
    limits.check_exact("first-production items must equal 1", source.items.len(), 1)?;
    limits.check_exact(
        "first-production loot tables must equal 1",
        source.loot_tables.len(),
        1,
    )?;
    let loot_entries = source.loot_tables.iter().try_fold(0usize, |sum, table| {
        sum.checked_add(table.entries.len())
            .ok_or(ContentError::InvalidSectionBounds)
    })?;
    limits.check_exact(
        "first-production loot entries must equal 1",
        loot_entries,
        1,
    )?;
    limits.check_exact(
        "first-production xp definitions must equal 1",
        source.xp_definitions.len(),
        1,
    )?;
    limits.check_exact(
        "first-production rng purpose keys must equal 1",
        source.rng.purpose_keys.len(),
        1,
    )?;

    let definitions = source
        .cells
        .len()
        .checked_add(18)
        .ok_or(ContentError::InvalidSectionBounds)?;
    limits.check(
        "first-production definitions",
        definitions,
        FIRST_PRODUCTION_MAX_DEFINITIONS,
    )?;
    let server_records = definitions
        .checked_add(1)
        .ok_or(ContentError::InvalidSectionBounds)?;
    limits.check(
        "first-production server records",
        server_records,
        FIRST_PRODUCTION_MAX_SERVER_RECORDS,
    )?;
    let references = source
        .cells
        .len()
        .checked_mul(3)
        .and_then(|value| value.checked_add(15))
        .ok_or(ContentError::InvalidSectionBounds)?;
    limits.check(
        "first-production references",
        references,
        FIRST_PRODUCTION_MAX_REFERENCES,
    )?;

    validate_cell_footprint(&source.cells)?;

    let mut aggregate_population = 0usize;
    for spawn in &source.spawns {
        let population = usize::from(spawn.population_limit);
        limits.check(
            "first-production spawn population per spawn",
            population,
            FIRST_PRODUCTION_MAX_SPAWN_POPULATION,
        )?;
        if population != FIRST_PRODUCTION_MAX_SPAWN_POPULATION {
            return Err(ContentError::InvalidArtifact(
                "first-production spawn population must equal 1",
            ));
        }
        aggregate_population = aggregate_population
            .checked_add(usize::from(spawn.population_limit))
            .ok_or(ContentError::InvalidSectionBounds)?;
    }
    limits.check_exact(
        "first-production aggregate population must equal 1",
        aggregate_population,
        FIRST_PRODUCTION_MAX_SCOPE_POPULATION,
    )
}

fn validate_cell_footprint(cells: &[FirstProductionCell]) -> Result<(), ContentError> {
    let mut xs = cells.iter().map(|cell| cell.x);
    let first_x = xs.next().ok_or(ContentError::InvalidArtifact(
        "first-production cells missing",
    ))?;
    let (min_x, max_x) = xs.fold((first_x, first_x), |(min, max), value| {
        (min.min(value), max.max(value))
    });
    let mut ys = cells.iter().map(|cell| cell.y);
    let first_y = ys.next().ok_or(ContentError::InvalidArtifact(
        "first-production cells missing",
    ))?;
    let (min_y, max_y) = ys.fold((first_y, first_y), |(min, max), value| {
        (min.min(value), max.max(value))
    });
    let x_span = i64::from(max_x)
        .checked_sub(i64::from(min_x))
        .and_then(|value| value.checked_add(1))
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(ContentError::InvalidSectionBounds)?;
    let y_span = i64::from(max_y)
        .checked_sub(i64::from(min_y))
        .and_then(|value| value.checked_add(1))
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(ContentError::InvalidSectionBounds)?;
    FirstProductionLimits::v1().check(
        "first-production x span",
        x_span,
        FIRST_PRODUCTION_MAX_X_SPAN,
    )?;
    FirstProductionLimits::v1().check(
        "first-production y span",
        y_span,
        FIRST_PRODUCTION_MAX_Y_SPAN,
    )?;
    let floors: BTreeSet<i16> = cells.iter().map(|cell| cell.z).collect();
    FirstProductionLimits::v1().check_exact(
        "first-production floors must equal 1",
        floors.len(),
        FIRST_PRODUCTION_MAX_FLOORS,
    )
}

fn validate_source_semantics(source: &FirstProductionContentSource) -> Result<(), ContentError> {
    let mut definitions = BTreeSet::new();

    let mut add_key = |key: &ProductionKey| -> Result<(), ContentError> {
        if !definitions.insert(key.as_str().to_owned()) {
            return Err(ContentError::DuplicateKey(key.as_str().to_owned()));
        }
        Ok(())
    };

    for value in &source.regions {
        add_key(&value.key)?;
    }
    for value in &source.areas {
        add_key(&value.key)?;
    }
    for value in &source.terrains {
        add_key(&value.key)?;
    }
    for value in &source.cells {
        add_key(&value.key)?;
    }
    for value in &source.relocations {
        add_key(&value.key)?;
    }
    for value in &source.behaviors {
        add_key(&value.key)?;
    }
    for value in &source.presentations {
        add_key(&value.key)?;
        validate_atom(
            "first-production presentation metadata",
            value.metadata_token.as_str(),
            FIRST_PRODUCTION_MAX_ATOM_BYTES,
            true,
        )?;
    }
    for value in &source.creatures {
        add_key(&value.key)?;
    }
    for value in &source.spawns {
        add_key(&value.key)?;
    }
    for value in &source.formula_profiles {
        add_key(&value.key)?;
    }
    for value in &source.effects {
        add_key(&value.key)?;
    }
    for value in &source.abilities {
        add_key(&value.key)?;
    }
    for value in &source.items {
        if !value.materializable {
            return Err(ContentError::InvalidArtifact(
                "first-production item must be materializable",
            ));
        }
        add_key(&value.key)?;
    }
    for table in &source.loot_tables {
        add_key(&table.key)?;
        for entry in &table.entries {
            add_key(&entry.key)?;
        }
    }
    for value in &source.xp_definitions {
        add_key(&value.key)?;
    }
    for value in &source.rng.purpose_keys {
        add_key(value)?;
    }

    let region_keys: BTreeSet<&str> = source.regions.iter().map(|v| v.key.as_str()).collect();
    let area_keys: BTreeSet<&str> = source.areas.iter().map(|v| v.key.as_str()).collect();
    let terrain_keys: BTreeSet<&str> = source.terrains.iter().map(|v| v.key.as_str()).collect();
    let cell_keys: BTreeSet<&str> = source.cells.iter().map(|v| v.key.as_str()).collect();
    let behavior_keys: BTreeSet<&str> = source.behaviors.iter().map(|v| v.key.as_str()).collect();
    let presentation_keys: BTreeSet<&str> = source
        .presentations
        .iter()
        .map(|v| v.key.as_str())
        .collect();
    let creature_keys: BTreeSet<&str> = source.creatures.iter().map(|v| v.key.as_str()).collect();
    let formula_keys: BTreeSet<&str> = source
        .formula_profiles
        .iter()
        .map(|v| v.key.as_str())
        .collect();
    let effect_keys: BTreeSet<&str> = source.effects.iter().map(|v| v.key.as_str()).collect();
    let item_keys: BTreeSet<&str> = source.items.iter().map(|v| v.key.as_str()).collect();
    let loot_table_keys: BTreeSet<&str> =
        source.loot_tables.iter().map(|v| v.key.as_str()).collect();
    let rng_purpose_keys: BTreeSet<&str> = source
        .rng
        .purpose_keys
        .iter()
        .map(ProductionKey::as_str)
        .collect();

    let require = |owner: &ProductionKey,
                   target: &ProductionKey,
                   family: &BTreeSet<&str>|
     -> Result<(), ContentError> {
        if !family.contains(target.as_str()) {
            return Err(ContentError::MissingReference {
                owner: owner.as_str().to_owned(),
                target: target.as_str().to_owned(),
            });
        }
        Ok(())
    };

    for cell in &source.cells {
        require(&cell.key, &cell.region_key, &region_keys)?;
        require(&cell.key, &cell.area_key, &area_keys)?;
        require(&cell.key, &cell.terrain_key, &terrain_keys)?;
    }
    for relocation in &source.relocations {
        require(&relocation.key, &relocation.from_cell, &cell_keys)?;
        require(&relocation.key, &relocation.to_cell, &cell_keys)?;
    }
    for creature in &source.creatures {
        require(&creature.key, &creature.behavior_key, &behavior_keys)?;
        require(
            &creature.key,
            &creature.presentation_key,
            &presentation_keys,
        )?;
    }
    for spawn in &source.spawns {
        require(&spawn.key, &spawn.creature_key, &creature_keys)?;
        require(&spawn.key, &spawn.behavior_key, &behavior_keys)?;
        require(&spawn.key, &spawn.cell_key, &cell_keys)?;
    }
    for effect in &source.effects {
        require(&effect.key, &effect.formula_profile_key, &formula_keys)?;
    }
    for ability in &source.abilities {
        require(&ability.key, &ability.effect_key, &effect_keys)?;
        require(&ability.key, &ability.presentation_key, &presentation_keys)?;
    }
    for item in &source.items {
        require(&item.key, &item.presentation_key, &presentation_keys)?;
    }
    for table in &source.loot_tables {
        for entry in &table.entries {
            require(&entry.key, &table.key, &loot_table_keys)?;
            require(&entry.key, &entry.item_key, &item_keys)?;
            require(&entry.key, &entry.rng_purpose_key, &rng_purpose_keys)?;
        }
    }
    for xp in &source.xp_definitions {
        require(&xp.key, &xp.formula_profile_key, &formula_keys)?;
    }

    let referenced_presentations: BTreeSet<&str> = [
        source
            .creatures
            .first()
            .ok_or(ContentError::InvalidArtifact(
                "first-production creature missing",
            ))?
            .presentation_key
            .as_str(),
        source
            .abilities
            .first()
            .ok_or(ContentError::InvalidArtifact(
                "first-production ability missing",
            ))?
            .presentation_key
            .as_str(),
        source
            .items
            .first()
            .ok_or(ContentError::InvalidArtifact(
                "first-production item missing",
            ))?
            .presentation_key
            .as_str(),
    ]
    .into_iter()
    .collect();
    if referenced_presentations.len() != 3 || referenced_presentations != presentation_keys {
        return Err(ContentError::InvalidArtifact(
            "first-production presentation references must be distinct and cover all presentations",
        ));
    }
    Ok(())
}

fn server_records(
    source: &FirstProductionContentSource,
) -> Result<Vec<ProductionRecord>, ContentError> {
    let mut records = Vec::new();
    for value in &source.regions {
        records.push(record(RECORD_REGION, [value.key.as_str()]));
    }
    for value in &source.areas {
        records.push(record(RECORD_AREA, [value.key.as_str()]));
    }
    for value in &source.terrains {
        records.push(record(RECORD_TERRAIN, [value.key.as_str()]));
    }
    for value in &source.cells {
        records.push(ProductionRecord::new(
            RECORD_CELL,
            vec![
                value.key.as_str().to_owned(),
                value.region_key.as_str().to_owned(),
                value.area_key.as_str().to_owned(),
                value.terrain_key.as_str().to_owned(),
                value.x.to_string(),
                value.y.to_string(),
                value.z.to_string(),
                match value.collision {
                    CollisionClass::Walkable => "walkable".to_owned(),
                    CollisionClass::Blocked => "blocked".to_owned(),
                },
            ],
        ));
    }
    for value in &source.relocations {
        records.push(record(
            RECORD_RELOCATION,
            [
                value.key.as_str(),
                value.from_cell.as_str(),
                value.to_cell.as_str(),
            ],
        ));
    }
    for value in &source.behaviors {
        records.push(record(
            RECORD_BEHAVIOR,
            [value.key.as_str(), value.policy_revision.as_str()],
        ));
    }
    for value in &source.presentations {
        records.push(record(
            RECORD_PRESENTATION,
            [value.key.as_str(), value.metadata_token.as_str()],
        ));
    }
    for value in &source.creatures {
        records.push(ProductionRecord::new(
            RECORD_CREATURE,
            vec![
                value.key.as_str().to_owned(),
                value.behavior_key.as_str().to_owned(),
                value.presentation_key.as_str().to_owned(),
                value.policy_revision.as_str().to_owned(),
            ],
        ));
    }
    for value in &source.spawns {
        records.push(ProductionRecord::new(
            RECORD_SPAWN,
            vec![
                value.key.as_str().to_owned(),
                value.creature_key.as_str().to_owned(),
                value.behavior_key.as_str().to_owned(),
                value.cell_key.as_str().to_owned(),
                value.population_limit.to_string(),
                match value.recovery {
                    SpawnRecoveryClass::EphemeralScopeReset => "EPHEMERAL_SCOPE_RESET".to_owned(),
                    SpawnRecoveryClass::CheckpointedRuntimeContinuity => {
                        "CHECKPOINTED_RUNTIME_CONTINUITY".to_owned()
                    }
                    SpawnRecoveryClass::DurableEventOccurrence => {
                        "DURABLE_EVENT_OCCURRENCE".to_owned()
                    }
                },
                match value.multiplicity {
                    MultiplicityClass::ChannelLocalRepeatable => {
                        "CHANNEL_LOCAL_REPEATABLE".to_owned()
                    }
                    MultiplicityClass::ChannelLocalSharedEligibility => {
                        "CHANNEL_LOCAL_SHARED_ELIGIBILITY".to_owned()
                    }
                    MultiplicityClass::WorldScopedUnique => "WORLD_SCOPED_UNIQUE".to_owned(),
                    MultiplicityClass::ExplicitEventPolicyRequired => {
                        "EXPLICIT_EVENT_POLICY_REQUIRED".to_owned()
                    }
                },
                match value.eligibility_scope {
                    EligibilityScope::CharacterWorld => "CHARACTER_WORLD".to_owned(),
                    EligibilityScope::AccountWorld => "ACCOUNT_WORLD".to_owned(),
                    EligibilityScope::World => "WORLD".to_owned(),
                },
            ],
        ));
    }
    for value in &source.formula_profiles {
        records.push(record(
            RECORD_FORMULA,
            [value.key.as_str(), "product-release"],
        ));
    }
    for value in &source.effects {
        records.push(record(
            RECORD_EFFECT,
            [
                value.key.as_str(),
                match value.family {
                    EffectFamily::Damage => "damage",
                },
                value.formula_profile_key.as_str(),
            ],
        ));
    }
    for value in &source.abilities {
        records.push(record(
            RECORD_ABILITY,
            [
                value.key.as_str(),
                value.effect_key.as_str(),
                value.presentation_key.as_str(),
            ],
        ));
    }
    for value in &source.items {
        records.push(record(
            RECORD_ITEM,
            [
                value.key.as_str(),
                value.presentation_key.as_str(),
                if value.materializable {
                    "true"
                } else {
                    "false"
                },
            ],
        ));
    }
    for table in &source.loot_tables {
        records.push(record(RECORD_LOOT_TABLE, [table.key.as_str()]));
        for entry in &table.entries {
            records.push(ProductionRecord::new(
                RECORD_LOOT_ENTRY,
                vec![
                    entry.key.as_str().to_owned(),
                    table.key.as_str().to_owned(),
                    entry.item_key.as_str().to_owned(),
                    entry.rng_purpose_key.as_str().to_owned(),
                    "product-release".to_owned(),
                ],
            ));
        }
    }
    for value in &source.xp_definitions {
        records.push(ProductionRecord::new(
            RECORD_XP,
            vec![
                value.key.as_str().to_owned(),
                value.formula_profile_key.as_str().to_owned(),
                "product-release".to_owned(),
            ],
        ));
    }
    records.push(record(
        RECORD_RNG_CONTEXT,
        [source.rng.profile_revision.as_str(), "metadata-only"],
    ));
    for value in &source.rng.purpose_keys {
        records.push(record(RECORD_RNG_PURPOSE, [value.as_str()]));
    }
    FirstProductionLimits::v1().check(
        "first-production server records",
        records.len(),
        FIRST_PRODUCTION_MAX_SERVER_RECORDS,
    )?;
    Ok(records)
}

fn client_records(source: &FirstProductionContentSource) -> Vec<ProductionRecord> {
    let mut records = Vec::new();
    for value in &source.presentations {
        records.push(record(
            RECORD_PRESENTATION,
            [value.key.as_str(), value.metadata_token.as_str()],
        ));
    }
    for value in &source.creatures {
        records.push(record(
            RECORD_CLIENT_CREATURE,
            [value.key.as_str(), value.presentation_key.as_str()],
        ));
    }
    for value in &source.abilities {
        records.push(record(
            RECORD_CLIENT_ABILITY,
            [value.key.as_str(), value.presentation_key.as_str()],
        ));
    }
    for value in &source.items {
        records.push(record(
            RECORD_CLIENT_ITEM,
            [value.key.as_str(), value.presentation_key.as_str()],
        ));
    }
    records
}

fn record<const N: usize>(kind: u8, fields: [&str; N]) -> ProductionRecord {
    ProductionRecord::new(kind, fields.into_iter().map(ToOwned::to_owned).collect())
}

#[derive(Debug, Clone)]
struct EncodedArtifact {
    bytes: Vec<u8>,
    digest: [u8; 32],
}

fn encode_artifact(
    metadata: &ProductionArtifactMetadata,
    records: &[ProductionRecord],
) -> Result<EncodedArtifact, ContentError> {
    let limits = FirstProductionLimits::v1();
    limits.check(
        "first-production records",
        records.len(),
        metadata.projection.record_limit(),
    )?;
    let manifest = encode_manifest(metadata)?;
    let body = encode_body(records)?;
    limits.check(
        "first-production manifest bytes",
        manifest.len(),
        FIRST_PRODUCTION_MAX_MANIFEST_BYTES,
    )?;
    limits.check(
        "first-production section bytes",
        body.len(),
        FIRST_PRODUCTION_MAX_SECTION_BYTES,
    )?;

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
    limits.check(
        projection_artifact_resource(metadata.projection),
        total_len,
        metadata.projection.artifact_limit(),
    )?;

    let mut bytes = Vec::with_capacity(total_len);
    bytes.extend_from_slice(&MAGIC);
    put_u16(&mut bytes, PROFILE_VERSION);
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
        FIRST_PRODUCTION_MAX_MANIFEST_FIELDS,
        sha256(&manifest),
    )?;
    encode_section_entry(
        &mut bytes,
        SECTION_BODY,
        body_offset,
        body.len(),
        records.len(),
        sha256(&body),
    )?;
    bytes.extend_from_slice(&manifest);
    bytes.extend_from_slice(&body);
    let digest = sha256(&bytes);
    bytes.extend_from_slice(&digest);
    Ok(EncodedArtifact { bytes, digest })
}

fn projection_artifact_resource(projection: ProductionProjection) -> &'static str {
    match projection {
        ProductionProjection::ServerAuthoritative => "first-production server artifact bytes",
        ProductionProjection::ClientSafe => "first-production client artifact bytes",
    }
}

fn encode_manifest(metadata: &ProductionArtifactMetadata) -> Result<Vec<u8>, ContentError> {
    let world_id = encode_world_id(metadata.world_id);
    let fields = [
        FIRST_PRODUCTION_ARTIFACT_PROFILE_ID,
        metadata.package_key.as_str(),
        metadata.package_revision.as_str(),
        metadata.semantic_schema_version.as_str(),
        metadata.licensing_metadata.as_str(),
        metadata.source_manifest_digest.as_str(),
        world_id.as_str(),
        metadata.revisions.content.as_str(),
        metadata.revisions.map.as_str(),
        metadata.revisions.ruleset.as_str(),
        metadata.revisions.world_policy.as_str(),
        metadata.revisions.compiler.as_str(),
        metadata.revisions.canonicalization.as_str(),
        metadata.content_lock_token.as_str(),
        metadata.package_provenance_digest.as_str(),
        metadata.revisions.sim_profile.as_str(),
        metadata.revisions.profile_revision.as_str(),
        metadata.projection.as_str(),
        metadata.migration_class.as_str(),
        metadata.capability_profile.as_str(),
    ];
    let mut bytes = Vec::new();
    for field in fields {
        put_string(&mut bytes, field)?;
    }
    FirstProductionLimits::v1().check_exact(
        "first-production manifest field count mismatch",
        fields.len(),
        FIRST_PRODUCTION_MAX_MANIFEST_FIELDS,
    )?;
    FirstProductionLimits::v1().check(
        "first-production manifest bytes",
        bytes.len(),
        FIRST_PRODUCTION_MAX_MANIFEST_BYTES,
    )?;
    Ok(bytes)
}

fn encode_body(records: &[ProductionRecord]) -> Result<Vec<u8>, ContentError> {
    let mut bytes = Vec::new();
    put_u32(&mut bytes, to_u32(records.len())?);
    for record in records {
        let expected = expected_field_count(record.kind).ok_or(ContentError::InvalidArtifact(
            "unknown production record kind",
        ))?;
        if record.fields.len() != usize::from(expected) {
            return Err(ContentError::InvalidArtifact(
                "production compiler emitted invalid record shape",
            ));
        }
        let mut encoded = Vec::new();
        encoded.push(record.kind);
        encoded.push(expected);
        for field in &record.fields {
            put_string(&mut encoded, field)?;
        }
        FirstProductionLimits::v1().check(
            "first-production record bytes",
            encoded.len(),
            FIRST_PRODUCTION_MAX_RECORD_BYTES,
        )?;
        let next_len = bytes
            .len()
            .checked_add(4)
            .and_then(|value| value.checked_add(encoded.len()))
            .ok_or(ContentError::InvalidSectionBounds)?;
        FirstProductionLimits::v1().check(
            "first-production section bytes",
            next_len,
            FIRST_PRODUCTION_MAX_SECTION_BYTES,
        )?;
        put_u32(&mut bytes, to_u32(encoded.len())?);
        bytes.extend_from_slice(&encoded);
    }
    Ok(bytes)
}

fn put_string(bytes: &mut Vec<u8>, value: &str) -> Result<(), ContentError> {
    validate_atom(
        "first-production artifact field",
        value,
        FIRST_PRODUCTION_MAX_ATOM_BYTES,
        false,
    )?;
    let length = u16::try_from(value.len()).map_err(|_| ContentError::LimitExceeded {
        resource: "first-production artifact field bytes",
        actual: value.len(),
        limit: usize::from(u16::MAX),
    })?;
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn encode_section_entry(
    bytes: &mut Vec<u8>,
    kind: u16,
    offset: usize,
    length: usize,
    item_count: usize,
    digest: [u8; 32],
) -> Result<(), ContentError> {
    put_u16(bytes, kind);
    put_u16(bytes, SECTION_FLAG_CRITICAL);
    put_u32(bytes, to_u32(offset)?);
    put_u32(bytes, to_u32(length)?);
    put_u32(bytes, to_u32(item_count)?);
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
pub(crate) struct ParsedRecord {
    pub(crate) kind: u8,
    pub(crate) fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FirstProductionRuntimeState {
    server_records: Vec<ParsedRecord>,
    client_records: Vec<ParsedRecord>,
}

impl FirstProductionRuntimeState {
    pub(crate) fn server_record_count(&self) -> usize {
        self.server_records.len()
    }

    pub(crate) fn client_record_count(&self) -> usize {
        self.client_records.len()
    }

    pub(crate) fn server_definition_fields(&self, key: &str) -> Option<&[String]> {
        self.server_records.iter().find_map(|record| {
            parsed_record_definition_key(record)
                .filter(|candidate| *candidate == key)
                .map(|_| record.fields.as_slice())
        })
    }
}

fn parsed_record_definition_key(record: &ParsedRecord) -> Option<&str> {
    if record.kind == RECORD_RNG_CONTEXT {
        return None;
    }
    record.fields.first().map(String::as_str)
}

fn verify_projection_pair_semantics(
    server: &[ParsedRecord],
    client: &[ParsedRecord],
) -> Result<(), ContentError> {
    for client_record in client {
        let server_record = server
            .iter()
            .find(|record| {
                let matching_kind = match client_record.kind {
                    RECORD_PRESENTATION => record.kind == RECORD_PRESENTATION,
                    RECORD_CLIENT_CREATURE => record.kind == RECORD_CREATURE,
                    RECORD_CLIENT_ABILITY => record.kind == RECORD_ABILITY,
                    RECORD_CLIENT_ITEM => record.kind == RECORD_ITEM,
                    _ => false,
                };
                matching_kind && record.fields.first() == client_record.fields.first()
            })
            .ok_or(ContentError::PairMismatch(
                "client-safe record has no authoritative counterpart",
            ))?;

        let matches = match client_record.kind {
            RECORD_PRESENTATION => server_record.fields == client_record.fields,
            RECORD_CLIENT_CREATURE => server_record.fields.get(2) == client_record.fields.get(1),
            RECORD_CLIENT_ABILITY => server_record.fields.get(2) == client_record.fields.get(1),
            RECORD_CLIENT_ITEM => server_record.fields.get(1) == client_record.fields.get(1),
            _ => false,
        };
        if !matches {
            return Err(ContentError::PairMismatch(
                "client-safe projection semantics differ from authoritative generation",
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedProductionArtifact {
    metadata: ProductionArtifactMetadata,
    artifact_digest: [u8; 32],
    body_digest: [u8; 32],
    decoded_fields: usize,
    records: Vec<ParsedRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationIdentity {
    package_key: ProductionKey,
    package_revision: ProductionAtom,
    semantic_schema_version: ProductionAtom,
    licensing_metadata: ProductionAtom,
    source_manifest_digest: Sha256HexDigest,
    package_provenance_digest: Sha256HexDigest,
    world_id: WorldId,
    revisions: FirstProductionRevisionSet,
    content_lock_token: ProductionAtom,
    migration_class: DurableMigrationClass,
    capability_profile: ProductionAtom,
    server_artifact_digest: [u8; 32],
    client_artifact_digest: [u8; 32],
    server_body_digest: [u8; 32],
    client_body_digest: [u8; 32],
}

impl GenerationIdentity {
    pub fn content_revision(&self) -> &str {
        self.revisions.content.as_str()
    }

    pub fn package_revision(&self) -> &str {
        self.package_revision.as_str()
    }

    pub fn package_provenance_digest(&self) -> &str {
        self.package_provenance_digest.as_str()
    }

    pub const fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub fn migration_class(&self) -> DurableMigrationClass {
        self.migration_class
    }

    pub fn server_artifact_digest(&self) -> [u8; 32] {
        self.server_artifact_digest
    }

    pub fn client_artifact_digest(&self) -> [u8; 32] {
        self.client_artifact_digest
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedGeneration {
    identity: GenerationIdentity,
    runtime_state: FirstProductionRuntimeState,
}

impl StagedGeneration {
    pub fn stage(
        server_bytes: &[u8],
        client_bytes: &[u8],
        expected: &FirstProductionExpectation,
    ) -> Result<Self, ContentError> {
        check_pair_lengths(server_bytes.len(), client_bytes.len())?;
        let server = stage_artifact(server_bytes, ProductionProjection::ServerAuthoritative)?;
        let client = stage_artifact(client_bytes, ProductionProjection::ClientSafe)?;
        if !server.metadata.same_generation_identity(&client.metadata) {
            return Err(ContentError::PairMismatch(
                "first-production server/client generation identity differs",
            ));
        }
        verify_projection_pair_semantics(&server.records, &client.records)?;
        verify_expected(&server.metadata, expected)?;
        verify_expected(&client.metadata, expected)?;
        if server.artifact_digest != expected.server_artifact_digest
            || client.artifact_digest != expected.client_artifact_digest
        {
            return Err(ContentError::RevisionMismatch(
                "first-production expected artifact digest pair",
            ));
        }
        let decoded_fields = server
            .decoded_fields
            .checked_add(client.decoded_fields)
            .ok_or(ContentError::InvalidSectionBounds)?;
        check_decoded_fields(decoded_fields)?;
        let identity = GenerationIdentity {
            package_key: server.metadata.package_key.clone(),
            package_revision: server.metadata.package_revision.clone(),
            semantic_schema_version: server.metadata.semantic_schema_version.clone(),
            licensing_metadata: server.metadata.licensing_metadata.clone(),
            source_manifest_digest: server.metadata.source_manifest_digest.clone(),
            package_provenance_digest: server.metadata.package_provenance_digest.clone(),
            world_id: server.metadata.world_id,
            revisions: server.metadata.revisions.clone(),
            content_lock_token: server.metadata.content_lock_token.clone(),
            migration_class: server.metadata.migration_class,
            capability_profile: server.metadata.capability_profile.clone(),
            server_artifact_digest: server.artifact_digest,
            client_artifact_digest: client.artifact_digest,
            server_body_digest: server.body_digest,
            client_body_digest: client.body_digest,
        };
        let runtime_state = FirstProductionRuntimeState {
            server_records: server.records,
            client_records: client.records,
        };
        Ok(Self {
            identity,
            runtime_state,
        })
    }

    pub fn identity(&self) -> &GenerationIdentity {
        &self.identity
    }

    #[cfg(test)]
    pub(crate) fn runtime_state(&self) -> &FirstProductionRuntimeState {
        &self.runtime_state
    }

    pub(crate) fn into_parts(self) -> (GenerationIdentity, FirstProductionRuntimeState) {
        (self.identity, self.runtime_state)
    }
}

fn verify_expected(
    metadata: &ProductionArtifactMetadata,
    expected: &FirstProductionExpectation,
) -> Result<(), ContentError> {
    let mut normalized = metadata.clone();
    normalized.projection = ProductionProjection::ServerAuthoritative;
    if normalized != expected.metadata {
        return Err(ContentError::RevisionMismatch(
            "first-production expected generation identity",
        ));
    }
    Ok(())
}

fn check_pair_lengths(server_len: usize, client_len: usize) -> Result<(), ContentError> {
    let pair = server_len
        .checked_add(client_len)
        .ok_or(ContentError::InvalidSectionBounds)?;
    FirstProductionLimits::v1().check(
        "first-production generation pair bytes",
        pair,
        FIRST_PRODUCTION_MAX_GENERATION_PAIR_BYTES,
    )
}

fn check_decoded_fields(count: usize) -> Result<(), ContentError> {
    FirstProductionLimits::v1().check(
        "first-production decoded field instances",
        count,
        FIRST_PRODUCTION_MAX_DECODED_FIELDS,
    )
}

fn stage_artifact(
    bytes: &[u8],
    expected_projection: ProductionProjection,
) -> Result<ParsedProductionArtifact, ContentError> {
    FirstProductionLimits::v1().check(
        projection_artifact_resource(expected_projection),
        bytes.len(),
        expected_projection.artifact_limit(),
    )?;
    if bytes.len() < HEADER_LEN + TRAILER_LEN {
        return Err(ContentError::Truncated);
    }
    if bytes.get(..8) != Some(MAGIC.as_slice()) {
        return Err(ContentError::InvalidMagic);
    }
    let profile_version = read_u16_at(bytes, 8)?;
    if profile_version != PROFILE_VERSION {
        return Err(ContentError::UnsupportedProfile(profile_version));
    }
    let flags = read_u16_at(bytes, 10)?;
    if flags != 0 {
        return Err(ContentError::UnknownCriticalFlags(flags));
    }
    let projection =
        ProductionProjection::from_byte(*bytes.get(12).ok_or(ContentError::Truncated)?)?;
    if projection != expected_projection {
        return Err(ContentError::PairMismatch(
            "first-production artifact has wrong projection",
        ));
    }
    if *bytes.get(13).ok_or(ContentError::Truncated)? != 0 {
        return Err(ContentError::InvalidArtifact(
            "first-production reserved header byte is nonzero",
        ));
    }
    let section_count = usize::from(read_u16_at(bytes, 14)?);
    FirstProductionLimits::v1().check_exact(
        "first-production artifact must contain exactly two sections",
        section_count,
        2,
    )?;
    let table_offset =
        usize::try_from(read_u32_at(bytes, 16)?).map_err(|_| ContentError::InvalidSectionBounds)?;
    if table_offset != HEADER_LEN {
        return Err(ContentError::InvalidSectionBounds);
    }
    let payload_end =
        usize::try_from(read_u32_at(bytes, 20)?).map_err(|_| ContentError::InvalidSectionBounds)?;
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
        return Err(ContentError::IntegrityMismatch("first-production artifact"));
    }

    let entries = parse_section_entries(bytes, section_count, table_offset)?;
    validate_section_ranges(&entries, table_end, payload_end)?;
    let manifest_entry = unique_section(&entries, SECTION_MANIFEST)?;
    let body_entry = unique_section(&entries, SECTION_BODY)?;
    FirstProductionLimits::v1().check_exact(
        "first-production manifest field count mismatch",
        manifest_entry.item_count,
        FIRST_PRODUCTION_MAX_MANIFEST_FIELDS,
    )?;
    FirstProductionLimits::v1().check(
        "first-production records",
        body_entry.item_count,
        projection.record_limit(),
    )?;

    let manifest_bytes = section_bytes(bytes, manifest_entry)?;
    let body_bytes = section_bytes(bytes, body_entry)?;
    verify_section_digest(manifest_bytes, manifest_entry)?;
    verify_section_digest(body_bytes, body_entry)?;
    let metadata = parse_manifest(manifest_bytes, projection)?;
    let records = parse_body(body_bytes, body_entry.item_count)?;
    let decoded_fields = validate_parsed_semantics(&records, projection)?
        .checked_add(FIRST_PRODUCTION_MAX_MANIFEST_FIELDS)
        .ok_or(ContentError::InvalidSectionBounds)?;

    Ok(ParsedProductionArtifact {
        metadata,
        artifact_digest: actual_digest,
        body_digest: body_entry.digest,
        decoded_fields,
        records,
    })
}

fn parse_section_entries(
    bytes: &[u8],
    count: usize,
    table_offset: usize,
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
        FirstProductionLimits::v1().check(
            "first-production section bytes",
            length,
            FIRST_PRODUCTION_MAX_SECTION_BYTES,
        )?;
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
        ranges.push((entry.offset, end));
    }
    ranges.sort_unstable_by_key(|range| range.0);
    if ranges.first().map(|range| range.0) != Some(table_end)
        || ranges.last().map(|range| range.1) != Some(payload_end)
    {
        return Err(ContentError::InvalidSectionBounds);
    }
    for pair in ranges.windows(2) {
        if pair[0].1 != pair[1].0 {
            return Err(ContentError::InvalidSectionBounds);
        }
    }
    Ok(())
}

fn unique_section(entries: &[SectionEntry], kind: u16) -> Result<&SectionEntry, ContentError> {
    let mut matches = entries.iter().filter(|entry| entry.kind == kind);
    let first = matches.next().ok_or(ContentError::InvalidArtifact(
        "first-production required section missing",
    ))?;
    if matches.next().is_some() {
        return Err(ContentError::InvalidArtifact(
            "first-production duplicate required section",
        ));
    }
    if first.flags & SECTION_FLAG_CRITICAL == 0 {
        return Err(ContentError::InvalidArtifact(
            "first-production required section is not critical",
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
        return Err(ContentError::IntegrityMismatch("first-production section"));
    }
    Ok(())
}

fn parse_manifest(
    bytes: &[u8],
    projection: ProductionProjection,
) -> Result<ProductionArtifactMetadata, ContentError> {
    FirstProductionLimits::v1().check(
        "first-production manifest bytes",
        bytes.len(),
        FIRST_PRODUCTION_MAX_MANIFEST_BYTES,
    )?;
    let mut reader = SliceReader::new(bytes);
    let profile_id = reader.read_string(FIRST_PRODUCTION_MAX_ATOM_BYTES)?;
    if profile_id != FIRST_PRODUCTION_ARTIFACT_PROFILE_ID {
        return Err(ContentError::InvalidArtifact(
            "unexpected first-production artifact profile id",
        ));
    }
    let package_key = ProductionKey::new(&reader.read_string(FIRST_PRODUCTION_MAX_KEY_BYTES)?)?;
    let package_revision = ProductionAtom::from_artifact(
        "first-production package revision",
        reader.read_string(FIRST_PRODUCTION_MAX_ATOM_BYTES)?,
    )?;
    let semantic_schema_version = ProductionAtom::from_artifact(
        "first-production semantic schema version",
        reader.read_string(FIRST_PRODUCTION_MAX_ATOM_BYTES)?,
    )?;
    let licensing_metadata = ProductionAtom::from_artifact(
        "first-production licensing metadata",
        reader.read_string(FIRST_PRODUCTION_MAX_ATOM_BYTES)?,
    )?;
    let source_manifest_digest = Sha256HexDigest::new(&reader.read_string(64)?)?;
    let world_id = parse_world_id(&reader.read_string(32)?)?;
    let content = ProductionAtom::from_artifact(
        "first-production content revision",
        reader.read_string(FIRST_PRODUCTION_MAX_ATOM_BYTES)?,
    )?;
    let map = ProductionAtom::from_artifact(
        "first-production map revision",
        reader.read_string(FIRST_PRODUCTION_MAX_ATOM_BYTES)?,
    )?;
    let ruleset = ProductionAtom::from_artifact(
        "first-production ruleset revision",
        reader.read_string(FIRST_PRODUCTION_MAX_ATOM_BYTES)?,
    )?;
    let world_policy = ProductionAtom::from_artifact(
        "first-production world policy revision",
        reader.read_string(FIRST_PRODUCTION_MAX_ATOM_BYTES)?,
    )?;
    let compiler = ProductionAtom::from_artifact(
        "first-production compiler revision",
        reader.read_string(FIRST_PRODUCTION_MAX_ATOM_BYTES)?,
    )?;
    let canonicalization = ProductionAtom::from_artifact(
        "first-production canonicalization revision",
        reader.read_string(FIRST_PRODUCTION_MAX_ATOM_BYTES)?,
    )?;
    let content_lock_token = ProductionAtom::from_artifact(
        "first-production Content Lock token",
        reader.read_string(FIRST_PRODUCTION_MAX_ATOM_BYTES)?,
    )?;
    let package_provenance_digest = Sha256HexDigest::new(&reader.read_string(64)?)?;
    let sim_profile = ProductionAtom::from_artifact(
        "first-production simulation profile revision",
        reader.read_string(FIRST_PRODUCTION_MAX_ATOM_BYTES)?,
    )?;
    let profile_revision = ProductionAtom::from_artifact(
        "first-production profile revision",
        reader.read_string(FIRST_PRODUCTION_MAX_ATOM_BYTES)?,
    )?;
    if profile_revision.as_str() != FIRST_PRODUCTION_PROFILE_ID {
        return Err(ContentError::InvalidArtifact(
            "first-production profile revision mismatch",
        ));
    }
    let manifest_projection = reader.read_string(FIRST_PRODUCTION_MAX_ATOM_BYTES)?;
    if manifest_projection != projection.as_str() {
        return Err(ContentError::InvalidArtifact(
            "first-production manifest/header projection mismatch",
        ));
    }
    let migration_class =
        DurableMigrationClass::from_str(&reader.read_string(FIRST_PRODUCTION_MAX_ATOM_BYTES)?)?;
    let capability_profile = ProductionAtom::from_artifact(
        "first-production capability profile",
        reader.read_string(FIRST_PRODUCTION_MAX_ATOM_BYTES)?,
    )?;
    if capability_profile.as_str() != FIRST_PRODUCTION_CAPABILITY_PROFILE {
        return Err(ContentError::InvalidArtifact(
            "first-production required capability profile mismatch",
        ));
    }
    reader.ensure_end()?;

    let revisions = FirstProductionRevisionSet {
        content,
        map,
        ruleset,
        world_policy,
        compiler,
        canonicalization,
        sim_profile,
        profile_revision,
    };
    let binding = PackageManifestBinding {
        package_key: package_key.clone(),
        package_revision: package_revision.clone(),
        semantic_schema_version: semantic_schema_version.clone(),
        licensing_metadata: licensing_metadata.clone(),
        source_manifest_digest: source_manifest_digest.clone(),
    };
    if binding.package_provenance_digest()? != package_provenance_digest {
        return Err(ContentError::InvalidArtifact(
            "first-production package provenance digest mismatch",
        ));
    }

    Ok(ProductionArtifactMetadata {
        package_key,
        package_revision,
        semantic_schema_version,
        licensing_metadata,
        source_manifest_digest,
        world_id,
        revisions,
        content_lock_token,
        package_provenance_digest,
        projection,
        migration_class,
        capability_profile,
    })
}

fn parse_body(bytes: &[u8], expected_count: usize) -> Result<Vec<ParsedRecord>, ContentError> {
    FirstProductionLimits::v1().check(
        "first-production section bytes",
        bytes.len(),
        FIRST_PRODUCTION_MAX_SECTION_BYTES,
    )?;
    let mut reader = SliceReader::new(bytes);
    let count = usize::try_from(reader.read_u32()?)
        .map_err(|_| ContentError::InvalidArtifact("production record count conversion"))?;
    if count != expected_count {
        return Err(ContentError::InvalidArtifact(
            "first-production record count mismatch",
        ));
    }
    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        let record_len = usize::try_from(reader.read_u32()?)
            .map_err(|_| ContentError::InvalidArtifact("production record length conversion"))?;
        FirstProductionLimits::v1().check(
            "first-production record bytes",
            record_len,
            FIRST_PRODUCTION_MAX_RECORD_BYTES,
        )?;
        let raw = reader.take(record_len)?;
        let mut record_reader = SliceReader::new(raw);
        let kind = record_reader.read_u8()?;
        let declared_fields = record_reader.read_u8()?;
        let expected_fields = expected_field_count(kind).ok_or(ContentError::InvalidArtifact(
            "unknown production record kind",
        ))?;
        if declared_fields != expected_fields {
            return Err(ContentError::InvalidArtifact(
                "first-production record field count mismatch",
            ));
        }
        let mut fields = Vec::with_capacity(usize::from(expected_fields));
        for _ in 0..expected_fields {
            fields.push(record_reader.read_string(FIRST_PRODUCTION_MAX_ATOM_BYTES)?);
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

fn validate_parsed_semantics(
    records: &[ParsedRecord],
    projection: ProductionProjection,
) -> Result<usize, ContentError> {
    let count_kind = |kind: u8| records.iter().filter(|record| record.kind == kind).count();
    match projection {
        ProductionProjection::ServerAuthoritative => {
            for kind in [
                RECORD_REGION,
                RECORD_AREA,
                RECORD_TERRAIN,
                RECORD_RELOCATION,
                RECORD_BEHAVIOR,
                RECORD_CREATURE,
                RECORD_SPAWN,
                RECORD_FORMULA,
                RECORD_EFFECT,
                RECORD_ABILITY,
                RECORD_ITEM,
                RECORD_LOOT_TABLE,
                RECORD_LOOT_ENTRY,
                RECORD_XP,
                RECORD_RNG_CONTEXT,
                RECORD_RNG_PURPOSE,
            ] {
                FirstProductionLimits::v1().check_exact(
                    "first-production server record family cardinality mismatch",
                    count_kind(kind),
                    1,
                )?;
            }
            FirstProductionLimits::v1().check_exact(
                "first-production presentation record cardinality mismatch",
                count_kind(RECORD_PRESENTATION),
                3,
            )?;
            let cell_count = count_kind(RECORD_CELL);
            if cell_count < 3 {
                return Err(ContentError::InvalidArtifact(
                    "first-production cells must contain at least 3 entries",
                ));
            }
            FirstProductionLimits::v1().check(
                "first-production cells",
                cell_count,
                FIRST_PRODUCTION_MAX_CELLS,
            )?;
            let expected_records = cell_count
                .checked_add(19)
                .ok_or(ContentError::InvalidSectionBounds)?;
            if records.len() != expected_records {
                return Err(ContentError::InvalidArtifact(
                    "first-production server record count does not match profile shape",
                ));
            }
        }
        ProductionProjection::ClientSafe => {
            FirstProductionLimits::v1().check_exact(
                "first-production client presentation count mismatch",
                count_kind(RECORD_PRESENTATION),
                3,
            )?;
            for kind in [
                RECORD_CLIENT_CREATURE,
                RECORD_CLIENT_ABILITY,
                RECORD_CLIENT_ITEM,
            ] {
                FirstProductionLimits::v1().check_exact(
                    "first-production client record family cardinality mismatch",
                    count_kind(kind),
                    1,
                )?;
            }
            FirstProductionLimits::v1().check_exact(
                "first-production client record count mismatch",
                records.len(),
                FIRST_PRODUCTION_MAX_CLIENT_RECORDS,
            )?;
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
                    "server-only record leaked into first-production client projection",
                ));
            }
        }
    }

    let mut definitions = BTreeSet::new();
    let mut references = Vec::new();
    let mut cell_points = Vec::new();
    let mut decoded_fields = 0usize;

    let family_keys = |kind: u8| -> BTreeSet<&str> {
        records
            .iter()
            .filter(|record| record.kind == kind)
            .filter_map(|record| record.fields.first().map(String::as_str))
            .collect()
    };
    let region_keys = family_keys(RECORD_REGION);
    let area_keys = family_keys(RECORD_AREA);
    let terrain_keys = family_keys(RECORD_TERRAIN);
    let cell_keys = family_keys(RECORD_CELL);
    let behavior_keys = family_keys(RECORD_BEHAVIOR);
    let presentation_keys = family_keys(RECORD_PRESENTATION);
    let creature_keys = family_keys(if projection == ProductionProjection::ServerAuthoritative {
        RECORD_CREATURE
    } else {
        RECORD_CLIENT_CREATURE
    });
    let formula_keys = family_keys(RECORD_FORMULA);
    let effect_keys = family_keys(RECORD_EFFECT);
    let item_keys = family_keys(if projection == ProductionProjection::ServerAuthoritative {
        RECORD_ITEM
    } else {
        RECORD_CLIENT_ITEM
    });
    let loot_table_keys = family_keys(RECORD_LOOT_TABLE);
    let rng_purpose_keys = family_keys(RECORD_RNG_PURPOSE);

    let require_family =
        |owner: &str, target: &str, family: &BTreeSet<&str>| -> Result<(), ContentError> {
            ProductionKey::new(target)?;
            if !family.contains(target) {
                return Err(ContentError::MissingReference {
                    owner: owner.to_owned(),
                    target: target.to_owned(),
                });
            }
            Ok(())
        };

    for record in records {
        decoded_fields = decoded_fields
            .checked_add(record.fields.len())
            .ok_or(ContentError::InvalidSectionBounds)?;

        let definition_index = match record.kind {
            RECORD_RNG_CONTEXT => None,
            _ => Some(0usize),
        };
        if let Some(index) = definition_index {
            let key = ProductionKey::new(&record.fields[index])?;
            if !definitions.insert(key.as_str().to_owned()) {
                return Err(ContentError::DuplicateKey(key.as_str().to_owned()));
            }
        }

        match record.kind {
            RECORD_CELL => {
                require_family(&record.fields[0], &record.fields[1], &region_keys)?;
                require_family(&record.fields[0], &record.fields[2], &area_keys)?;
                require_family(&record.fields[0], &record.fields[3], &terrain_keys)?;
                references.extend([
                    record.fields[1].clone(),
                    record.fields[2].clone(),
                    record.fields[3].clone(),
                ]);
                let x = parse_i32(&record.fields[4])?;
                let y = parse_i32(&record.fields[5])?;
                let z = parse_i16(&record.fields[6])?;
                if !matches!(record.fields[7].as_str(), "walkable" | "blocked") {
                    return Err(ContentError::InvalidArtifact(
                        "invalid first-production collision class",
                    ));
                }
                cell_points.push((x, y, z));
            }
            RECORD_RELOCATION => {
                require_family(&record.fields[0], &record.fields[1], &cell_keys)?;
                require_family(&record.fields[0], &record.fields[2], &cell_keys)?;
                references.extend([record.fields[1].clone(), record.fields[2].clone()]);
            }
            RECORD_PRESENTATION => {
                validate_atom(
                    "first-production presentation metadata",
                    &record.fields[1],
                    FIRST_PRODUCTION_MAX_ATOM_BYTES,
                    true,
                )?;
            }
            RECORD_BEHAVIOR => {
                ProductionAtom::new(
                    "first-production behavior policy revision",
                    &record.fields[1],
                )?;
            }
            RECORD_CREATURE => {
                require_family(&record.fields[0], &record.fields[1], &behavior_keys)?;
                require_family(&record.fields[0], &record.fields[2], &presentation_keys)?;
                references.extend([record.fields[1].clone(), record.fields[2].clone()]);
                ProductionAtom::new(
                    "first-production creature policy revision",
                    &record.fields[3],
                )?;
            }
            RECORD_SPAWN => {
                require_family(&record.fields[0], &record.fields[1], &creature_keys)?;
                require_family(&record.fields[0], &record.fields[2], &behavior_keys)?;
                require_family(&record.fields[0], &record.fields[3], &cell_keys)?;
                references.extend([
                    record.fields[1].clone(),
                    record.fields[2].clone(),
                    record.fields[3].clone(),
                ]);
                let population = usize::from(parse_positive_u16(&record.fields[4])?);
                FirstProductionLimits::v1().check(
                    "first-production spawn population per spawn",
                    population,
                    FIRST_PRODUCTION_MAX_SPAWN_POPULATION,
                )?;
                if population != FIRST_PRODUCTION_MAX_SPAWN_POPULATION {
                    return Err(ContentError::InvalidArtifact(
                        "first-production spawn population must equal 1",
                    ));
                }
                if !matches!(
                    record.fields[5].as_str(),
                    "EPHEMERAL_SCOPE_RESET"
                        | "CHECKPOINTED_RUNTIME_CONTINUITY"
                        | "DURABLE_EVENT_OCCURRENCE"
                ) {
                    return Err(ContentError::InvalidArtifact(
                        "invalid first-production spawn recovery class",
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
                        "invalid first-production multiplicity class",
                    ));
                }
                if !matches!(
                    record.fields[7].as_str(),
                    "CHARACTER_WORLD" | "ACCOUNT_WORLD" | "WORLD"
                ) {
                    return Err(ContentError::InvalidArtifact(
                        "invalid first-production eligibility scope",
                    ));
                }
            }
            RECORD_FORMULA => {
                if record.fields[1] != "product-release" {
                    return Err(ContentError::FixtureOnlyReleaseRejected);
                }
            }
            RECORD_EFFECT => {
                require_family(&record.fields[0], &record.fields[2], &formula_keys)?;
                if record.fields[1] != "damage" {
                    return Err(ContentError::InvalidArtifact(
                        "unsupported first-production effect family",
                    ));
                }
                references.push(record.fields[2].clone());
            }
            RECORD_ABILITY => {
                require_family(&record.fields[0], &record.fields[1], &effect_keys)?;
                require_family(&record.fields[0], &record.fields[2], &presentation_keys)?;
                references.extend([record.fields[1].clone(), record.fields[2].clone()]);
            }
            RECORD_ITEM => {
                require_family(&record.fields[0], &record.fields[1], &presentation_keys)?;
                references.push(record.fields[1].clone());
                if record.fields[2] != "true" {
                    return Err(ContentError::InvalidArtifact(
                        "first-production item is not materializable",
                    ));
                }
            }
            RECORD_LOOT_ENTRY => {
                require_family(&record.fields[0], &record.fields[1], &loot_table_keys)?;
                require_family(&record.fields[0], &record.fields[2], &item_keys)?;
                require_family(&record.fields[0], &record.fields[3], &rng_purpose_keys)?;
                references.extend([
                    record.fields[1].clone(),
                    record.fields[2].clone(),
                    record.fields[3].clone(),
                ]);
                if record.fields[4] != "product-release" {
                    return Err(ContentError::FixtureOnlyReleaseRejected);
                }
            }
            RECORD_XP => {
                require_family(&record.fields[0], &record.fields[1], &formula_keys)?;
                references.push(record.fields[1].clone());
                if record.fields[2] != "product-release" {
                    return Err(ContentError::FixtureOnlyReleaseRejected);
                }
            }
            RECORD_RNG_CONTEXT => {
                ProductionAtom::new("first-production rng profile revision", &record.fields[0])?;
                if record.fields[1] != "metadata-only" {
                    return Err(ContentError::InvalidArtifact(
                        "first-production RNG context cannot carry seed or secret state",
                    ));
                }
            }
            RECORD_CLIENT_CREATURE | RECORD_CLIENT_ABILITY | RECORD_CLIENT_ITEM => {
                require_family(&record.fields[0], &record.fields[1], &presentation_keys)?;
                references.push(record.fields[1].clone());
            }
            RECORD_REGION | RECORD_AREA | RECORD_TERRAIN | RECORD_LOOT_TABLE
            | RECORD_RNG_PURPOSE => {}
            _ => {
                return Err(ContentError::InvalidArtifact(
                    "unsupported first-production record semantics",
                ));
            }
        }
    }

    let presentation_ref = |kind: u8, field: usize| -> Result<&str, ContentError> {
        records
            .iter()
            .find(|record| record.kind == kind)
            .and_then(|record| record.fields.get(field))
            .map(String::as_str)
            .ok_or(ContentError::InvalidArtifact(
                "first-production presentation reference missing",
            ))
    };
    let referenced_presentations: BTreeSet<&str> = match projection {
        ProductionProjection::ServerAuthoritative => [
            presentation_ref(RECORD_CREATURE, 2)?,
            presentation_ref(RECORD_ABILITY, 2)?,
            presentation_ref(RECORD_ITEM, 1)?,
        ],
        ProductionProjection::ClientSafe => [
            presentation_ref(RECORD_CLIENT_CREATURE, 1)?,
            presentation_ref(RECORD_CLIENT_ABILITY, 1)?,
            presentation_ref(RECORD_CLIENT_ITEM, 1)?,
        ],
    }
    .into_iter()
    .collect();
    if referenced_presentations.len() != 3 || referenced_presentations != presentation_keys {
        return Err(ContentError::InvalidArtifact(
            "first-production presentation references must be distinct and cover all presentations",
        ));
    }

    for target in references {
        let key = ProductionKey::new(&target)?;
        if !definitions.contains(key.as_str()) {
            return Err(ContentError::MissingReference {
                owner: "first-production artifact".to_owned(),
                target,
            });
        }
    }

    if projection == ProductionProjection::ServerAuthoritative {
        validate_parsed_cell_footprint(&cell_points)?;
        let cell_count = count_kind(RECORD_CELL);
        let references = cell_count
            .checked_mul(3)
            .and_then(|value| value.checked_add(15))
            .ok_or(ContentError::InvalidSectionBounds)?;
        FirstProductionLimits::v1().check(
            "first-production references",
            references,
            FIRST_PRODUCTION_MAX_REFERENCES,
        )?;
        let definitions_count = definitions.len();
        FirstProductionLimits::v1().check(
            "first-production definitions",
            definitions_count,
            FIRST_PRODUCTION_MAX_DEFINITIONS,
        )?;
    }

    Ok(decoded_fields)
}

fn validate_parsed_cell_footprint(points: &[(i32, i32, i16)]) -> Result<(), ContentError> {
    let cells: Vec<FirstProductionCell> = points
        .iter()
        .enumerate()
        .map(|(index, (x, y, z))| FirstProductionCell {
            key: ProductionKey(format!("oteryn:parsed.cell.{index}")),
            region_key: ProductionKey("oteryn:parsed.region".to_owned()),
            area_key: ProductionKey("oteryn:parsed.area".to_owned()),
            terrain_key: ProductionKey("oteryn:parsed.terrain".to_owned()),
            x: *x,
            y: *y,
            z: *z,
            collision: CollisionClass::Walkable,
        })
        .collect();
    validate_cell_footprint(&cells)
}

fn parse_i32(value: &str) -> Result<i32, ContentError> {
    value
        .parse::<i32>()
        .map_err(|_| ContentError::InvalidArtifact("invalid first-production i32 field"))
}

fn parse_i16(value: &str) -> Result<i16, ContentError> {
    value
        .parse::<i16>()
        .map_err(|_| ContentError::InvalidArtifact("invalid first-production i16 field"))
}

fn parse_positive_u16(value: &str) -> Result<u16, ContentError> {
    let parsed = value
        .parse::<u16>()
        .map_err(|_| ContentError::InvalidArtifact("invalid first-production u16 field"))?;
    if parsed == 0 {
        return Err(ContentError::InvalidArtifact(
            "zero first-production value is invalid",
        ));
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

    fn read_string(&mut self, max: usize) -> Result<String, ContentError> {
        let length = usize::from(self.read_u16()?);
        FirstProductionLimits::v1().check("first-production artifact field bytes", length, max)?;
        let raw = self.take(length)?;
        if raw.is_empty() || !raw.iter().all(|byte| byte.is_ascii_graphic()) {
            return Err(ContentError::InvalidString(
                "first-production artifact field",
            ));
        }
        let value = std::str::from_utf8(raw)
            .map_err(|_| ContentError::InvalidString("first-production artifact field"))?;
        Ok(value.to_owned())
    }

    fn ensure_end(&self) -> Result<(), ContentError> {
        if self.position != self.bytes.len() {
            return Err(ContentError::InvalidArtifact(
                "trailing first-production artifact bytes",
            ));
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

#[cfg(test)]
fn test_world_id(seed: u8) -> Result<WorldId, ContentError> {
    let mut bytes = [0_u8; 16];
    bytes[0] = 1;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = seed;
    WorldId::from_bytes(bytes)
        .map_err(|_| ContentError::InvalidArtifact("test first-production WorldId invalid"))
}

#[cfg(test)]
pub(crate) fn test_source(cell_count: usize) -> Result<FirstProductionContentSource, ContentError> {
    let package_key = ProductionKey::new("oteryn:content.first-production")?;
    let package_revision = ProductionAtom::new("first-production package revision", "package-r1")?;
    let package_manifest = PackageManifestBinding::new(
        package_key.clone(),
        package_revision.clone(),
        ProductionAtom::new("semantic schema", "schema-v1")?,
        ProductionAtom::new("licensing metadata", "license:project-owned-v1")?,
        Sha256HexDigest::new("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")?,
    );
    let provenance = package_manifest.package_provenance_digest()?;
    let content_lock = ContentLockBinding {
        revision_digest_token: ProductionAtom::new("content lock token", "lock:content-r1")?,
        entries: vec![ContentLockEntry::exact(
            package_key,
            package_revision,
            provenance,
        )],
    };
    let region = ProductionKey::new("oteryn:prod.region")?;
    let area = ProductionKey::new("oteryn:prod.area")?;
    let terrain = ProductionKey::new("oteryn:prod.terrain")?;
    let mut cells = Vec::with_capacity(cell_count);
    for index in 0..cell_count {
        let x = i32::try_from(index % 32).map_err(|_| ContentError::InvalidSectionBounds)?;
        let y = i32::try_from(index / 32).map_err(|_| ContentError::InvalidSectionBounds)?;
        cells.push(FirstProductionCell {
            key: ProductionKey::new(&format!("oteryn:prod.cell.{index:04}"))?,
            region_key: region.clone(),
            area_key: area.clone(),
            terrain_key: terrain.clone(),
            x,
            y,
            z: 7,
            collision: if index == 1 {
                CollisionClass::Blocked
            } else {
                CollisionClass::Walkable
            },
        });
    }
    if cells.is_empty() {
        return Err(ContentError::InvalidArtifact(
            "test source needs at least one cell",
        ));
    }
    let first_cell = cells[0].key.clone();
    let last_cell = cells[cells.len() - 1].key.clone();
    let behavior = ProductionKey::new("oteryn:prod.behavior")?;
    let creature_presentation = ProductionKey::new("oteryn:prod.presentation.creature")?;
    let item_presentation = ProductionKey::new("oteryn:prod.presentation.item")?;
    let ability_presentation = ProductionKey::new("oteryn:prod.presentation.ability")?;
    let creature = ProductionKey::new("oteryn:prod.creature")?;
    let formula = ProductionKey::new("oteryn:prod.formula")?;
    let effect = ProductionKey::new("oteryn:prod.effect")?;
    let item = ProductionKey::new("oteryn:prod.item")?;
    let rng_purpose = ProductionKey::new("oteryn:prod.rng.loot")?;

    Ok(FirstProductionContentSource {
        package_manifest,
        content_lock,
        world_id: test_world_id(1)?,
        revisions: FirstProductionRevisionSet {
            content: ProductionAtom::new("content revision", "content-r1")?,
            map: ProductionAtom::new("map revision", "map-r1")?,
            ruleset: ProductionAtom::new("ruleset revision", "ruleset-r1")?,
            world_policy: ProductionAtom::new("world policy revision", "world-policy-r1")?,
            compiler: ProductionAtom::new("compiler revision", "compiler-prod-r1")?,
            canonicalization: ProductionAtom::new(
                "canonicalization revision",
                "canonicalization-r1",
            )?,
            sim_profile: ProductionAtom::new("sim profile", "sim-v1")?,
            profile_revision: ProductionAtom::new(
                "first-production profile revision",
                FIRST_PRODUCTION_PROFILE_ID,
            )?,
        },
        capability_profile: ProductionAtom::new(
            "capability profile",
            FIRST_PRODUCTION_CAPABILITY_PROFILE,
        )?,
        migration_class: DurableMigrationClass::CompatibleNoMigration,
        regions: vec![FirstProductionRegion { key: region }],
        areas: vec![FirstProductionArea { key: area }],
        terrains: vec![FirstProductionTerrain { key: terrain }],
        cells,
        relocations: vec![FirstProductionRelocation {
            key: ProductionKey::new("oteryn:prod.relocation")?,
            from_cell: first_cell,
            to_cell: last_cell.clone(),
        }],
        behaviors: vec![FirstProductionBehavior {
            key: behavior.clone(),
            policy_revision: ProductionAtom::new("behavior policy", "behavior-policy-r1")?,
        }],
        presentations: vec![
            FirstProductionPresentation {
                key: creature_presentation.clone(),
                metadata_token: ProductionAtom::new(
                    "presentation metadata",
                    "appearance:creature-v1",
                )?,
            },
            FirstProductionPresentation {
                key: item_presentation.clone(),
                metadata_token: ProductionAtom::new("presentation metadata", "appearance:item-v1")?,
            },
            FirstProductionPresentation {
                key: ability_presentation.clone(),
                metadata_token: ProductionAtom::new(
                    "presentation metadata",
                    "appearance:ability-v1",
                )?,
            },
        ],
        creatures: vec![FirstProductionCreature {
            key: creature.clone(),
            behavior_key: behavior.clone(),
            presentation_key: creature_presentation,
            policy_revision: ProductionAtom::new("creature policy", "creature-policy-r1")?,
        }],
        spawns: vec![FirstProductionSpawn {
            key: ProductionKey::new("oteryn:prod.spawn")?,
            creature_key: creature,
            behavior_key: behavior,
            cell_key: last_cell,
            population_limit: 1,
            recovery: SpawnRecoveryClass::CheckpointedRuntimeContinuity,
            multiplicity: MultiplicityClass::ChannelLocalSharedEligibility,
            eligibility_scope: EligibilityScope::CharacterWorld,
        }],
        formula_profiles: vec![FirstProductionFormulaProfile {
            key: formula.clone(),
        }],
        effects: vec![FirstProductionEffect {
            key: effect.clone(),
            family: EffectFamily::Damage,
            formula_profile_key: formula.clone(),
        }],
        abilities: vec![FirstProductionAbility {
            key: ProductionKey::new("oteryn:prod.ability")?,
            effect_key: effect,
            presentation_key: ability_presentation,
        }],
        items: vec![FirstProductionItem {
            key: item.clone(),
            presentation_key: item_presentation,
            materializable: true,
        }],
        loot_tables: vec![FirstProductionLootTable {
            key: ProductionKey::new("oteryn:prod.loot.table")?,
            entries: vec![FirstProductionLootEntry {
                key: ProductionKey::new("oteryn:prod.loot.entry")?,
                item_key: item,
                rng_purpose_key: rng_purpose.clone(),
            }],
        }],
        xp_definitions: vec![FirstProductionXpDefinition {
            key: ProductionKey::new("oteryn:prod.xp")?,
            formula_profile_key: formula,
        }],
        rng: FirstProductionRngContext {
            profile_revision: ProductionAtom::new("rng profile", "rng-metadata-r1")?,
            purpose_keys: vec![rng_purpose],
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_provenance_grammar_has_stable_golden_digest() -> Result<(), ContentError> {
        let source = test_source(1)?;
        let binding = &source.package_manifest;
        let digest = binding.package_provenance_digest()?;
        assert_eq!(
            digest.as_str(),
            "b1028ffc978a4b96cea3d43283c010242a3c2f46679521f930a902ee8f71370a"
        );
        let preimage = binding.provenance_preimage()?;
        assert_eq!(&preimage[..4], &(31_u32.to_be_bytes()));
        Ok(())
    }

    #[test]
    fn ordinary_release_is_deterministic_after_enumeration_shuffle() -> Result<(), ContentError> {
        let source = test_source(4)?;
        let mut shuffled = source.clone();
        shuffled.reverse_enumeration_for_test();
        let first =
            compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
        let second =
            compile_first_production(&shuffled, FirstProductionCompileTarget::OrdinaryRelease)?;
        assert_eq!(first.server_artifact, second.server_artifact);
        assert_eq!(first.client_artifact, second.client_artifact);
        Ok(())
    }

    #[test]
    fn exact_profile_maxima_and_footprint_are_enforced() -> Result<(), ContentError> {
        let source = test_source(FIRST_PRODUCTION_MAX_CELLS)?;
        compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;

        let minimum = test_source(3)?;
        compile_first_production(&minimum, FirstProductionCompileTarget::OrdinaryRelease)?;
        let below_minimum = test_source(2)?;
        assert!(matches!(
            compile_first_production(
                &below_minimum,
                FirstProductionCompileTarget::OrdinaryRelease
            ),
            Err(ContentError::InvalidArtifact(
                "first-production cells must contain at least 3 entries"
            ))
        ));

        let too_many = test_source(FIRST_PRODUCTION_MAX_CELLS + 1)?;
        assert!(matches!(
            compile_first_production(&too_many, FirstProductionCompileTarget::OrdinaryRelease),
            Err(ContentError::LimitExceeded {
                resource: "first-production cells",
                ..
            })
        ));

        let mut too_wide = test_source(3)?;
        too_wide.cells[2].x = 32;
        assert!(matches!(
            compile_first_production(&too_wide, FirstProductionCompileTarget::OrdinaryRelease),
            Err(ContentError::LimitExceeded {
                resource: "first-production x span",
                ..
            })
        ));

        let mut too_tall = test_source(3)?;
        too_tall.cells[2].y = 32;
        assert!(matches!(
            compile_first_production(&too_tall, FirstProductionCompileTarget::OrdinaryRelease),
            Err(ContentError::LimitExceeded {
                resource: "first-production y span",
                ..
            })
        ));

        let mut second_floor = test_source(3)?;
        second_floor.cells[1].z = 8;
        assert!(matches!(
            compile_first_production(&second_floor, FirstProductionCompileTarget::OrdinaryRelease),
            Err(ContentError::LimitExceeded {
                resource: "first-production floors must equal 1",
                ..
            })
        ));
        Ok(())
    }

    #[test]
    fn population_and_content_lock_fail_closed() -> Result<(), ContentError> {
        let mut zero = test_source(3)?;
        zero.spawns[0].population_limit = 0;
        assert!(
            compile_first_production(&zero, FirstProductionCompileTarget::OrdinaryRelease).is_err()
        );

        let mut two = test_source(3)?;
        two.spawns[0].population_limit = 2;
        assert!(matches!(
            compile_first_production(&two, FirstProductionCompileTarget::OrdinaryRelease),
            Err(ContentError::LimitExceeded {
                resource: "first-production spawn population per spawn",
                actual: 2,
                limit: 1,
            })
        ));

        let server_metadata = ProductionArtifactMetadata::from_source(
            &two,
            ProductionProjection::ServerAuthoritative,
        )?;
        let client_metadata =
            ProductionArtifactMetadata::from_source(&two, ProductionProjection::ClientSafe)?;
        let server = encode_artifact(&server_metadata, &server_records(&two)?)?;
        let client = encode_artifact(&client_metadata, &client_records(&two))?;
        let expected = FirstProductionExpectation::from_source(&two, server.digest, client.digest)?;
        assert!(matches!(
            StagedGeneration::stage(&server.bytes, &client.bytes, &expected),
            Err(ContentError::LimitExceeded {
                resource: "first-production spawn population per spawn",
                actual: 2,
                limit: 1,
            })
        ));

        let mut empty_lock = test_source(3)?;
        empty_lock.content_lock.entries.clear();
        assert!(
            compile_first_production(&empty_lock, FirstProductionCompileTarget::OrdinaryRelease)
                .is_err()
        );

        let mut dependency_lock = test_source(3)?;
        dependency_lock.content_lock.entries[0].dependency = true;
        assert!(
            compile_first_production(
                &dependency_lock,
                FirstProductionCompileTarget::OrdinaryRelease
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn nonproduction_markers_and_digest_shape_are_rejected() {
        assert!(ProductionKey::new("oteryn:fixture.item").is_err());
        assert!(ProductionKey::new("oteryn:test.formula").is_err());
        assert!(ProductionKey::new("oteryn:prod.test.formula").is_err());
        assert!(ProductionAtom::new("presentation", "synthetic://asset").is_err());
        assert!(ProductionAtom::new("profile", "evidence:test-v1").is_err());
        assert!(
            Sha256HexDigest::new(
                "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
            )
            .is_err()
        );
        assert!(Sha256HexDigest::new("abc").is_err());
    }

    #[test]
    fn staged_pair_revalidates_expected_provenance_and_projection() -> Result<(), ContentError> {
        let source = test_source(3)?;
        let compiled =
            compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
        let staged = StagedGeneration::stage(
            &compiled.server_artifact,
            &compiled.client_artifact,
            compiled.expectation(),
        )?;
        assert_eq!(
            staged.identity().package_provenance_digest(),
            compiled.expectation().package_provenance_digest()
        );

        assert_eq!(staged.runtime_state().server_record_count(), 22);
        assert_eq!(staged.runtime_state().client_record_count(), 6);
        assert_eq!(
            staged
                .runtime_state()
                .server_definition_fields("oteryn:prod.creature")
                .and_then(|fields| fields.get(3))
                .map(String::as_str),
            Some("creature-policy-r1")
        );
        assert!(matches!(
            stage_artifact(
                &compiled.client_artifact,
                ProductionProjection::ServerAuthoritative
            ),
            Err(ContentError::PairMismatch(_))
        ));
        Ok(())
    }

    #[test]
    fn staging_rejects_self_consistent_artifact_below_cell_minimum() -> Result<(), ContentError> {
        let source = test_source(2)?;
        let server_metadata = ProductionArtifactMetadata::from_source(
            &source,
            ProductionProjection::ServerAuthoritative,
        )?;
        let client_metadata =
            ProductionArtifactMetadata::from_source(&source, ProductionProjection::ClientSafe)?;
        let server = encode_artifact(&server_metadata, &server_records(&source)?)?;
        let client = encode_artifact(&client_metadata, &client_records(&source))?;
        let expected =
            FirstProductionExpectation::from_source(&source, server.digest, client.digest)?;

        assert!(matches!(
            StagedGeneration::stage(&server.bytes, &client.bytes, &expected),
            Err(ContentError::InvalidArtifact(
                "first-production cells must contain at least 3 entries"
            ))
        ));
        Ok(())
    }

    #[test]
    fn staging_expectation_binds_exact_artifact_digest_pair() -> Result<(), ContentError> {
        let source = test_source(3)?;
        let compiled =
            compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
        let mut records = server_records(&source)?;
        let cell = records
            .iter_mut()
            .find(|record| record.kind == RECORD_CELL)
            .ok_or(ContentError::InvalidArtifact("cell record missing in test"))?;
        cell.fields[7] = if cell.fields[7] == "walkable" {
            "blocked".to_owned()
        } else {
            "walkable".to_owned()
        };
        let metadata = ProductionArtifactMetadata::from_source(
            &source,
            ProductionProjection::ServerAuthoritative,
        )?;
        let crafted = encode_artifact(&metadata, &records)?;

        assert!(matches!(
            StagedGeneration::stage(
                &crafted.bytes,
                &compiled.client_artifact,
                compiled.expectation(),
            ),
            Err(ContentError::RevisionMismatch(
                "first-production expected artifact digest pair"
            ))
        ));
        Ok(())
    }

    #[test]
    fn world_id_is_typed_uuidv7_and_round_trips_through_artifacts() -> Result<(), ContentError> {
        let source = test_source(3)?;
        let compiled =
            compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
        let staged = StagedGeneration::stage(
            &compiled.server_artifact,
            &compiled.client_artifact,
            compiled.expectation(),
        )?;
        assert_eq!(staged.identity().world_id(), source.world_id);
        assert!(parse_world_id("world-prod-v1").is_err());
        assert!(parse_world_id("00000000000000000000000000000000").is_err());
        Ok(())
    }

    #[test]
    fn presentation_references_are_one_to_one_and_cover_all_presentations()
    -> Result<(), ContentError> {
        let mut source = test_source(3)?;
        source.abilities[0].presentation_key = source.creatures[0].presentation_key.clone();
        assert!(matches!(
            compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease),
            Err(ContentError::InvalidArtifact(
                "first-production presentation references must be distinct and cover all presentations"
            ))
        ));

        let server_metadata = ProductionArtifactMetadata::from_source(
            &source,
            ProductionProjection::ServerAuthoritative,
        )?;
        let client_metadata =
            ProductionArtifactMetadata::from_source(&source, ProductionProjection::ClientSafe)?;
        let server = encode_artifact(&server_metadata, &server_records(&source)?)?;
        let client = encode_artifact(&client_metadata, &client_records(&source))?;
        let expected =
            FirstProductionExpectation::from_source(&source, server.digest, client.digest)?;
        assert!(matches!(
            StagedGeneration::stage(&server.bytes, &client.bytes, &expected),
            Err(ContentError::InvalidArtifact(
                "first-production presentation references must be distinct and cover all presentations"
            ))
        ));
        Ok(())
    }

    #[test]
    fn protected_registry_constants_and_text_boundaries_match() -> Result<(), ContentError> {
        assert_eq!(FIRST_PRODUCTION_MAX_MANIFEST_FIELDS, 20);
        assert_eq!(FIRST_PRODUCTION_MAX_MANIFEST_BYTES, 9_384);
        assert_eq!(FIRST_PRODUCTION_MAX_SERVER_ARTIFACT_BYTES, 4_304_614);
        assert_eq!(FIRST_PRODUCTION_MAX_CLIENT_ARTIFACT_BYTES, 34_248);
        assert_eq!(FIRST_PRODUCTION_MAX_GENERATION_PAIR_BYTES, 4_338_862);
        assert_eq!(FIRST_PRODUCTION_MAX_DECODED_FIELDS, 8_432);
        assert_eq!(FIRST_PRODUCTION_MAX_SECTION_BYTES, 4_295_078);
        assert_eq!(FIRST_PRODUCTION_MAX_RECORD_BYTES, 4_114);
        assert_eq!(FIRST_PRODUCTION_MAX_DEFINITIONS, 1_042);
        assert_eq!(FIRST_PRODUCTION_MAX_REFERENCES, 3_087);
        assert_eq!(FIRST_PRODUCTION_MAX_SERVER_RECORDS, 1_043);
        assert_eq!(FIRST_PRODUCTION_MAX_CLIENT_RECORDS, 6);
        assert_eq!(
            FIRST_PRODUCTION_MAX_SERVER_ARTIFACT_BYTES + FIRST_PRODUCTION_MAX_CLIENT_ARTIFACT_BYTES,
            FIRST_PRODUCTION_MAX_GENERATION_PAIR_BYTES
        );

        let key_512 = format!("o:{}", "a".repeat(510));
        let key_513 = format!("o:{}", "a".repeat(511));
        assert_eq!(key_512.len(), 512);
        assert_eq!(key_513.len(), 513);
        ProductionKey::new(&key_512)?;
        assert!(matches!(
            ProductionKey::new(&key_513),
            Err(ContentError::LimitExceeded { .. })
        ));

        let atom_512 = "a".repeat(512);
        let atom_513 = "a".repeat(513);
        ProductionAtom::new("boundary atom", &atom_512)?;
        assert!(matches!(
            ProductionAtom::new("boundary atom", &atom_513),
            Err(ContentError::LimitExceeded { .. })
        ));
        assert!(Sha256HexDigest::new(&"a".repeat(63)).is_err());
        assert!(matches!(
            Sha256HexDigest::new(&"a".repeat(65)),
            Err(ContentError::LimitExceeded {
                resource: "first-production sha256 digest bytes",
                actual: 65,
                limit: 64,
            })
        ));
        assert!(Sha256HexDigest::new(&format!("{}g", "a".repeat(63))).is_err());
        Ok(())
    }

    #[test]
    fn every_runtime_representable_family_rejects_max_plus_one() -> Result<(), ContentError> {
        macro_rules! reject_extra {
            ($field:ident) => {{
                let mut source = test_source(3)?;
                let extra = source.$field[0].clone();
                source.$field.push(extra);
                assert!(
                    matches!(
                        compile_first_production(
                            &source,
                            FirstProductionCompileTarget::OrdinaryRelease
                        ),
                        Err(ContentError::LimitExceeded { .. })
                    ),
                    "{} did not classify max+1 as capacity exceeded",
                    stringify!($field)
                );
            }};
        }

        reject_extra!(regions);
        reject_extra!(areas);
        reject_extra!(terrains);
        reject_extra!(relocations);
        reject_extra!(behaviors);
        reject_extra!(presentations);
        reject_extra!(creatures);
        reject_extra!(spawns);
        reject_extra!(formula_profiles);
        reject_extra!(effects);
        reject_extra!(abilities);
        reject_extra!(items);
        reject_extra!(loot_tables);
        reject_extra!(xp_definitions);

        let mut loot_entry = test_source(3)?;
        let extra = loot_entry.loot_tables[0].entries[0].clone();
        loot_entry.loot_tables[0].entries.push(extra);
        assert!(matches!(
            compile_first_production(&loot_entry, FirstProductionCompileTarget::OrdinaryRelease),
            Err(ContentError::LimitExceeded { .. })
        ));

        let mut rng_purpose = test_source(3)?;
        rng_purpose
            .rng
            .purpose_keys
            .push(rng_purpose.rng.purpose_keys[0].clone());
        assert!(matches!(
            compile_first_production(&rng_purpose, FirstProductionCompileTarget::OrdinaryRelease),
            Err(ContentError::LimitExceeded { .. })
        ));

        let mut second_lock = test_source(3)?;
        second_lock
            .content_lock
            .entries
            .push(second_lock.content_lock.entries[0].clone());
        assert!(matches!(
            compile_first_production(&second_lock, FirstProductionCompileTarget::OrdinaryRelease),
            Err(ContentError::LimitExceeded { .. })
        ));
        Ok(())
    }

    #[test]
    fn duplicate_unresolved_capability_and_migration_escape_fail_closed() -> Result<(), ContentError>
    {
        let mut duplicate = test_source(3)?;
        duplicate.items[0].key = duplicate.abilities[0].key.clone();
        assert!(matches!(
            compile_first_production(&duplicate, FirstProductionCompileTarget::OrdinaryRelease),
            Err(ContentError::DuplicateKey(_))
        ));

        let mut unresolved = test_source(3)?;
        unresolved.abilities[0].effect_key = ProductionKey::new("oteryn:prod.effect.missing")?;
        assert!(matches!(
            compile_first_production(&unresolved, FirstProductionCompileTarget::OrdinaryRelease),
            Err(ContentError::MissingReference { .. })
        ));

        let mut capability = test_source(3)?;
        capability.capability_profile =
            ProductionAtom::new("capability profile", "content:first-production-v2")?;
        assert!(
            compile_first_production(&capability, FirstProductionCompileTarget::OrdinaryRelease)
                .is_err()
        );

        let mut migration = test_source(3)?;
        migration.migration_class = DurableMigrationClass::MigrationRequired;
        assert!(
            compile_first_production(&migration, FirstProductionCompileTarget::OrdinaryRelease)
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn cross_family_references_fail_in_compiler_and_staging() -> Result<(), ContentError> {
        let mut invalid_source = test_source(3)?;
        invalid_source.spawns[0].creature_key = invalid_source.cells[0].key.clone();
        assert!(matches!(
            compile_first_production(
                &invalid_source,
                FirstProductionCompileTarget::OrdinaryRelease
            ),
            Err(ContentError::MissingReference { .. })
        ));

        let source = test_source(3)?;
        let compiled =
            compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
        let mut records = server_records(&source)?;
        let spawn = records
            .iter_mut()
            .find(|record| record.kind == RECORD_SPAWN)
            .ok_or(ContentError::InvalidArtifact(
                "spawn record missing in test",
            ))?;
        spawn.fields[1] = source.cells[0].key.as_str().to_owned();

        let metadata = ProductionArtifactMetadata::from_source(
            &source,
            ProductionProjection::ServerAuthoritative,
        )?;
        let crafted = encode_artifact(&metadata, &records)?;
        assert!(matches!(
            StagedGeneration::stage(
                &crafted.bytes,
                &compiled.client_artifact,
                compiled.expectation(),
            ),
            Err(ContentError::MissingReference { .. })
        ));
        Ok(())
    }

    #[test]
    fn projection_pair_requires_shared_semantic_equivalence() -> Result<(), ContentError> {
        let source = test_source(3)?;
        let compiled =
            compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
        let mut records = client_records(&source);

        let presentation = records
            .iter_mut()
            .find(|record| record.kind == RECORD_PRESENTATION)
            .ok_or(ContentError::InvalidArtifact(
                "presentation record missing in test",
            ))?;
        presentation.fields[1] = "appearance:tampered-v2".to_owned();
        let metadata =
            ProductionArtifactMetadata::from_source(&source, ProductionProjection::ClientSafe)?;
        let crafted = encode_artifact(&metadata, &records)?;
        assert!(matches!(
            StagedGeneration::stage(
                &compiled.server_artifact,
                &crafted.bytes,
                compiled.expectation(),
            ),
            Err(ContentError::PairMismatch(_))
        ));
        Ok(())
    }

    #[test]
    fn staging_revalidates_behavior_policy_and_rejects_payload_gaps() -> Result<(), ContentError> {
        let source = test_source(3)?;
        let compiled =
            compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;

        let mut records = server_records(&source)?;
        let behavior = records
            .iter_mut()
            .find(|record| record.kind == RECORD_BEHAVIOR)
            .ok_or(ContentError::InvalidArtifact(
                "behavior record missing in test",
            ))?;
        behavior.fields[1] = "evidence:behavior-policy-r1".to_owned();
        let metadata = ProductionArtifactMetadata::from_source(
            &source,
            ProductionProjection::ServerAuthoritative,
        )?;
        let crafted = encode_artifact(&metadata, &records)?;
        assert!(matches!(
            StagedGeneration::stage(
                &crafted.bytes,
                &compiled.client_artifact,
                compiled.expectation(),
            ),
            Err(ContentError::InvalidString(
                "first-production behavior policy revision"
            ))
        ));

        let mut gap = compiled.server_artifact.clone();
        let old_payload_end = usize::try_from(read_u32_at(&gap, 20)?)
            .map_err(|_| ContentError::InvalidSectionBounds)?;
        gap.insert(old_payload_end, b'X');

        let new_payload_end = old_payload_end
            .checked_add(1)
            .ok_or(ContentError::InvalidSectionBounds)?;
        gap[20..24].copy_from_slice(&to_u32(new_payload_end)?.to_be_bytes());
        let digest = sha256(&gap[..new_payload_end]);
        gap[new_payload_end..new_payload_end + TRAILER_LEN].copy_from_slice(&digest);
        assert!(matches!(
            StagedGeneration::stage(&gap, &compiled.client_artifact, compiled.expectation(),),
            Err(ContentError::InvalidSectionBounds)
        ));
        Ok(())
    }

    #[test]
    fn activated_runtime_indexes_loot_entry_by_entry_key() -> Result<(), ContentError> {
        let source = test_source(3)?;
        let compiled =
            compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
        let staged = StagedGeneration::stage(
            &compiled.server_artifact,
            &compiled.client_artifact,
            compiled.expectation(),
        )?;

        let fields = staged
            .runtime_state()
            .server_definition_fields("oteryn:prod.loot.entry")
            .ok_or(ContentError::InvalidArtifact(
                "loot entry not indexed by entry key",
            ))?;
        assert_eq!(
            fields.first().map(String::as_str),
            Some("oteryn:prod.loot.entry")
        );
        assert_eq!(
            fields.get(1).map(String::as_str),
            Some("oteryn:prod.loot.table")
        );
        Ok(())
    }

    #[test]
    fn production_tampering_pair_mismatch_and_unknown_critical_fail_closed()
    -> Result<(), ContentError> {
        let source = test_source(3)?;
        let compiled =
            compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;

        let mut truncated = compiled.server_artifact.clone();
        truncated.truncate(20);
        assert!(matches!(
            StagedGeneration::stage(
                &truncated,
                &compiled.client_artifact,
                compiled.expectation(),
            ),
            Err(ContentError::Truncated)
        ));

        let mut other_source = test_source(3)?;
        other_source.revisions.content = ProductionAtom::new("content revision", "content-r2")?;
        let other =
            compile_first_production(&other_source, FirstProductionCompileTarget::OrdinaryRelease)?;
        assert!(matches!(
            StagedGeneration::stage(
                &compiled.server_artifact,
                &other.client_artifact,
                compiled.expectation(),
            ),
            Err(ContentError::PairMismatch(_))
        ));
        assert!(matches!(
            StagedGeneration::stage(
                &compiled.server_artifact,
                &compiled.client_artifact,
                other.expectation(),
            ),
            Err(ContentError::RevisionMismatch(_))
        ));

        let mut unknown = compiled.server_artifact.clone();
        let second_entry = HEADER_LEN + SECTION_ENTRY_LEN;
        unknown[second_entry..second_entry + 2].copy_from_slice(&0x7ffe_u16.to_be_bytes());
        let payload_end = unknown.len() - TRAILER_LEN;
        let digest = sha256(&unknown[..payload_end]);
        unknown[payload_end..].copy_from_slice(&digest);
        assert!(matches!(
            StagedGeneration::stage(&unknown, &compiled.client_artifact, compiled.expectation(),),
            Err(ContentError::UnknownCriticalSection(0x7ffe))
        ));
        Ok(())
    }

    #[test]
    fn byte_and_decoded_field_boundaries_fail_closed_before_parse() {
        assert!(
            check_pair_lengths(
                FIRST_PRODUCTION_MAX_SERVER_ARTIFACT_BYTES,
                FIRST_PRODUCTION_MAX_CLIENT_ARTIFACT_BYTES
            )
            .is_ok()
        );
        assert!(matches!(
            check_pair_lengths(
                FIRST_PRODUCTION_MAX_SERVER_ARTIFACT_BYTES,
                FIRST_PRODUCTION_MAX_CLIENT_ARTIFACT_BYTES + 1
            ),
            Err(ContentError::LimitExceeded {
                resource: "first-production generation pair bytes",
                ..
            })
        ));
        assert!(check_decoded_fields(FIRST_PRODUCTION_MAX_DECODED_FIELDS).is_ok());
        assert!(check_decoded_fields(FIRST_PRODUCTION_MAX_DECODED_FIELDS + 1).is_err());
    }

    #[test]
    fn corruption_and_oversize_fail_before_staging() -> Result<(), ContentError> {
        let source = test_source(3)?;
        let compiled =
            compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
        let mut corrupt = compiled.server_artifact.clone();
        let index = corrupt.len() - 33;
        corrupt[index] ^= 0x5a;
        assert!(matches!(
            StagedGeneration::stage(&corrupt, &compiled.client_artifact, compiled.expectation(),),
            Err(ContentError::IntegrityMismatch(_))
        ));

        let oversized = vec![0u8; FIRST_PRODUCTION_MAX_SERVER_ARTIFACT_BYTES + 1];
        assert!(matches!(
            stage_artifact(&oversized, ProductionProjection::ServerAuthoritative),
            Err(ContentError::LimitExceeded {
                resource: "first-production server artifact bytes",
                ..
            })
        ));
        Ok(())
    }
}
