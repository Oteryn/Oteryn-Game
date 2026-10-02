//! Explicit local candidate native gameplay artifact. No embedded/global catalogue is loaded.
//! The existing activation guard authorizes the outer artifact digest, including every profile.
use super::digest::sha256;
use super::model::ContentError;
use super::production::CompiledFirstProductionContent;
use super::project::{
    ProjectV2AppearanceSelection, ProjectV2AuthoringProfile, ProjectV2AuthoringProfileData,
    ProjectV2DefinitionRef, ProjectV2Family,
};
use crate::ability::condition::{ConditionType, DotElement};
use crate::foundation::{ChannelRuntimeV1, CompiledCreaturePolicies, CompiledCreaturePolicy};
use crate::spell::{SpellBook, executable_catalog};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
};

pub(crate) const MAGIC: &[u8; 8] = b"OTNGP01\0";
pub(crate) const MAGIC_V2: &[u8; 8] = b"OTNGP02\0";
pub(crate) const MAGIC_V3: &[u8; 8] = b"OTNGP03\0";
pub(crate) const MAGIC_V4: &[u8; 8] = b"OTNGP04\0";
pub(crate) const MAGIC_V5: &[u8; 8] = b"OTNGP05\0";
pub(crate) const MAX_ARTIFACT_BYTES: usize = 88 * 1024 * 1024;
pub(crate) fn is_envelope(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC)
        || bytes.starts_with(MAGIC_V2)
        || bytes.starts_with(MAGIC_V3)
        || bytes.starts_with(MAGIC_V4)
        || bytes.starts_with(MAGIC_V5)
}
const MAX_CATALOG: usize = 32 * 1024 * 1024;
const MAX_SELECTION: usize = 256 * 1024;
const MAX_PROFILES: usize = 8 * 1024 * 1024;
const MAX_RECORDS: usize = 4096;
const LIMITS: [usize; 5] = [
    super::production::FIRST_PRODUCTION_MAX_SERVER_ARTIFACT_BYTES,
    MAX_CATALOG,
    MAX_SELECTION,
    MAX_PROFILES,
    MAX_PROFILES,
];

#[derive(Debug, Clone)]
pub(crate) struct PinnedGameplayBytes {
    pub(crate) bytes: Vec<u8>,
    pub(crate) sha256: String,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum NativeGameplayMapProfile {
    #[default]
    #[serde(rename = "accepted-entry-r1")]
    AcceptedEntryR1,
    #[serde(rename = "source-qualified-spell-entry-r2")]
    SourceQualifiedSpellEntryR2,
}
#[derive(Debug, Clone)]
pub(crate) struct NativeGameplayInput {
    pub(crate) native_map_profile: NativeGameplayMapProfile,
    pub(crate) catalog: PinnedGameplayBytes,
    pub(crate) source_selection: PinnedGameplayBytes,
    pub(crate) creature_profiles: PinnedGameplayBytes,
    pub(crate) presentation_profiles: PinnedGameplayBytes,
    pub(crate) item_profiles: Option<PinnedGameplayBytes>,
    pub(crate) spell_appearances: Option<PinnedGameplayBytes>,
    pub(crate) build_training: Option<NativeTrainingInput>,
    pub(crate) familiar_config: Option<PinnedGameplayBytes>,
    pub(crate) familiar_defenses: Option<PinnedGameplayBytes>,
    pub(crate) wheel_profile: Option<PinnedGameplayBytes>,
    /// Separate outer server/client source-world compositor consumes this exact pinned input.
    pub(crate) source_world: Option<PinnedGameplayBytes>,
}
#[derive(Debug, Clone)]
pub(crate) struct NativeTrainingInput {
    pub(crate) profile: PinnedGameplayBytes,
    pub(crate) content_revision: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrainingFilePin {
    path: String,
    sha256: String,
    content_revision: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrainingEnvelope {
    content_revision: String,
    profile: serde_json::Value,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SupplementsEnvelope {
    native_map_profile: NativeGameplayMapProfile,
    familiar_config: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    familiar_defenses: Option<String>,
    #[serde(deserialize_with = "required_supplement_option")]
    wheel_profile: Option<String>,
}
fn required_supplement_option<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(d)
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FilePin {
    path: String,
    sha256: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProvisioningManifest {
    #[serde(default)]
    native_map_profile: NativeGameplayMapProfile,
    schema: String,
    catalog: FilePin,
    source_selection: FilePin,
    creature_profiles: FilePin,
    presentation_profiles: FilePin,
    #[serde(default)]
    item_profiles: Option<FilePin>,
    #[serde(default)]
    spell_appearances: Option<FilePin>,
    #[serde(default)]
    build_training: Option<TrainingFilePin>,
    #[serde(default)]
    familiar_config: Option<FilePin>,
    #[serde(default)]
    familiar_defenses: Option<FilePin>,
    #[serde(default)]
    wheel_profile: Option<FilePin>,
    #[serde(default)]
    source_world: Option<FilePin>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreatureProfileRecord {
    pub(crate) profile: ProjectV2AuthoringProfile,
    pub(crate) presentation: ProjectV2DefinitionRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) behavior: Option<ProjectV2AuthoringProfile>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreatureProfilesDocument {
    pub(crate) schema: String,
    pub(crate) records: Vec<CreatureProfileRecord>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PresentationProfilesDocument {
    pub(crate) schema: String,
    pub(crate) records: Vec<ProjectV2AuthoringProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativeItemAttributes {
    /// None is unknown, never a zero equipment contribution.
    pub(crate) speed_bonus: Option<i32>,
    pub(crate) blocks_movement: Option<bool>,
    pub(crate) blocks_projectile: Option<bool>,
    pub(crate) immovable_block_solid: Option<bool>,
    /// Missing is Unknown; only a source-qualified recipe may assert a non-damaging field.
    pub(crate) field_condition: Option<crate::ability::field_condition::QualifiedFieldCondition>,
    /// Explicit activated source/config policy; absence stays Unknown.
    pub(crate) rune_consumption: Option<NativeRuneConsumption>,
    pub(crate) field_replaceable: Option<bool>,
    pub(crate) has_height: Option<bool>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NativeRuneConsumption {
    ItemCountOne,
}
/// Explicit candidate durable admission, independent from presentation membership or stats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativeItemAdmission {
    pub(crate) materializable: bool,
    pub(crate) stack_class: NativeItemStackClass,
    pub(crate) legal_destinations: Vec<NativeItemDestination>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum NativeItemStackClass {
    NonStackable,
    StackCapable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) enum NativeItemDestination {
    CharacterInventory,
    Ground,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativeProductionItemDefinition {
    pub(crate) family: String,
    pub(crate) production_key: String,
    pub(crate) revision_ref: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemProfileRecord {
    pub(crate) authoring: super::ProjectV2ItemAuthoring,
    pub(crate) semantics: super::ReferenceItemSemantics,
    pub(crate) attributes: NativeItemAttributes,
    pub(crate) admission: NativeItemAdmission,
    pub(crate) production_definition: NativeProductionItemDefinition,
    pub(crate) production_binding: super::project::ProjectV2SourceIdentityBinding,
    pub(crate) production_binding_qualification: Option<serde_json::Value>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemProfilesDocument {
    pub(crate) schema: String,
    pub(crate) records: Vec<ItemProfileRecord>,
}
/// A borrowed member of the actual active artifact; caller JSON is never directly a policy.
#[derive(Debug, Clone, Copy)]
pub(crate) struct QualifiedItemPolicy<'a> {
    digest: [u8; 32],
    record: &'a ItemProfileRecord,
}
impl<'a> QualifiedItemPolicy<'a> {
    pub(crate) fn source_digest(self) -> [u8; 32] {
        self.digest
    }
    pub(crate) fn record(self) -> &'a ItemProfileRecord {
        self.record
    }
}

#[derive(Debug, Clone)]
pub(crate) struct NativeGameplayState {
    native_map_profile: NativeGameplayMapProfile,
    source_digest: [u8; 32],
    // Equality tracks the complete qualified source; compiled representations are deterministic.
    encoded: Arc<[u8]>,
    book: Arc<SpellBook>,
    catalog: Arc<executable_catalog::CompiledCatalog>,
    policies: CompiledCreaturePolicies,
    creatures: CreatureProfilesDocument,
    presentations: PresentationProfilesDocument,
    items: Option<ItemProfilesDocument>,
    appearances: Option<super::native_spell_appearances::CompiledSpellAppearances>,
    training: Option<crate::spell::mana_training::CompiledTrainingFormula>,
    familiar_config: Option<super::spell_familiar_config::CompiledFamiliarConfig>,
    familiar_defenses: Option<super::spell_familiar_defenses::CompiledFamiliarDefenses>,
    wheel_profile: Option<super::spell_wheel_profile::CompiledWheelProfile>,
}
impl PartialEq for NativeGameplayState {
    fn eq(&self, other: &Self) -> bool {
        self.source_digest == other.source_digest && self.encoded == other.encoded
    }
}
impl Eq for NativeGameplayState {}
impl NativeGameplayState {
    pub(crate) fn native_map_profile(&self) -> NativeGameplayMapProfile {
        self.native_map_profile
    }
    pub(crate) fn source_digest(&self) -> [u8; 32] {
        self.source_digest
    }
    pub(crate) fn spell_book(&self) -> &SpellBook {
        &self.book
    }
    pub(crate) fn catalog(&self) -> &executable_catalog::CompiledCatalog {
        &self.catalog
    }
    pub(crate) fn creature_profiles(&self) -> &CreatureProfilesDocument {
        &self.creatures
    }
    pub(crate) fn presentation_profiles(&self) -> &PresentationProfilesDocument {
        &self.presentations
    }
    pub(crate) fn training_formula(
        &self,
    ) -> Option<&crate::spell::mana_training::CompiledTrainingFormula> {
        self.training.as_ref()
    }
    pub(crate) fn spell_appearances(
        &self,
    ) -> Option<&super::native_spell_appearances::CompiledSpellAppearances> {
        self.appearances.as_ref()
    }
    pub(crate) fn item_policy(&self, key: &str, revision: &str) -> Option<QualifiedItemPolicy<'_>> {
        let record = self.items.as_ref()?.records.iter().find(|p| {
            (p.authoring.item.key == key && p.authoring.item.revision == revision)
                || (p.production_definition.production_key == key
                    && p.production_definition.revision_ref == revision)
        })?;
        Some(QualifiedItemPolicy {
            digest: self.source_digest,
            record,
        })
    }
    pub(crate) fn item_policy_for_source_id(&self, id: u32) -> Option<QualifiedItemPolicy<'_>> {
        let external = id.to_string();
        let record = self
            .items
            .as_ref()?
            .records
            .iter()
            .find(|p| p.production_binding.external_id == external)?;
        Some(QualifiedItemPolicy {
            digest: self.source_digest,
            record,
        })
    }
    pub(crate) fn familiar_defenses(
        &self,
    ) -> Option<&super::spell_familiar_defenses::CompiledFamiliarDefenses> {
        self.familiar_defenses.as_ref()
    }
    pub(crate) fn familiar_config(
        &self,
    ) -> Option<&super::spell_familiar_config::CompiledFamiliarConfig> {
        self.familiar_config.as_ref()
    }
    pub(crate) fn wheel_profile(
        &self,
    ) -> Option<&super::spell_wheel_profile::CompiledWheelProfile> {
        self.wheel_profile.as_ref()
    }
    /// Stage invokes this only after the complete outer artifact matches independent issuance.
    /// Rebinds already validated immutable data; creates no active-generation authority.
    pub(crate) fn bind_qualified_outer_artifact(
        &mut self,
        digest: [u8; 32],
    ) -> Result<(), ContentError> {
        if digest == [0; 32] {
            return Err(invalid("native gameplay outer content pin"));
        }
        self.policies
            .bind_qualified_outer_artifact(digest)
            .map_err(|_| invalid("native gameplay outer creature pin"))?;
        if let Some(value) = &mut self.appearances {
            value
                .bind_qualified_outer_artifact(digest)
                .map_err(|_| invalid("native gameplay outer profile pin"))?;
        }
        if let Some(value) = &mut self.familiar_config {
            value
                .bind_qualified_outer_artifact(digest)
                .map_err(|_| invalid("native gameplay outer profile pin"))?;
        }
        if let Some(value) = &mut self.familiar_defenses {
            value
                .bind_qualified_outer_artifact(digest)
                .map_err(|_| invalid("native gameplay outer familiar defense pin"))?;
        }
        if let Some(value) = &mut self.wheel_profile {
            value
                .bind_qualified_outer_artifact(digest)
                .map_err(|_| invalid("native gameplay outer Wheel pin"))?;
        }
        self.source_digest = digest;
        Ok(())
    }
    pub(crate) fn install_companion_policies(
        &self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<(), ContentError> {
        runtime
            .install_companion_policies(self.policies.clone())
            .map_err(|_| invalid("native gameplay active creature policy pin"))
    }
}
fn invalid(reason: &'static str) -> ContentError {
    ContentError::InvalidArtifact(reason)
}
fn hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn bounded(bytes: &[u8], max: usize) -> Result<(), ContentError> {
    if bytes.is_empty() || bytes.len() > max {
        return Err(invalid("native gameplay section bounds"));
    }
    Ok(())
}
fn qualify(pin: &PinnedGameplayBytes, max: usize) -> Result<(), ContentError> {
    bounded(&pin.bytes, max)?;
    if hex(sha256(&pin.bytes)) != pin.sha256 {
        return Err(ContentError::RevisionMismatch(
            "native gameplay input digest",
        ));
    }
    Ok(())
}
fn read_bounded(path: &Path, max: usize) -> Result<Vec<u8>, ContentError> {
    let metadata =
        std::fs::metadata(path).map_err(|_| invalid("native gameplay provisioning file"))?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > max as u64 {
        return Err(invalid("native gameplay provisioning file bounds"));
    }
    // Read at most max+1, even if the file grows between metadata and open.
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|_| invalid("native gameplay provisioning file"))?
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid("native gameplay provisioning read"))?;
    bounded(&bytes, max)?;
    Ok(bytes)
}
impl NativeGameplayInput {
    pub(crate) fn from_manifest(path: &Path) -> Result<Self, ContentError> {
        let bytes = read_bounded(path, 16 * 1024)?;
        let manifest: ProvisioningManifest = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("native gameplay provisioning manifest"))?;
        if !matches!(
            manifest.schema.as_str(),
            "OTERYN_NATIVE_GAMEPLAY_MANIFEST/v1"
                | "OTERYN_NATIVE_GAMEPLAY_MANIFEST/v2"
                | "OTERYN_NATIVE_GAMEPLAY_MANIFEST/v3"
                | "OTERYN_NATIVE_GAMEPLAY_MANIFEST/v4"
                | "OTERYN_NATIVE_GAMEPLAY_MANIFEST/v5"
        ) || ((!manifest.schema.ends_with("/v1")) != manifest.item_profiles.is_some())
            || ((manifest.schema.ends_with("/v3")
                || manifest.schema.ends_with("/v4")
                || manifest.schema.ends_with("/v5"))
                != manifest.spell_appearances.is_some())
            || ((manifest.schema.ends_with("/v4") || manifest.schema.ends_with("/v5"))
                != manifest.build_training.is_some())
            || (manifest.schema.ends_with("/v5") != manifest.familiar_config.is_some())
            || (manifest.wheel_profile.is_some() && !manifest.schema.ends_with("/v5"))
            || (manifest.native_map_profile != NativeGameplayMapProfile::AcceptedEntryR1
                && !manifest.schema.ends_with("/v5"))
            || (manifest.native_map_profile != NativeGameplayMapProfile::AcceptedEntryR1
                && manifest.source_world.is_some())
        {
            return Err(invalid("native gameplay provisioning discriminator"));
        }
        let directory = path.parent().unwrap_or(Path::new("."));
        let load = |pin: FilePin, limit| -> Result<PinnedGameplayBytes, ContentError> {
            if pin.path.is_empty() {
                return Err(invalid("native gameplay empty locator"));
            }
            let pinned = PinnedGameplayBytes {
                bytes: read_bounded(&directory.join(pin.path), limit)?,
                sha256: pin.sha256,
            };
            qualify(&pinned, limit)?;
            Ok(pinned)
        };
        Ok(Self {
            native_map_profile: manifest.native_map_profile,
            wheel_profile: manifest
                .wheel_profile
                .map(|pin| load(pin, 64 * 1024))
                .transpose()?,
            familiar_defenses: manifest
                .familiar_defenses
                .map(|pin| load(pin, 32 * 1024))
                .transpose()?,
            familiar_config: manifest
                .familiar_config
                .map(|pin| load(pin, 4096))
                .transpose()?,
            source_world: manifest
                .source_world
                .map(|pin| load(pin, MAX_PROFILES))
                .transpose()?,
            catalog: load(manifest.catalog, MAX_CATALOG)?,
            source_selection: load(manifest.source_selection, MAX_SELECTION)?,
            creature_profiles: load(manifest.creature_profiles, MAX_PROFILES)?,
            presentation_profiles: load(manifest.presentation_profiles, MAX_PROFILES)?,
            item_profiles: manifest
                .item_profiles
                .map(|pin| load(pin, MAX_PROFILES))
                .transpose()?,
            spell_appearances: manifest
                .spell_appearances
                .map(|pin| load(pin, MAX_PROFILES))
                .transpose()?,
            build_training: manifest
                .build_training
                .map(|pin| {
                    super::DefinitionRevisionRef::new(&pin.content_revision)?;
                    Ok::<_, ContentError>(NativeTrainingInput {
                        profile: load(
                            FilePin {
                                path: pin.path,
                                sha256: pin.sha256,
                            },
                            MAX_PROFILES,
                        )?,
                        content_revision: pin.content_revision,
                    })
                })
                .transpose()?,
        })
    }
}
/// Retains the exact admitted baseline artifact. This envelope changes neither its record limits
/// nor client projection, and only a control-plane issuance for the new OUTER digest activates it.
pub(crate) fn compile_native_gameplay(
    base: &CompiledFirstProductionContent,
    input: &NativeGameplayInput,
) -> Result<CompiledFirstProductionContent, ContentError> {
    if input.native_map_profile != NativeGameplayMapProfile::AcceptedEntryR1
        && (input.familiar_config.is_none() || input.source_world.is_some())
    {
        return Err(invalid(
            "native gameplay explicit map profile requires v5 and no world addon",
        ));
    }
    let pins = [
        &input.catalog,
        &input.source_selection,
        &input.creature_profiles,
        &input.presentation_profiles,
    ];
    for (pin, limit) in pins.iter().zip(&LIMITS[1..]) {
        qualify(pin, *limit)?;
    }
    if is_envelope(&base.server_artifact) {
        return Err(invalid("nested native gameplay envelope"));
    }
    // Validate base before wrapping (also catches callers mutating public artifact bytes).
    super::production::StagedGeneration::stage(
        &base.server_artifact,
        &base.client_artifact,
        base.expectation(),
    )?;
    if let Some(items) = &input.item_profiles {
        qualify(items, MAX_PROFILES)?;
    }
    if let Some(appearances) = &input.spell_appearances {
        if input.item_profiles.is_none() {
            return Err(invalid("native gameplay v3 requires item profiles"));
        }
        qualify(appearances, MAX_PROFILES)?;
    }
    let training_section = if let Some(training) = &input.build_training {
        if input.spell_appearances.is_none() {
            return Err(invalid("native gameplay v4 requires appearances"));
        }
        qualify(&training.profile, MAX_PROFILES)?;
        super::DefinitionRevisionRef::new(&training.content_revision)?;
        let profile = serde_json::from_slice(&training.profile.bytes)
            .map_err(|_| invalid("native gameplay training JSON"))?;
        let bytes = serde_json::to_vec(&TrainingEnvelope {
            content_revision: training.content_revision.clone(),
            profile,
        })
        .map_err(|_| invalid("native gameplay training envelope"))?;
        bounded(&bytes, MAX_PROFILES)?;
        Some(bytes)
    } else {
        None
    };
    if let Some(source) = &input.source_world {
        qualify(source, MAX_PROFILES)?;
    }
    if let Some(config) = &input.familiar_config {
        if training_section.is_none() {
            return Err(invalid("native gameplay v5 requires training"));
        }
        qualify(config, 4096)?;
    }
    if let Some(defenses) = &input.familiar_defenses {
        if input.familiar_config.is_none() {
            return Err(invalid("familiar defense requires explicit v5"));
        }
        qualify(defenses, 32 * 1024)?;
    }
    if let Some(wheel) = &input.wheel_profile {
        if input.familiar_config.is_none() {
            return Err(invalid("native gameplay Wheel requires explicit v5"));
        }
        qualify(wheel, 64 * 1024)?;
    }
    let mut bytes = if input.familiar_config.is_some() {
        MAGIC_V5
    } else if training_section.is_some() {
        MAGIC_V4
    } else if input.spell_appearances.is_some() {
        MAGIC_V3
    } else if input.item_profiles.is_some() {
        MAGIC_V2
    } else {
        MAGIC
    }
    .to_vec();
    for section in std::iter::once(base.server_artifact.as_slice())
        .chain(pins.iter().map(|p| p.bytes.as_slice()))
    {
        bytes.extend_from_slice(&(section.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&sha256(section));
        bytes.extend_from_slice(section);
    }
    if let Some(items) = &input.item_profiles {
        bytes.extend_from_slice(&(items.bytes.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&sha256(&items.bytes));
        bytes.extend_from_slice(&items.bytes);
    }
    if let Some(appearances) = &input.spell_appearances {
        bytes.extend_from_slice(&(appearances.bytes.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&sha256(&appearances.bytes));
        bytes.extend_from_slice(&appearances.bytes);
    }
    if let Some(training) = training_section {
        bytes.extend_from_slice(&(training.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&sha256(&training));
        bytes.extend_from_slice(&training);
    }
    if let Some(config) = &input.familiar_config {
        let supplements = SupplementsEnvelope {
            native_map_profile: input.native_map_profile,
            familiar_config: String::from_utf8(config.bytes.clone())
                .map_err(|_| invalid("native gameplay config UTF8"))?,
            familiar_defenses: input
                .familiar_defenses
                .as_ref()
                .map(|pin| {
                    String::from_utf8(pin.bytes.clone())
                        .map_err(|_| invalid("native gameplay familiar defense UTF8"))
                })
                .transpose()?,
            wheel_profile: input
                .wheel_profile
                .as_ref()
                .map(|pin| {
                    String::from_utf8(pin.bytes.clone())
                        .map_err(|_| invalid("native gameplay Wheel UTF8"))
                })
                .transpose()?,
        };
        let section = serde_json::to_vec(&supplements)
            .map_err(|_| invalid("native gameplay supplements encoding"))?;
        bounded(&section, 128 * 1024)?;
        bytes.extend_from_slice(&(section.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&sha256(&section));
        bytes.extend_from_slice(&section);
    }
    bounded(&bytes, MAX_ARTIFACT_BYTES)?;
    decode(&bytes)?; // validate the full source policy, book and decoded creature policies now
    Ok(base.with_native_server_artifact(bytes))
}

pub(crate) struct DecodedNativeGameplay<'a> {
    pub(crate) baseline: &'a [u8],
    pub(crate) state: NativeGameplayState,
}
pub(crate) fn decode(bytes: &[u8]) -> Result<DecodedNativeGameplay<'_>, ContentError> {
    bounded(bytes, MAX_ARTIFACT_BYTES)?;
    if !is_envelope(bytes) {
        return Err(invalid("native gameplay discriminator"));
    }
    let mut cursor = MAGIC.len();
    let is_v5 = bytes.starts_with(MAGIC_V5);
    if !is_v5 && bytes.len() > 80 * 1024 * 1024 {
        return Err(invalid("native gameplay v4 bounds"));
    }
    let is_v4 = bytes.starts_with(MAGIC_V4) || is_v5;
    let is_v3 = bytes.starts_with(MAGIC_V3) || is_v4;
    if !is_v4 && bytes.len() > 72 * 1024 * 1024 {
        return Err(invalid("native gameplay v3 bounds"));
    }
    let is_v2 = bytes.starts_with(MAGIC_V2) || is_v3;
    if !is_v3 && bytes.len() > 64 * 1024 * 1024 {
        return Err(invalid("native gameplay v2 bounds"));
    }
    if !is_v2 && bytes.len() > 56 * 1024 * 1024 {
        return Err(invalid("native gameplay v1 bounds"));
    }
    let mut sections = Vec::with_capacity(if is_v5 {
        9
    } else if is_v4 {
        8
    } else if is_v3 {
        7
    } else if is_v2 {
        6
    } else {
        5
    });
    for limit in LIMITS
        .into_iter()
        .chain(is_v2.then_some(MAX_PROFILES))
        .chain(is_v3.then_some(MAX_PROFILES))
        .chain(is_v4.then_some(MAX_PROFILES))
        .chain(is_v5.then_some(128 * 1024))
    {
        let header_end = cursor
            .checked_add(36)
            .ok_or(ContentError::InvalidSectionBounds)?;
        let header = bytes
            .get(cursor..header_end)
            .ok_or(ContentError::InvalidSectionBounds)?;
        let len = u32::from_be_bytes(
            header[..4]
                .try_into()
                .map_err(|_| ContentError::InvalidSectionBounds)?,
        ) as usize;
        if len == 0 || len > limit {
            return Err(invalid("native gameplay section bounds"));
        }
        let end = header_end
            .checked_add(len)
            .ok_or(ContentError::InvalidSectionBounds)?;
        let section = bytes
            .get(header_end..end)
            .ok_or(ContentError::InvalidSectionBounds)?;
        if sha256(section).as_slice() != &header[4..] {
            return Err(ContentError::RevisionMismatch(
                "native gameplay section hash",
            ));
        }
        sections.push(section);
        cursor = end;
    }
    if cursor != bytes.len() || is_envelope(sections[0]) {
        return Err(invalid("native gameplay trailing or nested bytes"));
    }
    let catalog = executable_catalog::compile(sections[1], &hex(sha256(sections[1])))
        .map_err(|_| invalid("native gameplay executable catalog"))?;
    let selections = executable_catalog::compile_source_selection(
        sections[2],
        &hex(sha256(sections[2])),
        &catalog,
    )
    .map_err(|_| invalid("native gameplay source selections"))?;
    let book = catalog
        .clone()
        .into_spell_book_with_selections(&selections)
        .map_err(|_| invalid("native gameplay spell book"))?;
    let creatures: CreatureProfilesDocument = serde_json::from_slice(sections[3])
        .map_err(|_| invalid("native gameplay creature profiles"))?;
    let presentations: PresentationProfilesDocument = serde_json::from_slice(sections[4])
        .map_err(|_| invalid("native gameplay presentation profiles"))?;
    if creatures.schema != "OTERYN_NATIVE_CREATURE_PROFILES/v1"
        || presentations.schema != "OTERYN_NATIVE_PRESENTATION_PROFILES/v1"
        || creatures.records.len() > MAX_RECORDS
        || presentations.records.len() > MAX_RECORDS
    {
        return Err(invalid("native gameplay profile discriminator or bounds"));
    }
    let records = creature_policies(&creatures, &presentations)?;
    let items = if is_v2 {
        let items: ItemProfilesDocument = serde_json::from_slice(sections[5])
            .map_err(|_| invalid("native gameplay item profiles"))?;
        if items.schema != "OTERYN_NATIVE_ITEM_PROFILES/v1" || items.records.len() > MAX_RECORDS {
            return Err(invalid(
                "native gameplay item profile discriminator or bounds",
            ));
        }
        let mut identities = BTreeSet::new();
        let mut production_identities = BTreeSet::new();
        let admitted: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../imports/crystalserver/bindings/items.json"
        ))
        .map_err(|_| invalid("native gameplay admitted item binding table"))?;
        let mut admitted: BTreeMap<String, super::project::ProjectV2SourceIdentityBinding> =
            admitted
                .get("bindings")
                .and_then(|v| v.as_array())
                .ok_or_else(|| invalid("native gameplay admitted item bindings"))?
                .iter()
                .map(|value| {
                    let binding: super::project::ProjectV2SourceIdentityBinding =
                        serde_json::from_value(value.clone())
                            .map_err(|_| invalid("native gameplay admitted item binding"))?;
                    Ok((binding.external_id.clone(), binding))
                })
                .collect::<Result<_, ContentError>>()?;
        let candidate_bytes =
            include_bytes!("../../../../imports/canary/bindings/spell-items-candidate.json");
        if hex(sha256(candidate_bytes))
            != "da75016a8948fe7e49279c6700acc6f6808b0f4890df89f4ad1048e4d738dd93"
        {
            return Err(invalid(
                "native gameplay candidate Item binding source hash",
            ));
        }
        let candidate: serde_json::Value = serde_json::from_slice(candidate_bytes)
            .map_err(|_| invalid("native gameplay candidate Item binding packet"))?;
        for value in candidate
            .get("bindings")
            .and_then(|v| v.as_array())
            .ok_or_else(|| invalid("native gameplay candidate Item bindings"))?
        {
            let binding: super::project::ProjectV2SourceIdentityBinding =
                serde_json::from_value(value.clone())
                    .map_err(|_| invalid("native gameplay candidate Item binding"))?;
            if admitted
                .insert(binding.external_id.clone(), binding)
                .is_some()
            {
                return Err(invalid("native gameplay candidate Item binding conflict"));
            }
        }
        for record in &items.records {
            let binding = &record.production_binding;
            let external = binding
                .external_id
                .parse::<u32>()
                .map_err(|_| invalid("native gameplay item source identity"))?;
            if external == 0
                || external.to_string() != binding.external_id
                || admitted.get(&binding.external_id) != Some(binding)
                || record.production_definition.family != "Item"
                || record.production_definition.production_key != binding.target.key
                || record.production_definition.revision_ref != binding.target.revision
                || (record.authoring.item.key != format!("candidate:item/{external}")
                    && record.authoring.item.key != binding.target.key)
                || !production_identities.insert(binding.target.clone())
            {
                return Err(invalid("native gameplay item production binding"));
            }
            if binding.external_id == "40450" {
                use super::ReferenceItemField as F;
                let F::Known(physical) = &record.semantics.physical else {
                    return Err(invalid("native gameplay40450 source physics"));
                };
                let F::Known(stack) = &record.semantics.stack else {
                    return Err(invalid("native gameplay40450 source stack"));
                };
                let F::Known(classification) = &record.semantics.classification else {
                    return Err(invalid("native gameplay40450 source classification"));
                };
                if record.production_binding_qualification.as_ref() != Some(&candidate)
                    || !record.admission.materializable
                    || record.admission.stack_class != NativeItemStackClass::NonStackable
                    || record.admission.legal_destinations != [NativeItemDestination::Ground]
                    || physical.weight != F::Known(0)
                    || physical.movable != F::Known(false)
                    || physical.pickupable != F::Known(false)
                    || stack.stackable != F::Known(false)
                    || classification.item_type != F::NotApplicable
                    || record.attributes.speed_bonus != Some(0)
                    || record.attributes.blocks_movement != Some(false)
                    || record.attributes.blocks_projectile != Some(false)
                    || record.attributes.immovable_block_solid != Some(false)
                    || record.attributes.field_condition.is_some()
                    || record.attributes.rune_consumption.is_some()
                    || record.attributes.field_replaceable.is_some()
                    || record.attributes.has_height.is_some_and(|height| height)
                {
                    return Err(invalid(
                        "native gameplay40450 closed candidate source policy",
                    ));
                }
            } else if record.production_binding_qualification.is_some() {
                return Err(invalid("native gameplay unexpected Item candidate proof"));
            }
            exact(&binding.target, ProjectV2Family::Item)?;
            if record.attributes.rune_consumption.is_some() {
                use super::{ReferenceItemField as F, ReferenceItemType};
                if !matches!(&record.semantics.classification,
                    F::Known(classification) if classification.item_type == F::Known(ReferenceItemType::Rune))
                {
                    return Err(invalid("native gameplay Rune consumption classification"));
                }
            }
            if let Some(field) = &record.attributes.field_condition {
                field
                    .validate_for(&field.source.server, external)
                    .map_err(|_| invalid("native gameplay item field source recipe"))?;
            }
            exact(&record.authoring.item, ProjectV2Family::Item)?;
            super::project::validate_native_gameplay_item(&record.authoring)
                .map_err(|_| invalid("native gameplay item authoring"))?;
            if !identities.insert(record.authoring.item.clone()) {
                return Err(invalid("native gameplay duplicate item profile"));
            }
            // The same closed semantic validator used by the Reference linker.
            super::reference_playable::validate_native_item_semantics(&record.semantics)?;
            if record.admission.legal_destinations.is_empty()
                || record.admission.legal_destinations.len() > 2
                || record
                    .admission
                    .legal_destinations
                    .windows(2)
                    .any(|w| w[0] >= w[1])
            {
                return Err(invalid("native gameplay item admission destinations"));
            }
        }
        Some(items)
    } else {
        None
    };
    let source_digest = sha256(bytes);
    let appearances = if is_v3 {
        let compiled = super::native_spell_appearances::compile(sections[6], source_digest)?;
        for (creature, look_type) in compiled.creature_links() {
            if !creatures
                .records
                .iter()
                .any(|r| &r.profile.target == creature)
                || !records.iter().any(|p| {
                    p.definition_key == creature.key
                        && p.definition_revision == creature.revision
                        && p.outfit_appearance() == Some(look_type)
                })
            {
                return Err(invalid("native gameplay appearance creature binding"));
            }
        }
        for policy in &records {
            if policy.flags.illusionable
                && compiled
                    .for_creature(&policy.definition_key, &policy.definition_revision)
                    .is_none()
            {
                return Err(invalid(
                    "native gameplay missing illusionable creature appearance",
                ));
            }
        }
        Some(compiled)
    } else {
        None
    };
    let training = if is_v4 {
        let envelope: TrainingEnvelope = serde_json::from_slice(sections[7])
            .map_err(|_| invalid("native gameplay training section"))?;
        super::DefinitionRevisionRef::new(&envelope.content_revision)?;
        let bytes = serde_json::to_vec(&envelope.profile)
            .map_err(|_| invalid("native gameplay training profile"))?;
        Some(
            crate::spell::mana_training::CompiledTrainingFormula::from_profile(
                &bytes,
                &envelope.content_revision,
            )
            .map_err(|_| invalid("native gameplay training qualification"))?,
        )
    } else {
        None
    };
    let (familiar_config, familiar_defenses, wheel_profile, native_map_profile) = if is_v5 {
        let envelope: SupplementsEnvelope = serde_json::from_slice(sections[8])
            .map_err(|_| invalid("native gameplay source supplements"))?;
        let config = super::spell_familiar_config::CompiledFamiliarConfig::from_active_artifact(
            envelope.familiar_config.as_bytes(),
            source_digest,
        )
        .map_err(|_| invalid("native gameplay familiar configuration"))?;
        let wheel = envelope
            .wheel_profile
            .as_ref()
            .map(|bytes| {
                super::spell_wheel_profile::CompiledWheelProfile::from_active_artifact(
                    bytes.as_bytes(),
                    source_digest,
                )
                .map_err(|_| invalid("native gameplay Wheel source profile"))
            })
            .transpose()?;
        let defenses = envelope
            .familiar_defenses
            .as_ref()
            .map(|bytes| {
                super::spell_familiar_defenses::CompiledFamiliarDefenses::from_active_artifact(
                    bytes.as_bytes(),
                    source_digest,
                    &records,
                )
                .map_err(|_| invalid("native gameplay familiar source defense"))
            })
            .transpose()?;
        (Some(config), defenses, wheel, envelope.native_map_profile)
    } else {
        (None, None, None, NativeGameplayMapProfile::AcceptedEntryR1)
    };
    let policies = CompiledCreaturePolicies::from_active_artifact(source_digest, records)
        .map_err(|_| invalid("native gameplay creature policy table"))?;
    Ok(DecodedNativeGameplay {
        baseline: sections[0],
        state: NativeGameplayState {
            native_map_profile,
            source_digest,
            encoded: Arc::from(bytes),
            book: Arc::new(book),
            catalog: Arc::new(catalog),
            policies,
            creatures,
            presentations,
            items,
            appearances,
            training,
            familiar_config,
            familiar_defenses,
            wheel_profile,
        },
    })
}
fn exact(reference: &ProjectV2DefinitionRef, family: ProjectV2Family) -> Result<(), ContentError> {
    if reference.family != family {
        return Err(invalid("native gameplay exact profile family"));
    }
    super::ProductionKey::new(&reference.key)?;
    super::DefinitionRevisionRef::new(&reference.revision)?;
    Ok(())
}
fn condition(text: &str) -> Result<Option<ConditionType>, ContentError> {
    // Source-only families have no accepted speed/DOT analogue. Preserve them in the complete
    // decoded details rather than inventing a typed immunity for another effect family.
    if text == "drunk" {
        return Ok(None);
    }
    Ok(Some(match text {
        "invisible" | "invisibility" => ConditionType::Invisible,
        "haste" => ConditionType::Haste,
        "paralysis" | "paralyze" => ConditionType::Paralysis,
        "poison" | "earth" => ConditionType::DamageOverTime(DotElement::Poison),
        "fire" => ConditionType::DamageOverTime(DotElement::Fire),
        "energy" => ConditionType::DamageOverTime(DotElement::Energy),
        "bleeding" | "bleed" | "physical" => ConditionType::DamageOverTime(DotElement::Bleeding),
        "drown" => ConditionType::DamageOverTime(DotElement::Drown),
        "freezing" | "ice" => ConditionType::DamageOverTime(DotElement::Freezing),
        "dazzled" | "holy" => ConditionType::DamageOverTime(DotElement::Dazzled),
        "cursed" | "death" => ConditionType::DamageOverTime(DotElement::Cursed),
        "food_regeneration" => ConditionType::FoodRegeneration,
        "recovery" => ConditionType::Recovery,
        "mana_shield" => ConditionType::ManaShield,
        "light" => ConditionType::Light,
        "attributes" => ConditionType::Attributes,
        "outfit" => ConditionType::Outfit,
        _ => return Err(invalid("native gameplay unsupported condition immunity")),
    }))
}
fn creature_policies(
    creatures: &CreatureProfilesDocument,
    presentations: &PresentationProfilesDocument,
) -> Result<Vec<CompiledCreaturePolicy>, ContentError> {
    let mut appearance = BTreeMap::new();
    for profile in &presentations.records {
        super::project::validate_native_gameplay_profile(profile)
            .map_err(|_| invalid("native gameplay invalid V2 presentation authoring"))?;
        exact(&profile.target, ProjectV2Family::Presentation)?;
        let ProjectV2AuthoringProfileData::Presentation(value) = &profile.data else {
            return Err(invalid("native gameplay presentation profile kind"));
        };
        let (look, object) = match (&value.asset_binding, value.selection) {
            (Some(token), None | Some(ProjectV2AppearanceSelection::OwnerFamiliarLook)) => {
                let (suffix, is_object) =
                    if let Some(suffix) = token.strip_prefix("canary.appearance:outfit/") {
                        (suffix, false)
                    } else if let Some(suffix) = token.strip_prefix("canary.appearance:object/") {
                        if value.selection.is_some() {
                            return Err(invalid("native gameplay object appearance selection"));
                        }
                        (suffix, true)
                    } else {
                        return Err(invalid("native gameplay unsupported appearance binding"));
                    };
                let look = suffix
                    .parse::<u32>()
                    .map_err(|_| invalid("native gameplay appearance binding"))?;
                if look == 0 || suffix != look.to_string() {
                    return Err(invalid("native gameplay noncanonical appearance binding"));
                }
                if is_object {
                    (0, Some(look))
                } else {
                    (look, None)
                }
            }
            (None, Some(ProjectV2AppearanceSelection::Invisible)) => (0, None),
            _ => return Err(invalid("native gameplay missing or conflicting appearance")),
        };
        if appearance
            .insert(profile.target.clone(), (look, object))
            .is_some()
        {
            return Err(invalid("native gameplay duplicate presentation"));
        }
    }
    let mut seen = BTreeSet::new();
    let mut records = Vec::new();
    for record in &creatures.records {
        super::project::validate_native_gameplay_profile(&record.profile)
            .map_err(|_| invalid("native gameplay invalid V2 creature authoring"))?;
        exact(&record.profile.target, ProjectV2Family::Creature)?;
        exact(&record.presentation, ProjectV2Family::Presentation)?;
        if !seen.insert(record.profile.target.key.clone()) {
            return Err(invalid("native gameplay duplicate creature"));
        }
        let ProjectV2AuthoringProfileData::Creature(value) = &record.profile.data else {
            return Err(invalid("native gameplay creature profile kind"));
        };
        let details = value.details.as_ref().ok_or(invalid(
            "native gameplay legacy creature lacks complete details",
        ))?;
        let maximum_health = value
            .health
            .and_then(|v| i64::try_from(v).ok())
            .filter(|v| *v > 0)
            .ok_or(invalid("native gameplay creature health"))?;
        let base_speed = value
            .speed
            .and_then(|v| i32::try_from(v).ok())
            .ok_or(invalid("native gameplay creature speed"))?;
        if details.critical_chance_ppm > 1_000_000
            || details
                .condition_immunities
                .windows(2)
                .any(|p| p[0] >= p[1])
        {
            return Err(invalid("native gameplay creature details"));
        }
        let condition_immunities = details
            .condition_immunities
            .iter()
            .map(|c| condition(c))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();
        let (outfit_look_type, object_look_type) = *appearance.get(&record.presentation).ok_or(
            invalid("native gameplay exact creature presentation missing"),
        )?;
        if object_look_type.is_some()
            && (details.flags.illusionable || details.summoning.is_familiar)
        {
            return Err(invalid(
                "native gameplay object appearance incompatible creature flags",
            ));
        }
        let preferred_distance = if let Some(profile) = &record.behavior {
            exact(&profile.target, ProjectV2Family::Behavior)?;
            super::project::validate_native_gameplay_profile(profile)
                .map_err(|_| invalid("native gameplay creature behavior"))?;
            let ProjectV2AuthoringProfileData::Behavior(value) = &profile.data else {
                return Err(invalid("native gameplay behavior kind"));
            };
            Some(u32::from(value.targeting.target_distance_tiles))
        } else {
            None
        };
        records.push(CompiledCreaturePolicy {
            preferred_distance,
            reward_boss: Some(details.system_eligibility.reward_boss),
            armor: value.armor,
            mitigation: value
                .mitigation
                .map(|v| crate::foundation::CreatureExactRatio {
                    numerator: v.numerator,
                    denominator: v.denominator,
                }),
            resistances: value
                .resistances
                .iter()
                .map(|v| crate::foundation::CreatureResistance {
                    damage_type: v.damage_type.clone(),
                    percent: crate::foundation::CreatureExactRatio {
                        numerator: v.percent.numerator,
                        denominator: v.percent.denominator,
                    },
                })
                .collect(),
            damage_immunities: value.immunities.clone(),
            flags: crate::foundation::CreatureFlags {
                attackable: details.flags.attackable,
                illusionable: details.flags.illusionable,
                health_hidden: details.flags.health_hidden,
            },
            definition_key: record.profile.target.key.clone(),
            definition_revision: record.profile.target.revision.clone(),
            display_name: details.display_name.clone(),
            maximum_health,
            base_speed,
            outfit_look_type,
            object_look_type,
            summonable: details.summoning.summonable,
            convinceable: details.summoning.convinceable,
            mana_cost: details.summoning.mana_cost,
            is_familiar: details.summoning.is_familiar,
            condition_immunities,
        });
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::production::{StagedGeneration, test_source};
    use crate::content::{
        ContentActivationController, NativeEntryActivationIssuance, NodeBootQuiescence,
        activate_native_entry_room_with_gameplay, qualify_native_entry_room,
        qualify_native_entry_room_with_gameplay,
    };
    use serde_json::Value;
    fn pinned(bytes: &[u8]) -> PinnedGameplayBytes {
        PinnedGameplayBytes {
            bytes: bytes.to_vec(),
            sha256: hex(sha256(bytes)),
        }
    }
    fn input() -> NativeGameplayInput {
        NativeGameplayInput {
            native_map_profile: NativeGameplayMapProfile::AcceptedEntryR1,
            catalog: pinned(include_bytes!(
                "../../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
            )),
            source_selection: pinned(include_bytes!(
                "../../../../tools/content-schema/spell-authoring/samples/executable-spell-source-selection.json"
            )),
            creature_profiles: pinned(include_bytes!(
                "../../../../tools/content-schema/native-gameplay/creature_profiles.json"
            )),
            presentation_profiles: pinned(include_bytes!(
                "../../../../tools/content-schema/native-gameplay/presentation_profiles.json"
            )),
            item_profiles: None,
            spell_appearances: None,
            build_training: None,
            familiar_config: None,
            familiar_defenses: None,
            wheel_profile: None,
            source_world: None,
        }
    }
    /// Explicit local qualification of the real composed input, without runtime activation.
    #[test]
    #[ignore = "requires OTERYN_FULL_SPELL_TEST_MANIFEST from the explicit source producer"]
    fn actual_full_manifest_qualifies_source_world_and_all_owner_profiles() {
        let path = std::env::var_os("OTERYN_FULL_SPELL_TEST_MANIFEST")
            .expect("explicit full source manifest locator");
        let manifest_bytes = std::fs::read(Path::new(&path)).unwrap();
        let input = NativeGameplayInput::from_manifest(Path::new(&path)).unwrap();
        assert_eq!(manifest_bytes, std::fs::read(Path::new(&path)).unwrap());
        let world = test_source(1).unwrap().world_id;
        let source = input
            .source_world
            .as_ref()
            .expect("actual source world pin");
        let room = super::super::qualify_native_source_spell_world_with_gameplay(
            world,
            &input,
            &source.bytes,
        )
        .unwrap();
        let compiled = room.compiled();
        let staged = StagedGeneration::stage(
            &compiled.server_artifact,
            &compiled.client_artifact,
            compiled.expectation(),
        )
        .unwrap();
        assert_eq!(
            staged.runtime_state().native_source_world().unwrap(),
            source.bytes.as_slice()
        );
        // Change exactly one issued outer pin while retaining all source bytes.
        let mut other_server_bytes = compiled.server_artifact.clone();
        other_server_bytes[0] ^= 1;
        let wrong_server = compiled
            .with_native_artifact_pair(other_server_bytes, compiled.client_artifact.clone());
        assert!(matches!(
            StagedGeneration::stage(
                &compiled.server_artifact,
                &compiled.client_artifact,
                wrong_server.expectation()
            ),
            Err(ContentError::RevisionMismatch(
                "native source world outer issuance pins"
            ))
        ));
        let mut other_client_bytes = compiled.client_artifact.clone();
        other_client_bytes[0] ^= 1;
        let wrong_client = compiled
            .with_native_artifact_pair(compiled.server_artifact.clone(), other_client_bytes);
        assert!(matches!(
            StagedGeneration::stage(
                &compiled.server_artifact,
                &compiled.client_artifact,
                wrong_client.expectation()
            ),
            Err(ContentError::RevisionMismatch(
                "native source world outer issuance pins"
            ))
        ));
        let native = staged.runtime_state().native_gameplay().unwrap();
        assert_eq!(native.source_digest(), compiled.server_digest());
        assert_eq!(native.catalog().entries.len(), 246);
        assert_eq!(native.creature_profiles().records.len(), 162);
        assert_eq!(native.presentation_profiles().records.len(), 162);
        assert_eq!(native.items.as_ref().unwrap().records.len(), 118);
        for species in [
            "rat",
            "skeleton",
            "druid_familiar",
            "knight_familiar",
            "monk_familiar",
            "paladin_familiar",
            "sorcerer_familiar",
        ] {
            let key = format!("canary:creature/{species}");
            assert!(
                native
                    .creature_profiles()
                    .records
                    .iter()
                    .any(|r| r.profile.target.key == key
                        && r.profile.target.revision == "canary-47dfd51f")
            );
            if species.ends_with("_familiar") {
                assert!(
                    native
                        .familiar_defenses()
                        .unwrap()
                        .defense(&key, "canary-47dfd51f")
                        .is_some()
                );
            }
        }
        let item = native.item_policy_for_source_id(40450).unwrap();
        assert_eq!(item.source_digest(), compiled.server_digest());
        assert!(item.record().production_binding_qualification.is_some());
        assert_eq!(
            native.training_formula().unwrap().content_revision(),
            "build-content-r1"
        );
        assert_eq!(
            native.familiar_config().unwrap().source_digest(),
            compiled.server_digest()
        );
        assert_eq!(
            native.wheel_profile().unwrap().source_digest(),
            compiled.server_digest()
        );
        assert_eq!(
            native.spell_appearances().unwrap().source_digest(),
            compiled.server_digest()
        );
        assert_eq!(
            room.movement_cells().scope().generation_digest,
            compiled.server_digest()
        );
        if let Some(directory) = std::env::var_os("OTERYN_FULL_SPELL_TEST_OUTPUT") {
            let directory = Path::new(&directory);
            std::fs::create_dir_all(directory).unwrap();
            std::fs::write(
                directory.join("server-artifact.bin"),
                &compiled.server_artifact,
            )
            .unwrap();
            std::fs::write(
                directory.join("client-artifact.bin"),
                &compiled.client_artifact,
            )
            .unwrap();
            let proof = serde_json::json!({
                "schema": "OTERYN_LOCAL_FULL_SPELL_ARTIFACT_QUALIFICATION/v1",
                "manifest_sha256": hex(sha256(&manifest_bytes)),
                "server_sha256": hex(compiled.server_digest()),
                "client_sha256": hex(compiled.client_digest()),
                "source_world_sha256": source.sha256,
                "executable_spell_count": 246,
                "creature_profile_count": native.creature_profiles().records.len(),
                "presentation_profile_count": native.presentation_profiles().records.len(),
                "item_policy_count": 118,
                "qualification_boundary": "Actual source-world candidate compiled, strictly decoded and staged under both exact outer pins; runtime activation not performed",
                "production_stage": "Passed; altered server and client issuance pins rejected",
                "runtime_activation": "Not performed"
            });
            std::fs::write(
                directory.join("qualification-proof.json"),
                serde_json::to_vec_pretty(&proof).unwrap(),
            )
            .unwrap();
        }
        println!(
            "qualified and staged full source candidate: server={}, client={}, source_world={}",
            hex(compiled.server_digest()),
            hex(compiled.client_digest()),
            source.sha256
        );
    }
    #[test]
    fn actual_profile_policies_preserve_health_speed_look_cost_and_flags() {
        let input = input();
        let creatures: CreatureProfilesDocument =
            serde_json::from_slice(&input.creature_profiles.bytes).unwrap();
        let presentations: PresentationProfilesDocument =
            serde_json::from_slice(&input.presentation_profiles.bytes).unwrap();
        let policies = creature_policies(&creatures, &presentations).unwrap();
        let rat = policies.iter().find(|p| p.display_name == "Rat").unwrap();
        assert_eq!(
            (
                rat.maximum_health,
                rat.base_speed,
                rat.outfit_look_type,
                rat.mana_cost
            ),
            (20, 67, 21, Some(200))
        );
        assert!(rat.summonable && rat.convinceable && !rat.is_familiar);
        assert_eq!(rat.preferred_distance, Some(1));
        assert_eq!(rat.reward_boss, Some(false));
        let familiar = policies.iter().find(|p| p.is_familiar).unwrap();
        assert_eq!((familiar.maximum_health, familiar.base_speed), (10000, 154));
        assert_eq!(familiar.preferred_distance, Some(4));
        assert_eq!(
            familiar.condition_immunities,
            vec![ConditionType::Invisible, ConditionType::Paralysis]
        );
        let ProjectV2AuthoringProfileData::Creature(value) = &creatures.records[1].profile.data
        else {
            panic!("creature profile");
        };
        assert!(!value.details.as_ref().unwrap().flags.illusionable);
        assert!(
            value
                .details
                .as_ref()
                .unwrap()
                .condition_immunities
                .contains(&"invisible".into())
        );
    }
    fn object_profile_fixture() -> (CreatureProfilesDocument, PresentationProfilesDocument) {
        let supplied = input();
        let mut creatures: CreatureProfilesDocument =
            serde_json::from_slice(&supplied.creature_profiles.bytes).unwrap();
        let mut presentations: PresentationProfilesDocument =
            serde_json::from_slice(&supplied.presentation_profiles.bytes).unwrap();
        creatures.records.truncate(1);
        presentations.records.truncate(1);
        let ProjectV2AuthoringProfileData::Creature(creature) =
            &mut creatures.records[0].profile.data
        else {
            panic!("creature fixture");
        };
        creature.details.as_mut().unwrap().flags.illusionable = false;
        let ProjectV2AuthoringProfileData::Presentation(presentation) =
            &mut presentations.records[0].data
        else {
            panic!("presentation fixture");
        };
        presentation.asset_binding = Some("canary.appearance:object/2122".into());
        (creatures, presentations)
    }
    #[test]
    fn actual_object_presentation_qualifies_as_object_without_outfit_or_invisibility() {
        let (creatures, presentations) = object_profile_fixture();
        let policies = creature_policies(&creatures, &presentations).unwrap();
        let object = &policies[0];
        assert_eq!(object.outfit_look_type, 0);
        assert_eq!(object.object_look_type, Some(2122));
        assert_eq!(object.outfit_appearance(), None);
        assert!(!object.flags.illusionable);
        CompiledCreaturePolicies::from_active_artifact([1; 32], policies).unwrap();
    }
    #[test]
    fn object_presentation_rejects_noncanonical_bindings_and_conflicting_selection() {
        for binding in [
            "canary.appearance:object/0",
            "canary.appearance:object/02122",
            "canary.appearance:object/+2122",
            "canary.appearance:object/-2122",
            "canary.appearance:object/4294967296",
            "canary.appearance:object/",
            "canary.appearance:object/2122/extra",
            "crystal.appearance:object/2122",
        ] {
            let (creatures, mut presentations) = object_profile_fixture();
            let ProjectV2AuthoringProfileData::Presentation(presentation) =
                &mut presentations.records[0].data
            else {
                panic!("presentation fixture");
            };
            presentation.asset_binding = Some(binding.into());
            assert!(
                creature_policies(&creatures, &presentations).is_err(),
                "{binding}"
            );
        }
        for selection in [
            ProjectV2AppearanceSelection::OwnerFamiliarLook,
            ProjectV2AppearanceSelection::Invisible,
        ] {
            let (creatures, mut presentations) = object_profile_fixture();
            let ProjectV2AuthoringProfileData::Presentation(presentation) =
                &mut presentations.records[0].data
            else {
                panic!("presentation fixture");
            };
            presentation.selection = Some(selection);
            assert!(creature_policies(&creatures, &presentations).is_err());
        }
        for change in 0..2 {
            let (mut creatures, presentations) = object_profile_fixture();
            let ProjectV2AuthoringProfileData::Creature(creature) =
                &mut creatures.records[0].profile.data
            else {
                panic!("creature fixture");
            };
            let details = creature.details.as_mut().unwrap();
            if change == 0 {
                details.flags.illusionable = true;
            } else {
                details.summoning.is_familiar = true;
            }
            assert!(creature_policies(&creatures, &presentations).is_err());
        }
    }
    #[test]
    fn caller_artifact_activates_full_book_under_outer_pin_without_changing_baseline() {
        let world = test_source(1).unwrap().world_id;
        let baseline = qualify_native_entry_room(world).unwrap();
        let room = qualify_native_entry_room_with_gameplay(world, &input()).unwrap();
        assert_ne!(
            baseline.compiled().server_digest(),
            room.compiled().server_digest()
        );
        assert_eq!(
            baseline.compiled().client_artifact,
            room.compiled().client_artifact
        );
        assert_eq!(
            room.movement_cells().scope().generation_digest,
            room.compiled().server_digest()
        );
        assert_eq!(room.map_revision_digest(), baseline.map_revision_digest());
        assert_eq!(
            room.frame_binding().digest(),
            baseline.frame_binding().digest()
        );
        let entry = room.entry_start();
        assert!(matches!(
            room.movement_cells().spell_tiles().lookup(
                room.movement_cells().scope(),
                super::super::LogicalCell {
                    x: entry.x,
                    y: entry.y,
                    z: i32::from(entry.floor)
                }
            ),
            Err(super::super::project::SpellTileLookupError::Unknown)
        ));

        let issuance = NativeEntryActivationIssuance {
            world_id: world,
            activation_sequence: 1,
            server_artifact_digest: room.compiled().server_digest(),
            client_artifact_digest: room.compiled().client_digest(),
            frame_binding_digest: room.frame_binding().digest(),
        };
        let mut controller = ContentActivationController::new();
        let pin = activate_native_entry_room_with_gameplay(
            &mut controller,
            &NodeBootQuiescence::before_channel_runtime(),
            world,
            &issuance,
            &input(),
        )
        .unwrap();
        let active = controller.active().unwrap();
        let baseline_staged = StagedGeneration::stage(
            &baseline.compiled().server_artifact,
            &baseline.compiled().client_artifact,
            baseline.compiled().expectation(),
        )
        .unwrap();
        assert_eq!(
            active.server_record_count(),
            baseline_staged.runtime_state().server_record_count()
        );
        assert_eq!(
            decode(&room.compiled().server_artifact).unwrap().baseline,
            baseline.compiled().server_artifact.as_slice()
        );
        assert_eq!(
            pin.identity().server_artifact_digest(),
            room.compiled().server_digest()
        );
        let native = active.native_gameplay().unwrap();
        assert_eq!(
            native.source_digest(),
            pin.identity().server_artifact_digest()
        );
        assert_eq!(native.catalog().entries.len(), 246);
        for number in 1..=246 {
            assert!(
                native
                    .spell_book()
                    .source_indexed(std::num::NonZeroU32::new(number).unwrap())
                    .is_some()
            );
        }
        let classic = StagedGeneration::stage(
            &baseline.compiled().server_artifact,
            &baseline.compiled().client_artifact,
            baseline.compiled().expectation(),
        )
        .unwrap();
        assert!(classic.runtime_state().native_gameplay().is_none());
    }
    #[test]
    fn explicit_v2_items_bind_hash_and_reject_duplicate_or_invalid_authoring() {
        let world = test_source(1).unwrap().world_id;
        let baseline = qualify_native_entry_room(world).unwrap();
        let mut extended = input();
        extended.item_profiles = Some(pinned(
            br#"{"schema":"OTERYN_NATIVE_ITEM_PROFILES/v1","records":[]}"#,
        ));
        let artifact = compile_native_gameplay(baseline.compiled(), &extended).unwrap();
        assert!(artifact.server_artifact.starts_with(MAGIC_V2));
        let decoded = decode(&artifact.server_artifact).unwrap();
        assert!(decoded.state.item_policy("missing", "missing").is_none());
        assert!(decoded.state.spell_appearances().is_none());
        assert!(decoded.state.training_formula().is_none());
        let mut changed = extended.clone();
        changed.item_profiles.as_mut().unwrap().bytes.push(b' ');
        assert!(compile_native_gameplay(baseline.compiled(), &changed).is_err());
        changed.item_profiles = Some(pinned(br#"{"schema":"wrong","records":[]}"#));
        assert!(compile_native_gameplay(baseline.compiled(), &changed).is_err());
        // A later section cannot implicitly authorize omitted owner providers.
        changed.item_profiles = None;
        changed.spell_appearances = Some(pinned(b"{}"));
        assert!(compile_native_gameplay(baseline.compiled(), &changed).is_err());
    }
    #[test]
    fn existing_twenty_two_record_baseline_is_preserved_exactly_by_envelope() {
        let source = test_source(3).unwrap();
        let baseline = super::super::production::compile_first_production(
            &source,
            super::super::production::FirstProductionCompileTarget::OrdinaryRelease,
        )
        .unwrap();
        let wrapped = compile_native_gameplay(&baseline, &input()).unwrap();
        let staged = StagedGeneration::stage(
            &wrapped.server_artifact,
            &wrapped.client_artifact,
            wrapped.expectation(),
        )
        .unwrap();
        assert_eq!(staged.runtime_state().server_record_count(), 22);
        assert_eq!(staged.runtime_state().client_record_count(), 6);
        assert_eq!(
            decode(&wrapped.server_artifact).unwrap().baseline,
            baseline.server_artifact.as_slice()
        );
    }
    #[test]
    fn explicit_v4_profiles_activate_and_bind_training_revision_and_appearance() {
        let world = test_source(1).unwrap().world_id;
        let baseline = qualify_native_entry_room(world).unwrap();
        let mut supplied = input();
        supplied.item_profiles = Some(pinned(
            br#"{"schema":"OTERYN_NATIVE_ITEM_PROFILES/v1","records":[]}"#,
        ));
        supplied.spell_appearances = Some(pinned(include_bytes!(
            "../../../../tools/content-schema/native-gameplay/spell_appearances.json"
        )));
        supplied.build_training = Some(NativeTrainingInput {
            profile: pinned(include_bytes!(
                "../../../../tools/content-schema/native-gameplay/build-training.json"
            )),
            content_revision: "build-content-r1".into(),
        });
        let artifact = compile_native_gameplay(baseline.compiled(), &supplied).unwrap();
        assert!(artifact.server_artifact.starts_with(MAGIC_V4));
        let state = decode(&artifact.server_artifact).unwrap().state;
        assert_eq!(
            state.training_formula().unwrap().content_revision(),
            "build-content-r1"
        );
        let appearances = state.spell_appearances().unwrap();
        let rat = appearances
            .for_creature("canary:creature/rat", "canary-47dfd51f")
            .unwrap();
        assert_eq!(rat.look_type(), 21);
        assert_eq!(rat.source_digest(), artifact.server_digest());
        let mut changed = supplied.clone();
        changed.build_training.as_mut().unwrap().content_revision = "build-content-r2".into();
        let other = compile_native_gameplay(baseline.compiled(), &changed).unwrap();
        assert_ne!(artifact.server_digest(), other.server_digest());
        assert!(
            StagedGeneration::stage(
                &other.server_artifact,
                &other.client_artifact,
                artifact.expectation()
            )
            .is_err()
        );
        let mut profile: Value =
            serde_json::from_slice(&changed.build_training.as_ref().unwrap().profile.bytes)
                .unwrap();
        profile["magic_base"] = 999.into();
        changed.build_training.as_mut().unwrap().profile =
            pinned(&serde_json::to_vec(&profile).unwrap());
        assert!(compile_native_gameplay(baseline.compiled(), &changed).is_err());
        let mut mismatched = supplied;
        let mut creatures: Value =
            serde_json::from_slice(&mismatched.creature_profiles.bytes).unwrap();
        creatures["records"][0]["profile"]["target"]["revision"] = "source-revision-changed".into();
        mismatched.creature_profiles = pinned(&serde_json::to_vec(&creatures).unwrap());
        assert!(compile_native_gameplay(baseline.compiled(), &mismatched).is_err());
    }
    #[test]
    fn altered_payload_with_correct_inner_hash_still_fails_independent_outer_pin() {
        let world = test_source(1).unwrap().world_id;
        let baseline = qualify_native_entry_room(world).unwrap();
        let original = compile_native_gameplay(baseline.compiled(), &input()).unwrap();
        let mut changed = input();
        let mut document: Value = serde_json::from_slice(&changed.creature_profiles.bytes).unwrap();
        document["records"][0]["profile"]["target"]["revision"] =
            Value::String("canary-other-revision".into());
        changed.creature_profiles = pinned(&serde_json::to_vec(&document).unwrap());
        let altered = compile_native_gameplay(baseline.compiled(), &changed).unwrap();
        assert!(
            StagedGeneration::stage(
                &altered.server_artifact,
                &altered.client_artifact,
                original.expectation()
            )
            .is_err()
        );
        assert!(
            StagedGeneration::stage(
                &original.server_artifact,
                &original.client_artifact,
                baseline.compiled().expectation()
            )
            .is_err()
        );
    }
    #[test]
    fn explicit_v5_familiar_config_is_qualified_and_every_table_binds_outer_digest() {
        let world = test_source(1).unwrap().world_id;
        let baseline = qualify_native_entry_room(world).unwrap();
        let mut supplied = input();
        supplied.item_profiles = Some(pinned(
            br#"{"schema":"OTERYN_NATIVE_ITEM_PROFILES/v1","records":[]}"#,
        ));
        supplied.spell_appearances = Some(pinned(include_bytes!(
            "../../../../tools/content-schema/native-gameplay/spell_appearances.json"
        )));
        supplied.build_training = Some(NativeTrainingInput {
            profile: pinned(include_bytes!(
                "../../../../tools/content-schema/native-gameplay/build-training.json"
            )),
            content_revision: "build-content-r1".into(),
        });
        supplied.familiar_config = Some(pinned(include_bytes!(
            "../../../../tools/content-schema/native-gameplay/familiar-config.json"
        )));
        supplied.wheel_profile = Some(pinned(include_bytes!(
            "../../../../tools/content-schema/native-gameplay/wheel-profile.json"
        )));
        let artifact = compile_native_gameplay(baseline.compiled(), &supplied).unwrap();
        assert!(artifact.server_artifact.starts_with(MAGIC_V5));
        let mut state = decode(&artifact.server_artifact).unwrap().state;
        assert_eq!(
            state.familiar_config().unwrap().source_digest(),
            artifact.server_digest()
        );
        assert_eq!(state.familiar_config().unwrap().familiar_minutes(), 30);
        assert!(state.bind_qualified_outer_artifact([0; 32]).is_err());
        state.bind_qualified_outer_artifact([9; 32]).unwrap();
        assert_eq!(state.source_digest(), [9; 32]);
        assert_eq!(state.wheel_profile().unwrap().source_digest(), [9; 32]);
        assert_eq!(state.familiar_config().unwrap().source_digest(), [9; 32]);
        assert_eq!(state.spell_appearances().unwrap().source_digest(), [9; 32]);
        let mut changed: Value =
            serde_json::from_slice(&supplied.familiar_config.as_ref().unwrap().bytes).unwrap();
        changed["familiar_minutes"] = 31.into();
        supplied.familiar_config = Some(pinned(&serde_json::to_vec(&changed).unwrap()));
        assert!(compile_native_gameplay(baseline.compiled(), &supplied).is_err());
        supplied = input();
        supplied.source_world = Some(pinned(b"source-qualified-world"));
        supplied.source_world.as_mut().unwrap().sha256 = "0".repeat(64);
        assert!(compile_native_gameplay(baseline.compiled(), &supplied).is_err());
    }
    #[test]
    fn substituted_source_revision_and_incomplete_creature_data_are_rejected() {
        let world = test_source(1).unwrap().world_id;
        let baseline = qualify_native_entry_room(world).unwrap();
        let mut changed = input();
        let mut selection: Value = serde_json::from_slice(&changed.source_selection.bytes).unwrap();
        selection["selections"][0]["source_proofs"][0]["revision"] =
            Value::String("0000000000000000000000000000000000000000".into());
        changed.source_selection = pinned(&serde_json::to_vec(&selection).unwrap());
        assert!(compile_native_gameplay(baseline.compiled(), &changed).is_err());
        for field in ["health", "speed", "details"] {
            let mut changed = input();
            let mut document: Value =
                serde_json::from_slice(&changed.creature_profiles.bytes).unwrap();
            document["records"][0]["profile"]["data"]["profile"]
                .as_object_mut()
                .unwrap()
                .remove(field);
            changed.creature_profiles = pinned(&serde_json::to_vec(&document).unwrap());
            assert!(
                compile_native_gameplay(baseline.compiled(), &changed).is_err(),
                "missing {field}"
            );
        }
        let mut changed = input();
        let mut presentations: Value =
            serde_json::from_slice(&changed.presentation_profiles.bytes).unwrap();
        presentations["records"][0]["target"]["revision"] =
            Value::String("canary-other-revision".into());
        changed.presentation_profiles = pinned(&serde_json::to_vec(&presentations).unwrap());
        assert!(compile_native_gameplay(baseline.compiled(), &changed).is_err());
    }
    #[test]
    fn provisioning_reads_caller_files_and_rejects_partial_or_stale_pins() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/content-schema/native-gameplay");
        let loaded = NativeGameplayInput::from_manifest(&directory.join("manifest.json")).unwrap();
        assert_eq!(loaded.catalog.bytes, input().catalog.bytes);
        let incomplete: Result<ProvisioningManifest, _> = serde_json::from_slice(br#"{"schema":"OTERYN_NATIVE_GAMEPLAY_MANIFEST/v1","catalog":{"path":"catalog.json","sha256":"bad"}}"#);
        assert!(incomplete.is_err());
        let mut wrong_pin = loaded.catalog;
        wrong_pin.sha256 = "0".repeat(64);
        assert!(qualify(&wrong_pin, MAX_CATALOG).is_err());
    }
}

impl crate::durability::equipment_policy_abi::policy_registration::Registered
    for NativeGameplayState
{
}
impl crate::durability::equipment_policy_abi::EquipmentPolicyLookup for NativeGameplayState {
    fn source_digest(&self) -> [u8; 32] {
        self.source_digest
    }
    fn equipment_policy(
        &self,
        key: &str,
        revision: &str,
    ) -> Option<crate::durability::equipment_policy_abi::QualifiedEquipmentPolicy> {
        use super::ReferenceItemField;
        let policy = self.item_policy(key, revision)?;
        let semantics = &policy.record().semantics;
        let claims = [1, 2, 3, 4, 5, 6, 7, 8, 10]
            .into_iter()
            .map(|slot| (slot, projected_equipment_claims(semantics, slot)))
            .collect();
        let capacity = match &semantics.container {
            ReferenceItemField::Known(container) => match container.capacity {
                ReferenceItemField::Known(capacity) => Some(capacity),
                _ => None,
            },
            _ => None,
        };
        crate::durability::equipment_policy_abi::QualifiedEquipmentPolicy::from_qualified_content(
            self.source_digest,
            claims,
            capacity,
        )
    }
}

fn projected_equipment_claims(
    semantics: &super::ReferenceItemSemantics,
    slot: u8,
) -> Result<crate::durability::equipment_policy_abi::EquipmentClaims, &'static str> {
    use super::{ReferenceBaseVocation, ReferenceEquipmentSlot, ReferenceItemField};
    use crate::durability::equipment_policy_abi::{EquipmentBaseVocation, EquipmentClaims};
    let rejected = |s| s;
    let ReferenceItemField::Known(equipment) = &semantics.equipment else {
        return Err(rejected("unknown equipment pattern"));
    };
    let ReferenceItemField::Known(patterns) = &equipment.patterns else {
        return Err(rejected("unknown equipment patterns"));
    };
    let expected = match slot {
        1 => ReferenceEquipmentSlot::Head,
        2 => ReferenceEquipmentSlot::Torso,
        3 => ReferenceEquipmentSlot::Legs,
        4 => ReferenceEquipmentSlot::Feet,
        5 => ReferenceEquipmentSlot::Weapon,
        6 => ReferenceEquipmentSlot::Shield,
        7 => ReferenceEquipmentSlot::Amulet,
        8 => ReferenceEquipmentSlot::Ring,
        10 => ReferenceEquipmentSlot::Extra,
        _ => return Err(rejected("equipment slot")),
    };
    let mut matching = patterns
        .iter()
        .filter(|p| p.primary_slot == ReferenceItemField::Known(expected));
    let pattern = matching
        .next()
        .ok_or(rejected("equipment slot incompatible with Content"))?;
    if matching.next().is_some() {
        return Err(rejected("equipment ambiguous source pattern"));
    }
    let ReferenceItemField::Known(additional) = &pattern.additional_reserved_slots else {
        return Err(rejected("unknown equipment reservations"));
    };
    let ReferenceItemField::Known(groups) = &pattern.mutually_exclusive_groups else {
        return Err(rejected("unknown equipment groups"));
    };
    let ReferenceItemField::Known(level) = pattern.level else {
        return Err(rejected("unknown equipment required level"));
    };
    let ReferenceItemField::Known(vocations) = &pattern.vocations else {
        return Err(rejected("unknown equipment required vocations"));
    };
    let mut occupied = vec![slot];
    for value in additional {
        occupied.push(match value {
            ReferenceEquipmentSlot::Head => 1,
            ReferenceEquipmentSlot::Torso => 2,
            ReferenceEquipmentSlot::Legs => 3,
            ReferenceEquipmentSlot::Feet => 4,
            ReferenceEquipmentSlot::Weapon => 5,
            ReferenceEquipmentSlot::Shield => 6,
            ReferenceEquipmentSlot::Amulet => 7,
            ReferenceEquipmentSlot::Ring => 8,
            ReferenceEquipmentSlot::Extra => 10,
            ReferenceEquipmentSlot::Container => {
                return Err(rejected("equipment container reservation"));
            }
        });
    }
    occupied.sort();
    occupied.dedup();
    Ok(EquipmentClaims {
        slots: occupied,
        groups: groups.iter().map(|g| g.as_str().to_owned()).collect(),
        level,
        vocations: vocations
            .iter()
            .map(|v| match v {
                ReferenceBaseVocation::Druid => EquipmentBaseVocation::Druid,
                ReferenceBaseVocation::Knight => EquipmentBaseVocation::Knight,
                ReferenceBaseVocation::Monk => EquipmentBaseVocation::Monk,
                ReferenceBaseVocation::Paladin => EquipmentBaseVocation::Paladin,
                ReferenceBaseVocation::Sorcerer => EquipmentBaseVocation::Sorcerer,
            })
            .collect(),
    })
}

#[cfg(test)]
mod equipment_projection_tests {
    use super::*;
    use crate::content::{
        ReferenceBaseVocation, ReferenceEquipmentPattern, ReferenceEquipmentSlot,
        ReferenceItemEquipment, ReferenceItemField, ReferenceItemSemantics,
    };
    #[test]
    fn producer_preserves_source_reservations_requirements_and_unknown_ambiguity() {
        use ReferenceItemField::{Known, Unknown};
        let mut semantics: ReferenceItemSemantics =
            serde_json::from_str("{}").expect("unknown semantics");
        assert!(projected_equipment_claims(&semantics, 5).is_err());
        let pattern = ReferenceEquipmentPattern {
            pattern_id: 1,
            primary_slot: Known(ReferenceEquipmentSlot::Weapon),
            additional_reserved_slots: Known(vec![ReferenceEquipmentSlot::Shield]),
            mutually_exclusive_groups: Known(vec![]),
            vocations: Known(vec![ReferenceBaseVocation::Paladin]),
            level: Known(20),
            compatibility_rule: Unknown,
        };
        semantics.equipment = Known(ReferenceItemEquipment {
            patterns: Known(vec![pattern.clone()]),
        });
        let claims = projected_equipment_claims(&semantics, 5).expect("qualified weapon");
        assert_eq!(claims.slots, vec![5, 6]);
        assert_eq!(claims.level, 20);
        assert_eq!(
            claims.vocations,
            vec![crate::durability::equipment_policy_abi::EquipmentBaseVocation::Paladin]
        );
        assert!(projected_equipment_claims(&semantics, 6).is_err());
        let mut incomplete = pattern.clone();
        incomplete.level = Unknown;
        semantics.equipment = Known(ReferenceItemEquipment {
            patterns: Known(vec![incomplete]),
        });
        assert!(projected_equipment_claims(&semantics, 5).is_err());
        semantics.equipment = Known(ReferenceItemEquipment {
            patterns: Known(vec![pattern.clone(), pattern]),
        });
        assert_eq!(
            projected_equipment_claims(&semantics, 5).err(),
            Some("equipment ambiguous source pattern")
        );
    }
}
