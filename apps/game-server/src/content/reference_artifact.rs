//! Bounded successor artifacts for the protected one-Item and native Item batch Reference corpora.
//!
//! The carrier follows the existing production Content envelope lineage: a fixed header,
//! critical section table, per-section SHA-256 and a whole-artifact trailer. Its separate magic
//! and profile id prevent either this profile or FIRST_PRODUCTION from being reinterpreted as the
//! other. The identity index is decoded independently of the body and addresses records by exact
//! typed semantic identity rather than by a storage-derived identity.

use super::digest::sha256;
use super::production::{encode_world_id, parse_world_id};
use super::{
    CanonicalReferencePlayableContent, ClientProjectionClass, ContentError, DefinitionFamily,
    DefinitionRevisionRef, PackageManifestBinding, ProductionAtom, ProductionKey,
    REFERENCE_PLAYABLE_CAPABILITY_PROFILE, REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
    ReferenceDefinitionKind, ReferenceItemDestination, ReferenceItemPhysicalClass,
    ReferenceItemStackClass, Sha256HexDigest, TypedDefinitionRef,
};
use crate::foundation::WorldId;

pub const OTERYN_REFERENCE_PLAYABLE_ARTIFACT_PROFILE_ID: &str =
    "OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v1";
pub const REFERENCE_PLAYABLE_MAX_SERVER_ARTIFACT_BYTES: usize = 8_715;
pub const REFERENCE_PLAYABLE_MAX_CLIENT_ARTIFACT_BYTES: usize = 8_675;
pub const REFERENCE_PLAYABLE_MAX_GENERATION_PAIR_BYTES: usize = 17_390;

const COMPILER_PROFILE: &str = "OTERYN_REFERENCE_PLAYABLE_COMPILER/v1";
const CANONICALIZATION_PROFILE: &str = "OTERYN_REFERENCE_PLAYABLE_CANONICALIZATION/v1";
const MAGIC: [u8; 8] = *b"OTRPA01\0";
const PROFILE_VERSION: u16 = 1;
const HEADER_LEN: usize = 24;
const SECTION_ENTRY_LEN: usize = 48;
const TRAILER_LEN: usize = 32;
const SECTION_COUNT: usize = 3;
const SECTION_FLAG_CRITICAL: u16 = 0x0001;
const SECTION_MANIFEST: u16 = 1;
const SECTION_INDEX: u16 = 2;
const SECTION_BODY: u16 = 3;
const MANIFEST_FIELD_COUNT: usize = 15;
const MAX_MANIFEST_BYTES: usize = 7_500;
const MAX_INDEX_BYTES: usize = 1_100;
const MAX_BODY_BYTES: usize = 128;
const MAX_KEY_BYTES: usize = 512;
const MAX_ATOM_BYTES: usize = 512;
const MAX_INDEX_ENTRIES: usize = 1;
const MAX_BODY_RECORD_BYTES: usize = 64;
const BODY_RECORD_VERSION: u8 = 1;
const FAMILY_ITEM: u8 = 5;

const BATCH_ARTIFACT_PROFILE_ID: &str = "OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v2";
const BATCH_COMPILER_PROFILE: &str = "OTERYN_REFERENCE_PLAYABLE_COMPILER/v2";
const BATCH_CANONICALIZATION_PROFILE: &str = "OTERYN_REFERENCE_PLAYABLE_CANONICALIZATION/v2";
const BATCH_MAGIC: [u8; 8] = *b"OTRPA02\0";
const BATCH_PROFILE_VERSION: u16 = 2;
const BATCH_MAX_INDEX_ENTRIES: usize = 64;
const BATCH_MAX_INDEX_BYTES: usize =
    4 + BATCH_MAX_INDEX_ENTRIES * (1 + 2 + MAX_KEY_BYTES + 2 + MAX_ATOM_BYTES + 4 + 4 + 32);
const BATCH_MAX_BODY_BYTES: usize = BATCH_MAX_INDEX_ENTRIES * MAX_BODY_RECORD_BYTES;
const BATCH_MAX_ARTIFACT_BYTES: usize = HEADER_LEN
    + SECTION_ENTRY_LEN * SECTION_COUNT
    + MAX_MANIFEST_BYTES
    + BATCH_MAX_INDEX_BYTES
    + BATCH_MAX_BODY_BYTES
    + TRAILER_LEN;
const BATCH_MAX_GENERATION_PAIR_BYTES: usize = BATCH_MAX_ARTIFACT_BYTES * 2;

const FAMILY_ARTIFACT_PROFILE_ID: &str = "OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v3";
const FAMILY_COMPILER_PROFILE: &str = "OTERYN_REFERENCE_PLAYABLE_COMPILER/v3";
const FAMILY_CANONICALIZATION_PROFILE: &str = "OTERYN_REFERENCE_PLAYABLE_CANONICALIZATION/v3";
const FAMILY_MAGIC: [u8; 8] = *b"OTRPA03\0";
const FAMILY_PROFILE_VERSION: u16 = 3;
pub const FAMILY_MAX_INDEX_ENTRIES: usize = 38_157;
const FAMILY_MAX_INDEX_BYTES: usize =
    4 + FAMILY_MAX_INDEX_ENTRIES * (1 + 2 + MAX_KEY_BYTES + 2 + MAX_ATOM_BYTES + 4 + 4 + 32);
const FAMILY_MAX_BODY_BYTES: usize = FAMILY_MAX_INDEX_ENTRIES * MAX_BODY_RECORD_BYTES;
const FAMILY_MAX_ARTIFACT_BYTES: usize = HEADER_LEN
    + SECTION_ENTRY_LEN * SECTION_COUNT
    + MAX_MANIFEST_BYTES
    + FAMILY_MAX_INDEX_BYTES
    + FAMILY_MAX_BODY_BYTES
    + TRAILER_LEN;
const FAMILY_MAX_GENERATION_PAIR_BYTES: usize = FAMILY_MAX_ARTIFACT_BYTES * 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReferenceArtifactProfile {
    OneItemV1,
    NativeItemBatchV2,
    NativeItemFamilyV3,
}

impl ReferenceArtifactProfile {
    fn for_definition_count(count: usize) -> Result<Self, ContentError> {
        match count {
            0 => Err(ContentError::InvalidArtifact(
                "Reference artifact requires at least one typed Item",
            )),
            1 => Ok(Self::OneItemV1),
            2..=BATCH_MAX_INDEX_ENTRIES => Ok(Self::NativeItemBatchV2),
            65..=FAMILY_MAX_INDEX_ENTRIES => Ok(Self::NativeItemFamilyV3),
            _ => Err(ContentError::LimitExceeded {
                resource: "Reference playable definitions",
                actual: count,
                limit: FAMILY_MAX_INDEX_ENTRIES,
            }),
        }
    }

    fn detect(bytes: &[u8], projection: ReferenceArtifactProjection) -> Result<Self, ContentError> {
        if bytes.get(..FAMILY_MAGIC.len()) == Some(FAMILY_MAGIC.as_slice()) {
            let profile = Self::NativeItemFamilyV3;
            check_artifact_length(profile, bytes.len(), projection)?;
            return Ok(profile);
        }
        if bytes.get(..BATCH_MAGIC.len()) == Some(BATCH_MAGIC.as_slice()) {
            let profile = Self::NativeItemBatchV2;
            check_artifact_length(profile, bytes.len(), projection)?;
            return Ok(profile);
        }

        let profile = Self::OneItemV1;
        check_artifact_length(profile, bytes.len(), projection)?;
        if bytes.get(..MAGIC.len()) != Some(MAGIC.as_slice()) {
            return Err(ContentError::InvalidMagic);
        }
        Ok(profile)
    }

    const fn magic(self) -> [u8; 8] {
        match self {
            Self::OneItemV1 => MAGIC,
            Self::NativeItemBatchV2 => BATCH_MAGIC,
            Self::NativeItemFamilyV3 => FAMILY_MAGIC,
        }
    }

    const fn profile_version(self) -> u16 {
        match self {
            Self::OneItemV1 => PROFILE_VERSION,
            Self::NativeItemBatchV2 => BATCH_PROFILE_VERSION,
            Self::NativeItemFamilyV3 => FAMILY_PROFILE_VERSION,
        }
    }

    const fn artifact_profile_id(self) -> &'static str {
        match self {
            Self::OneItemV1 => OTERYN_REFERENCE_PLAYABLE_ARTIFACT_PROFILE_ID,
            Self::NativeItemBatchV2 => BATCH_ARTIFACT_PROFILE_ID,
            Self::NativeItemFamilyV3 => FAMILY_ARTIFACT_PROFILE_ID,
        }
    }

    const fn compiler_profile(self) -> &'static str {
        match self {
            Self::OneItemV1 => COMPILER_PROFILE,
            Self::NativeItemBatchV2 => BATCH_COMPILER_PROFILE,
            Self::NativeItemFamilyV3 => FAMILY_COMPILER_PROFILE,
        }
    }

    const fn canonicalization_profile(self) -> &'static str {
        match self {
            Self::OneItemV1 => CANONICALIZATION_PROFILE,
            Self::NativeItemBatchV2 => BATCH_CANONICALIZATION_PROFILE,
            Self::NativeItemFamilyV3 => FAMILY_CANONICALIZATION_PROFILE,
        }
    }

    const fn max_index_entries(self) -> usize {
        match self {
            Self::OneItemV1 => MAX_INDEX_ENTRIES,
            Self::NativeItemBatchV2 => BATCH_MAX_INDEX_ENTRIES,
            Self::NativeItemFamilyV3 => FAMILY_MAX_INDEX_ENTRIES,
        }
    }

    const fn max_index_bytes(self) -> usize {
        match self {
            Self::OneItemV1 => MAX_INDEX_BYTES,
            Self::NativeItemBatchV2 => BATCH_MAX_INDEX_BYTES,
            Self::NativeItemFamilyV3 => FAMILY_MAX_INDEX_BYTES,
        }
    }

    const fn max_body_bytes(self) -> usize {
        match self {
            Self::OneItemV1 => MAX_BODY_BYTES,
            Self::NativeItemBatchV2 => BATCH_MAX_BODY_BYTES,
            Self::NativeItemFamilyV3 => FAMILY_MAX_BODY_BYTES,
        }
    }

    const fn artifact_limit(self, projection: ReferenceArtifactProjection) -> usize {
        match self {
            Self::OneItemV1 => projection.artifact_limit(),
            Self::NativeItemBatchV2 => BATCH_MAX_ARTIFACT_BYTES,
            Self::NativeItemFamilyV3 => FAMILY_MAX_ARTIFACT_BYTES,
        }
    }

    const fn generation_pair_limit(self) -> usize {
        match self {
            Self::OneItemV1 => REFERENCE_PLAYABLE_MAX_GENERATION_PAIR_BYTES,
            Self::NativeItemBatchV2 => BATCH_MAX_GENERATION_PAIR_BYTES,
            Self::NativeItemFamilyV3 => FAMILY_MAX_GENERATION_PAIR_BYTES,
        }
    }

    const fn accepts_item_count(self, count: usize) -> bool {
        match self {
            Self::OneItemV1 => count == 1,
            Self::NativeItemBatchV2 => count >= 2 && count <= BATCH_MAX_INDEX_ENTRIES,
            Self::NativeItemFamilyV3 => {
                count > BATCH_MAX_INDEX_ENTRIES && count <= FAMILY_MAX_INDEX_ENTRIES
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceArtifactProjection {
    ServerAuthoritative,
    ClientSafe,
}

impl ReferenceArtifactProjection {
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
            _ => Err(ContentError::InvalidArtifact(
                "unknown Reference artifact projection",
            )),
        }
    }

    const fn artifact_limit(self) -> usize {
        match self {
            Self::ServerAuthoritative => REFERENCE_PLAYABLE_MAX_SERVER_ARTIFACT_BYTES,
            Self::ClientSafe => REFERENCE_PLAYABLE_MAX_CLIENT_ARTIFACT_BYTES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReferenceArtifactMetadata {
    package_key: ProductionKey,
    package_revision: ProductionAtom,
    semantic_schema_version: ProductionAtom,
    licensing_metadata: ProductionAtom,
    source_manifest_digest: Sha256HexDigest,
    world_id: WorldId,
    coordinate_frame: ProductionAtom,
    content_lock_token: ProductionAtom,
    package_provenance_digest: Sha256HexDigest,
    source_profile: ProductionAtom,
    capability_profile: ProductionAtom,
    compiler_profile: ProductionAtom,
    canonicalization_profile: ProductionAtom,
    projection: ReferenceArtifactProjection,
}

impl ReferenceArtifactMetadata {
    fn from_source(
        source: &CanonicalReferencePlayableContent,
        projection: ReferenceArtifactProjection,
        profile: ReferenceArtifactProfile,
    ) -> Result<Self, ContentError> {
        Ok(Self {
            package_key: source.package_manifest.package_key.clone(),
            package_revision: source.package_manifest.package_revision.clone(),
            semantic_schema_version: source.package_manifest.semantic_schema_version.clone(),
            licensing_metadata: source.package_manifest.licensing_metadata.clone(),
            source_manifest_digest: source.package_manifest.source_manifest_digest.clone(),
            world_id: source.world_id,
            coordinate_frame: ProductionAtom::new(
                "Reference coordinate frame",
                source.coordinate_frame.as_str(),
            )?,
            content_lock_token: source.content_lock.revision_digest_token.clone(),
            package_provenance_digest: source.package_manifest.package_provenance_digest()?,
            source_profile: source.profile_revision.clone(),
            capability_profile: source.capability_profile.clone(),
            compiler_profile: ProductionAtom::new(
                "Reference compiler profile",
                profile.compiler_profile(),
            )?,
            canonicalization_profile: ProductionAtom::new(
                "Reference canonicalization profile",
                profile.canonicalization_profile(),
            )?,
            projection,
        })
    }

    fn same_generation_identity(&self, other: &Self) -> bool {
        self.package_key == other.package_key
            && self.package_revision == other.package_revision
            && self.semantic_schema_version == other.semantic_schema_version
            && self.licensing_metadata == other.licensing_metadata
            && self.source_manifest_digest == other.source_manifest_digest
            && self.world_id == other.world_id
            && self.coordinate_frame == other.coordinate_frame
            && self.content_lock_token == other.content_lock_token
            && self.package_provenance_digest == other.package_provenance_digest
            && self.source_profile == other.source_profile
            && self.capability_profile == other.capability_profile
            && self.compiler_profile == other.compiler_profile
            && self.canonicalization_profile == other.canonicalization_profile
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferencePlayableExpectation {
    metadata: ReferenceArtifactMetadata,
    server_artifact_digest: [u8; 32],
    client_artifact_digest: [u8; 32],
}

impl ReferencePlayableExpectation {
    pub fn package_provenance_digest(&self) -> &str {
        self.metadata.package_provenance_digest.as_str()
    }

    pub fn package_revision(&self) -> &str {
        self.metadata.package_revision.as_str()
    }

    pub const fn server_artifact_digest(&self) -> [u8; 32] {
        self.server_artifact_digest
    }

    pub const fn client_artifact_digest(&self) -> [u8; 32] {
        self.client_artifact_digest
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledReferencePlayableContent {
    pub server_artifact: Vec<u8>,
    pub client_artifact: Vec<u8>,
    expectation: ReferencePlayableExpectation,
}

impl CompiledReferencePlayableContent {
    pub fn expectation(&self) -> &ReferencePlayableExpectation {
        &self.expectation
    }

    pub fn server_digest(&self) -> [u8; 32] {
        self.expectation.server_artifact_digest
    }

    pub fn client_digest(&self) -> [u8; 32] {
        self.expectation.client_artifact_digest
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IndexEntry {
    identity: TypedDefinitionRef,
    body_offset: usize,
    body_length: usize,
    body_digest: [u8; 32],
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

/// Integrity-checked borrowed view over one Reference artifact.
///
/// Loading parses only the bounded manifest and typed identity index. Body records are decoded
/// only after an exact typed lookup succeeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferencePlayableArtifactView<'a> {
    bytes: &'a [u8],
    profile: ReferenceArtifactProfile,
    projection: ReferenceArtifactProjection,
    metadata: ReferenceArtifactMetadata,
    artifact_digest: [u8; 32],
    body_start: usize,
    body_length: usize,
    index: Vec<IndexEntry>,
}

impl<'a> ReferencePlayableArtifactView<'a> {
    pub(crate) fn load(
        bytes: &'a [u8],
        expected_projection: ReferenceArtifactProjection,
    ) -> Result<Self, ContentError> {
        let profile = ReferenceArtifactProfile::detect(bytes, expected_projection)?;
        if read_u16_at(bytes, 8)? != profile.profile_version() {
            return Err(ContentError::UnsupportedProfile(read_u16_at(bytes, 8)?));
        }
        let flags = read_u16_at(bytes, 10)?;
        if flags != 0 {
            return Err(ContentError::UnknownCriticalFlags(flags));
        }
        let projection =
            ReferenceArtifactProjection::from_byte(*bytes.get(12).ok_or(ContentError::Truncated)?)?;
        if projection != expected_projection {
            return Err(ContentError::PairMismatch(
                "Reference artifact has wrong projection",
            ));
        }
        if *bytes.get(13).ok_or(ContentError::Truncated)? != 0 {
            return Err(ContentError::InvalidArtifact(
                "Reference artifact reserved header byte is nonzero",
            ));
        }
        let section_count = usize::from(read_u16_at(bytes, 14)?);
        if section_count != SECTION_COUNT {
            return Err(ContentError::InvalidArtifact(
                "Reference artifact must contain exactly three sections",
            ));
        }
        let table_offset = usize::try_from(read_u32_at(bytes, 16)?)
            .map_err(|_| ContentError::InvalidSectionBounds)?;
        if table_offset != HEADER_LEN {
            return Err(ContentError::InvalidSectionBounds);
        }
        let payload_end = usize::try_from(read_u32_at(bytes, 20)?)
            .map_err(|_| ContentError::InvalidSectionBounds)?;
        let table_end = table_offset
            .checked_add(
                SECTION_ENTRY_LEN
                    .checked_mul(section_count)
                    .ok_or(ContentError::InvalidSectionBounds)?,
            )
            .ok_or(ContentError::InvalidSectionBounds)?;
        let expected_total = payload_end
            .checked_add(TRAILER_LEN)
            .ok_or(ContentError::InvalidSectionBounds)?;
        if expected_total != bytes.len() || table_end > payload_end {
            return Err(ContentError::Truncated);
        }

        let entries = parse_section_entries(bytes, section_count, table_offset)?;
        validate_section_ranges(&entries, table_end, payload_end)?;
        let manifest_entry = unique_section(&entries, SECTION_MANIFEST)?;
        let index_entry = unique_section(&entries, SECTION_INDEX)?;
        let body_entry = unique_section(&entries, SECTION_BODY)?;
        if manifest_entry.item_count != MANIFEST_FIELD_COUNT
            || index_entry.item_count != body_entry.item_count
            || !profile.accepts_item_count(index_entry.item_count)
        {
            return Err(ContentError::InvalidArtifact(
                "Reference artifact section cardinality mismatch",
            ));
        }
        check_section_length(profile, manifest_entry.kind, manifest_entry.length)?;
        check_section_length(profile, index_entry.kind, index_entry.length)?;
        check_section_length(profile, body_entry.kind, body_entry.length)?;

        let expected_digest = array32(
            bytes
                .get(payload_end..expected_total)
                .ok_or(ContentError::Truncated)?,
        )?;
        let actual_digest = sha256(bytes.get(..payload_end).ok_or(ContentError::Truncated)?);
        if expected_digest != actual_digest {
            return Err(ContentError::IntegrityMismatch("Reference artifact"));
        }
        for entry in &entries {
            let section = section_bytes(bytes, entry)?;
            if sha256(section) != entry.digest {
                return Err(ContentError::IntegrityMismatch(
                    "Reference artifact section",
                ));
            }
        }

        let metadata = parse_manifest(profile, section_bytes(bytes, manifest_entry)?, projection)?;
        let index = parse_index(
            profile,
            section_bytes(bytes, index_entry)?,
            index_entry.item_count,
        )?;
        validate_body_ranges(&index, body_entry.length)?;

        Ok(Self {
            bytes,
            profile,
            projection,
            metadata,
            artifact_digest: actual_digest,
            body_start: body_entry.offset,
            body_length: body_entry.length,
            index,
        })
    }

    pub const fn projection(&self) -> ReferenceArtifactProjection {
        self.projection
    }

    pub fn artifact_digest(&self) -> [u8; 32] {
        self.artifact_digest
    }

    pub fn artifact_profile_id(&self) -> &'static str {
        self.profile.artifact_profile_id()
    }

    pub fn indexed_identity_count(&self) -> usize {
        self.index.len()
    }

    pub fn indexed_identities(&self) -> impl ExactSizeIterator<Item = &TypedDefinitionRef> + '_ {
        self.index.iter().map(|entry| &entry.identity)
    }

    /// Compatibility accessor for the first canonical Item identity.
    ///
    /// One-Item v1 callers retain their original behavior. Batch-aware consumers should iterate
    /// Self::indexed_identities instead of treating this value as the complete generation.
    pub fn indexed_identity(&self) -> &TypedDefinitionRef {
        &self.index[0].identity
    }

    pub fn lookup_server_item(
        &self,
        identity: &TypedDefinitionRef,
    ) -> Result<Option<ReferenceServerItem>, ContentError> {
        if self.projection != ReferenceArtifactProjection::ServerAuthoritative {
            return Err(ContentError::PairMismatch(
                "server lookup requires server-authoritative projection",
            ));
        }
        self.selected_record(identity)?
            .map(parse_server_item)
            .transpose()
    }

    pub fn lookup_client_item(
        &self,
        identity: &TypedDefinitionRef,
    ) -> Result<Option<ReferenceClientItem>, ContentError> {
        if self.projection != ReferenceArtifactProjection::ClientSafe {
            return Err(ContentError::PairMismatch(
                "client lookup requires client-safe projection",
            ));
        }
        self.selected_record(identity)?
            .map(parse_client_item)
            .transpose()
    }

    fn selected_record(
        &self,
        identity: &TypedDefinitionRef,
    ) -> Result<Option<&'a [u8]>, ContentError> {
        let Ok(position) = self
            .index
            .binary_search_by(|entry| entry.identity.cmp(identity))
        else {
            return Ok(None);
        };
        let entry = &self.index[position];
        let start = self
            .body_start
            .checked_add(entry.body_offset)
            .ok_or(ContentError::InvalidSectionBounds)?;
        let end = start
            .checked_add(entry.body_length)
            .ok_or(ContentError::InvalidSectionBounds)?;
        let body_end = self
            .body_start
            .checked_add(self.body_length)
            .ok_or(ContentError::InvalidSectionBounds)?;
        if end > body_end {
            return Err(ContentError::InvalidSectionBounds);
        }
        let record = self
            .bytes
            .get(start..end)
            .ok_or(ContentError::InvalidSectionBounds)?;
        if sha256(record) != entry.body_digest {
            return Err(ContentError::IntegrityMismatch(
                "Reference indexed body record",
            ));
        }
        Ok(Some(record))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceServerItem {
    pub physical_class: ReferenceItemPhysicalClass,
    pub materializable: bool,
    pub stack_class: ReferenceItemStackClass,
    pub legal_destinations: Vec<ReferenceItemDestination>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceClientItem {
    pub physical_class: ReferenceItemPhysicalClass,
    pub stack_class: ReferenceItemStackClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferencePlayableGenerationIdentity {
    package_key: ProductionKey,
    package_revision: ProductionAtom,
    package_provenance_digest: Sha256HexDigest,
    world_id: WorldId,
    item_identities: Vec<TypedDefinitionRef>,
    server_artifact_digest: [u8; 32],
    client_artifact_digest: [u8; 32],
}

impl ReferencePlayableGenerationIdentity {
    pub fn package_key(&self) -> &ProductionKey {
        &self.package_key
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

    /// Compatibility accessor for the first canonical Item identity.
    ///
    /// Batch-aware consumers must use Self::item_identities to observe the complete set.
    pub fn item_identity(&self) -> &TypedDefinitionRef {
        &self.item_identities[0]
    }

    pub fn item_identities(&self) -> &[TypedDefinitionRef] {
        &self.item_identities
    }

    pub fn server_artifact_digest(&self) -> [u8; 32] {
        self.server_artifact_digest
    }

    pub fn client_artifact_digest(&self) -> [u8; 32] {
        self.client_artifact_digest
    }
}

/// A validated Reference pair that has no activation conversion or controller slot.
///
/// ```compile_fail
/// use oteryn_game_server::content::{ContentActivationController, compile_reference_playable};
/// # fn rejected(
/// #     linked: &oteryn_game_server::content::CanonicalReferencePlayableContent,
/// # ) -> Result<(), Box<dyn std::error::Error>> {
/// let compiled = compile_reference_playable(linked)?;
/// let mut controller = ContentActivationController::new();
/// controller.stage_primary(
///     &compiled.server_artifact,
///     &compiled.client_artifact,
///     compiled.expectation(),
/// )?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonAuthoritativeReferenceStage<'a> {
    server: ReferencePlayableArtifactView<'a>,
    client: ReferencePlayableArtifactView<'a>,
    identity: ReferencePlayableGenerationIdentity,
}

impl<'a> NonAuthoritativeReferenceStage<'a> {
    pub(crate) fn stage(
        server_bytes: &'a [u8],
        client_bytes: &'a [u8],
        expected: &ReferencePlayableExpectation,
    ) -> Result<Self, ContentError> {
        let server = ReferencePlayableArtifactView::load(
            server_bytes,
            ReferenceArtifactProjection::ServerAuthoritative,
        )?;
        let client = ReferencePlayableArtifactView::load(
            client_bytes,
            ReferenceArtifactProjection::ClientSafe,
        )?;
        if server.profile != client.profile {
            return Err(ContentError::PairMismatch(
                "Reference server/client artifact profile differs",
            ));
        }
        check_pair_lengths(server.profile, server_bytes.len(), client_bytes.len())?;
        if !server.metadata.same_generation_identity(&client.metadata) {
            return Err(ContentError::PairMismatch(
                "Reference server/client generation identity differs",
            ));
        }
        verify_expected(&server, expected)?;
        verify_expected(&client, expected)?;
        if server.artifact_digest != expected.server_artifact_digest
            || client.artifact_digest != expected.client_artifact_digest
        {
            return Err(ContentError::RevisionMismatch(
                "Reference expected artifact digest pair",
            ));
        }
        if server.index.len() != client.index.len()
            || server
                .index
                .iter()
                .zip(&client.index)
                .any(|(left, right)| left.identity != right.identity)
        {
            return Err(ContentError::PairMismatch(
                "Reference server/client typed identity set differs",
            ));
        }
        for entry in &server.index {
            let identity = &entry.identity;
            let server_item =
                server
                    .lookup_server_item(identity)?
                    .ok_or(ContentError::InvalidArtifact(
                        "Reference server item missing after indexed load",
                    ))?;
            let client_item =
                client
                    .lookup_client_item(identity)?
                    .ok_or(ContentError::InvalidArtifact(
                        "Reference client item missing after indexed load",
                    ))?;
            if server_item.physical_class != client_item.physical_class
                || server_item.stack_class != client_item.stack_class
            {
                return Err(ContentError::PairMismatch(
                    "Reference client-safe item differs from authoritative item",
                ));
            }
        }

        let identity = ReferencePlayableGenerationIdentity {
            package_key: server.metadata.package_key.clone(),
            package_revision: server.metadata.package_revision.clone(),
            package_provenance_digest: server.metadata.package_provenance_digest.clone(),
            world_id: server.metadata.world_id,
            item_identities: server
                .index
                .iter()
                .map(|entry| entry.identity.clone())
                .collect(),
            server_artifact_digest: server.artifact_digest,
            client_artifact_digest: client.artifact_digest,
        };
        Ok(Self {
            server,
            client,
            identity,
        })
    }

    pub fn identity(&self) -> &ReferencePlayableGenerationIdentity {
        &self.identity
    }

    pub fn server_artifact(&self) -> &ReferencePlayableArtifactView<'a> {
        &self.server
    }

    pub fn client_artifact(&self) -> &ReferencePlayableArtifactView<'a> {
        &self.client
    }
}

pub(crate) fn compile(
    source: &CanonicalReferencePlayableContent,
) -> Result<CompiledReferencePlayableContent, ContentError> {
    let profile = ReferenceArtifactProfile::for_definition_count(source.definitions.len())?;
    validate_compile_source(source, profile)?;

    let mut server_records = Vec::with_capacity(source.definitions.len());
    let mut client_records = Vec::with_capacity(source.definitions.len());
    for definition in &source.definitions {
        let ReferenceDefinitionKind::Item(item) = &definition.kind else {
            return Err(ContentError::InvalidArtifact(
                "Reference artifact requires typed Item definitions",
            ));
        };
        server_records.push(EncodedRecord {
            identity: &definition.definition,
            body: encode_server_item(item)?,
        });
        client_records.push(EncodedRecord {
            identity: &definition.definition,
            body: encode_client_item(item)?,
        });
    }

    let server_metadata = ReferenceArtifactMetadata::from_source(
        source,
        ReferenceArtifactProjection::ServerAuthoritative,
        profile,
    )?;
    let client_metadata = ReferenceArtifactMetadata::from_source(
        source,
        ReferenceArtifactProjection::ClientSafe,
        profile,
    )?;
    let server = encode_artifact(&server_metadata, profile, &server_records)?;
    let client = encode_artifact(&client_metadata, profile, &client_records)?;
    check_pair_lengths(profile, server.bytes.len(), client.bytes.len())?;
    Ok(CompiledReferencePlayableContent {
        server_artifact: server.bytes,
        client_artifact: client.bytes,
        expectation: ReferencePlayableExpectation {
            metadata: server_metadata,
            server_artifact_digest: server.digest,
            client_artifact_digest: client.digest,
        },
    })
}

fn validate_compile_source(
    source: &CanonicalReferencePlayableContent,
    profile: ReferenceArtifactProfile,
) -> Result<(), ContentError> {
    if source.profile_revision.as_str() != REFERENCE_PLAYABLE_CONTENT_PROFILE_ID {
        return Err(ContentError::RevisionMismatch(
            "Reference playable source profile",
        ));
    }
    if source.capability_profile.as_str() != REFERENCE_PLAYABLE_CAPABILITY_PROFILE {
        return Err(ContentError::RevisionMismatch(
            "Reference playable capability profile",
        ));
    }
    if source.content_lock.entries.len() != 1 {
        return Err(ContentError::LimitExceeded {
            resource: "Reference playable Content Lock entries",
            actual: source.content_lock.entries.len(),
            limit: 1,
        });
    }
    let root = &source.content_lock.entries[0];
    let expected_provenance = source.package_manifest.package_provenance_digest()?;
    if root.floating
        || root.dependency
        || root.package_key != source.package_manifest.package_key
        || root.package_revision != source.package_manifest.package_revision
        || root.package_provenance_digest != expected_provenance
    {
        return Err(ContentError::InvalidArtifact(
            "Reference playable Content Lock does not bind the exact root package",
        ));
    }
    if !profile.accepts_item_count(source.definitions.len()) {
        return Err(ContentError::LimitExceeded {
            resource: "Reference playable definitions",
            actual: source.definitions.len(),
            limit: profile.max_index_entries(),
        });
    }
    if !source.placements.is_empty()
        || !source.ordered_placements.is_empty()
        || !source.transitions.is_empty()
    {
        return Err(ContentError::InvalidArtifact(
            "Reference artifact does not admit spatial or transition records",
        ));
    }
    if source
        .definitions
        .windows(2)
        .any(|pair| pair[0].definition >= pair[1].definition)
    {
        return Err(ContentError::InvalidArtifact(
            "Reference artifact definitions are not strictly identity sorted",
        ));
    }
    for pair in source.definitions.windows(2) {
        if pair[0].definition.family() == pair[1].definition.family()
            && pair[0].definition.key() == pair[1].definition.key()
        {
            return Err(ContentError::DuplicateKey(
                pair[1].definition.key().as_str().to_owned(),
            ));
        }
    }
    for definition in &source.definitions {
        let ReferenceDefinitionKind::Item(item) = &definition.kind else {
            return Err(ContentError::InvalidArtifact(
                "Reference artifact requires client-safe typed Items",
            ));
        };
        if definition.definition.family() != DefinitionFamily::Item
            || definition.client_projection != ClientProjectionClass::ClientSafe
        {
            return Err(ContentError::InvalidArtifact(
                "Reference artifact requires client-safe typed Items",
            ));
        }
        let identity_only = item.physical_class == ReferenceItemPhysicalClass::Unknown
            || item.stack_class == ReferenceItemStackClass::Unknown;
        if identity_only && profile != ReferenceArtifactProfile::NativeItemFamilyV3 {
            return Err(ContentError::InvalidArtifact(
                "identity-only Items require the family-scale v3 artifact profile",
            ));
        }
    }
    Ok(())
}

fn encode_server_item(item: &super::ReferenceItemDefinition) -> Result<Vec<u8>, ContentError> {
    if item.legal_destinations.len() > 1 {
        return Err(ContentError::LimitExceeded {
            resource: "Reference Item legal destinations",
            actual: item.legal_destinations.len(),
            limit: 1,
        });
    }
    let mut bytes = Vec::with_capacity(6);
    bytes.push(BODY_RECORD_VERSION);
    bytes.push(physical_class_byte(item.physical_class));
    bytes.push(u8::from(item.materializable));
    bytes.push(stack_class_byte(item.stack_class));
    bytes.push(
        u8::try_from(item.legal_destinations.len())
            .map_err(|_| ContentError::InvalidSectionBounds)?,
    );
    for destination in &item.legal_destinations {
        bytes.push(destination_byte(*destination));
    }
    check_body_record_length(bytes.len())?;
    Ok(bytes)
}

fn encode_client_item(item: &super::ReferenceItemDefinition) -> Result<Vec<u8>, ContentError> {
    let bytes = vec![
        BODY_RECORD_VERSION,
        physical_class_byte(item.physical_class),
        stack_class_byte(item.stack_class),
    ];
    check_body_record_length(bytes.len())?;
    Ok(bytes)
}

fn physical_class_byte(value: ReferenceItemPhysicalClass) -> u8 {
    match value {
        ReferenceItemPhysicalClass::Physical => 1,
        ReferenceItemPhysicalClass::Unknown => 2,
    }
}

fn parse_physical_class(value: u8) -> Result<ReferenceItemPhysicalClass, ContentError> {
    match value {
        1 => Ok(ReferenceItemPhysicalClass::Physical),
        2 => Ok(ReferenceItemPhysicalClass::Unknown),
        _ => Err(ContentError::InvalidArtifact(
            "unknown Reference Item physical class",
        )),
    }
}

fn stack_class_byte(value: ReferenceItemStackClass) -> u8 {
    match value {
        ReferenceItemStackClass::NonStackable => 1,
        ReferenceItemStackClass::StackCapable => 2,
        ReferenceItemStackClass::Unknown => 3,
    }
}

fn parse_stack_class(value: u8) -> Result<ReferenceItemStackClass, ContentError> {
    match value {
        1 => Ok(ReferenceItemStackClass::NonStackable),
        2 => Ok(ReferenceItemStackClass::StackCapable),
        3 => Ok(ReferenceItemStackClass::Unknown),
        _ => Err(ContentError::InvalidArtifact(
            "unknown Reference Item stack class",
        )),
    }
}

fn destination_byte(value: ReferenceItemDestination) -> u8 {
    match value {
        ReferenceItemDestination::CharacterInventory => 1,
    }
}

fn parse_destination(value: u8) -> Result<ReferenceItemDestination, ContentError> {
    match value {
        1 => Ok(ReferenceItemDestination::CharacterInventory),
        _ => Err(ContentError::InvalidArtifact(
            "unknown Reference Item destination",
        )),
    }
}

fn parse_server_item(bytes: &[u8]) -> Result<ReferenceServerItem, ContentError> {
    check_body_record_length(bytes.len())?;
    let mut reader = SliceReader::new(bytes);
    if reader.read_u8()? != BODY_RECORD_VERSION {
        return Err(ContentError::InvalidArtifact(
            "unsupported Reference Item body version",
        ));
    }
    let physical_class = parse_physical_class(reader.read_u8()?)?;
    let materializable = match reader.read_u8()? {
        0 => false,
        1 => true,
        _ => {
            return Err(ContentError::InvalidArtifact(
                "invalid Reference Item materializable flag",
            ));
        }
    };
    let stack_class = parse_stack_class(reader.read_u8()?)?;
    let destination_count = usize::from(reader.read_u8()?);
    if destination_count > 1 {
        return Err(ContentError::LimitExceeded {
            resource: "Reference Item legal destinations",
            actual: destination_count,
            limit: 1,
        });
    }
    let mut legal_destinations = Vec::with_capacity(destination_count);
    for _ in 0..destination_count {
        legal_destinations.push(parse_destination(reader.read_u8()?)?);
    }
    reader.ensure_end()?;
    if materializable != legal_destinations.contains(&ReferenceItemDestination::CharacterInventory)
    {
        return Err(ContentError::InvalidArtifact(
            "Reference Item materialization and destination capability differ",
        ));
    }
    Ok(ReferenceServerItem {
        physical_class,
        materializable,
        stack_class,
        legal_destinations,
    })
}

fn parse_client_item(bytes: &[u8]) -> Result<ReferenceClientItem, ContentError> {
    check_body_record_length(bytes.len())?;
    let mut reader = SliceReader::new(bytes);
    if reader.read_u8()? != BODY_RECORD_VERSION {
        return Err(ContentError::InvalidArtifact(
            "unsupported Reference Item body version",
        ));
    }
    let item = ReferenceClientItem {
        physical_class: parse_physical_class(reader.read_u8()?)?,
        stack_class: parse_stack_class(reader.read_u8()?)?,
    };
    reader.ensure_end()?;
    Ok(item)
}

#[derive(Debug)]
struct EncodedRecord<'a> {
    identity: &'a TypedDefinitionRef,
    body: Vec<u8>,
}

#[derive(Debug)]
struct EncodedArtifact {
    bytes: Vec<u8>,
    digest: [u8; 32],
}

fn encode_artifact(
    metadata: &ReferenceArtifactMetadata,
    profile: ReferenceArtifactProfile,
    records: &[EncodedRecord<'_>],
) -> Result<EncodedArtifact, ContentError> {
    if !profile.accepts_item_count(records.len()) {
        return Err(ContentError::InvalidArtifact(
            "Reference artifact record count does not match profile",
        ));
    }

    let mut body = Vec::new();
    for record in records {
        check_body_record_length(record.body.len())?;
        body.extend_from_slice(&record.body);
    }
    let manifest = encode_manifest(metadata, profile)?;
    let index = encode_index(profile, records)?;
    check_section_length(profile, SECTION_MANIFEST, manifest.len())?;
    check_section_length(profile, SECTION_INDEX, index.len())?;
    check_section_length(profile, SECTION_BODY, body.len())?;

    let table_end = HEADER_LEN
        .checked_add(
            SECTION_ENTRY_LEN
                .checked_mul(SECTION_COUNT)
                .ok_or(ContentError::InvalidSectionBounds)?,
        )
        .ok_or(ContentError::InvalidSectionBounds)?;
    let manifest_offset = table_end;
    let index_offset = manifest_offset
        .checked_add(manifest.len())
        .ok_or(ContentError::InvalidSectionBounds)?;
    let body_offset = index_offset
        .checked_add(index.len())
        .ok_or(ContentError::InvalidSectionBounds)?;
    let payload_end = body_offset
        .checked_add(body.len())
        .ok_or(ContentError::InvalidSectionBounds)?;
    let total_len = payload_end
        .checked_add(TRAILER_LEN)
        .ok_or(ContentError::InvalidSectionBounds)?;
    check_artifact_length(profile, total_len, metadata.projection)?;

    let mut bytes = Vec::with_capacity(total_len);
    bytes.extend_from_slice(&profile.magic());
    put_u16(&mut bytes, profile.profile_version());
    put_u16(&mut bytes, 0);
    bytes.push(metadata.projection.as_byte());
    bytes.push(0);
    put_u16(&mut bytes, to_u16(SECTION_COUNT)?);
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
        SECTION_INDEX,
        index_offset,
        index.len(),
        records.len(),
        sha256(&index),
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
    bytes.extend_from_slice(&index);
    bytes.extend_from_slice(&body);
    let digest = sha256(&bytes);
    bytes.extend_from_slice(&digest);
    Ok(EncodedArtifact { bytes, digest })
}

fn encode_manifest(
    metadata: &ReferenceArtifactMetadata,
    profile: ReferenceArtifactProfile,
) -> Result<Vec<u8>, ContentError> {
    let world_id = encode_world_id(metadata.world_id);
    let fields = [
        profile.artifact_profile_id(),
        metadata.source_profile.as_str(),
        metadata.capability_profile.as_str(),
        metadata.package_key.as_str(),
        metadata.package_revision.as_str(),
        metadata.semantic_schema_version.as_str(),
        metadata.licensing_metadata.as_str(),
        metadata.source_manifest_digest.as_str(),
        world_id.as_str(),
        metadata.coordinate_frame.as_str(),
        metadata.content_lock_token.as_str(),
        metadata.package_provenance_digest.as_str(),
        metadata.compiler_profile.as_str(),
        metadata.canonicalization_profile.as_str(),
        metadata.projection.as_str(),
    ];
    let mut bytes = Vec::new();
    for field in fields {
        put_string(&mut bytes, field, MAX_ATOM_BYTES)?;
    }
    check_section_length(profile, SECTION_MANIFEST, bytes.len())?;
    Ok(bytes)
}

fn parse_manifest(
    profile: ReferenceArtifactProfile,
    bytes: &[u8],
    projection: ReferenceArtifactProjection,
) -> Result<ReferenceArtifactMetadata, ContentError> {
    check_section_length(profile, SECTION_MANIFEST, bytes.len())?;
    let mut reader = SliceReader::new(bytes);
    if reader.read_string(MAX_ATOM_BYTES)? != profile.artifact_profile_id() {
        return Err(ContentError::InvalidArtifact(
            "unexpected Reference artifact profile id",
        ));
    }
    let source_profile = ProductionAtom::from_artifact(
        "Reference source profile",
        reader.read_string(MAX_ATOM_BYTES)?,
    )?;
    if source_profile.as_str() != REFERENCE_PLAYABLE_CONTENT_PROFILE_ID {
        return Err(ContentError::RevisionMismatch(
            "Reference playable source profile",
        ));
    }
    let capability_profile = ProductionAtom::from_artifact(
        "Reference capability profile",
        reader.read_string(MAX_ATOM_BYTES)?,
    )?;
    if capability_profile.as_str() != REFERENCE_PLAYABLE_CAPABILITY_PROFILE {
        return Err(ContentError::RevisionMismatch(
            "Reference playable capability profile",
        ));
    }
    let package_key = ProductionKey::new(&reader.read_string(MAX_KEY_BYTES)?)?;
    let package_revision = ProductionAtom::from_artifact(
        "Reference package revision",
        reader.read_string(MAX_ATOM_BYTES)?,
    )?;
    let semantic_schema_version = ProductionAtom::from_artifact(
        "Reference semantic schema version",
        reader.read_string(MAX_ATOM_BYTES)?,
    )?;
    let licensing_metadata = ProductionAtom::from_artifact(
        "Reference licensing metadata",
        reader.read_string(MAX_ATOM_BYTES)?,
    )?;
    let source_manifest_digest = Sha256HexDigest::new(&reader.read_string(64)?)?;
    let world_id = parse_world_id(&reader.read_string(32)?)?;
    let coordinate_frame = ProductionAtom::from_artifact(
        "Reference coordinate frame",
        reader.read_string(MAX_ATOM_BYTES)?,
    )?;
    let content_lock_token = ProductionAtom::from_artifact(
        "Reference Content Lock token",
        reader.read_string(MAX_ATOM_BYTES)?,
    )?;
    let package_provenance_digest = Sha256HexDigest::new(&reader.read_string(64)?)?;
    let compiler_profile = ProductionAtom::from_artifact(
        "Reference compiler profile",
        reader.read_string(MAX_ATOM_BYTES)?,
    )?;
    if compiler_profile.as_str() != profile.compiler_profile() {
        return Err(ContentError::RevisionMismatch("Reference compiler profile"));
    }
    let canonicalization_profile = ProductionAtom::from_artifact(
        "Reference canonicalization profile",
        reader.read_string(MAX_ATOM_BYTES)?,
    )?;
    if canonicalization_profile.as_str() != profile.canonicalization_profile() {
        return Err(ContentError::RevisionMismatch(
            "Reference canonicalization profile",
        ));
    }
    if reader.read_string(MAX_ATOM_BYTES)? != projection.as_str() {
        return Err(ContentError::PairMismatch(
            "Reference manifest/header projection differs",
        ));
    }
    reader.ensure_end()?;

    let package = PackageManifestBinding::new(
        package_key.clone(),
        package_revision.clone(),
        semantic_schema_version.clone(),
        licensing_metadata.clone(),
        source_manifest_digest.clone(),
    );
    if package.package_provenance_digest()? != package_provenance_digest {
        return Err(ContentError::InvalidArtifact(
            "Reference package provenance digest mismatch",
        ));
    }
    Ok(ReferenceArtifactMetadata {
        package_key,
        package_revision,
        semantic_schema_version,
        licensing_metadata,
        source_manifest_digest,
        world_id,
        coordinate_frame,
        content_lock_token,
        package_provenance_digest,
        source_profile,
        capability_profile,
        compiler_profile,
        canonicalization_profile,
        projection,
    })
}

fn encode_index(
    profile: ReferenceArtifactProfile,
    records: &[EncodedRecord<'_>],
) -> Result<Vec<u8>, ContentError> {
    if !profile.accepts_item_count(records.len()) {
        return Err(ContentError::InvalidArtifact(
            "Reference artifact index count does not match profile",
        ));
    }
    let mut bytes = Vec::new();
    put_u32(&mut bytes, to_u32(records.len())?);
    let mut body_offset = 0_usize;
    let mut previous_identity: Option<&TypedDefinitionRef> = None;
    for record in records {
        if record.identity.family() != DefinitionFamily::Item {
            return Err(ContentError::InvalidArtifact(
                "Reference artifact index requires Item identity",
            ));
        }
        if previous_identity.is_some_and(|previous| previous >= record.identity) {
            return Err(ContentError::InvalidArtifact(
                "Reference artifact index is not strictly identity sorted",
            ));
        }
        check_body_record_length(record.body.len())?;
        bytes.push(FAMILY_ITEM);
        put_string(&mut bytes, record.identity.key().as_str(), MAX_KEY_BYTES)?;
        put_string(
            &mut bytes,
            record.identity.revision().as_str(),
            MAX_ATOM_BYTES,
        )?;
        put_u32(&mut bytes, to_u32(body_offset)?);
        put_u32(&mut bytes, to_u32(record.body.len())?);
        bytes.extend_from_slice(&sha256(&record.body));
        body_offset = body_offset
            .checked_add(record.body.len())
            .ok_or(ContentError::InvalidSectionBounds)?;
        previous_identity = Some(record.identity);
    }
    check_section_length(profile, SECTION_INDEX, bytes.len())?;
    Ok(bytes)
}

fn parse_index(
    profile: ReferenceArtifactProfile,
    bytes: &[u8],
    expected_count: usize,
) -> Result<Vec<IndexEntry>, ContentError> {
    check_section_length(profile, SECTION_INDEX, bytes.len())?;
    let mut reader = SliceReader::new(bytes);
    let count =
        usize::try_from(reader.read_u32()?).map_err(|_| ContentError::InvalidSectionBounds)?;
    if count != expected_count || !profile.accepts_item_count(count) {
        return Err(ContentError::InvalidArtifact(
            "Reference artifact index count mismatch",
        ));
    }
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        if reader.read_u8()? != FAMILY_ITEM {
            return Err(ContentError::InvalidArtifact(
                "unknown Reference artifact identity family",
            ));
        }
        let key = ProductionKey::new(&reader.read_string(MAX_KEY_BYTES)?)?;
        let revision = DefinitionRevisionRef::new(&reader.read_string(MAX_ATOM_BYTES)?)?;
        let body_offset =
            usize::try_from(reader.read_u32()?).map_err(|_| ContentError::InvalidSectionBounds)?;
        let body_length =
            usize::try_from(reader.read_u32()?).map_err(|_| ContentError::InvalidSectionBounds)?;
        check_body_record_length(body_length)?;
        let body_digest = array32(reader.take(32)?)?;
        entries.push(IndexEntry {
            identity: TypedDefinitionRef::new(DefinitionFamily::Item, key, revision),
            body_offset,
            body_length,
            body_digest,
        });
    }
    reader.ensure_end()?;
    if entries
        .windows(2)
        .any(|pair| pair[0].identity >= pair[1].identity)
    {
        return Err(ContentError::InvalidArtifact(
            "Reference artifact index is not strictly identity sorted",
        ));
    }
    Ok(entries)
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
        if !matches!(kind, SECTION_MANIFEST | SECTION_INDEX | SECTION_BODY)
            && flags & SECTION_FLAG_CRITICAL != 0
        {
            return Err(ContentError::UnknownCriticalSection(kind));
        }
        let offset = usize::try_from(read_u32_at(bytes, start + 4)?)
            .map_err(|_| ContentError::InvalidSectionBounds)?;
        let length = usize::try_from(read_u32_at(bytes, start + 8)?)
            .map_err(|_| ContentError::InvalidSectionBounds)?;
        let item_count = usize::try_from(read_u32_at(bytes, start + 12)?)
            .map_err(|_| ContentError::InvalidSectionBounds)?;
        entries.push(SectionEntry {
            kind,
            flags,
            offset,
            length,
            item_count,
            digest: array32(
                bytes
                    .get(start + 16..start + SECTION_ENTRY_LEN)
                    .ok_or(ContentError::Truncated)?,
            )?,
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
        || ranges.windows(2).any(|pair| pair[0].1 != pair[1].0)
    {
        return Err(ContentError::InvalidSectionBounds);
    }
    Ok(())
}

fn validate_body_ranges(entries: &[IndexEntry], body_length: usize) -> Result<(), ContentError> {
    let mut ranges = Vec::with_capacity(entries.len());
    for entry in entries {
        let end = entry
            .body_offset
            .checked_add(entry.body_length)
            .ok_or(ContentError::InvalidSectionBounds)?;
        if end > body_length {
            return Err(ContentError::InvalidSectionBounds);
        }
        ranges.push((entry.body_offset, end));
    }
    ranges.sort_unstable_by_key(|range| range.0);
    if ranges.first().map(|range| range.0) != Some(0)
        || ranges.last().map(|range| range.1) != Some(body_length)
        || ranges.windows(2).any(|pair| pair[0].1 != pair[1].0)
    {
        return Err(ContentError::InvalidSectionBounds);
    }
    Ok(())
}

fn unique_section(entries: &[SectionEntry], kind: u16) -> Result<&SectionEntry, ContentError> {
    let mut matches = entries.iter().filter(|entry| entry.kind == kind);
    let first = matches.next().ok_or(ContentError::InvalidArtifact(
        "Reference artifact required section missing",
    ))?;
    if matches.next().is_some() {
        return Err(ContentError::InvalidArtifact(
            "Reference artifact duplicates required section",
        ));
    }
    if first.flags & SECTION_FLAG_CRITICAL == 0 {
        return Err(ContentError::InvalidArtifact(
            "Reference artifact required section is not critical",
        ));
    }
    Ok(first)
}

fn verify_expected(
    artifact: &ReferencePlayableArtifactView<'_>,
    expected: &ReferencePlayableExpectation,
) -> Result<(), ContentError> {
    let mut normalized = artifact.metadata.clone();
    normalized.projection = ReferenceArtifactProjection::ServerAuthoritative;
    if normalized != expected.metadata {
        return Err(ContentError::RevisionMismatch(
            "Reference expected generation identity",
        ));
    }
    Ok(())
}

fn check_artifact_length(
    profile: ReferenceArtifactProfile,
    actual: usize,
    projection: ReferenceArtifactProjection,
) -> Result<(), ContentError> {
    let limit = profile.artifact_limit(projection);
    if actual > limit {
        return Err(ContentError::LimitExceeded {
            resource: "Reference artifact bytes",
            actual,
            limit,
        });
    }
    Ok(())
}

fn check_pair_lengths(
    profile: ReferenceArtifactProfile,
    server: usize,
    client: usize,
) -> Result<(), ContentError> {
    check_artifact_length(
        profile,
        server,
        ReferenceArtifactProjection::ServerAuthoritative,
    )?;
    check_artifact_length(profile, client, ReferenceArtifactProjection::ClientSafe)?;
    let actual = server
        .checked_add(client)
        .ok_or(ContentError::InvalidSectionBounds)?;
    let limit = profile.generation_pair_limit();
    if actual > limit {
        return Err(ContentError::LimitExceeded {
            resource: "Reference generation pair bytes",
            actual,
            limit,
        });
    }
    Ok(())
}

fn check_section_length(
    profile: ReferenceArtifactProfile,
    kind: u16,
    actual: usize,
) -> Result<(), ContentError> {
    let limit = match kind {
        SECTION_MANIFEST => MAX_MANIFEST_BYTES,
        SECTION_INDEX => profile.max_index_bytes(),
        SECTION_BODY => profile.max_body_bytes(),
        _ => return Err(ContentError::UnknownCriticalSection(kind)),
    };
    if actual > limit {
        return Err(ContentError::LimitExceeded {
            resource: "Reference artifact section bytes",
            actual,
            limit,
        });
    }
    Ok(())
}

fn check_body_record_length(actual: usize) -> Result<(), ContentError> {
    if actual > MAX_BODY_RECORD_BYTES {
        return Err(ContentError::LimitExceeded {
            resource: "Reference artifact body record bytes",
            actual,
            limit: MAX_BODY_RECORD_BYTES,
        });
    }
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

fn section_bytes<'a>(bytes: &'a [u8], entry: &SectionEntry) -> Result<&'a [u8], ContentError> {
    let end = entry
        .offset
        .checked_add(entry.length)
        .ok_or(ContentError::InvalidSectionBounds)?;
    bytes
        .get(entry.offset..end)
        .ok_or(ContentError::InvalidSectionBounds)
}

fn put_string(bytes: &mut Vec<u8>, value: &str, max: usize) -> Result<(), ContentError> {
    if value.is_empty()
        || value.len() > max
        || !value.is_ascii()
        || !value.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(ContentError::InvalidString(
            "Reference artifact string field",
        ));
    }
    put_u16(
        bytes,
        u16::try_from(value.len()).map_err(|_| ContentError::InvalidSectionBounds)?,
    );
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn put_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn put_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn to_u16(value: usize) -> Result<u16, ContentError> {
    u16::try_from(value).map_err(|_| ContentError::InvalidSectionBounds)
}

fn to_u32(value: usize) -> Result<u32, ContentError> {
    u32::try_from(value).map_err(|_| ContentError::InvalidSectionBounds)
}

fn read_u16_at(bytes: &[u8], offset: usize) -> Result<u16, ContentError> {
    let raw: [u8; 2] = bytes
        .get(offset..offset + 2)
        .ok_or(ContentError::Truncated)?
        .try_into()
        .map_err(|_| ContentError::Truncated)?;
    Ok(u16::from_be_bytes(raw))
}

fn read_u32_at(bytes: &[u8], offset: usize) -> Result<u32, ContentError> {
    let raw: [u8; 4] = bytes
        .get(offset..offset + 4)
        .ok_or(ContentError::Truncated)?
        .try_into()
        .map_err(|_| ContentError::Truncated)?;
    Ok(u32::from_be_bytes(raw))
}

fn array32(bytes: &[u8]) -> Result<[u8; 32], ContentError> {
    bytes.try_into().map_err(|_| ContentError::Truncated)
}

#[derive(Debug)]
struct SliceReader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> SliceReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn read_u8(&mut self) -> Result<u8, ContentError> {
        let value = *self.bytes.get(self.cursor).ok_or(ContentError::Truncated)?;
        self.cursor = self
            .cursor
            .checked_add(1)
            .ok_or(ContentError::InvalidSectionBounds)?;
        Ok(value)
    }

    fn read_u16(&mut self) -> Result<u16, ContentError> {
        let value = read_u16_at(self.bytes, self.cursor)?;
        self.cursor = self
            .cursor
            .checked_add(2)
            .ok_or(ContentError::InvalidSectionBounds)?;
        Ok(value)
    }

    fn read_u32(&mut self) -> Result<u32, ContentError> {
        let value = read_u32_at(self.bytes, self.cursor)?;
        self.cursor = self
            .cursor
            .checked_add(4)
            .ok_or(ContentError::InvalidSectionBounds)?;
        Ok(value)
    }

    fn read_string(&mut self, max: usize) -> Result<String, ContentError> {
        let length = usize::from(self.read_u16()?);
        if length > max {
            return Err(ContentError::LimitExceeded {
                resource: "Reference artifact string bytes",
                actual: length,
                limit: max,
            });
        }
        let raw = self.take(length)?;
        let value = std::str::from_utf8(raw)
            .map_err(|_| ContentError::InvalidString("Reference artifact UTF-8 field"))?;
        if value.is_empty()
            || !value.is_ascii()
            || !value.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Err(ContentError::InvalidString(
                "Reference artifact string field",
            ));
        }
        Ok(value.to_owned())
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], ContentError> {
        let end = self
            .cursor
            .checked_add(length)
            .ok_or(ContentError::InvalidSectionBounds)?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or(ContentError::Truncated)?;
        self.cursor = end;
        Ok(value)
    }

    fn ensure_end(&self) -> Result<(), ContentError> {
        if self.cursor != self.bytes.len() {
            return Err(ContentError::InvalidArtifact(
                "Reference artifact section has trailing bytes",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod native_item_batch_tests {
    use super::*;
    use crate::content::{
        CW2_B1_NATIVE_ITEM_BATCH_COUNT, CanonicalProjectDocuments, ProjectDraft,
        ProjectEvidenceLimits, protected_cw2_b1_native_item_batch_import,
    };

    const B1_EVIDENCE: &[u8] = include_bytes!(
        "../../../../docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
    );

    fn batch_limits() -> ProjectEvidenceLimits {
        ProjectEvidenceLimits {
            max_documents: 8,
            max_document_bytes: 2_097_152,
            max_total_bytes: 4_194_304,
            max_json_depth: 24,
            max_decoded_fields: 32_768,
            max_string_bytes: 2_097_152,
            max_locator_bytes: 160,
            max_locator_segments: 8,
            max_reference_records: CW2_B1_NATIVE_ITEM_BATCH_COUNT,
            max_import_records: CW2_B1_NATIVE_ITEM_BATCH_COUNT,
            max_reimport_states: CW2_B1_NATIVE_ITEM_BATCH_COUNT,
        }
    }

    fn linked_batch() -> Result<CanonicalReferencePlayableContent, Box<dyn std::error::Error>> {
        let imported = protected_cw2_b1_native_item_batch_import(B1_EVIDENCE)?;
        let canonical = CanonicalProjectDocuments::from_draft(
            ProjectDraft {
                project_revision: "project-r1".to_owned(),
                package_key: "oteryn:content.world-project".to_owned(),
                semantic_schema_version: "reference-schema-v1".to_owned(),
                licensing_metadata: "PENDING".to_owned(),
                world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
                coordinate_frame: "global-target-2026-07-28".to_owned(),
                records: imported.records,
                imports: vec![imported.batch],
                metadata: Vec::new(),
            },
            batch_limits(),
        )?;
        Ok(canonical
            .into_snapshot(batch_limits())?
            .parse(batch_limits())?
            .link()?)
    }

    #[test]
    fn frozen_native_item_batch_compiles_loads_and_stages_as_v2()
    -> Result<(), Box<dyn std::error::Error>> {
        let linked = linked_batch()?;
        assert_eq!(linked.definitions.len(), BATCH_MAX_INDEX_ENTRIES);

        let first = compile(&linked)?;
        let repeated = compile(&linked)?;
        assert_eq!(first.server_artifact, repeated.server_artifact);
        assert_eq!(first.client_artifact, repeated.client_artifact);

        let server = ReferencePlayableArtifactView::load(
            &first.server_artifact,
            ReferenceArtifactProjection::ServerAuthoritative,
        )?;
        let client = ReferencePlayableArtifactView::load(
            &first.client_artifact,
            ReferenceArtifactProjection::ClientSafe,
        )?;
        assert_eq!(server.artifact_profile_id(), BATCH_ARTIFACT_PROFILE_ID);
        assert_eq!(client.artifact_profile_id(), BATCH_ARTIFACT_PROFILE_ID);
        assert_eq!(
            server.indexed_identity_count(),
            CW2_B1_NATIVE_ITEM_BATCH_COUNT
        );
        assert_eq!(
            client.indexed_identity_count(),
            CW2_B1_NATIVE_ITEM_BATCH_COUNT
        );
        assert_eq!(
            server.indexed_identities().cloned().collect::<Vec<_>>(),
            linked
                .definitions
                .iter()
                .map(|definition| definition.definition.clone())
                .collect::<Vec<_>>()
        );

        for definition in &linked.definitions {
            let authoritative = server
                .lookup_server_item(&definition.definition)?
                .ok_or(ContentError::InvalidArtifact("authoritative batch Item"))?;
            let projected = client
                .lookup_client_item(&definition.definition)?
                .ok_or(ContentError::InvalidArtifact("client batch Item"))?;
            assert_eq!(authoritative.physical_class, projected.physical_class);
            assert_eq!(authoritative.stack_class, projected.stack_class);
        }

        let staged = NonAuthoritativeReferenceStage::stage(
            &first.server_artifact,
            &first.client_artifact,
            first.expectation(),
        )?;
        assert_eq!(
            staged.identity().item_identities().len(),
            CW2_B1_NATIVE_ITEM_BATCH_COUNT
        );
        assert_eq!(
            staged.identity().item_identity(),
            &linked.definitions[0].definition
        );
        Ok(())
    }

    #[test]
    fn successor_profiles_preserve_v2_bound_and_select_v3_for_the_full_family() {
        assert!(
            !ReferenceArtifactProfile::NativeItemBatchV2
                .accepts_item_count(BATCH_MAX_INDEX_ENTRIES + 1)
        );
        assert!(matches!(
            ReferenceArtifactProfile::for_definition_count(BATCH_MAX_INDEX_ENTRIES + 1),
            Ok(ReferenceArtifactProfile::NativeItemFamilyV3)
        ));
        assert!(
            ReferenceArtifactProfile::NativeItemFamilyV3
                .accepts_item_count(FAMILY_MAX_INDEX_ENTRIES)
        );
        assert!(matches!(
            ReferenceArtifactProfile::for_definition_count(FAMILY_MAX_INDEX_ENTRIES + 1),
            Err(ContentError::LimitExceeded {
                resource: "Reference playable definitions",
                actual,
                limit: FAMILY_MAX_INDEX_ENTRIES,
            }) if actual == FAMILY_MAX_INDEX_ENTRIES + 1
        ));
    }

    #[test]
    fn compiler_rejects_duplicate_item_key_across_revisions()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut linked = linked_batch()?;
        let duplicate_key = linked.definitions[0].definition.key().clone();
        linked.definitions[1].definition = TypedDefinitionRef::new(
            DefinitionFamily::Item,
            duplicate_key.clone(),
            DefinitionRevisionRef::new("definition-r2")?,
        );

        assert!(matches!(
            compile(&linked),
            Err(ContentError::DuplicateKey(key)) if key == duplicate_key.as_str()
        ));
        Ok(())
    }
}
