use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const FIXTURE_SCHEMA_VERSION: u16 = 1;
pub const EVIDENCE_PROFILE_VERSION: u16 = 1;
pub const EVIDENCE_PROFILE_ID: &str = "VSL_BUNDLE_EVIDENCE_PROFILE/v1/non-production";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentError {
    InvalidLimitProfile,
    InvalidLimit(&'static str),
    LimitExceeded {
        resource: &'static str,
        actual: usize,
        limit: usize,
    },
    InvalidKey(String),
    InvalidPackageKey(String),
    InvalidWorldId(String),
    InvalidRevision(String),
    InvalidString(&'static str),
    DuplicateKey(String),
    MissingReference {
        owner: String,
        target: String,
    },
    MissingSourceClassification(String),
    FixtureOnlyReleaseRejected,
    Truncated,
    InvalidMagic,
    UnsupportedProfile(u16),
    UnknownCriticalFlags(u16),
    UnknownCriticalSection(u16),
    IntegrityMismatch(&'static str),
    InvalidSectionBounds,
    InvalidArtifact(&'static str),
    PairMismatch(&'static str),
    RevisionMismatch(&'static str),
}

impl Display for ContentError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimitProfile => formatter.write_str("limit profile is not evidence-only"),
            Self::InvalidLimit(resource) => write!(formatter, "invalid zero limit for {resource}"),
            Self::LimitExceeded {
                resource,
                actual,
                limit,
            } => write!(
                formatter,
                "{resource} exceeds evidence limit: {actual} > {limit}"
            ),
            Self::InvalidKey(value) => write!(formatter, "invalid evidence ContentKey: {value}"),
            Self::InvalidPackageKey(value) => {
                write!(formatter, "invalid evidence PackageKey: {value}")
            }
            Self::InvalidWorldId(value) => write!(formatter, "invalid evidence WorldId: {value}"),
            Self::InvalidRevision(value) => {
                write!(formatter, "invalid evidence revision token: {value}")
            }
            Self::InvalidString(field) => {
                write!(formatter, "invalid evidence string field: {field}")
            }
            Self::DuplicateKey(value) => {
                write!(formatter, "duplicate canonical ContentKey: {value}")
            }
            Self::MissingReference { owner, target } => {
                write!(
                    formatter,
                    "content reference from {owner} to {target} is unresolved"
                )
            }
            Self::MissingSourceClassification(value) => {
                write!(
                    formatter,
                    "value-producing source {value} lacks explicit channel classification"
                )
            }
            Self::FixtureOnlyReleaseRejected => formatter.write_str(
                "non-production VSL fixture profile cannot compile for ordinary release",
            ),
            Self::Truncated => formatter.write_str("evidence artifact is truncated"),
            Self::InvalidMagic => formatter.write_str("invalid VSL evidence artifact magic"),
            Self::UnsupportedProfile(version) => {
                write!(
                    formatter,
                    "unsupported VSL evidence profile version {version}"
                )
            }
            Self::UnknownCriticalFlags(flags) => {
                write!(formatter, "unknown critical evidence flags 0x{flags:04x}")
            }
            Self::UnknownCriticalSection(kind) => {
                write!(formatter, "unknown critical evidence section {kind}")
            }
            Self::IntegrityMismatch(scope) => write!(formatter, "integrity mismatch in {scope}"),
            Self::InvalidSectionBounds => formatter.write_str("invalid evidence section bounds"),
            Self::InvalidArtifact(reason) => {
                write!(formatter, "invalid evidence artifact: {reason}")
            }
            Self::PairMismatch(reason) => {
                write!(formatter, "server/client evidence pair mismatch: {reason}")
            }
            Self::RevisionMismatch(field) => {
                write!(formatter, "artifact revision mismatch: {field}")
            }
        }
    }
}

impl Error for ContentError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceLimits {
    profile_id: String,
    max_artifact_bytes: usize,
    max_sections: usize,
    max_section_bytes: usize,
    max_records: usize,
    max_record_bytes: usize,
    max_key_bytes: usize,
    max_string_bytes: usize,
    max_definitions: usize,
    max_cells: usize,
    max_references: usize,
}

impl EvidenceLimits {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        profile_id: &str,
        max_artifact_bytes: usize,
        max_sections: usize,
        max_section_bytes: usize,
        max_records: usize,
        max_record_bytes: usize,
        max_key_bytes: usize,
        max_string_bytes: usize,
        max_definitions: usize,
        max_cells: usize,
        max_references: usize,
    ) -> Result<Self, ContentError> {
        if !profile_id.starts_with("evidence:") {
            return Err(ContentError::InvalidLimitProfile);
        }
        let values = [
            ("artifact bytes", max_artifact_bytes),
            ("sections", max_sections),
            ("section bytes", max_section_bytes),
            ("records", max_records),
            ("record bytes", max_record_bytes),
            ("key bytes", max_key_bytes),
            ("string bytes", max_string_bytes),
            ("definitions", max_definitions),
            ("cells", max_cells),
            ("references", max_references),
        ];
        if let Some((resource, _)) = values.iter().find(|(_, value)| *value == 0) {
            return Err(ContentError::InvalidLimit(resource));
        }
        Ok(Self {
            profile_id: profile_id.to_owned(),
            max_artifact_bytes,
            max_sections,
            max_section_bytes,
            max_records,
            max_record_bytes,
            max_key_bytes,
            max_string_bytes,
            max_definitions,
            max_cells,
            max_references,
        })
    }

    pub fn with_max_artifact_bytes(&self, max_artifact_bytes: usize) -> Result<Self, ContentError> {
        Self::new(
            &self.profile_id,
            max_artifact_bytes,
            self.max_sections,
            self.max_section_bytes,
            self.max_records,
            self.max_record_bytes,
            self.max_key_bytes,
            self.max_string_bytes,
            self.max_definitions,
            self.max_cells,
            self.max_references,
        )
    }

    pub(crate) fn max_artifact_bytes(&self) -> usize {
        self.max_artifact_bytes
    }
    pub(crate) fn max_sections(&self) -> usize {
        self.max_sections
    }
    pub(crate) fn max_section_bytes(&self) -> usize {
        self.max_section_bytes
    }
    pub(crate) fn max_records(&self) -> usize {
        self.max_records
    }
    pub(crate) fn max_record_bytes(&self) -> usize {
        self.max_record_bytes
    }
    pub(crate) fn max_key_bytes(&self) -> usize {
        self.max_key_bytes
    }
    pub(crate) fn max_string_bytes(&self) -> usize {
        self.max_string_bytes
    }
    pub(crate) fn max_definitions(&self) -> usize {
        self.max_definitions
    }
    pub(crate) fn max_cells(&self) -> usize {
        self.max_cells
    }
    pub(crate) fn max_references(&self) -> usize {
        self.max_references
    }

    pub(crate) fn check(
        &self,
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

fn valid_evidence_atom(value: &str) -> bool {
    !value.is_empty()
        && value.is_ascii()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'.' | b'_' | b'-' | b'/')
        })
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContentKey(String);

impl ContentKey {
    pub fn new(value: &str, limits: &EvidenceLimits) -> Result<Self, ContentError> {
        limits.check("key bytes", value.len(), limits.max_key_bytes())?;
        let valid_namespace = value
            .split_once(':')
            .is_some_and(|(namespace, local)| !namespace.is_empty() && !local.is_empty());
        if !valid_namespace || !valid_evidence_atom(value) {
            return Err(ContentError::InvalidKey(value.to_owned()));
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for ContentKey {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PackageKey(String);

impl PackageKey {
    pub fn new(value: &str, limits: &EvidenceLimits) -> Result<Self, ContentError> {
        limits.check("key bytes", value.len(), limits.max_key_bytes())?;
        let valid_namespace = value
            .split_once(':')
            .is_some_and(|(namespace, local)| !namespace.is_empty() && !local.is_empty());
        if !valid_namespace || !valid_evidence_atom(value) {
            return Err(ContentError::InvalidPackageKey(value.to_owned()));
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorldId(String);

impl WorldId {
    pub fn new(value: &str, limits: &EvidenceLimits) -> Result<Self, ContentError> {
        limits.check("string bytes", value.len(), limits.max_string_bytes())?;
        if !valid_evidence_atom(value) {
            return Err(ContentError::InvalidWorldId(value.to_owned()));
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RevisionToken(String);

impl RevisionToken {
    pub fn new(value: &str, limits: &EvidenceLimits) -> Result<Self, ContentError> {
        limits.check("string bytes", value.len(), limits.max_string_bytes())?;
        if !valid_evidence_atom(value) {
            return Err(ContentError::InvalidRevision(value.to_owned()));
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionSet {
    pub content: RevisionToken,
    pub map: RevisionToken,
    pub ruleset: RevisionToken,
    pub world_policy: RevisionToken,
    pub compiler: RevisionToken,
    pub canonicalization: RevisionToken,
    pub content_lock: RevisionToken,
    pub provenance: RevisionToken,
    pub sim_profile: RevisionToken,
    pub fixture_profile: RevisionToken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileTarget {
    Evidence,
    OrdinaryRelease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionClass {
    Walkable,
    Blocked,
}

impl CollisionClass {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Walkable => "walkable",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiplicityClass {
    ChannelLocalRepeatable,
    ChannelLocalSharedEligibility,
    WorldScopedUnique,
    ExplicitEventPolicyRequired,
}

impl MultiplicityClass {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ChannelLocalRepeatable => "CHANNEL_LOCAL_REPEATABLE",
            Self::ChannelLocalSharedEligibility => "CHANNEL_LOCAL_SHARED_ELIGIBILITY",
            Self::WorldScopedUnique => "WORLD_SCOPED_UNIQUE",
            Self::ExplicitEventPolicyRequired => "EXPLICIT_EVENT_POLICY_REQUIRED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EligibilityScope {
    CharacterWorld,
    AccountWorld,
    World,
}

impl EligibilityScope {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::CharacterWorld => "CHARACTER_WORLD",
            Self::AccountWorld => "ACCOUNT_WORLD",
            Self::World => "WORLD",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnRecoveryClass {
    EphemeralScopeReset,
    CheckpointedRuntimeContinuity,
    DurableEventOccurrence,
}

impl SpawnRecoveryClass {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::EphemeralScopeReset => "EPHEMERAL_SCOPE_RESET",
            Self::CheckpointedRuntimeContinuity => "CHECKPOINTED_RUNTIME_CONTINUITY",
            Self::DurableEventOccurrence => "DURABLE_EVENT_OCCURRENCE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectFamily {
    Damage,
}

impl EffectFamily {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Damage => "damage",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionDefinition {
    pub key: ContentKey,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaDefinition {
    pub key: ContentKey,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainDefinition {
    pub key: ContentKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellDefinition {
    pub key: ContentKey,
    pub region_key: ContentKey,
    pub area_key: ContentKey,
    pub terrain_key: ContentKey,
    pub x: i32,
    pub y: i32,
    pub z: i16,
    pub collision: CollisionClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelocationDefinition {
    pub key: ContentKey,
    pub from_cell: ContentKey,
    pub to_cell: ContentKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorDefinition {
    pub key: ContentKey,
    pub policy_revision: RevisionToken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationDefinition {
    pub key: ContentKey,
    pub synthetic_asset_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureDefinition {
    pub key: ContentKey,
    pub behavior_key: ContentKey,
    pub presentation_key: ContentKey,
    pub fixture_max_hp: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnDefinition {
    pub key: ContentKey,
    pub creature_key: ContentKey,
    pub behavior_key: ContentKey,
    pub cell_key: ContentKey,
    pub fixture_population_limit: u16,
    pub recovery: SpawnRecoveryClass,
    pub multiplicity: Option<MultiplicityClass>,
    pub eligibility_scope: Option<EligibilityScope>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaProfileDefinition {
    pub key: ContentKey,
    pub fixture_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectDefinition {
    pub key: ContentKey,
    pub family: EffectFamily,
    pub formula_profile_key: ContentKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbilityDefinition {
    pub key: ContentKey,
    pub effect_key: ContentKey,
    pub presentation_key: ContentKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemDefinition {
    pub key: ContentKey,
    pub presentation_key: ContentKey,
    pub materializable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootEntryDefinition {
    pub key: ContentKey,
    pub item_key: ContentKey,
    pub rng_purpose_key: ContentKey,
    pub fixture_weight: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootTableDefinition {
    pub key: ContentKey,
    pub entries: Vec<LootEntryDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XpDefinition {
    pub key: ContentKey,
    pub formula_profile_key: ContentKey,
    pub fixture_amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RngFixtureContext {
    pub profile_revision: RevisionToken,
    pub synthetic_root_label: String,
    pub purpose_keys: Vec<ContentKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureSource {
    pub schema_version: u16,
    pub package_key: PackageKey,
    pub package_revision: RevisionToken,
    pub world_id: WorldId,
    pub revisions: RevisionSet,
    pub regions: Vec<RegionDefinition>,
    pub areas: Vec<AreaDefinition>,
    pub terrains: Vec<TerrainDefinition>,
    pub cells: Vec<CellDefinition>,
    pub relocations: Vec<RelocationDefinition>,
    pub behaviors: Vec<BehaviorDefinition>,
    pub presentations: Vec<PresentationDefinition>,
    pub creatures: Vec<CreatureDefinition>,
    pub spawns: Vec<SpawnDefinition>,
    pub formula_profiles: Vec<FormulaProfileDefinition>,
    pub effects: Vec<EffectDefinition>,
    pub abilities: Vec<AbilityDefinition>,
    pub items: Vec<ItemDefinition>,
    pub loot_tables: Vec<LootTableDefinition>,
    pub xp_definitions: Vec<XpDefinition>,
    pub rng: RngFixtureContext,
}

impl FixtureSource {
    #[cfg(test)]
    pub fn reverse_enumeration_for_test(&mut self) {
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
pub struct CanonicalGraph {
    pub(crate) source: FixtureSource,
}

impl CanonicalGraph {
    pub fn revisions(&self) -> &RevisionSet {
        &self.source.revisions
    }
    pub fn package_key(&self) -> &PackageKey {
        &self.source.package_key
    }
    pub fn package_revision(&self) -> &RevisionToken {
        &self.source.package_revision
    }
    pub fn world_id(&self) -> &WorldId {
        &self.source.world_id
    }
}
