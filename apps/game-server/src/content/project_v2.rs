//! Canonical declarative WorldProject/v2 source carrier.
//!
//! V2 is an authoring/source schema. It deliberately reuses the existing
//! ProjectReferenceRecord for already-accepted executable Reference families and
//! keeps newer/deferred authoring families declarative until their runtime owners
//! accept a lowering contract.

use super::{DefinitionRevisionRef, ProductionKey, ProjectReferenceRecord};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};

pub const WORLD_PROJECT_V2_SOURCE_PROFILE: &str = "OTERYN_WORLD_PROJECT_SOURCE_PROFILE/v2";
pub const WORLD_PROJECT_V2_SCHEMA: &str = "OTERYN_WORLD_PROJECT/v2";

const MAX_ALIAS_COUNT: usize = 16;
const MAX_TAG_COUNT: usize = 32;
const MAX_TAXONOMY_BYTES: usize = 128;
const MAX_ALIAS_BYTES: usize = 128;
const MAX_TAG_BYTES: usize = 96;
const MAX_PERK_LEVELS: usize = 32;
const MAX_PERKS_PER_LEVEL: usize = 16;
const MAX_RANK_VALUES: usize = 32;
const MAX_ITEM_RELATIONSHIPS: usize = 32;
const MAX_DIALOGUE_NODES: usize = 512;
const MAX_DIALOGUE_EDGES: usize = 1024;
const MAX_SERVICE_OFFERS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldProjectV2Error {
    InvalidSchema,
    InvalidIdentity,
    DuplicateIdentity(String),
    DuplicatePlacement(String),
    DuplicateTransition(String),
    InvalidReference(&'static str),
    InvalidItemOverlay(&'static str),
    InvalidForgeProfile,
    InvalidProficiencyProfile(&'static str),
    InvalidAugment(&'static str),
    InvalidInteraction(&'static str),
    InvalidRelationship(&'static str),
    InvalidBound(&'static str),
}

impl Display for WorldProjectV2Error {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSchema => write!(formatter, "invalid WorldProject/v2 schema"),
            Self::InvalidIdentity => write!(formatter, "invalid WorldProject/v2 identity"),
            Self::DuplicateIdentity(value) => {
                write!(formatter, "duplicate WorldProject/v2 identity: {value}")
            }
            Self::DuplicatePlacement(value) => {
                write!(formatter, "duplicate WorldProject/v2 placement: {value}")
            }
            Self::DuplicateTransition(value) => {
                write!(formatter, "duplicate WorldProject/v2 transition: {value}")
            }
            Self::InvalidReference(value) => write!(formatter, "invalid v2 reference: {value}"),
            Self::InvalidItemOverlay(value) => {
                write!(formatter, "invalid v2 Item overlay: {value}")
            }
            Self::InvalidForgeProfile => write!(formatter, "invalid v2 Forge profile"),
            Self::InvalidProficiencyProfile(value) => {
                write!(formatter, "invalid v2 proficiency profile: {value}")
            }
            Self::InvalidAugment(value) => write!(formatter, "invalid v2 augment: {value}"),
            Self::InvalidInteraction(value) => {
                write!(formatter, "invalid v2 interaction: {value}")
            }
            Self::InvalidRelationship(value) => {
                write!(formatter, "invalid v2 relationship: {value}")
            }
            Self::InvalidBound(value) => write!(formatter, "invalid v2 bound: {value}"),
        }
    }
}

impl std::error::Error for WorldProjectV2Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorldProjectV2Family {
    Terrain,
    Presentation,
    Asset,
    LocalObject,
    Creature,
    Item,
    Loot,
    Ability,
    Effect,
    Formula,
    Behavior,
    Npc,
    Dialogue,
    Service,
    Interaction,
    Quest,
    House,
    Encounter,
    Proficiency,
    Augment,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldProjectV2Reference {
    pub family: WorldProjectV2Family,
    pub key: String,
    pub revision: String,
}

impl WorldProjectV2Reference {
    fn validate(&self) -> Result<(), WorldProjectV2Error> {
        ProductionKey::new(&self.key).map_err(|_| WorldProjectV2Error::InvalidIdentity)?;
        DefinitionRevisionRef::new(&self.revision)
            .map_err(|_| WorldProjectV2Error::InvalidIdentity)?;
        Ok(())
    }

    fn require(&self, family: WorldProjectV2Family) -> Result<(), WorldProjectV2Error> {
        self.validate()?;
        if self.family != family {
            return Err(WorldProjectV2Error::InvalidReference(
                "family does not match typed field",
            ));
        }
        Ok(())
    }

    fn identity_key(&self) -> String {
        format!("{:?}:{}", self.family, self.key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ItemPresentationAuthoringV2 {
    #[serde(default)]
    pub appearance: Option<WorldProjectV2Reference>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub taxonomy: Option<ItemTaxonomyV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemTaxonomyV2 {
    pub primary: String,
    #[serde(default)]
    pub secondary: Option<String>,
    #[serde(default)]
    pub tertiary: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemForgeProfileV2 {
    pub classification: u8,
    pub max_tier: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ItemUseRequirementsV2 {
    #[serde(default)]
    pub required_magic_level: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ItemLifecycleAuthoringV2 {
    #[serde(default)]
    pub enchantable: Option<bool>,
    #[serde(default)]
    pub destructible: Option<bool>,
    #[serde(default)]
    pub enchant_interactions: Vec<WorldProjectV2Reference>,
    #[serde(default)]
    pub destroy_interactions: Vec<WorldProjectV2Reference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct SourceLifecycleV2 {
    #[serde(default)]
    pub implemented: Option<String>,
    #[serde(default)]
    pub removed: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAuthoringV2 {
    pub item: WorldProjectV2Reference,
    #[serde(default)]
    pub presentation: ItemPresentationAuthoringV2,
    #[serde(default)]
    pub forge: Option<ItemForgeProfileV2>,
    #[serde(default)]
    pub proficiency_profile: Option<WorldProjectV2Reference>,
    #[serde(default)]
    pub augment_bindings: Vec<WorldProjectV2Reference>,
    #[serde(default)]
    pub on_use_interactions: Vec<WorldProjectV2Reference>,
    #[serde(default)]
    pub use_requirements: ItemUseRequirementsV2,
    #[serde(default)]
    pub lifecycle: ItemLifecycleAuthoringV2,
    #[serde(default)]
    pub source_lifecycle: SourceLifecycleV2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProficiencyLevelV2 {
    pub level: u8,
    #[serde(default)]
    pub perks: Vec<WorldProjectV2Reference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerkShapingV2 {
    pub max_rank: u8,
    pub replace_slots: u8,
    pub refine_enabled: bool,
    pub reshape_enabled: bool,
    pub clear_enabled: bool,
    pub lunar_ascension_enabled: bool,
    #[serde(default)]
    pub cost_service: Option<WorldProjectV2Reference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProficiencyDefinitionV2 {
    pub identity: WorldProjectV2Reference,
    pub weapon_item: WorldProjectV2Reference,
    pub levels: Vec<ProficiencyLevelV2>,
    #[serde(default)]
    pub shaping: Option<PerkShapingV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum AugmentTargetV2 {
    Ability { ability: WorldProjectV2Reference },
    AutoAttack,
    OffensiveRune,
    CreatureClass { class_key: String },
    Generic { target_key: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AugmentRankValueV2 {
    pub rank: u8,
    pub numerator: i64,
    pub denominator: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AugmentDefinitionV2 {
    pub identity: WorldProjectV2Reference,
    pub target: AugmentTargetV2,
    pub effect: WorldProjectV2Reference,
    #[serde(default)]
    pub rank_values: Vec<AugmentRankValueV2>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InteractionTriggerV2 {
    Use,
    UseWith,
    Open,
    Close,
    OpenContainer,
    Read,
    Write,
    Rotate,
    Wrap,
    Unwrap,
    Sleep,
    StepIn,
    StepOut,
    AddItem,
    RemoveItem,
    Equip,
    Deequip,
    Pickup,
    Move,
    Teleport,
    FloorTransition,
    Hang,
    NpcTalk,
    Service,
    QuestTrigger,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum InteractionExecutionV2 {
    Ability {
        ability: WorldProjectV2Reference,
    },
    Effect {
        effect: WorldProjectV2Reference,
    },
    Service {
        service: WorldProjectV2Reference,
    },
    Quest {
        quest: WorldProjectV2Reference,
    },
    TransformItem {
        from: WorldProjectV2Reference,
        to: WorldProjectV2Reference,
    },
    NativeRule {
        key: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InteractionDefinitionV2 {
    pub identity: WorldProjectV2Reference,
    pub trigger: InteractionTriggerV2,
    pub execution: InteractionExecutionV2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ServiceOfferDirectionV2 {
    BuyFromPlayer,
    SellToPlayer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceOfferV2 {
    pub item: WorldProjectV2Reference,
    pub direction: ServiceOfferDirectionV2,
    pub unit_price: u64,
    #[serde(default)]
    pub currency: Option<WorldProjectV2Reference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceDefinitionV2 {
    pub identity: WorldProjectV2Reference,
    #[serde(default)]
    pub offers: Vec<ServiceOfferV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcDefinitionV2 {
    pub identity: WorldProjectV2Reference,
    #[serde(default)]
    pub dialogues: Vec<WorldProjectV2Reference>,
    #[serde(default)]
    pub services: Vec<WorldProjectV2Reference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueNodeV2 {
    pub key: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueEdgeV2 {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueDefinitionV2 {
    pub identity: WorldProjectV2Reference,
    #[serde(default)]
    pub nodes: Vec<DialogueNodeV2>,
    #[serde(default)]
    pub edges: Vec<DialogueEdgeV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestDefinitionV2 {
    pub identity: WorldProjectV2Reference,
    #[serde(default)]
    pub stages: Vec<String>,
    #[serde(default)]
    pub objectives: Vec<String>,
    #[serde(default)]
    pub transitions: Vec<String>,
    #[serde(default)]
    pub rewards: Vec<WorldProjectV2Reference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HouseDefinitionV2 {
    pub identity: WorldProjectV2Reference,
    #[serde(default)]
    pub entry_interactions: Vec<WorldProjectV2Reference>,
    #[serde(default)]
    pub exit_interactions: Vec<WorldProjectV2Reference>,
    #[serde(default)]
    pub access_policy_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterDefinitionV2 {
    pub identity: WorldProjectV2Reference,
    #[serde(default)]
    pub participants: Vec<WorldProjectV2Reference>,
    #[serde(default)]
    pub trigger_interactions: Vec<WorldProjectV2Reference>,
    #[serde(default)]
    pub loot: Vec<WorldProjectV2Reference>,
    #[serde(default)]
    pub reset_policy_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetDefinitionV2 {
    pub identity: WorldProjectV2Reference,
    pub locator: String,
    #[serde(default)]
    pub content_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldPlacementV2 {
    pub placement_key: String,
    pub definition: WorldProjectV2Reference,
    pub world_key: String,
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldPositionV2 {
    pub world_key: String,
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldTransitionV2 {
    pub transition_key: String,
    pub from: WorldPositionV2,
    pub to: WorldPositionV2,
    #[serde(default)]
    pub interaction: Option<WorldProjectV2Reference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceRecordV2 {
    pub entity: WorldProjectV2Reference,
    pub field_selector: String,
    pub value_digest: String,
    pub source_id: String,
    pub source_revision: String,
    #[serde(default)]
    pub target_cut: Option<String>,
    pub observation_classification: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditorMetadataV2 {
    pub entity: WorldProjectV2Reference,
    #[serde(default)]
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldProjectV2 {
    pub schema: String,
    #[serde(default)]
    pub reference_records: Vec<ProjectReferenceRecord>,
    #[serde(default)]
    pub items: Vec<ItemAuthoringV2>,
    #[serde(default)]
    pub proficiencies: Vec<ProficiencyDefinitionV2>,
    #[serde(default)]
    pub augments: Vec<AugmentDefinitionV2>,
    #[serde(default)]
    pub interactions: Vec<InteractionDefinitionV2>,
    #[serde(default)]
    pub npcs: Vec<NpcDefinitionV2>,
    #[serde(default)]
    pub dialogues: Vec<DialogueDefinitionV2>,
    #[serde(default)]
    pub services: Vec<ServiceDefinitionV2>,
    #[serde(default)]
    pub quests: Vec<QuestDefinitionV2>,
    #[serde(default)]
    pub houses: Vec<HouseDefinitionV2>,
    #[serde(default)]
    pub encounters: Vec<EncounterDefinitionV2>,
    #[serde(default)]
    pub assets: Vec<AssetDefinitionV2>,
    #[serde(default)]
    pub placements: Vec<WorldPlacementV2>,
    #[serde(default)]
    pub transitions: Vec<WorldTransitionV2>,
    #[serde(default)]
    pub provenance: Vec<ProvenanceRecordV2>,
    #[serde(default)]
    pub editor: Vec<EditorMetadataV2>,
}

impl WorldProjectV2 {
    pub fn validate(&self) -> Result<(), WorldProjectV2Error> {
        if self.schema != WORLD_PROJECT_V2_SCHEMA {
            return Err(WorldProjectV2Error::InvalidSchema);
        }

        let mut identities = BTreeSet::new();
        for record in &self.reference_records {
            let (family, key, revision) = reference_identity(record);
            validate_reference_identity(family, key, revision)?;
            insert_identity(&mut identities, format!("REFERENCE:{family}:{key}"))?;
        }

        for item in &self.items {
            item.validate()?;
            insert_identity(
                &mut identities,
                format!("ITEM_OVERLAY:{}", item.item.identity_key()),
            )?;
        }
        for value in &self.proficiencies {
            value.validate()?;
            insert_identity(&mut identities, value.identity.identity_key())?;
        }
        for value in &self.augments {
            value.validate()?;
            insert_identity(&mut identities, value.identity.identity_key())?;
        }
        for value in &self.interactions {
            value.validate()?;
            insert_identity(&mut identities, value.identity.identity_key())?;
        }
        for value in &self.npcs {
            value.validate()?;
            insert_identity(&mut identities, value.identity.identity_key())?;
        }
        for value in &self.dialogues {
            value.validate()?;
            insert_identity(&mut identities, value.identity.identity_key())?;
        }
        for value in &self.services {
            value.validate()?;
            insert_identity(&mut identities, value.identity.identity_key())?;
        }
        for value in &self.quests {
            value.validate()?;
            insert_identity(&mut identities, value.identity.identity_key())?;
        }
        for value in &self.houses {
            value.validate()?;
            insert_identity(&mut identities, value.identity.identity_key())?;
        }
        for value in &self.encounters {
            value.validate()?;
            insert_identity(&mut identities, value.identity.identity_key())?;
        }
        for value in &self.assets {
            value.validate()?;
            insert_identity(&mut identities, value.identity.identity_key())?;
        }

        let mut placements = BTreeSet::new();
        for value in &self.placements {
            validate_key(&value.placement_key)?;
            value.definition.validate()?;
            validate_key(&value.world_key)?;
            if !placements.insert(value.placement_key.clone()) {
                return Err(WorldProjectV2Error::DuplicatePlacement(
                    value.placement_key.clone(),
                ));
            }
        }

        let mut transitions = BTreeSet::new();
        for value in &self.transitions {
            validate_key(&value.transition_key)?;
            validate_key(&value.from.world_key)?;
            validate_key(&value.to.world_key)?;
            if let Some(interaction) = &value.interaction {
                interaction.require(WorldProjectV2Family::Interaction)?;
            }
            if !transitions.insert(value.transition_key.clone()) {
                return Err(WorldProjectV2Error::DuplicateTransition(
                    value.transition_key.clone(),
                ));
            }
        }

        for value in &self.provenance {
            value.entity.validate()?;
            validate_nonempty_text(&value.field_selector, 256)?;
            validate_nonempty_text(&value.value_digest, 128)?;
            validate_nonempty_text(&value.source_id, 256)?;
            validate_nonempty_text(&value.source_revision, 256)?;
            validate_nonempty_text(&value.observation_classification, 128)?;
        }
        for value in &self.editor {
            value.entity.validate()?;
            validate_text_set(&value.labels, MAX_TAG_COUNT, MAX_TAG_BYTES)?;
        }
        Ok(())
    }
}

impl ItemAuthoringV2 {
    fn validate(&self) -> Result<(), WorldProjectV2Error> {
        self.item.require(WorldProjectV2Family::Item)?;
        if let Some(appearance) = &self.presentation.appearance {
            appearance.require(WorldProjectV2Family::Asset)?;
        }
        validate_text_set(&self.presentation.aliases, MAX_ALIAS_COUNT, MAX_ALIAS_BYTES)?;
        validate_text_set(&self.presentation.tags, MAX_TAG_COUNT, MAX_TAG_BYTES)?;
        if let Some(taxonomy) = &self.presentation.taxonomy {
            validate_nonempty_text(&taxonomy.primary, MAX_TAXONOMY_BYTES)?;
            if let Some(value) = &taxonomy.secondary {
                validate_nonempty_text(value, MAX_TAXONOMY_BYTES)?;
            }
            if let Some(value) = &taxonomy.tertiary {
                validate_nonempty_text(value, MAX_TAXONOMY_BYTES)?;
            }
        }
        if let Some(forge) = self.forge
            && (!(1..=4).contains(&forge.classification) || forge.max_tier == 0)
        {
            return Err(WorldProjectV2Error::InvalidForgeProfile);
        }
        if let Some(profile) = &self.proficiency_profile {
            profile.require(WorldProjectV2Family::Proficiency)?;
        }
        validate_ref_family_set(
            &self.augment_bindings,
            WorldProjectV2Family::Augment,
            MAX_ITEM_RELATIONSHIPS,
        )?;
        validate_ref_family_set(
            &self.on_use_interactions,
            WorldProjectV2Family::Interaction,
            MAX_ITEM_RELATIONSHIPS,
        )?;
        validate_ref_family_set(
            &self.lifecycle.enchant_interactions,
            WorldProjectV2Family::Interaction,
            MAX_ITEM_RELATIONSHIPS,
        )?;
        validate_ref_family_set(
            &self.lifecycle.destroy_interactions,
            WorldProjectV2Family::Interaction,
            MAX_ITEM_RELATIONSHIPS,
        )?;
        if let Some(value) = &self.source_lifecycle.implemented {
            validate_nonempty_text(value, 64)?;
        }
        if let Some(value) = &self.source_lifecycle.removed {
            validate_nonempty_text(value, 64)?;
        }
        Ok(())
    }
}

impl ProficiencyDefinitionV2 {
    fn validate(&self) -> Result<(), WorldProjectV2Error> {
        self.identity.require(WorldProjectV2Family::Proficiency)?;
        self.weapon_item.require(WorldProjectV2Family::Item)?;
        if self.levels.is_empty() || self.levels.len() > MAX_PERK_LEVELS {
            return Err(WorldProjectV2Error::InvalidProficiencyProfile(
                "level count",
            ));
        }
        let mut levels = BTreeSet::new();
        for level in &self.levels {
            if level.level == 0 || !levels.insert(level.level) {
                return Err(WorldProjectV2Error::InvalidProficiencyProfile(
                    "duplicate/zero level",
                ));
            }
            validate_ref_family_set(
                &level.perks,
                WorldProjectV2Family::Augment,
                MAX_PERKS_PER_LEVEL,
            )?;
        }
        if let Some(shaping) = &self.shaping {
            if shaping.max_rank == 0 || shaping.max_rank > 32 || shaping.replace_slots > 8 {
                return Err(WorldProjectV2Error::InvalidProficiencyProfile(
                    "invalid shaping bounds",
                ));
            }
            if let Some(service) = &shaping.cost_service {
                service.require(WorldProjectV2Family::Service)?;
            }
        }
        Ok(())
    }
}

impl AugmentDefinitionV2 {
    fn validate(&self) -> Result<(), WorldProjectV2Error> {
        self.identity.require(WorldProjectV2Family::Augment)?;
        self.effect.require(WorldProjectV2Family::Effect)?;
        match &self.target {
            AugmentTargetV2::Ability { ability } => {
                ability.require(WorldProjectV2Family::Ability)?;
            }
            AugmentTargetV2::CreatureClass { class_key }
            | AugmentTargetV2::Generic {
                target_key: class_key,
            } => validate_key(class_key)?,
            AugmentTargetV2::AutoAttack | AugmentTargetV2::OffensiveRune => {}
        }
        if self.rank_values.len() > MAX_RANK_VALUES {
            return Err(WorldProjectV2Error::InvalidAugment("rank value count"));
        }
        let mut ranks = BTreeSet::new();
        for value in &self.rank_values {
            if value.denominator == 0 || !ranks.insert(value.rank) {
                return Err(WorldProjectV2Error::InvalidAugment(
                    "duplicate rank or zero denominator",
                ));
            }
        }
        Ok(())
    }
}

impl InteractionDefinitionV2 {
    fn validate(&self) -> Result<(), WorldProjectV2Error> {
        self.identity.require(WorldProjectV2Family::Interaction)?;
        match &self.execution {
            InteractionExecutionV2::Ability { ability } => {
                ability.require(WorldProjectV2Family::Ability)?;
            }
            InteractionExecutionV2::Effect { effect } => {
                effect.require(WorldProjectV2Family::Effect)?;
            }
            InteractionExecutionV2::Service { service } => {
                service.require(WorldProjectV2Family::Service)?;
            }
            InteractionExecutionV2::Quest { quest } => {
                quest.require(WorldProjectV2Family::Quest)?;
            }
            InteractionExecutionV2::TransformItem { from, to } => {
                from.require(WorldProjectV2Family::Item)?;
                to.require(WorldProjectV2Family::Item)?;
            }
            InteractionExecutionV2::NativeRule { key } => validate_key(key)?,
        }
        Ok(())
    }
}

impl ServiceDefinitionV2 {
    fn validate(&self) -> Result<(), WorldProjectV2Error> {
        self.identity.require(WorldProjectV2Family::Service)?;
        if self.offers.len() > MAX_SERVICE_OFFERS {
            return Err(WorldProjectV2Error::InvalidBound("service offers"));
        }
        for offer in &self.offers {
            offer.item.require(WorldProjectV2Family::Item)?;
            if let Some(currency) = &offer.currency {
                currency.require(WorldProjectV2Family::Item)?;
            }
        }
        Ok(())
    }
}

impl NpcDefinitionV2 {
    fn validate(&self) -> Result<(), WorldProjectV2Error> {
        self.identity.require(WorldProjectV2Family::Npc)?;
        validate_ref_family_set(
            &self.dialogues,
            WorldProjectV2Family::Dialogue,
            MAX_ITEM_RELATIONSHIPS,
        )?;
        validate_ref_family_set(
            &self.services,
            WorldProjectV2Family::Service,
            MAX_ITEM_RELATIONSHIPS,
        )?;
        Ok(())
    }
}

impl DialogueDefinitionV2 {
    fn validate(&self) -> Result<(), WorldProjectV2Error> {
        self.identity.require(WorldProjectV2Family::Dialogue)?;
        if self.nodes.len() > MAX_DIALOGUE_NODES || self.edges.len() > MAX_DIALOGUE_EDGES {
            return Err(WorldProjectV2Error::InvalidBound("dialogue graph"));
        }
        let mut nodes = BTreeSet::new();
        for node in &self.nodes {
            validate_key(&node.key)?;
            validate_nonempty_text(&node.text, 4096)?;
            if !nodes.insert(node.key.clone()) {
                return Err(WorldProjectV2Error::InvalidRelationship(
                    "duplicate dialogue node",
                ));
            }
        }
        for edge in &self.edges {
            if !nodes.contains(&edge.from) || !nodes.contains(&edge.to) {
                return Err(WorldProjectV2Error::InvalidRelationship(
                    "dialogue edge references missing node",
                ));
            }
        }
        Ok(())
    }
}

impl QuestDefinitionV2 {
    fn validate(&self) -> Result<(), WorldProjectV2Error> {
        self.identity.require(WorldProjectV2Family::Quest)?;
        validate_unique_keys(&self.stages)?;
        validate_unique_keys(&self.objectives)?;
        validate_unique_keys(&self.transitions)?;
        for reward in &self.rewards {
            reward.validate()?;
        }
        Ok(())
    }
}

impl HouseDefinitionV2 {
    fn validate(&self) -> Result<(), WorldProjectV2Error> {
        self.identity.require(WorldProjectV2Family::House)?;
        validate_ref_family_set(
            &self.entry_interactions,
            WorldProjectV2Family::Interaction,
            MAX_ITEM_RELATIONSHIPS,
        )?;
        validate_ref_family_set(
            &self.exit_interactions,
            WorldProjectV2Family::Interaction,
            MAX_ITEM_RELATIONSHIPS,
        )?;
        if let Some(key) = &self.access_policy_key {
            validate_key(key)?;
        }
        Ok(())
    }
}

impl EncounterDefinitionV2 {
    fn validate(&self) -> Result<(), WorldProjectV2Error> {
        self.identity.require(WorldProjectV2Family::Encounter)?;
        for participant in &self.participants {
            participant.require(WorldProjectV2Family::Creature)?;
        }
        validate_ref_family_set(
            &self.trigger_interactions,
            WorldProjectV2Family::Interaction,
            MAX_ITEM_RELATIONSHIPS,
        )?;
        for loot in &self.loot {
            loot.require(WorldProjectV2Family::Loot)?;
        }
        if let Some(key) = &self.reset_policy_key {
            validate_key(key)?;
        }
        Ok(())
    }
}

impl AssetDefinitionV2 {
    fn validate(&self) -> Result<(), WorldProjectV2Error> {
        self.identity.require(WorldProjectV2Family::Asset)?;
        validate_nonempty_text(&self.locator, 1024)?;
        if let Some(value) = &self.content_sha256
            && (value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            return Err(WorldProjectV2Error::InvalidRelationship(
                "asset digest is not sha256 hex",
            ));
        }
        Ok(())
    }
}

fn validate_ref_family_set(
    refs: &[WorldProjectV2Reference],
    family: WorldProjectV2Family,
    max: usize,
) -> Result<(), WorldProjectV2Error> {
    if refs.len() > max {
        return Err(WorldProjectV2Error::InvalidBound("reference collection"));
    }
    let mut unique = BTreeSet::new();
    for value in refs {
        value.require(family)?;
        if !unique.insert(value.clone()) {
            return Err(WorldProjectV2Error::InvalidRelationship(
                "duplicate typed reference",
            ));
        }
    }
    Ok(())
}

fn validate_text_set(
    values: &[String],
    max_count: usize,
    max_bytes: usize,
) -> Result<(), WorldProjectV2Error> {
    if values.len() > max_count {
        return Err(WorldProjectV2Error::InvalidBound("text set count"));
    }
    let mut unique = BTreeSet::new();
    for value in values {
        validate_nonempty_text(value, max_bytes)?;
        if !unique.insert(value.clone()) {
            return Err(WorldProjectV2Error::InvalidRelationship(
                "duplicate text value",
            ));
        }
    }
    Ok(())
}

fn validate_nonempty_text(value: &str, max_bytes: usize) -> Result<(), WorldProjectV2Error> {
    if value.trim().is_empty() || value.len() > max_bytes {
        return Err(WorldProjectV2Error::InvalidBound("text bytes"));
    }
    Ok(())
}

fn validate_key(value: &str) -> Result<(), WorldProjectV2Error> {
    ProductionKey::new(value)
        .map(|_| ())
        .map_err(|_| WorldProjectV2Error::InvalidIdentity)
}

fn validate_unique_keys(values: &[String]) -> Result<(), WorldProjectV2Error> {
    let mut unique = BTreeSet::new();
    for value in values {
        validate_key(value)?;
        if !unique.insert(value.clone()) {
            return Err(WorldProjectV2Error::InvalidRelationship(
                "duplicate stable key",
            ));
        }
    }
    Ok(())
}

fn insert_identity(
    identities: &mut BTreeSet<String>,
    identity: String,
) -> Result<(), WorldProjectV2Error> {
    if identities.insert(identity.clone()) {
        Ok(())
    } else {
        Err(WorldProjectV2Error::DuplicateIdentity(identity))
    }
}

fn validate_reference_identity(
    family: &str,
    key: &str,
    revision: &str,
) -> Result<(), WorldProjectV2Error> {
    if !matches!(
        family,
        "Terrain"
            | "Presentation"
            | "LocalObject"
            | "Behavior"
            | "Creature"
            | "Item"
            | "Loot"
            | "Ability"
            | "Effect"
            | "Formula"
    ) {
        return Err(WorldProjectV2Error::InvalidReference(
            "v1 executable family",
        ));
    }
    validate_key(key)?;
    DefinitionRevisionRef::new(revision).map_err(|_| WorldProjectV2Error::InvalidIdentity)?;
    Ok(())
}

fn reference_identity(record: &ProjectReferenceRecord) -> (&str, &str, &str) {
    match record {
        ProjectReferenceRecord::Ability { identity, .. }
        | ProjectReferenceRecord::Effect { identity, .. }
        | ProjectReferenceRecord::Formula { identity }
        | ProjectReferenceRecord::Item { identity, .. }
        | ProjectReferenceRecord::Generic { identity, .. }
        | ProjectReferenceRecord::Creature { identity, .. }
        | ProjectReferenceRecord::Loot { identity, .. }
        | ProjectReferenceRecord::LocalObject { identity, .. } => (
            identity.family.as_str(),
            identity.key.as_str(),
            identity.revision.as_str(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(family: WorldProjectV2Family, key: &str) -> WorldProjectV2Reference {
        WorldProjectV2Reference {
            family,
            key: key.to_owned(),
            revision: "r1".to_owned(),
        }
    }

    fn empty_project() -> WorldProjectV2 {
        WorldProjectV2 {
            schema: WORLD_PROJECT_V2_SCHEMA.to_owned(),
            reference_records: Vec::new(),
            items: Vec::new(),
            proficiencies: Vec::new(),
            augments: Vec::new(),
            interactions: Vec::new(),
            npcs: Vec::new(),
            dialogues: Vec::new(),
            services: Vec::new(),
            quests: Vec::new(),
            houses: Vec::new(),
            encounters: Vec::new(),
            assets: Vec::new(),
            placements: Vec::new(),
            transitions: Vec::new(),
            provenance: Vec::new(),
            editor: Vec::new(),
        }
    }

    #[test]
    fn v2_accepts_modern_item_authoring_without_mutable_instance_state() {
        let mut project = empty_project();
        project.items.push(ItemAuthoringV2 {
            item: reference(WorldProjectV2Family::Item, "oteryn:item.weapon.test"),
            presentation: ItemPresentationAuthoringV2 {
                appearance: Some(reference(
                    WorldProjectV2Family::Asset,
                    "oteryn:asset.item.test",
                )),
                aliases: vec!["Test Weapon".to_owned()],
                tags: vec!["weapon".to_owned()],
                taxonomy: Some(ItemTaxonomyV2 {
                    primary: "Weapons".to_owned(),
                    secondary: Some("Fist".to_owned()),
                    tertiary: None,
                }),
            },
            forge: Some(ItemForgeProfileV2 {
                classification: 4,
                max_tier: 10,
            }),
            proficiency_profile: Some(reference(
                WorldProjectV2Family::Proficiency,
                "oteryn:proficiency.weapon.test",
            )),
            augment_bindings: vec![reference(
                WorldProjectV2Family::Augment,
                "oteryn:augment.test",
            )],
            on_use_interactions: vec![reference(
                WorldProjectV2Family::Interaction,
                "oteryn:interaction.item.test",
            )],
            use_requirements: ItemUseRequirementsV2 {
                required_magic_level: Some(15),
            },
            lifecycle: ItemLifecycleAuthoringV2 {
                enchantable: Some(true),
                destructible: Some(false),
                enchant_interactions: Vec::new(),
                destroy_interactions: Vec::new(),
            },
            source_lifecycle: SourceLifecycleV2 {
                implemented: Some("2026-01-01".to_owned()),
                removed: None,
            },
        });
        project.assets.push(AssetDefinitionV2 {
            identity: reference(WorldProjectV2Family::Asset, "oteryn:asset.item.test"),
            locator: "assets/items/test".to_owned(),
            content_sha256: None,
        });
        project.proficiencies.push(ProficiencyDefinitionV2 {
            identity: reference(
                WorldProjectV2Family::Proficiency,
                "oteryn:proficiency.weapon.test",
            ),
            weapon_item: reference(WorldProjectV2Family::Item, "oteryn:item.weapon.test"),
            levels: vec![ProficiencyLevelV2 {
                level: 1,
                perks: vec![reference(
                    WorldProjectV2Family::Augment,
                    "oteryn:augment.test",
                )],
            }],
            shaping: Some(PerkShapingV2 {
                max_rank: 10,
                replace_slots: 2,
                refine_enabled: true,
                reshape_enabled: true,
                clear_enabled: true,
                lunar_ascension_enabled: true,
                cost_service: None,
            }),
        });
        project.augments.push(AugmentDefinitionV2 {
            identity: reference(WorldProjectV2Family::Augment, "oteryn:augment.test"),
            target: AugmentTargetV2::AutoAttack,
            effect: reference(WorldProjectV2Family::Effect, "oteryn:effect.test"),
            rank_values: vec![AugmentRankValueV2 {
                rank: 0,
                numerator: 2,
                denominator: 100,
            }],
        });
        project.interactions.push(InteractionDefinitionV2 {
            identity: reference(
                WorldProjectV2Family::Interaction,
                "oteryn:interaction.item.test",
            ),
            trigger: InteractionTriggerV2::Use,
            execution: InteractionExecutionV2::Ability {
                ability: reference(WorldProjectV2Family::Ability, "oteryn:ability.test"),
            },
        });
        assert!(project.validate().is_ok());
    }

    #[test]
    fn v2_rejects_instance_forge_tier_and_proficiency_progress_fields() {
        let item_json = r#"{
          "item":{"family":"ITEM","key":"oteryn:item.test","revision":"r1"},
          "current_forge_tier":3
        }"#;
        assert!(serde_json::from_str::<ItemAuthoringV2>(item_json).is_err());

        let proficiency_json = r#"{
          "identity":{"family":"PROFICIENCY","key":"oteryn:proficiency.test","revision":"r1"},
          "weapon_item":{"family":"ITEM","key":"oteryn:item.test","revision":"r1"},
          "levels":[{"level":1,"perks":[]}],
          "current_xp":25000
        }"#;
        assert!(serde_json::from_str::<ProficiencyDefinitionV2>(proficiency_json).is_err());
    }

    #[test]
    fn v2_routes_shop_and_drop_relationships_outside_item_overlay() {
        let item_json = r#"{
          "item":{"family":"ITEM","key":"oteryn:item.test","revision":"r1"},
          "npcprice":100,
          "droppedby":["Demon"]
        }"#;
        assert!(serde_json::from_str::<ItemAuthoringV2>(item_json).is_err());

        let service = ServiceDefinitionV2 {
            identity: reference(WorldProjectV2Family::Service, "oteryn:service.shop.test"),
            offers: vec![ServiceOfferV2 {
                item: reference(WorldProjectV2Family::Item, "oteryn:item.test"),
                direction: ServiceOfferDirectionV2::SellToPlayer,
                unit_price: 100,
                currency: None,
            }],
        };
        assert!(service.validate().is_ok());
    }

    #[test]
    fn v2_typed_references_fail_closed() {
        let interaction = InteractionDefinitionV2 {
            identity: reference(WorldProjectV2Family::Interaction, "oteryn:interaction.bad"),
            trigger: InteractionTriggerV2::Use,
            execution: InteractionExecutionV2::Ability {
                ability: reference(WorldProjectV2Family::Item, "oteryn:item.not-an-ability"),
            },
        };
        assert!(matches!(
            interaction.validate(),
            Err(WorldProjectV2Error::InvalidReference(_))
        ));
    }

    #[test]
    fn v2_duplicate_declaration_identity_rejects() {
        let mut project = empty_project();
        let dialogue = DialogueDefinitionV2 {
            identity: reference(WorldProjectV2Family::Dialogue, "oteryn:dialogue.test"),
            nodes: Vec::new(),
            edges: Vec::new(),
        };
        project.dialogues.push(dialogue.clone());
        project.dialogues.push(dialogue);
        assert!(matches!(
            project.validate(),
            Err(WorldProjectV2Error::DuplicateIdentity(_))
        ));
    }

    #[test]
    fn v2_unknown_members_reject() {
        let json = r#"{"schema":"OTERYN_WORLD_PROJECT/v2","unknown":true}"#;
        assert!(serde_json::from_str::<WorldProjectV2>(json).is_err());
    }
}
