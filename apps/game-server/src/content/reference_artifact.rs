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

pub const TYPED_REFERENCE_ITEM_MAX_SERVER_RECORD_BYTES: usize = 3_555;
pub const TYPED_REFERENCE_ITEM_MAX_CLIENT_RECORD_BYTES: usize = 3_433;
pub const TYPED_REFERENCE_ITEM_MAX_SERVER_BODY_BYTES: usize = 135_648_135;
pub const TYPED_REFERENCE_ITEM_MAX_CLIENT_BODY_BYTES: usize = 130_992_981;
pub const TYPED_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES: usize = 176_445_672;
pub const TYPED_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES: usize = 171_790_518;
pub const TYPED_REFERENCE_ITEM_MAX_GENERATION_PAIR_BYTES: usize = 348_236_190;
const TYPED_ARTIFACT_PROFILE_ID: &str = "OTERYN_REFERENCE_PLAYABLE_ARTIFACT/v4";
const TYPED_COMPILER_PROFILE: &str = "OTERYN_REFERENCE_PLAYABLE_COMPILER/v4";
const TYPED_CANONICALIZATION_PROFILE: &str = "OTERYN_REFERENCE_PLAYABLE_CANONICALIZATION/v4";
const TYPED_MAGIC: [u8; 8] = *b"OTRPA04\0";
const TYPED_PROFILE_VERSION: u16 = 4;
const TYPED_BODY_RECORD_VERSION: u8 = 2;
const TYPED_MAX_INDEX_BYTES: usize = FAMILY_MAX_INDEX_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReferenceArtifactProfile {
    OneItemV1,
    NativeItemBatchV2,
    NativeItemFamilyV3,
    TypedItemV4,
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

    fn for_source(source: &CanonicalReferencePlayableContent) -> Result<Self, ContentError> {
        if source.definitions.iter().any(|definition| {
            matches!(&definition.kind, ReferenceDefinitionKind::Item(item) if !item.semantics.is_all_unknown())
        }) {
            if source.definitions.len() != FAMILY_MAX_INDEX_ENTRIES {
                return Err(ContentError::LimitExceeded {
                    resource: "Reference playable definitions",
                    actual: source.definitions.len(),
                    limit: FAMILY_MAX_INDEX_ENTRIES,
                });
            }
            return Ok(Self::TypedItemV4);
        }
        Self::for_definition_count(source.definitions.len())
    }

    fn detect(bytes: &[u8], projection: ReferenceArtifactProjection) -> Result<Self, ContentError> {
        if bytes.get(..TYPED_MAGIC.len()) == Some(TYPED_MAGIC.as_slice()) {
            let profile = Self::TypedItemV4;
            check_artifact_length(profile, bytes.len(), projection)?;
            return Ok(profile);
        }
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
            Self::TypedItemV4 => TYPED_MAGIC,
        }
    }

    const fn profile_version(self) -> u16 {
        match self {
            Self::OneItemV1 => PROFILE_VERSION,
            Self::NativeItemBatchV2 => BATCH_PROFILE_VERSION,
            Self::NativeItemFamilyV3 => FAMILY_PROFILE_VERSION,
            Self::TypedItemV4 => TYPED_PROFILE_VERSION,
        }
    }

    const fn artifact_profile_id(self) -> &'static str {
        match self {
            Self::OneItemV1 => OTERYN_REFERENCE_PLAYABLE_ARTIFACT_PROFILE_ID,
            Self::NativeItemBatchV2 => BATCH_ARTIFACT_PROFILE_ID,
            Self::NativeItemFamilyV3 => FAMILY_ARTIFACT_PROFILE_ID,
            Self::TypedItemV4 => TYPED_ARTIFACT_PROFILE_ID,
        }
    }

    const fn compiler_profile(self) -> &'static str {
        match self {
            Self::OneItemV1 => COMPILER_PROFILE,
            Self::NativeItemBatchV2 => BATCH_COMPILER_PROFILE,
            Self::NativeItemFamilyV3 => FAMILY_COMPILER_PROFILE,
            Self::TypedItemV4 => TYPED_COMPILER_PROFILE,
        }
    }

    const fn canonicalization_profile(self) -> &'static str {
        match self {
            Self::OneItemV1 => CANONICALIZATION_PROFILE,
            Self::NativeItemBatchV2 => BATCH_CANONICALIZATION_PROFILE,
            Self::NativeItemFamilyV3 => FAMILY_CANONICALIZATION_PROFILE,
            Self::TypedItemV4 => TYPED_CANONICALIZATION_PROFILE,
        }
    }

    const fn max_index_entries(self) -> usize {
        match self {
            Self::OneItemV1 => MAX_INDEX_ENTRIES,
            Self::NativeItemBatchV2 => BATCH_MAX_INDEX_ENTRIES,
            Self::NativeItemFamilyV3 => FAMILY_MAX_INDEX_ENTRIES,
            Self::TypedItemV4 => FAMILY_MAX_INDEX_ENTRIES,
        }
    }

    const fn max_index_bytes(self) -> usize {
        match self {
            Self::OneItemV1 => MAX_INDEX_BYTES,
            Self::NativeItemBatchV2 => BATCH_MAX_INDEX_BYTES,
            Self::NativeItemFamilyV3 => FAMILY_MAX_INDEX_BYTES,
            Self::TypedItemV4 => TYPED_MAX_INDEX_BYTES,
        }
    }

    const fn max_body_bytes(self) -> usize {
        match self {
            Self::OneItemV1 => MAX_BODY_BYTES,
            Self::NativeItemBatchV2 => BATCH_MAX_BODY_BYTES,
            Self::NativeItemFamilyV3 => FAMILY_MAX_BODY_BYTES,
            Self::TypedItemV4 => TYPED_REFERENCE_ITEM_MAX_SERVER_BODY_BYTES,
        }
    }

    const fn artifact_limit(self, projection: ReferenceArtifactProjection) -> usize {
        match self {
            Self::OneItemV1 => projection.artifact_limit(),
            Self::NativeItemBatchV2 => BATCH_MAX_ARTIFACT_BYTES,
            Self::NativeItemFamilyV3 => FAMILY_MAX_ARTIFACT_BYTES,
            Self::TypedItemV4 => match projection {
                ReferenceArtifactProjection::ServerAuthoritative => {
                    TYPED_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES
                }
                ReferenceArtifactProjection::ClientSafe => {
                    TYPED_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES
                }
            },
        }
    }

    const fn generation_pair_limit(self) -> usize {
        match self {
            Self::OneItemV1 => REFERENCE_PLAYABLE_MAX_GENERATION_PAIR_BYTES,
            Self::NativeItemBatchV2 => BATCH_MAX_GENERATION_PAIR_BYTES,
            Self::NativeItemFamilyV3 => FAMILY_MAX_GENERATION_PAIR_BYTES,
            Self::TypedItemV4 => TYPED_REFERENCE_ITEM_MAX_GENERATION_PAIR_BYTES,
        }
    }

    const fn accepts_item_count(self, count: usize) -> bool {
        match self {
            Self::OneItemV1 => count == 1,
            Self::NativeItemBatchV2 => count >= 2 && count <= BATCH_MAX_INDEX_ENTRIES,
            Self::NativeItemFamilyV3 => {
                count > BATCH_MAX_INDEX_ENTRIES && count <= FAMILY_MAX_INDEX_ENTRIES
            }
            Self::TypedItemV4 => count == FAMILY_MAX_INDEX_ENTRIES,
        }
    }

    const fn is_typed(self) -> bool {
        matches!(self, Self::TypedItemV4)
    }

    const fn body_record_limit(self, projection: ReferenceArtifactProjection) -> usize {
        if self.is_typed() {
            match projection {
                ReferenceArtifactProjection::ServerAuthoritative => {
                    TYPED_REFERENCE_ITEM_MAX_SERVER_RECORD_BYTES
                }
                ReferenceArtifactProjection::ClientSafe => {
                    TYPED_REFERENCE_ITEM_MAX_CLIENT_RECORD_BYTES
                }
            }
        } else {
            MAX_BODY_RECORD_BYTES
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
        check_body_section_length(profile, projection, body_entry.length)?;

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
            projection,
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
            .map(|record| parse_server_item(self.profile, record, &self.index))
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
            .map(|record| parse_client_item(self.profile, record, &self.index))
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
    pub semantics: super::ReferenceItemSemantics,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceClientItem {
    pub physical_class: ReferenceItemPhysicalClass,
    pub stack_class: ReferenceItemStackClass,
    pub semantics: super::ReferenceItemSemantics,
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
    let profile = ReferenceArtifactProfile::for_source(source)?;
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
            body: encode_server_item(item, profile, &source.definitions)?,
        });
        client_records.push(EncodedRecord {
            identity: &definition.definition,
            body: encode_client_item(item, profile, &source.definitions)?,
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
        if identity_only
            && !matches!(
                profile,
                ReferenceArtifactProfile::NativeItemFamilyV3
                    | ReferenceArtifactProfile::TypedItemV4
            )
        {
            return Err(ContentError::InvalidArtifact(
                "identity-only Items require the family-scale v3 artifact profile",
            ));
        }
    }
    Ok(())
}

fn encode_server_item(
    item: &super::ReferenceItemDefinition,
    profile: ReferenceArtifactProfile,
    definitions: &[super::ReferenceDefinition],
) -> Result<Vec<u8>, ContentError> {
    if profile.is_typed() {
        return encode_typed_item(
            item,
            ReferenceArtifactProjection::ServerAuthoritative,
            definitions,
        );
    }
    if !item.semantics.is_all_unknown() {
        return Err(ContentError::InvalidArtifact(
            "typed Reference Item semantics require artifact v4",
        ));
    }
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
    check_body_record_length(
        profile,
        ReferenceArtifactProjection::ServerAuthoritative,
        bytes.len(),
    )?;
    Ok(bytes)
}

fn encode_client_item(
    item: &super::ReferenceItemDefinition,
    profile: ReferenceArtifactProfile,
    definitions: &[super::ReferenceDefinition],
) -> Result<Vec<u8>, ContentError> {
    if profile.is_typed() {
        return encode_typed_item(item, ReferenceArtifactProjection::ClientSafe, definitions);
    }
    if !item.semantics.is_all_unknown() {
        return Err(ContentError::InvalidArtifact(
            "typed Reference Item semantics require artifact v4",
        ));
    }
    let bytes = vec![
        BODY_RECORD_VERSION,
        physical_class_byte(item.physical_class),
        stack_class_byte(item.stack_class),
    ];
    check_body_record_length(
        profile,
        ReferenceArtifactProjection::ClientSafe,
        bytes.len(),
    )?;
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

fn parse_server_item(
    profile: ReferenceArtifactProfile,
    bytes: &[u8],
    identities: &[IndexEntry],
) -> Result<ReferenceServerItem, ContentError> {
    if profile.is_typed() {
        return parse_typed_server_item(bytes, identities);
    }
    check_body_record_length(
        profile,
        ReferenceArtifactProjection::ServerAuthoritative,
        bytes.len(),
    )?;
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
        semantics: Default::default(),
    })
}

fn parse_client_item(
    profile: ReferenceArtifactProfile,
    bytes: &[u8],
    identities: &[IndexEntry],
) -> Result<ReferenceClientItem, ContentError> {
    if profile.is_typed() {
        return parse_typed_client_item(bytes, identities);
    }
    check_body_record_length(
        profile,
        ReferenceArtifactProjection::ClientSafe,
        bytes.len(),
    )?;
    let mut reader = SliceReader::new(bytes);
    if reader.read_u8()? != BODY_RECORD_VERSION {
        return Err(ContentError::InvalidArtifact(
            "unsupported Reference Item body version",
        ));
    }
    let item = ReferenceClientItem {
        physical_class: parse_physical_class(reader.read_u8()?)?,
        stack_class: parse_stack_class(reader.read_u8()?)?,
        semantics: Default::default(),
    };
    reader.ensure_end()?;
    Ok(item)
}

const ITEM_GROUP_PRESENTATION: u8 = 1;
const ITEM_GROUP_CLASSIFICATION: u8 = 2;
const ITEM_GROUP_PHYSICAL: u8 = 3;
const ITEM_GROUP_STACK: u8 = 4;
const ITEM_GROUP_EQUIPMENT: u8 = 5;
const ITEM_GROUP_WEAPON: u8 = 6;
const ITEM_GROUP_PROTECTION: u8 = 7;
const ITEM_GROUP_SKILL_MODIFIERS: u8 = 8;
const ITEM_GROUP_CHARGES: u8 = 9;
const ITEM_GROUP_TEMPORAL: u8 = 10;
const ITEM_GROUP_CONTAINER: u8 = 11;
const ITEM_GROUP_IMBUEMENT: u8 = 12;
const ITEM_GROUP_USE_TRANSFORM: u8 = 13;
const ITEM_GROUP_TRADE_RESTRICTIONS: u8 = 14;
const ITEM_GROUP_FLUID: u8 = 15;
const ITEM_GROUP_READABLE_WRITEABLE: u8 = 16;
const CLIENT_ITEM_GROUPS: [u8; 11] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 12];

fn put_field<T>(
    bytes: &mut Vec<u8>,
    field: &super::ReferenceItemField<T>,
    encode: impl FnOnce(&mut Vec<u8>, &T) -> Result<(), ContentError>,
) -> Result<(), ContentError> {
    use super::ReferenceItemField::{Conflict, Known, NotApplicable, Unknown};
    match field {
        Unknown => bytes.push(0),
        NotApplicable => bytes.push(1),
        Conflict => bytes.push(2),
        Known(value) => {
            bytes.push(3);
            encode(bytes, value)?;
        }
    }
    Ok(())
}

fn read_field<T>(
    reader: &mut SliceReader<'_>,
    decode: impl FnOnce(&mut SliceReader<'_>) -> Result<T, ContentError>,
) -> Result<super::ReferenceItemField<T>, ContentError> {
    Ok(match reader.read_u8()? {
        0 => super::ReferenceItemField::Unknown,
        1 => super::ReferenceItemField::NotApplicable,
        2 => super::ReferenceItemField::Conflict,
        3 => super::ReferenceItemField::Known(decode(reader)?),
        _ => {
            return Err(ContentError::InvalidArtifact(
                "invalid Reference Item field state",
            ));
        }
    })
}

fn put_bool(bytes: &mut Vec<u8>, value: bool) {
    bytes.push(u8::from(value));
}

fn read_bool(reader: &mut SliceReader<'_>) -> Result<bool, ContentError> {
    match reader.read_u8()? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(ContentError::InvalidArtifact(
            "invalid Reference Item boolean",
        )),
    }
}

fn put_utf8(bytes: &mut Vec<u8>, value: &str, max: usize) -> Result<(), ContentError> {
    if value.len() > max {
        return Err(ContentError::LimitExceeded {
            resource: "Reference Item UTF-8 atom bytes",
            actual: value.len(),
            limit: max,
        });
    }
    put_u16(bytes, to_u16(value.len())?);
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn read_utf8(reader: &mut SliceReader<'_>, max: usize) -> Result<String, ContentError> {
    let length = usize::from(reader.read_u16()?);
    if length > max {
        return Err(ContentError::LimitExceeded {
            resource: "Reference Item UTF-8 atom bytes",
            actual: length,
            limit: max,
        });
    }
    std::str::from_utf8(reader.take(length)?)
        .map(str::to_owned)
        .map_err(|_| ContentError::InvalidString("Reference Item UTF-8 atom"))
}

fn put_rational(
    bytes: &mut Vec<u8>,
    value: &super::ReferenceRationalPercent,
) -> Result<(), ContentError> {
    value.validate()?;
    bytes.extend_from_slice(&value.numerator.to_be_bytes());
    bytes.extend_from_slice(&value.denominator.to_be_bytes());
    Ok(())
}

fn read_rational(
    reader: &mut SliceReader<'_>,
) -> Result<super::ReferenceRationalPercent, ContentError> {
    let numerator = i64::from_be_bytes(
        reader
            .take(8)?
            .try_into()
            .map_err(|_| ContentError::Truncated)?,
    );
    let denominator = u64::from_be_bytes(
        reader
            .take(8)?
            .try_into()
            .map_err(|_| ContentError::Truncated)?,
    );
    super::ReferenceRationalPercent::new(numerator, denominator)
}

fn target_ordinal(
    target: &super::ReferenceItemTarget,
    definitions: &[super::ReferenceDefinition],
) -> Result<u32, ContentError> {
    let target = target.typed_ref()?;
    let position = definitions
        .binary_search_by(|definition| definition.definition.cmp(&target))
        .map_err(|_| ContentError::InvalidArtifact("dangling Reference Item target"))?;
    u32::try_from(position).map_err(|_| ContentError::InvalidSectionBounds)
}

fn target_from_ordinal(
    ordinal: u32,
    identities: &[IndexEntry],
) -> Result<super::ReferenceItemTarget, ContentError> {
    let entry = identities
        .get(usize::try_from(ordinal).map_err(|_| ContentError::InvalidSectionBounds)?)
        .ok_or(ContentError::InvalidArtifact(
            "dangling Reference Item target ordinal",
        ))?;
    super::ReferenceItemTarget::new(
        entry.identity.key().as_str(),
        entry.identity.revision().as_str(),
    )
}

fn put_target(
    bytes: &mut Vec<u8>,
    target: &super::ReferenceItemTarget,
    definitions: &[super::ReferenceDefinition],
) -> Result<(), ContentError> {
    put_u32(bytes, target_ordinal(target, definitions)?);
    Ok(())
}

fn read_target(
    reader: &mut SliceReader<'_>,
    identities: &[IndexEntry],
) -> Result<super::ReferenceItemTarget, ContentError> {
    target_from_ordinal(reader.read_u32()?, identities)
}

fn put_count(
    bytes: &mut Vec<u8>,
    actual: usize,
    limit: usize,
    resource: &'static str,
) -> Result<(), ContentError> {
    if actual > limit {
        return Err(ContentError::LimitExceeded {
            resource,
            actual,
            limit,
        });
    }
    bytes.push(u8::try_from(actual).map_err(|_| ContentError::InvalidSectionBounds)?);
    Ok(())
}

fn read_count(
    reader: &mut SliceReader<'_>,
    limit: usize,
    resource: &'static str,
) -> Result<usize, ContentError> {
    let actual = usize::from(reader.read_u8()?);
    if actual > limit {
        return Err(ContentError::LimitExceeded {
            resource,
            actual,
            limit,
        });
    }
    Ok(actual)
}

fn push_group(
    bytes: &mut Vec<u8>,
    group_id: u8,
    encode: impl FnOnce(&mut Vec<u8>) -> Result<(), ContentError>,
) -> Result<(), ContentError> {
    let mut payload = Vec::new();
    encode(&mut payload)?;
    bytes.push(group_id);
    put_u16(bytes, to_u16(payload.len())?);
    bytes.extend_from_slice(&payload);
    Ok(())
}

fn encode_typed_item(
    item: &super::ReferenceItemDefinition,
    projection: ReferenceArtifactProjection,
    definitions: &[super::ReferenceDefinition],
) -> Result<Vec<u8>, ContentError> {
    super::reference_playable::validate_item_definition(item)?;
    let semantics = if projection == ReferenceArtifactProjection::ClientSafe {
        item.semantics.client_projection()
    } else {
        item.semantics.clone()
    };
    let mut bytes = Vec::new();
    bytes.push(TYPED_BODY_RECORD_VERSION);
    bytes.push(physical_class_byte(item.physical_class));
    if projection == ReferenceArtifactProjection::ServerAuthoritative {
        put_bool(&mut bytes, item.materializable);
    }
    bytes.push(stack_class_byte(item.stack_class));
    if projection == ReferenceArtifactProjection::ServerAuthoritative {
        put_count(
            &mut bytes,
            item.legal_destinations.len(),
            1,
            "Reference Item legal destinations",
        )?;
        for destination in &item.legal_destinations {
            bytes.push(destination_byte(*destination));
        }
    }
    let count_offset = bytes.len();
    put_u16(&mut bytes, 0);
    let mut group_count = 0_u16;
    macro_rules! group {
        ($id:expr, $field:expr, $body:expr) => {
            if !matches!($field, super::ReferenceItemField::Unknown) {
                push_group(&mut bytes, $id, |payload| put_field(payload, $field, $body))?;
                group_count = group_count
                    .checked_add(1)
                    .ok_or(ContentError::InvalidSectionBounds)?;
            }
        };
    }
    group!(
        ITEM_GROUP_PRESENTATION,
        &semantics.presentation,
        |out, value| {
            put_field(out, &value.name, |out, text| {
                put_utf8(out, text, super::REFERENCE_ITEM_MAX_NAME_BYTES)
            })?;
            put_field(out, &value.description, |out, text| {
                put_utf8(out, text, super::REFERENCE_ITEM_MAX_DESCRIPTION_BYTES)
            })
        }
    );
    group!(
        ITEM_GROUP_CLASSIFICATION,
        &semantics.classification,
        |out, value| {
            put_field(out, &value.item_type, |out, item_type| {
                out.push(item_type.wire());
                Ok(())
            })?;
            put_field(out, &value.capabilities, |out, capabilities| {
                for capability in capabilities {
                    put_field(out, capability, |out, value| {
                        put_bool(out, *value);
                        Ok(())
                    })?;
                }
                Ok(())
            })
        }
    );
    group!(ITEM_GROUP_PHYSICAL, &semantics.physical, |out, value| {
        put_field(out, &value.weight, |out, value| {
            put_u32(out, *value);
            Ok(())
        })?;
        put_field(out, &value.movable, |out, value| {
            put_bool(out, *value);
            Ok(())
        })?;
        put_field(out, &value.pickupable, |out, value| {
            put_bool(out, *value);
            Ok(())
        })
    });
    group!(ITEM_GROUP_STACK, &semantics.stack, |out, value| {
        put_field(out, &value.stackable, |out, value| {
            put_bool(out, *value);
            Ok(())
        })?;
        put_field(out, &value.stack_max, |out, value| {
            put_u16(out, *value);
            Ok(())
        })
    });
    group!(ITEM_GROUP_EQUIPMENT, &semantics.equipment, |out, value| {
        put_field(out, &value.patterns, |out, patterns| {
            put_count(
                out,
                patterns.len(),
                super::REFERENCE_ITEM_MAX_EQUIPMENT_PATTERNS,
                "Reference Item Equipment patterns",
            )?;
            for pattern in patterns {
                out.push(pattern.pattern_id);
                put_field(out, &pattern.primary_slot, |out, value| {
                    out.push(value.wire());
                    Ok(())
                })?;
                put_field(out, &pattern.additional_reserved_slots, |out, values| {
                    put_count(
                        out,
                        values.len(),
                        super::REFERENCE_ITEM_MAX_ADDITIONAL_SLOTS,
                        "Reference Item Equipment additional slots",
                    )?;
                    out.extend(values.iter().map(|value| value.wire()));
                    Ok(())
                })?;
                put_field(out, &pattern.mutually_exclusive_groups, |out, values| {
                    put_count(
                        out,
                        values.len(),
                        super::REFERENCE_ITEM_MAX_EXCLUSIVE_GROUPS,
                        "Reference Item Equipment groups",
                    )?;
                    for value in values {
                        put_utf8(out, value.as_str(), MAX_KEY_BYTES)?;
                    }
                    Ok(())
                })?;
                put_field(out, &pattern.vocations, |out, values| {
                    put_count(
                        out,
                        values.len(),
                        super::REFERENCE_ITEM_MAX_BASE_VOCATIONS,
                        "Reference Item Equipment vocations",
                    )?;
                    out.extend(values.iter().map(|value| value.wire()));
                    Ok(())
                })?;
                put_field(out, &pattern.level, |out, value| {
                    put_u16(out, *value);
                    Ok(())
                })?;
                put_field(out, &pattern.compatibility_rule, |_out, _| {
                    Err(ContentError::InvalidArtifact(
                        "Reference Item Equipment compatibility grammar is unsupported in v1",
                    ))
                })?;
            }
            Ok(())
        })
    });
    group!(ITEM_GROUP_WEAPON, &semantics.weapon, |out, value| {
        encode_weapon(out, value)
    });
    group!(
        ITEM_GROUP_PROTECTION,
        &semantics.protection,
        encode_protection
    );
    group!(
        ITEM_GROUP_SKILL_MODIFIERS,
        &semantics.skill_modifiers,
        encode_skill_modifiers
    );
    group!(ITEM_GROUP_CHARGES, &semantics.charges, |out, value| {
        put_field(out, &value.count, |out, value| {
            put_u32(out, *value);
            Ok(())
        })
    });
    if projection == ReferenceArtifactProjection::ServerAuthoritative {
        group!(ITEM_GROUP_TEMPORAL, &semantics.temporal, |out, value| {
            put_field(out, &value.consumption_mode, |out, value| {
                out.push(value.wire());
                Ok(())
            })?;
            put_field(out, &value.duration, |out, value| {
                out.extend_from_slice(&value.0.to_be_bytes());
                Ok(())
            })?;
            put_field(out, &value.stop_duration, |out, value| {
                put_bool(out, *value);
                Ok(())
            })?;
            put_field(out, &value.decay_target, |out, value| {
                put_target(out, value, definitions)
            })
        });
    }
    group!(ITEM_GROUP_CONTAINER, &semantics.container, |out, value| {
        put_field(out, &value.capacity, |out, value| {
            put_u16(out, *value);
            Ok(())
        })
    });
    group!(ITEM_GROUP_IMBUEMENT, &semantics.imbuement, |out, value| {
        encode_imbuement(out, value)
    });
    if projection == ReferenceArtifactProjection::ServerAuthoritative {
        group!(
            ITEM_GROUP_USE_TRANSFORM,
            &semantics.use_transform,
            |out, value| {
                if value.targets.len() != 10 {
                    return Err(ContentError::InvalidArtifact(
                        "Reference Item UseTransform target count",
                    ));
                }
                for target in &value.targets {
                    put_field(out, &target.target, |out, value| {
                        put_target(out, value, definitions)
                    })?;
                }
                Ok(())
            }
        );
        group!(
            ITEM_GROUP_TRADE_RESTRICTIONS,
            &semantics.trade_restrictions,
            encode_trade
        );
        group!(ITEM_GROUP_FLUID, &semantics.fluid, |out, value| {
            put_field(out, &value.fluid_type, |out, value| {
                out.push(value.wire());
                Ok(())
            })
        });
        group!(
            ITEM_GROUP_READABLE_WRITEABLE,
            &semantics.readable_writeable,
            |out, value| {
                put_field(out, &value.readable, |out, value| {
                    put_bool(out, *value);
                    Ok(())
                })?;
                put_field(out, &value.writeable, |out, value| {
                    put_bool(out, *value);
                    Ok(())
                })?;
                put_field(out, &value.distance_read, |out, value| {
                    put_bool(out, *value);
                    Ok(())
                })?;
                put_field(out, &value.max_text_length, |out, value| {
                    put_u32(out, *value);
                    Ok(())
                })?;
                put_field(out, &value.write_once_target, |out, value| {
                    put_target(out, value, definitions)
                })
            }
        );
    }
    bytes[count_offset..count_offset + 2].copy_from_slice(&group_count.to_be_bytes());
    check_body_record_length(
        ReferenceArtifactProfile::TypedItemV4,
        projection,
        bytes.len(),
    )?;
    Ok(bytes)
}

fn encode_weapon(
    bytes: &mut Vec<u8>,
    value: &super::ReferenceItemWeapon,
) -> Result<(), ContentError> {
    put_field(bytes, &value.weapon_type, |out, value| {
        out.push(value.wire());
        Ok(())
    })?;
    for field in [&value.attack, &value.defense, &value.extra_defense] {
        put_field(bytes, field, |out, value| {
            out.extend_from_slice(&value.0.to_be_bytes());
            Ok(())
        })?;
    }
    put_field(bytes, &value.range, |out, value| {
        put_u16(out, value.0);
        Ok(())
    })?;
    put_field(bytes, &value.hit_chance, put_rational)?;
    put_field(bytes, &value.max_hit_chance, put_rational)?;
    put_field(bytes, &value.ammunition, |out, value| {
        out.push(value.wire());
        Ok(())
    })?;
    put_field(bytes, &value.elemental, |out, entries| {
        put_count(
            out,
            entries.len(),
            super::REFERENCE_ITEM_MAX_WEAPON_ELEMENTS,
            "Reference Item Weapon elements",
        )?;
        for entry in entries {
            out.push(entry.element.wire());
            put_field(out, &entry.points, |out, value| {
                out.extend_from_slice(&value.0.to_be_bytes());
                Ok(())
            })?;
        }
        Ok(())
    })
}

fn encode_protection(
    bytes: &mut Vec<u8>,
    value: &super::ReferenceItemProtection,
) -> Result<(), ContentError> {
    put_field(bytes, &value.armor, |out, value| {
        out.extend_from_slice(&value.0.to_be_bytes());
        Ok(())
    })?;
    put_field(bytes, &value.resistances, |out, entries| {
        put_count(
            out,
            entries.len(),
            super::REFERENCE_ITEM_MAX_RESISTANCES,
            "Reference Item resistances",
        )?;
        for entry in entries {
            out.push(entry.kind.wire());
            put_field(out, &entry.percent, put_rational)?;
        }
        Ok(())
    })
}

fn modifier_parameter_shape(kind: super::ReferenceSkillModifierKind) -> u8 {
    use super::ReferenceSkillModifierKind as Kind;
    match kind {
        Kind::Invisibility | Kind::ManaShield | Kind::SuppressDrown | Kind::SuppressDrunk => 1,
        Kind::CleavePercent
        | Kind::CriticalHitChance
        | Kind::CriticalHitDamage
        | Kind::LifeLeechAmount
        | Kind::LifeLeechChance
        | Kind::MagicShieldCapacityPercent
        | Kind::ManaLeechAmount
        | Kind::ManaLeechChance => 2,
        Kind::HealthTicks | Kind::ManaTicks => 3,
        Kind::PerfectShotRange => 4,
        Kind::ElementalBond => 5,
        _ => 6,
    }
}

fn encode_modifier_parameter(
    bytes: &mut Vec<u8>,
    kind: super::ReferenceSkillModifierKind,
    value: &super::ReferenceModifierParameter,
) -> Result<(), ContentError> {
    use super::ReferenceModifierParameter as Parameter;
    match (modifier_parameter_shape(kind), value) {
        (1, Parameter::Boolean(value)) => {
            put_bool(bytes, *value);
            Ok(())
        }
        (2, Parameter::RationalPercent(value)) => put_rational(bytes, value),
        (3, Parameter::Milliseconds(value)) => {
            bytes.extend_from_slice(&value.0.to_be_bytes());
            Ok(())
        }
        (4, Parameter::Cells(value)) => {
            put_u16(bytes, value.0);
            Ok(())
        }
        (5, Parameter::Element(value)) => {
            bytes.push(value.wire());
            Ok(())
        }
        (6, Parameter::SignedPoints(value)) => {
            bytes.extend_from_slice(&value.0.to_be_bytes());
            Ok(())
        }
        _ => Err(ContentError::InvalidArtifact(
            "Reference Item modifier parameter shape",
        )),
    }
}

fn encode_skill_modifiers(
    bytes: &mut Vec<u8>,
    value: &super::ReferenceItemSkillModifiers,
) -> Result<(), ContentError> {
    put_field(bytes, &value.modifiers, |out, entries| {
        put_count(
            out,
            entries.len(),
            super::REFERENCE_ITEM_MAX_MODIFIERS,
            "Reference Item SkillModifiers",
        )?;
        for entry in entries {
            out.push(entry.kind.wire());
            put_field(out, &entry.target_domain, |out, value| {
                out.push(*value);
                Ok(())
            })?;
            put_field(out, &entry.evaluation_phase, |out, value| {
                out.push(*value);
                Ok(())
            })?;
            put_field(out, &entry.priority, |out, value| {
                out.extend_from_slice(&value.to_be_bytes());
                Ok(())
            })?;
            put_field(out, &entry.parameter, |out, value| {
                encode_modifier_parameter(out, entry.kind, value)
            })?;
        }
        Ok(())
    })
}

fn encode_imbuement(
    bytes: &mut Vec<u8>,
    value: &super::ReferenceItemImbuement,
) -> Result<(), ContentError> {
    put_field(bytes, &value.slot_count, |out, value| {
        out.push(*value);
        Ok(())
    })?;
    put_field(bytes, &value.allowed_family_tiers, |out, entries| {
        put_count(
            out,
            entries.len(),
            super::REFERENCE_ITEM_MAX_IMBUEMENT_FAMILIES,
            "Reference Item imbuement allowances",
        )?;
        for entry in entries {
            out.push(entry.family.wire());
            out.push(match entry.tier {
                super::ReferenceImbuementTier::Two => 2,
                super::ReferenceImbuementTier::Three => 3,
                super::ReferenceImbuementTier::Ten => 10,
            });
        }
        Ok(())
    })?;
    put_field(bytes, &value.excluded_families, |out, entries| {
        put_count(
            out,
            entries.len(),
            super::REFERENCE_ITEM_MAX_IMBUEMENT_FAMILIES,
            "Reference Item excluded imbuement families",
        )?;
        out.extend(entries.iter().map(|entry| entry.wire()));
        Ok(())
    })
}

fn encode_trade(
    bytes: &mut Vec<u8>,
    value: &super::ReferenceItemTradeRestrictions,
) -> Result<(), ContentError> {
    put_field(bytes, &value.tradeable, |out, value| {
        put_bool(out, *value);
        Ok(())
    })?;
    put_field(bytes, &value.marketable, |out, value| {
        put_bool(out, *value);
        Ok(())
    })?;
    put_field(bytes, &value.vocations, |out, entries| {
        put_count(
            out,
            entries.len(),
            super::REFERENCE_ITEM_MAX_BASE_VOCATIONS,
            "Reference Item trade vocations",
        )?;
        out.extend(entries.iter().map(|entry| entry.wire()));
        Ok(())
    })?;
    put_field(bytes, &value.account_binding_policy, |_out, _| {
        Err(ContentError::InvalidArtifact(
            "Reference Item immutable account binding is unsupported in v1",
        ))
    })?;
    put_field(bytes, &value.character_binding_policy, |_out, _| {
        Err(ContentError::InvalidArtifact(
            "Reference Item immutable character binding is unsupported in v1",
        ))
    })
}

fn parse_typed_server_item(
    bytes: &[u8],
    identities: &[IndexEntry],
) -> Result<ReferenceServerItem, ContentError> {
    check_body_record_length(
        ReferenceArtifactProfile::TypedItemV4,
        ReferenceArtifactProjection::ServerAuthoritative,
        bytes.len(),
    )?;
    let mut reader = SliceReader::new(bytes);
    if reader.read_u8()? != TYPED_BODY_RECORD_VERSION {
        return Err(ContentError::InvalidArtifact(
            "unsupported typed Reference Item body version",
        ));
    }
    let physical_class = parse_physical_class(reader.read_u8()?)?;
    let materializable = read_bool(&mut reader)?;
    let stack_class = parse_stack_class(reader.read_u8()?)?;
    let destination_count = read_count(&mut reader, 1, "Reference Item legal destinations")?;
    let mut legal_destinations = Vec::with_capacity(destination_count);
    for _ in 0..destination_count {
        legal_destinations.push(parse_destination(reader.read_u8()?)?);
    }
    let semantics = decode_typed_groups(
        &mut reader,
        ReferenceArtifactProjection::ServerAuthoritative,
        identities,
    )?;
    reader.ensure_end()?;
    let definition = super::ReferenceItemDefinition {
        physical_class,
        materializable,
        stack_class,
        legal_destinations: legal_destinations.clone(),
        semantics: semantics.clone(),
    };
    super::reference_playable::validate_item_definition(&definition)?;
    Ok(ReferenceServerItem {
        physical_class,
        materializable,
        stack_class,
        legal_destinations,
        semantics,
    })
}

fn parse_typed_client_item(
    bytes: &[u8],
    identities: &[IndexEntry],
) -> Result<ReferenceClientItem, ContentError> {
    check_body_record_length(
        ReferenceArtifactProfile::TypedItemV4,
        ReferenceArtifactProjection::ClientSafe,
        bytes.len(),
    )?;
    let mut reader = SliceReader::new(bytes);
    if reader.read_u8()? != TYPED_BODY_RECORD_VERSION {
        return Err(ContentError::InvalidArtifact(
            "unsupported typed Reference Item body version",
        ));
    }
    let physical_class = parse_physical_class(reader.read_u8()?)?;
    let stack_class = parse_stack_class(reader.read_u8()?)?;
    let semantics = decode_typed_groups(
        &mut reader,
        ReferenceArtifactProjection::ClientSafe,
        identities,
    )?;
    reader.ensure_end()?;
    let definition = super::ReferenceItemDefinition {
        physical_class,
        materializable: false,
        stack_class,
        legal_destinations: Vec::new(),
        semantics: semantics.clone(),
    };
    super::reference_playable::validate_item_definition(&definition)?;
    Ok(ReferenceClientItem {
        physical_class,
        stack_class,
        semantics,
    })
}

fn decode_typed_groups(
    reader: &mut SliceReader<'_>,
    projection: ReferenceArtifactProjection,
    identities: &[IndexEntry],
) -> Result<super::ReferenceItemSemantics, ContentError> {
    let count = usize::from(reader.read_u16()?);
    let limit = if projection == ReferenceArtifactProjection::ClientSafe {
        11
    } else {
        16
    };
    if count > limit {
        return Err(ContentError::LimitExceeded {
            resource: "Reference Item groups",
            actual: count,
            limit,
        });
    }
    let mut semantics = super::ReferenceItemSemantics::default();
    let mut previous = 0_u8;
    for _ in 0..count {
        let group_id = reader.read_u8()?;
        let length = usize::from(reader.read_u16()?);
        let allowed = projection == ReferenceArtifactProjection::ServerAuthoritative
            || CLIENT_ITEM_GROUPS.contains(&group_id);
        if !allowed || !(1..=16).contains(&group_id) || group_id <= previous {
            return Err(ContentError::InvalidArtifact(
                "Reference Item group is unknown, duplicated, unordered or excluded from projection",
            ));
        }
        let mut payload = SliceReader::new(reader.take(length)?);
        match group_id {
            ITEM_GROUP_PRESENTATION => {
                semantics.presentation = read_field(&mut payload, |source| {
                    Ok(super::ReferenceItemPresentation {
                        name: read_field(source, |source| {
                            read_utf8(source, super::REFERENCE_ITEM_MAX_NAME_BYTES)
                        })?,
                        description: read_field(source, |source| {
                            read_utf8(source, super::REFERENCE_ITEM_MAX_DESCRIPTION_BYTES)
                        })?,
                    })
                })?
            }
            ITEM_GROUP_CLASSIFICATION => {
                semantics.classification = read_field(&mut payload, |source| {
                    let item_type = read_field(source, |source| {
                        super::ReferenceItemType::from_wire(source.read_u8()?)
                    })?;
                    let capabilities = read_field(source, |source| {
                        let mut capabilities = Vec::with_capacity(24);
                        for _ in 0..24 {
                            capabilities.push(read_field(source, read_bool)?);
                        }
                        capabilities.try_into().map_err(|_| {
                            ContentError::InvalidArtifact("Reference Item capability count")
                        })
                    })?;
                    Ok(super::ReferenceItemClassification {
                        item_type,
                        capabilities,
                    })
                })?
            }
            ITEM_GROUP_PHYSICAL => {
                semantics.physical = read_field(&mut payload, |source| {
                    Ok(super::ReferenceItemPhysical {
                        weight: read_field(source, |source| source.read_u32())?,
                        movable: read_field(source, read_bool)?,
                        pickupable: read_field(source, read_bool)?,
                    })
                })?
            }
            ITEM_GROUP_STACK => {
                semantics.stack = read_field(&mut payload, |source| {
                    Ok(super::ReferenceItemStack {
                        stackable: read_field(source, read_bool)?,
                        stack_max: read_field(source, |source| source.read_u16())?,
                    })
                })?
            }
            ITEM_GROUP_EQUIPMENT => {
                semantics.equipment = read_field(&mut payload, decode_equipment)?
            }
            ITEM_GROUP_WEAPON => semantics.weapon = read_field(&mut payload, decode_weapon)?,
            ITEM_GROUP_PROTECTION => {
                semantics.protection = read_field(&mut payload, decode_protection)?
            }
            ITEM_GROUP_SKILL_MODIFIERS => {
                semantics.skill_modifiers = read_field(&mut payload, decode_skill_modifiers)?
            }
            ITEM_GROUP_CHARGES => {
                semantics.charges = read_field(&mut payload, |source| {
                    Ok(super::ReferenceItemCharges {
                        count: read_field(source, |source| source.read_u32())?,
                    })
                })?
            }
            ITEM_GROUP_TEMPORAL => {
                semantics.temporal = read_field(&mut payload, |source| {
                    Ok(super::ReferenceItemTemporal {
                        consumption_mode: read_field(source, |source| {
                            super::ReferenceTemporalMode::from_wire(source.read_u8()?)
                        })?,
                        duration: read_field(source, |source| {
                            Ok(super::ReferenceMilliseconds(source.read_u64()?))
                        })?,
                        stop_duration: read_field(source, read_bool)?,
                        decay_target: read_field(source, |source| read_target(source, identities))?,
                    })
                })?
            }
            ITEM_GROUP_CONTAINER => {
                semantics.container = read_field(&mut payload, |source| {
                    Ok(super::ReferenceItemContainer {
                        capacity: read_field(source, |source| source.read_u16())?,
                    })
                })?
            }
            ITEM_GROUP_IMBUEMENT => {
                semantics.imbuement = read_field(&mut payload, decode_imbuement)?
            }
            ITEM_GROUP_USE_TRANSFORM => {
                semantics.use_transform = read_field(&mut payload, |source| {
                    let mut targets = Vec::with_capacity(10);
                    for kind in 1..=10 {
                        targets.push(super::ReferenceTransformTarget {
                            kind: super::ReferenceTransformKind::from_wire(kind)?,
                            target: read_field(source, |source| read_target(source, identities))?,
                        });
                    }
                    Ok(super::ReferenceItemUseTransform { targets })
                })?
            }
            ITEM_GROUP_TRADE_RESTRICTIONS => {
                semantics.trade_restrictions = read_field(&mut payload, decode_trade)?
            }
            ITEM_GROUP_FLUID => {
                semantics.fluid = read_field(&mut payload, |source| {
                    Ok(super::ReferenceItemFluid {
                        fluid_type: read_field(source, |source| {
                            super::ReferenceFluidType::from_wire(source.read_u8()?)
                        })?,
                    })
                })?
            }
            ITEM_GROUP_READABLE_WRITEABLE => {
                semantics.readable_writeable = read_field(&mut payload, |source| {
                    Ok(super::ReferenceItemReadableWriteable {
                        readable: read_field(source, read_bool)?,
                        writeable: read_field(source, read_bool)?,
                        distance_read: read_field(source, read_bool)?,
                        max_text_length: read_field(source, |source| source.read_u32())?,
                        write_once_target: read_field(source, |source| {
                            read_target(source, identities)
                        })?,
                    })
                })?
            }
            _ => {
                return Err(ContentError::InvalidArtifact(
                    "unknown Reference Item group",
                ));
            }
        }
        payload.ensure_end()?;
        previous = group_id;
    }
    Ok(semantics)
}

fn decode_equipment(
    source: &mut SliceReader<'_>,
) -> Result<super::ReferenceItemEquipment, ContentError> {
    Ok(super::ReferenceItemEquipment {
        patterns: read_field(source, |source| {
            let count = read_count(
                source,
                super::REFERENCE_ITEM_MAX_EQUIPMENT_PATTERNS,
                "Reference Item Equipment patterns",
            )?;
            let mut patterns = Vec::with_capacity(count);
            for _ in 0..count {
                let pattern_id = source.read_u8()?;
                let primary_slot = read_field(source, |source| {
                    super::ReferenceEquipmentSlot::from_wire(source.read_u8()?)
                })?;
                let additional_reserved_slots = read_field(source, |source| {
                    let count = read_count(
                        source,
                        super::REFERENCE_ITEM_MAX_ADDITIONAL_SLOTS,
                        "Reference Item Equipment additional slots",
                    )?;
                    let mut values = Vec::with_capacity(count);
                    for _ in 0..count {
                        values.push(super::ReferenceEquipmentSlot::from_wire(source.read_u8()?)?);
                    }
                    Ok(values)
                })?;
                let mutually_exclusive_groups = read_field(source, |source| {
                    let count = read_count(
                        source,
                        super::REFERENCE_ITEM_MAX_EXCLUSIVE_GROUPS,
                        "Reference Item Equipment groups",
                    )?;
                    let mut values = Vec::with_capacity(count);
                    for _ in 0..count {
                        values.push(super::ReferenceItemGroupKey::new(&read_utf8(
                            source,
                            MAX_KEY_BYTES,
                        )?)?);
                    }
                    Ok(values)
                })?;
                let vocations = read_field(source, |source| {
                    let count = read_count(
                        source,
                        super::REFERENCE_ITEM_MAX_BASE_VOCATIONS,
                        "Reference Item Equipment vocations",
                    )?;
                    let mut values = Vec::with_capacity(count);
                    for _ in 0..count {
                        values.push(super::ReferenceBaseVocation::from_wire(source.read_u8()?)?);
                    }
                    Ok(values)
                })?;
                let level = read_field(source, |source| source.read_u16())?;
                let compatibility_rule = read_field(source, |_source| {
                    Err(ContentError::InvalidArtifact(
                        "Reference Item Equipment compatibility grammar is unsupported in v1",
                    ))
                })?;
                patterns.push(super::ReferenceEquipmentPattern {
                    pattern_id,
                    primary_slot,
                    additional_reserved_slots,
                    mutually_exclusive_groups,
                    vocations,
                    level,
                    compatibility_rule,
                });
            }
            Ok(patterns)
        })?,
    })
}

fn read_i32(source: &mut SliceReader<'_>) -> Result<i32, ContentError> {
    Ok(i32::from_be_bytes(
        source
            .take(4)?
            .try_into()
            .map_err(|_| ContentError::Truncated)?,
    ))
}

fn read_i16(source: &mut SliceReader<'_>) -> Result<i16, ContentError> {
    Ok(i16::from_be_bytes(
        source
            .take(2)?
            .try_into()
            .map_err(|_| ContentError::Truncated)?,
    ))
}

fn decode_weapon(source: &mut SliceReader<'_>) -> Result<super::ReferenceItemWeapon, ContentError> {
    let weapon_type = read_field(source, |source| {
        super::ReferenceWeaponType::from_wire(source.read_u8()?)
    })?;
    let attack = read_field(source, |source| {
        Ok(super::ReferenceSignedPoints(read_i32(source)?))
    })?;
    let defense = read_field(source, |source| {
        Ok(super::ReferenceSignedPoints(read_i32(source)?))
    })?;
    let extra_defense = read_field(source, |source| {
        Ok(super::ReferenceSignedPoints(read_i32(source)?))
    })?;
    let range = read_field(source, |source| {
        Ok(super::ReferenceCells(source.read_u16()?))
    })?;
    let hit_chance = read_field(source, read_rational)?;
    let max_hit_chance = read_field(source, read_rational)?;
    let ammunition = read_field(source, |source| {
        super::ReferenceAmmoType::from_wire(source.read_u8()?)
    })?;
    let elemental = read_field(source, |source| {
        let count = read_count(
            source,
            super::REFERENCE_ITEM_MAX_WEAPON_ELEMENTS,
            "Reference Item Weapon elements",
        )?;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            entries.push(super::ReferenceElementalAttack {
                element: super::ReferenceWeaponElement::from_wire(source.read_u8()?)?,
                points: read_field(source, |source| {
                    Ok(super::ReferenceSignedPoints(read_i32(source)?))
                })?,
            });
        }
        Ok(entries)
    })?;
    Ok(super::ReferenceItemWeapon {
        weapon_type,
        attack,
        defense,
        extra_defense,
        range,
        hit_chance,
        max_hit_chance,
        ammunition,
        elemental,
    })
}

fn decode_protection(
    source: &mut SliceReader<'_>,
) -> Result<super::ReferenceItemProtection, ContentError> {
    let armor = read_field(source, |source| {
        Ok(super::ReferenceSignedPoints(read_i32(source)?))
    })?;
    let resistances = read_field(source, |source| {
        let count = read_count(
            source,
            super::REFERENCE_ITEM_MAX_RESISTANCES,
            "Reference Item resistances",
        )?;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            entries.push(super::ReferenceResistance {
                kind: super::ReferenceResistanceKind::from_wire(source.read_u8()?)?,
                percent: read_field(source, read_rational)?,
            });
        }
        Ok(entries)
    })?;
    Ok(super::ReferenceItemProtection { armor, resistances })
}

fn decode_modifier_parameter(
    source: &mut SliceReader<'_>,
    kind: super::ReferenceSkillModifierKind,
) -> Result<super::ReferenceModifierParameter, ContentError> {
    use super::ReferenceModifierParameter as Parameter;
    Ok(match modifier_parameter_shape(kind) {
        1 => Parameter::Boolean(read_bool(source)?),
        2 => Parameter::RationalPercent(read_rational(source)?),
        3 => Parameter::Milliseconds(super::ReferenceMilliseconds(source.read_u64()?)),
        4 => Parameter::Cells(super::ReferenceCells(source.read_u16()?)),
        5 => Parameter::Element(super::ReferenceModifierElement::from_wire(
            source.read_u8()?,
        )?),
        6 => Parameter::SignedPoints(super::ReferenceSignedPoints(read_i32(source)?)),
        _ => {
            return Err(ContentError::InvalidArtifact(
                "Reference Item modifier parameter shape",
            ));
        }
    })
}

fn decode_skill_modifiers(
    source: &mut SliceReader<'_>,
) -> Result<super::ReferenceItemSkillModifiers, ContentError> {
    Ok(super::ReferenceItemSkillModifiers {
        modifiers: read_field(source, |source| {
            let count = read_count(
                source,
                super::REFERENCE_ITEM_MAX_MODIFIERS,
                "Reference Item SkillModifiers",
            )?;
            let mut entries = Vec::with_capacity(count);
            for _ in 0..count {
                let kind = super::ReferenceSkillModifierKind::from_wire(source.read_u8()?)?;
                entries.push(super::ReferenceModifierBinding {
                    kind,
                    target_domain: read_field(source, |source| source.read_u8())?,
                    evaluation_phase: read_field(source, |source| source.read_u8())?,
                    priority: read_field(source, read_i16)?,
                    parameter: read_field(source, |source| {
                        decode_modifier_parameter(source, kind)
                    })?,
                });
            }
            Ok(entries)
        })?,
    })
}

fn decode_imbuement(
    source: &mut SliceReader<'_>,
) -> Result<super::ReferenceItemImbuement, ContentError> {
    let slot_count = read_field(source, |source| source.read_u8())?;
    let allowed_family_tiers = read_field(source, |source| {
        let count = read_count(
            source,
            super::REFERENCE_ITEM_MAX_IMBUEMENT_FAMILIES,
            "Reference Item imbuement allowances",
        )?;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let family = super::ReferenceImbuementFamily::from_wire(source.read_u8()?)?;
            let tier = match source.read_u8()? {
                2 => super::ReferenceImbuementTier::Two,
                3 => super::ReferenceImbuementTier::Three,
                10 => super::ReferenceImbuementTier::Ten,
                _ => {
                    return Err(ContentError::InvalidArtifact(
                        "unknown Reference Item imbuement tier",
                    ));
                }
            };
            entries.push(super::ReferenceImbuementAllowance { family, tier });
        }
        Ok(entries)
    })?;
    let excluded_families = read_field(source, |source| {
        let count = read_count(
            source,
            super::REFERENCE_ITEM_MAX_IMBUEMENT_FAMILIES,
            "Reference Item excluded imbuement families",
        )?;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            entries.push(super::ReferenceImbuementFamily::from_wire(
                source.read_u8()?,
            )?);
        }
        Ok(entries)
    })?;
    Ok(super::ReferenceItemImbuement {
        slot_count,
        allowed_family_tiers,
        excluded_families,
    })
}

fn decode_trade(
    source: &mut SliceReader<'_>,
) -> Result<super::ReferenceItemTradeRestrictions, ContentError> {
    let tradeable = read_field(source, read_bool)?;
    let marketable = read_field(source, read_bool)?;
    let vocations = read_field(source, |source| {
        let count = read_count(
            source,
            super::REFERENCE_ITEM_MAX_BASE_VOCATIONS,
            "Reference Item trade vocations",
        )?;
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(super::ReferenceBaseVocation::from_wire(source.read_u8()?)?);
        }
        Ok(values)
    })?;
    let account_binding_policy = read_field(source, |_source| {
        Err(ContentError::InvalidArtifact(
            "Reference Item immutable account binding is unsupported in v1",
        ))
    })?;
    let character_binding_policy = read_field(source, |_source| {
        Err(ContentError::InvalidArtifact(
            "Reference Item immutable character binding is unsupported in v1",
        ))
    })?;
    Ok(super::ReferenceItemTradeRestrictions {
        tradeable,
        marketable,
        vocations,
        account_binding_policy,
        character_binding_policy,
    })
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
        check_body_record_length(profile, metadata.projection, record.body.len())?;
        body.extend_from_slice(&record.body);
    }
    let manifest = encode_manifest(metadata, profile)?;
    let index = encode_index(profile, metadata.projection, records)?;
    check_section_length(profile, SECTION_MANIFEST, manifest.len())?;
    check_section_length(profile, SECTION_INDEX, index.len())?;
    check_section_length(profile, SECTION_BODY, body.len())?;
    check_body_section_length(profile, metadata.projection, body.len())?;

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
    projection: ReferenceArtifactProjection,
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
        check_body_record_length(profile, projection, record.body.len())?;
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
    projection: ReferenceArtifactProjection,
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
        check_body_record_length(profile, projection, body_length)?;
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

fn check_body_record_length(
    profile: ReferenceArtifactProfile,
    projection: ReferenceArtifactProjection,
    actual: usize,
) -> Result<(), ContentError> {
    let limit = profile.body_record_limit(projection);
    if actual > limit {
        return Err(ContentError::LimitExceeded {
            resource: "Reference artifact body record bytes",
            actual,
            limit,
        });
    }
    Ok(())
}

fn check_body_section_length(
    profile: ReferenceArtifactProfile,
    projection: ReferenceArtifactProjection,
    actual: usize,
) -> Result<(), ContentError> {
    let limit = if profile.is_typed() {
        match projection {
            ReferenceArtifactProjection::ServerAuthoritative => {
                TYPED_REFERENCE_ITEM_MAX_SERVER_BODY_BYTES
            }
            ReferenceArtifactProjection::ClientSafe => TYPED_REFERENCE_ITEM_MAX_CLIENT_BODY_BYTES,
        }
    } else {
        profile.max_body_bytes()
    };
    if actual > limit {
        return Err(ContentError::LimitExceeded {
            resource: "Reference artifact body section bytes",
            actual,
            limit,
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

    fn read_u64(&mut self) -> Result<u64, ContentError> {
        let raw: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| ContentError::Truncated)?;
        Ok(u64::from_be_bytes(raw))
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

#[cfg(test)]
mod typed_item_codec_tests {
    use super::*;
    use crate::content::*;

    fn identity() -> Result<TypedDefinitionRef, ContentError> {
        Ok(TypedDefinitionRef::new(
            DefinitionFamily::Item,
            ProductionKey::new("oteryn:item.schema.boundary-witness")?,
            DefinitionRevisionRef::new("definition-r1")?,
        ))
    }

    fn source_definition(
        item: ReferenceItemDefinition,
    ) -> Result<ReferenceDefinition, ContentError> {
        Ok(ReferenceDefinition {
            definition: identity()?,
            kind: ReferenceDefinitionKind::Item(item),
            client_projection: ClientProjectionClass::ClientSafe,
        })
    }

    fn worst_definitions(
        item: ReferenceItemDefinition,
    ) -> Result<Vec<ReferenceDefinition>, ContentError> {
        let identity_only = ReferenceItemDefinition {
            physical_class: ReferenceItemPhysicalClass::Unknown,
            materializable: false,
            stack_class: ReferenceItemStackClass::Unknown,
            legal_destinations: Vec::new(),
            semantics: Default::default(),
        };
        let mut definitions = (0..REFERENCE_ITEM_REGISTRY_SIZE - 1)
            .map(|ordinal| {
                Ok(ReferenceDefinition {
                    definition: TypedDefinitionRef::new(
                        DefinitionFamily::Item,
                        ProductionKey::new(&format!("oteryn:item.schema.boundary-{ordinal:05}"))?,
                        DefinitionRevisionRef::new("definition-r1")?,
                    ),
                    kind: ReferenceDefinitionKind::Item(identity_only.clone()),
                    client_projection: ClientProjectionClass::ClientSafe,
                })
            })
            .collect::<Result<Vec<_>, ContentError>>()?;
        definitions.push(source_definition(item)?);
        Ok(definitions)
    }

    fn target() -> Result<ReferenceItemTarget, ContentError> {
        ReferenceItemTarget::new("oteryn:item.schema.boundary-witness", "definition-r1")
    }

    fn hex_bytes(value: &str) -> Result<Vec<u8>, ContentError> {
        fn nibble(value: u8) -> Result<u8, ContentError> {
            match value {
                b'0'..=b'9' => Ok(value - b'0'),
                b'a'..=b'f' => Ok(value - b'a' + 10),
                _ => Err(ContentError::InvalidArtifact("invalid fixture hex")),
            }
        }

        let chunks = value.as_bytes().chunks_exact(2);
        if !chunks.remainder().is_empty() {
            return Err(ContentError::InvalidArtifact("odd fixture hex length"));
        }
        chunks
            .map(|pair| Ok((nibble(pair[0])? << 4) | nibble(pair[1])?))
            .collect()
    }

    fn oracle_weapon(distance: bool) -> Result<ReferenceItemDefinition, ContentError> {
        use ReferenceItemField::{Known, NotApplicable};
        let elements = (1..=5)
            .map(|wire| {
                Ok(ReferenceElementalAttack {
                    element: ReferenceWeaponElement::from_wire(wire)?,
                    points: Known(ReferenceSignedPoints(0)),
                })
            })
            .collect::<Result<Vec<_>, ContentError>>()?;
        Ok(ReferenceItemDefinition {
            physical_class: ReferenceItemPhysicalClass::Physical,
            materializable: true,
            stack_class: ReferenceItemStackClass::StackCapable,
            legal_destinations: vec![ReferenceItemDestination::CharacterInventory],
            semantics: ReferenceItemSemantics {
                weapon: Known(ReferenceItemWeapon {
                    weapon_type: Known(if distance {
                        ReferenceWeaponType::Distance
                    } else {
                        ReferenceWeaponType::Sword
                    }),
                    attack: Known(ReferenceSignedPoints(if distance { 35 } else { 42 })),
                    defense: Known(ReferenceSignedPoints(0)),
                    extra_defense: Known(ReferenceSignedPoints(0)),
                    range: Known(ReferenceCells(if distance { 7 } else { 1 })),
                    hit_chance: Known(ReferenceRationalPercent::new(
                        if distance { 9 } else { 0 },
                        if distance { 10 } else { 1 },
                    )?),
                    max_hit_chance: Known(ReferenceRationalPercent::new(
                        if distance { 1 } else { 0 },
                        1,
                    )?),
                    ammunition: if distance {
                        Known(ReferenceAmmoType::Arrow)
                    } else {
                        NotApplicable
                    },
                    elemental: Known(elements),
                }),
                ..Default::default()
            },
        })
    }

    fn group_keys() -> Result<Vec<ReferenceItemGroupKey>, ContentError> {
        let prefix = "oteryn:equipment-group.";
        (0..2)
            .map(|index| {
                let suffix = char::from(b'a' + index);
                ReferenceItemGroupKey::new(&format!(
                    "{prefix}{}{suffix}",
                    "a".repeat(MAX_KEY_BYTES - prefix.len() - 1)
                ))
            })
            .collect()
    }

    fn modifier_parameter(
        kind: ReferenceSkillModifierKind,
    ) -> Result<ReferenceModifierParameter, ContentError> {
        Ok(match modifier_parameter_shape(kind) {
            1 => ReferenceModifierParameter::Boolean(false),
            2 => ReferenceModifierParameter::RationalPercent(ReferenceRationalPercent::new(0, 1)?),
            3 => ReferenceModifierParameter::Milliseconds(ReferenceMilliseconds(0)),
            4 => ReferenceModifierParameter::Cells(ReferenceCells(0)),
            5 => ReferenceModifierParameter::Element(ReferenceModifierElement::Death),
            6 => ReferenceModifierParameter::SignedPoints(ReferenceSignedPoints(0)),
            _ => unreachable!(),
        })
    }

    fn worst_item() -> Result<ReferenceItemDefinition, ContentError> {
        use ReferenceItemField::{Known, Unknown};
        let target = target()?;
        let capabilities = std::array::from_fn(|index| Known(index % 2 == 0));
        let elements = (1..=5)
            .map(|wire| {
                Ok(ReferenceElementalAttack {
                    element: ReferenceWeaponElement::from_wire(wire)?,
                    points: Known(ReferenceSignedPoints(0)),
                })
            })
            .collect::<Result<Vec<_>, ContentError>>()?;
        let resistances = (1..=12)
            .map(|wire| {
                Ok(ReferenceResistance {
                    kind: ReferenceResistanceKind::from_wire(wire)?,
                    percent: Known(ReferenceRationalPercent::new(0, 1)?),
                })
            })
            .collect::<Result<Vec<_>, ContentError>>()?;
        let modifiers = (1..=37)
            .map(|wire| {
                let kind = ReferenceSkillModifierKind::from_wire(wire)?;
                Ok(ReferenceModifierBinding {
                    kind,
                    target_domain: Known(wire),
                    evaluation_phase: Known(wire),
                    priority: Known(0),
                    parameter: Known(modifier_parameter(kind)?),
                })
            })
            .collect::<Result<Vec<_>, ContentError>>()?;
        let allowances = (1..=20)
            .map(|wire| {
                Ok(ReferenceImbuementAllowance {
                    family: ReferenceImbuementFamily::from_wire(wire)?,
                    tier: ReferenceImbuementTier::Three,
                })
            })
            .collect::<Result<Vec<_>, ContentError>>()?;
        let excluded = (1..=20)
            .map(ReferenceImbuementFamily::from_wire)
            .collect::<Result<Vec<_>, ContentError>>()?;
        let patterns = (1..=2)
            .map(|pattern_id| {
                let primary_slot = ReferenceEquipmentSlot::from_wire(pattern_id)?;
                let additional_reserved_slots = (1..=10)
                    .map(ReferenceEquipmentSlot::from_wire)
                    .collect::<Result<Vec<_>, ContentError>>()?
                    .into_iter()
                    .filter(|slot| *slot != primary_slot)
                    .collect();
                Ok(ReferenceEquipmentPattern {
                    pattern_id,
                    primary_slot: Known(primary_slot),
                    additional_reserved_slots: Known(additional_reserved_slots),
                    mutually_exclusive_groups: Known(group_keys()?),
                    vocations: Known(
                        (1..=5)
                            .map(ReferenceBaseVocation::from_wire)
                            .collect::<Result<Vec<_>, _>>()?,
                    ),
                    level: Known(0),
                    compatibility_rule: Unknown,
                })
            })
            .collect::<Result<Vec<_>, ContentError>>()?;
        let transforms = (1..=10)
            .map(|wire| {
                Ok(ReferenceTransformTarget {
                    kind: ReferenceTransformKind::from_wire(wire)?,
                    target: Known(target.clone()),
                })
            })
            .collect::<Result<Vec<_>, ContentError>>()?;
        Ok(ReferenceItemDefinition {
            physical_class: ReferenceItemPhysicalClass::Physical,
            materializable: true,
            stack_class: ReferenceItemStackClass::StackCapable,
            legal_destinations: vec![ReferenceItemDestination::CharacterInventory],
            semantics: ReferenceItemSemantics {
                presentation: Known(ReferenceItemPresentation {
                    name: Known("N".repeat(REFERENCE_ITEM_MAX_NAME_BYTES)),
                    description: Known("D".repeat(REFERENCE_ITEM_MAX_DESCRIPTION_BYTES)),
                }),
                classification: Known(ReferenceItemClassification {
                    item_type: Known(ReferenceItemType::Bed),
                    capabilities: Known(capabilities),
                }),
                physical: Known(ReferenceItemPhysical {
                    weight: Known(0),
                    movable: Known(false),
                    pickupable: Known(true),
                }),
                stack: Known(ReferenceItemStack {
                    stackable: Known(true),
                    stack_max: Known(100),
                }),
                equipment: Known(ReferenceItemEquipment {
                    patterns: Known(patterns),
                }),
                weapon: Known(ReferenceItemWeapon {
                    weapon_type: Known(ReferenceWeaponType::Ammunition),
                    attack: Known(ReferenceSignedPoints(0)),
                    defense: Known(ReferenceSignedPoints(0)),
                    extra_defense: Known(ReferenceSignedPoints(0)),
                    range: Known(ReferenceCells(0)),
                    hit_chance: Known(ReferenceRationalPercent::new(0, 1)?),
                    max_hit_chance: Known(ReferenceRationalPercent::new(0, 1)?),
                    ammunition: Known(ReferenceAmmoType::Arrow),
                    elemental: Known(elements),
                }),
                protection: Known(ReferenceItemProtection {
                    armor: Known(ReferenceSignedPoints(0)),
                    resistances: Known(resistances),
                }),
                skill_modifiers: Known(ReferenceItemSkillModifiers {
                    modifiers: Known(modifiers),
                }),
                charges: Known(ReferenceItemCharges { count: Known(0) }),
                temporal: Known(ReferenceItemTemporal {
                    consumption_mode: Known(ReferenceTemporalMode::DurableAbsoluteDeadline),
                    duration: Known(ReferenceMilliseconds(0)),
                    stop_duration: Known(false),
                    decay_target: Known(target.clone()),
                }),
                container: Known(ReferenceItemContainer { capacity: Known(0) }),
                imbuement: Known(ReferenceItemImbuement {
                    slot_count: Known(3),
                    allowed_family_tiers: Known(allowances),
                    excluded_families: Known(excluded),
                }),
                use_transform: Known(ReferenceItemUseTransform {
                    targets: transforms,
                }),
                trade_restrictions: Known(ReferenceItemTradeRestrictions {
                    tradeable: Known(false),
                    marketable: Known(false),
                    vocations: Known(
                        (1..=5)
                            .map(ReferenceBaseVocation::from_wire)
                            .collect::<Result<Vec<_>, _>>()?,
                    ),
                    account_binding_policy: Unknown,
                    character_binding_policy: Unknown,
                }),
                fluid: Known(ReferenceItemFluid {
                    fluid_type: Known(ReferenceFluidType::Beer),
                }),
                readable_writeable: Known(ReferenceItemReadableWriteable {
                    readable: Known(true),
                    writeable: Known(true),
                    distance_read: Known(true),
                    max_text_length: Known(0),
                    write_once_target: Known(target),
                }),
            },
        })
    }

    #[test]
    fn typed_body_matches_accepted_exact_record_maxima_and_round_trips() -> Result<(), ContentError>
    {
        let item = worst_item()?;
        let definitions = worst_definitions(item.clone())?;
        let server = encode_typed_item(
            &item,
            ReferenceArtifactProjection::ServerAuthoritative,
            &definitions,
        )?;
        let client =
            encode_typed_item(&item, ReferenceArtifactProjection::ClientSafe, &definitions)?;
        assert_eq!(server.len(), TYPED_REFERENCE_ITEM_MAX_SERVER_RECORD_BYTES);
        assert_eq!(client.len(), TYPED_REFERENCE_ITEM_MAX_CLIENT_RECORD_BYTES);
        assert_eq!(
            sha256(&server).as_slice(),
            hex_bytes("19c38b3cd18c7e2765aefb9ec2c7b0da9a31be66eaa30b84c63a7cf9ee132fc5")?
        );
        assert_eq!(
            sha256(&client).as_slice(),
            hex_bytes("0f9f25815a61fa292d6a74f89974f8b4bbefcca3d70a19ee023d730e88261e21")?
        );
        assert_eq!(
            encode_typed_item(
                &item,
                ReferenceArtifactProjection::ServerAuthoritative,
                &definitions
            )?,
            server
        );
        let index = definitions
            .iter()
            .map(|definition| IndexEntry {
                identity: definition.definition.clone(),
                body_offset: 0,
                body_length: server.len(),
                body_digest: sha256(&server),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            parse_typed_server_item(&server, &index)?.semantics,
            item.semantics
        );
        assert_eq!(
            parse_typed_client_item(&client, &index)?.semantics,
            item.semantics.client_projection()
        );
        let mut over = server;
        over.push(0);
        assert!(matches!(
            parse_typed_server_item(&over, &index),
            Err(ContentError::LimitExceeded {
                actual: 3_556,
                limit: 3_555,
                ..
            })
        ));
        let mut over = client;
        over.push(0);
        assert!(matches!(
            parse_typed_client_item(&over, &index),
            Err(ContentError::LimitExceeded {
                actual: 3_434,
                limit: 3_433,
                ..
            })
        ));
        Ok(())
    }

    #[test]
    fn typed_resource_limits_accept_exact_max_and_reject_max_plus_one() {
        let profile = ReferenceArtifactProfile::TypedItemV4;
        for (projection, body, artifact) in [
            (
                ReferenceArtifactProjection::ServerAuthoritative,
                TYPED_REFERENCE_ITEM_MAX_SERVER_BODY_BYTES,
                TYPED_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES,
            ),
            (
                ReferenceArtifactProjection::ClientSafe,
                TYPED_REFERENCE_ITEM_MAX_CLIENT_BODY_BYTES,
                TYPED_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES,
            ),
        ] {
            assert!(check_body_section_length(profile, projection, body).is_ok());
            assert!(check_body_section_length(profile, projection, body + 1).is_err());
            assert!(check_artifact_length(profile, artifact, projection).is_ok());
            assert!(check_artifact_length(profile, artifact + 1, projection).is_err());
        }
        assert!(
            check_pair_lengths(
                profile,
                TYPED_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES,
                TYPED_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES,
            )
            .is_ok()
        );
        assert!(
            check_pair_lengths(
                profile,
                TYPED_REFERENCE_ITEM_MAX_SERVER_ARTIFACT_BYTES + 1,
                TYPED_REFERENCE_ITEM_MAX_CLIENT_ARTIFACT_BYTES,
            )
            .is_err()
        );
    }

    #[test]
    fn retained_core_wire_goldens_and_partial_unknown_guard_hold() -> Result<(), ContentError> {
        let definitions = [source_definition(ReferenceItemDefinition {
            physical_class: ReferenceItemPhysicalClass::Physical,
            materializable: true,
            stack_class: ReferenceItemStackClass::NonStackable,
            legal_destinations: vec![ReferenceItemDestination::CharacterInventory],
            semantics: Default::default(),
        })?];
        let ReferenceDefinitionKind::Item(materializable) = &definitions[0].kind else {
            unreachable!()
        };
        assert_eq!(
            encode_typed_item(
                materializable,
                ReferenceArtifactProjection::ServerAuthoritative,
                &definitions
            )?,
            [2, 1, 1, 1, 1, 1, 0, 0],
        );
        assert_eq!(
            encode_typed_item(
                materializable,
                ReferenceArtifactProjection::ClientSafe,
                &definitions
            )?,
            [2, 1, 1, 0, 0],
        );
        let identity_only = ReferenceItemDefinition {
            physical_class: ReferenceItemPhysicalClass::Unknown,
            materializable: false,
            stack_class: ReferenceItemStackClass::Unknown,
            legal_destinations: Vec::new(),
            semantics: Default::default(),
        };
        let identity_definitions = [source_definition(identity_only.clone())?];
        assert_eq!(
            encode_typed_item(
                &identity_only,
                ReferenceArtifactProjection::ServerAuthoritative,
                &identity_definitions
            )?,
            hex_bytes("02020003000000")?,
        );
        assert_eq!(
            encode_typed_item(
                &identity_only,
                ReferenceArtifactProjection::ClientSafe,
                &identity_definitions
            )?,
            hex_bytes("0202030000")?,
        );
        let partial = ReferenceItemDefinition {
            physical_class: ReferenceItemPhysicalClass::Unknown,
            materializable: false,
            stack_class: ReferenceItemStackClass::Unknown,
            legal_destinations: Vec::new(),
            semantics: ReferenceItemSemantics {
                physical: ReferenceItemField::Known(ReferenceItemPhysical {
                    weight: ReferenceItemField::Known(123),
                    movable: ReferenceItemField::Unknown,
                    pickupable: ReferenceItemField::Unknown,
                }),
                ..Default::default()
            },
        };
        let partial_definitions = [source_definition(partial.clone())?];
        let server = encode_typed_item(
            &partial,
            ReferenceArtifactProjection::ServerAuthoritative,
            &partial_definitions,
        )?;
        assert_eq!(server.len(), 18);
        let index = [IndexEntry {
            identity: identity()?,
            body_offset: 0,
            body_length: server.len(),
            body_digest: sha256(&server),
        }];
        let decoded = parse_typed_server_item(&server, &index)?;
        assert!(!decoded.materializable);
        assert_eq!(decoded.semantics.physical, partial.semantics.physical);
        Ok(())
    }

    #[test]
    fn typed_weapon_bytes_match_independent_python_oracle() -> Result<(), ContentError> {
        let fixtures = [
            (
                false,
                "0201010201010001060058030308030000002a0300000000030000000003000103000000000000000000000000000000010300000000000000000000000000000001010305010300000000020300000000030300000000040300000000050300000000",
                "0201020001060058030308030000002a0300000000030000000003000103000000000000000000000000000000010300000000000000000000000000000001010305010300000000020300000000030300000000040300000000050300000000",
                "c7ea4af176b3eebe80390f9a9ee26deb4cdc9f67695ef5e77390f2bc5c6c68b6",
                "f4396b9af4322ac0841fb9783aa51efb0a333d2d83113c943a4dc28526249114",
            ),
            (
                true,
                "0201010201010001060059030304030000002303000000000300000000030007030000000000000009000000000000000a030000000000000001000000000000000103010305010300000000020300000000030300000000040300000000050300000000",
                "0201020001060059030304030000002303000000000300000000030007030000000000000009000000000000000a030000000000000001000000000000000103010305010300000000020300000000030300000000040300000000050300000000",
                "d4649e3a70e117c485975174cc6c1f0977618a26a9d18bf01539d87be3d7c6a9",
                "8c1c2aaea9349e75be386299afc350a0d8ac0acb27bb44cf205a3f972f27ee11",
            ),
        ];
        for (distance, server_hex, client_hex, server_sha, client_sha) in fixtures {
            let item = oracle_weapon(distance)?;
            let definitions = [source_definition(item.clone())?];
            let server = encode_typed_item(
                &item,
                ReferenceArtifactProjection::ServerAuthoritative,
                &definitions,
            )?;
            let client =
                encode_typed_item(&item, ReferenceArtifactProjection::ClientSafe, &definitions)?;
            assert_eq!(server, hex_bytes(server_hex)?);
            assert_eq!(client, hex_bytes(client_hex)?);
            assert_eq!(sha256(&server).as_slice(), hex_bytes(server_sha)?);
            assert_eq!(sha256(&client).as_slice(), hex_bytes(client_sha)?);
        }
        Ok(())
    }

    #[test]
    fn typed_codec_rejects_noncanonical_and_projection_unsafe_shapes() -> Result<(), ContentError> {
        use ReferenceItemField::Known;
        assert!(ReferenceRationalPercent::new(2, 4).is_err());
        assert!(ReferenceRationalPercent::new(0, 2).is_err());
        assert!(ReferenceRationalPercent::new(1, 0).is_err());
        let mut noncanonical_rational = Vec::new();
        noncanonical_rational.extend_from_slice(&2_i64.to_be_bytes());
        noncanonical_rational.extend_from_slice(&4_u64.to_be_bytes());
        assert!(read_rational(&mut SliceReader::new(&noncanonical_rational)).is_err());
        assert!(ReferenceItemType::from_wire(u8::MAX).is_err());

        let mut item = worst_item()?;
        let ReferenceItemField::Known(stack) = &mut item.semantics.stack else {
            unreachable!()
        };
        stack.stackable = Known(false);
        assert!(crate::content::reference_playable::validate_item_definition(&item).is_err());

        let mut item = worst_item()?;
        let ReferenceItemField::Known(equipment) = &mut item.semantics.equipment else {
            unreachable!()
        };
        let ReferenceItemField::Known(patterns) = &mut equipment.patterns else {
            unreachable!()
        };
        let mut third = patterns[1].clone();
        third.pattern_id = 3;
        patterns.push(third);
        assert!(crate::content::reference_playable::validate_item_definition(&item).is_err());

        let mut item = worst_item()?;
        let ReferenceItemField::Known(imbuement) = &mut item.semantics.imbuement else {
            unreachable!()
        };
        imbuement.slot_count = Known(REFERENCE_ITEM_MAX_IMBUEMENT_SLOTS + 1);
        assert!(crate::content::reference_playable::validate_item_definition(&item).is_err());

        let identity = identity()?;
        let index = [IndexEntry {
            identity,
            body_offset: 0,
            body_length: 0,
            body_digest: [0; 32],
        }];
        for group in [
            ITEM_GROUP_TEMPORAL,
            ITEM_GROUP_USE_TRANSFORM,
            ITEM_GROUP_TRADE_RESTRICTIONS,
            ITEM_GROUP_FLUID,
            ITEM_GROUP_READABLE_WRITEABLE,
        ] {
            let client_with_server_group = [2, 1, 2, 0, 1, group, 0, 1, 0];
            assert!(parse_typed_client_item(&client_with_server_group, &index).is_err());
        }
        let duplicate_group = [2, 1, 2, 0, 2, 1, 0, 1, 0, 1, 0, 1, 0];
        assert!(parse_typed_client_item(&duplicate_group, &index).is_err());
        let stack_contradiction = [2, 1, 2, 0, 1, ITEM_GROUP_STACK, 0, 4, 3, 3, 0, 0];
        assert!(parse_typed_client_item(&stack_contradiction, &index).is_err());
        Ok(())
    }
}
