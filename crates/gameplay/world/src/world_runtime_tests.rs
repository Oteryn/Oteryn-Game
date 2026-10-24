#![allow(clippy::items_after_test_module)]
use super::*;
use crate::content::{
    CW2_B1_VASE_KEY, CW2_B1_VASE_REVISION, CanonicalProjectDocuments, ClientProjectionClass,
    ContentActivationController, ContentLockBinding, ContentLockEntry, CoordinateFrameRef,
    DefinitionRevisionRef, EvidenceBindingRef, EvidenceDisposition, FootprintCell,
    LOCAL_OBJECT_RETAG_INTENT_FAMILY, MapRevisionRef, OwnerCapabilityRequirement,
    PackageManifestBinding, PlacementRef, ProductionAtom, ProjectDraft, ProjectEvidenceLimits,
    ReferenceDefinition, ReferenceItemDestination, ReferenceItemPhysicalClass,
    ReferenceItemStackClass, Sha256HexDigest, TypedDefinitionRef, compile_reference_playable,
    protected_cw2_b1_vase_import, stage_reference_playable,
};
use crate::foundation::{
    ChannelId, CharacterId, CharacterLease, CommandIdError, FoundationProtocolError,
    FreshAdmissionCommit, FreshAdmissionFacts, GameSessionId, GenerationError,
    MAX_OUTSTANDING_COMMANDS,
};

const OPEN_TRANSITION: &str = "oteryn:reference.transition.local-object-open";
const CLOSE_TRANSITION: &str = "oteryn:reference.transition.local-object-close";
const PLACEMENT_A: &str = "oteryn:reference.placement.local-object-a";
const PLACEMENT_B: &str = "oteryn:reference.placement.local-object-b";
const B1_EVIDENCE: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
);

fn fixture_error(_reason: &'static str) -> WorldRuntimeError {
    WorldRuntimeError::InvalidBinding("invalid CW4 test fixture")
}

fn uuid_v7(seed: u8) -> [u8; 16] {
    let mut bytes = [0_u8; 16];
    bytes[0] = 1;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = seed;
    bytes
}

fn decode_world(seed: u8) -> Result<crate::foundation::WorldId, WorldRuntimeError> {
    crate::foundation::WorldId::decode(&uuid_v7(seed))
        .map_err(|_error: FoundationProtocolError| fixture_error("world"))
}

fn reference_project_limits() -> ProjectEvidenceLimits {
    ProjectEvidenceLimits {
        max_documents: 6,
        max_document_bytes: 32_768,
        max_total_bytes: 65_536,
        max_json_depth: 20,
        max_decoded_fields: 1_024,
        max_string_bytes: 32_768,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: 1,
        max_import_records: 1,
        max_reimport_states: 2,
    }
}

// Include topic files in this module to preserve test paths and private fixture access.
include!("world_runtime_tests_b1.rs");
include!("world_runtime_tests_fixtures.rs");
include!("world_runtime_tests_authority.rs");
include!("world_runtime_tests_overlay.rs");
include!("world_runtime_tests_use.rs");
