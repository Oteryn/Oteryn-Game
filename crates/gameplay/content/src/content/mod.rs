//! Bounded CONTENT seam: non-production evidence plus FIRST_PRODUCTION_CONTENT_PROFILE/v1.
//! Live deployment authority is deliberately absent; activation consumes sealed Game authority.
//!
//! ```compile_fail
//! use oteryn_gameplay_content::content::ActivationSlot;
//! let _ = ActivationSlot::new();
//! ```
//!
//! Full staged runtime generations are catalog-owned and are deliberately not public.
//! Downstream consumers must stage through `ContentActivationController`, which enforces the
//! registered single-staged / two-resident generation ceiling.
//!
//! ```compile_fail
//! use oteryn_gameplay_content::content::StagedGeneration;
//! ```
//!
//! First-production CONTENT uses Foundation's canonical cross-boundary `WorldId`; the
//! domain-local nominal type cannot be substituted implicitly.
//!
//! ```compile_fail
//! use oteryn_gameplay_domain::domain;
//! use oteryn_protocol_oteryn::WorldId;
//! let domain_world = domain::WorldId::from_bytes([
//!     0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0x70, 0xcd,
//!     0x8e, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
//! ]).unwrap();
//! let _: WorldId = domain_world;
//! ```

#![forbid(unsafe_code)]

mod activation;
mod artifact;
pub mod blessing_catalogue;
pub mod bundle_spell_tiles;
pub mod channel_content_pin;
pub mod character_progression_content;
pub mod charm_source;
pub mod charm_source_effect;
pub mod charm_source_json;
mod compiler;
pub mod creature_attack_geometry;
pub mod creature_policy;
pub mod creature_reward;
mod cw2_b1_import;
mod cw2_b4_import;
pub mod daily_reward;
mod digest;
pub mod encounter_map_item;
pub mod field_condition;
mod fixture;
pub mod item_abilities;
pub mod item_admission;
pub mod item_bed_promotion;
pub mod item_capacity_promotion;
pub mod item_consumption_promotion;
pub mod item_description_promotion;
pub mod item_description_wiki_promotion;
pub mod item_document_promotion;
pub mod item_elemental_magic_modifier_promotion;
pub mod item_forge289_promotion;
pub mod item_forge3332_promotion;
pub mod item_hit_magic_promotion;
pub mod item_identity;
pub mod item_mantra_bond_modifier_promotion;
pub mod item_market_true_promotion;
pub mod item_movable_promotion;
pub mod item_name15_promotion;
pub mod item_name_promotion;
pub mod item_numeric_modifier_promotion;
pub mod item_physical_promotion;
pub mod item_ranged_promotion;
pub mod item_ref;
pub mod item_stack_default_promotion;
pub mod item_stack_default_successor8_promotion;
pub mod item_stack_false_promotion;
pub mod item_stack_historical_promotion;
pub mod item_stats_promotion;
pub mod item_timed_promotion;
pub mod item_use_observation_promotion;
pub mod item_weapon_metadata_promotion;
mod model;
pub mod native_cell_lookup;
pub mod native_gameplay;
pub mod native_items;
pub mod native_source_world_carrier;
pub mod native_spell_appearances;
pub mod newhaven_travel;
pub mod newhaven_tutorial_land;
pub mod npc_catalogue;
mod production;
mod project;
mod project_fs;
mod reference_artifact;
mod reference_playable;
#[cfg(any(test, feature = "test-support"))]
pub mod reference_static_cell;
pub mod spell_wheel_profile;
#[allow(
    dead_code,
    reason = "unactivated engineering Content carrier awaits the separate Movement consumer lease"
)]
pub mod static_cell_engine;
#[allow(
    dead_code,
    reason = "preserves the existing Spell profile lint scope for the candidate training owner"
)]
pub mod training_formula;
pub mod world_reward_claims;

pub use activation::*;
#[cfg(test)]
pub use artifact::{ActivationSlot, ActiveContent};
pub use artifact::{ArtifactExpectation, ProjectionClass, StagedArtifact, StagedContentPair};
pub use compiler::{CompiledContent, compile, compile_reference_playable};
pub use cw2_b1_import::*;
pub use cw2_b4_import::*;
pub use fixture::synthetic_vsl_fixture;
pub use model::*;
pub use npc_catalogue::{
    NpcDataCatalogue, load_data_only_npc_catalogue, npc_catalogue_preproduction_limits,
};
#[allow(
    unused_imports,
    reason = "read by NPC talk (NPC-TALK-1), which has no boot caller yet"
)]
pub use npc_catalogue::{
    NpcEntry, NpcGeneratedReplies, NpcReplyLine, NpcReplyTemplate, NpcServiceModel,
};
pub use production::{
    CompiledFirstProductionContent, ContentLockBinding, ContentLockEntry, DurableMigrationClass,
    FIRST_PRODUCTION_ARTIFACT_PROFILE_ID, FIRST_PRODUCTION_CAPABILITY_PROFILE,
    FIRST_PRODUCTION_MAX_ATOM_BYTES, FIRST_PRODUCTION_MAX_CELLS,
    FIRST_PRODUCTION_MAX_CLIENT_ARTIFACT_BYTES, FIRST_PRODUCTION_MAX_CLIENT_RECORDS,
    FIRST_PRODUCTION_MAX_CONTENT_LOCK_ENTRIES, FIRST_PRODUCTION_MAX_DECODED_FIELDS,
    FIRST_PRODUCTION_MAX_DEFINITIONS, FIRST_PRODUCTION_MAX_FLOORS,
    FIRST_PRODUCTION_MAX_GENERATION_PAIR_BYTES, FIRST_PRODUCTION_MAX_KEY_BYTES,
    FIRST_PRODUCTION_MAX_MANIFEST_BYTES, FIRST_PRODUCTION_MAX_MANIFEST_FIELDS,
    FIRST_PRODUCTION_MAX_RECORD_BYTES, FIRST_PRODUCTION_MAX_REFERENCES,
    FIRST_PRODUCTION_MAX_SCOPE_POPULATION, FIRST_PRODUCTION_MAX_SECTION_BYTES,
    FIRST_PRODUCTION_MAX_SERVER_ARTIFACT_BYTES, FIRST_PRODUCTION_MAX_SERVER_RECORDS,
    FIRST_PRODUCTION_MAX_SPAWN_POPULATION, FIRST_PRODUCTION_MAX_X_SPAN,
    FIRST_PRODUCTION_MAX_Y_SPAN, FIRST_PRODUCTION_PROFILE_ID, FirstProductionAbility,
    FirstProductionArea, FirstProductionBehavior, FirstProductionCell,
    FirstProductionCompileTarget, FirstProductionContentSource, FirstProductionCreature,
    FirstProductionEffect, FirstProductionExpectation, FirstProductionFormulaProfile,
    FirstProductionItem, FirstProductionLimits, FirstProductionLootEntry, FirstProductionLootTable,
    FirstProductionPresentation, FirstProductionRegion, FirstProductionRelocation,
    FirstProductionRevisionSet, FirstProductionRngContext, FirstProductionSpawn,
    FirstProductionTerrain, FirstProductionXpDefinition, GenerationIdentity,
    PackageManifestBinding, ProductionAtom, ProductionKey, Sha256HexDigest,
    compile_first_production, load_reference_playable_artifact, stage_reference_playable,
};
pub use project::*;
pub use project_fs::*;
pub use reference_artifact::{
    CompiledReferencePlayableContent, NonAuthoritativeReferenceStage,
    OTERYN_REFERENCE_PLAYABLE_ARTIFACT_PROFILE_ID, REFERENCE_PLAYABLE_MAX_CLIENT_ARTIFACT_BYTES,
    REFERENCE_PLAYABLE_MAX_GENERATION_PAIR_BYTES, REFERENCE_PLAYABLE_MAX_SERVER_ARTIFACT_BYTES,
    ReferenceArtifactProjection, ReferenceClientItem, ReferencePlayableArtifactView,
    ReferencePlayableExpectation, ReferencePlayableGenerationIdentity, ReferenceServerItem,
};
pub use reference_playable::*;

#[cfg(test)]
mod tests;

pub mod spell_familiar_config;
pub mod spell_familiar_defenses;

pub use project::SourceFloorChange;

// Crate test composition may inspect an actually staged native pair; the Stage owner
// and its complete runtime generation remain private to CONTENT and absent downstream.
#[cfg(any(test, feature = "test-support"))]
pub fn native_gameplay_test_world_id() -> Result<oteryn_protocol_oteryn::WorldId, ContentError> {
    Ok(production::test_source(1)?.world_id)
}
#[cfg(any(test, feature = "test-support"))]
pub fn qualified_native_gameplay_test_state(
    compiled: &CompiledFirstProductionContent,
) -> Result<native_gameplay::NativeGameplayState, ContentError> {
    let staged = production::StagedGeneration::stage(
        &compiled.server_artifact,
        &compiled.client_artifact,
        compiled.expectation(),
    )?;
    staged
        .runtime_state()
        .native_gameplay()
        .cloned()
        .ok_or(ContentError::InvalidArtifact(
            "explicit native gameplay fixture state absent",
        ))
}

pub mod spell_data;

mod timed_item_definition;

pub mod bosses;
