//! Bounded CONTENT seam: non-production evidence plus FIRST_PRODUCTION_CONTENT_PROFILE/v1.
//! Live deployment authority is deliberately absent; activation consumes sealed Game authority.
//!
//! ```compile_fail
//! use oteryn_game_server::content::ActivationSlot;
//! let _ = ActivationSlot::new();
//! ```
//!
//! Full staged runtime generations are catalog-owned and are deliberately not public.
//! Downstream consumers must stage through `ContentActivationController`, which enforces the
//! registered single-staged / two-resident generation ceiling.
//!
//! ```compile_fail
//! use oteryn_game_server::content::StagedGeneration;
//! ```

#![forbid(unsafe_code)]

mod activation;
mod artifact;
mod compiler;
mod digest;
mod fixture;
mod model;
mod production;

pub use activation::*;
#[cfg(test)]
pub(crate) use artifact::{ActivationSlot, ActiveContent};
pub use artifact::{ArtifactExpectation, ProjectionClass, StagedArtifact, StagedContentPair};
pub use compiler::{CompiledContent, compile};
pub use fixture::synthetic_vsl_fixture;
pub use model::*;
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
    compile_first_production,
};

#[cfg(test)]
mod tests;
