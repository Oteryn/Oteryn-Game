use crate::content::{
    CanonicalReferencePlayableContent, ContentError, DefinitionFamily, FootprintRelation,
    LocalObjectCollisionPresence, LocalObjectIntentFamily, LocalObjectStateAttributes,
    LocalObjectStateDefinition, LogicalCell, LoweredActionId, NonAuthoritativeReferenceStage,
    PlacementKey, ProductionKey, REFERENCE_PLAYABLE_CAPABILITY_PROFILE,
    REFERENCE_PLAYABLE_CONTENT_PROFILE_ID, ReferenceDefinitionKind, ReferencePlayableContentSource,
    ReferencePlayableGenerationIdentity, ReferenceServerItem, TransitionBinding,
    TransitionEventOwner, TransitionKey, TransitionOrigin, TypedDefinitionRef,
    link_reference_playable,
};
use crate::foundation::{
    CharacterWorldEligibilityClaimV1, CommandId, CommandIngress, CommandLifecycleError, CommandRef,
    CommandSemanticIdentity, ConnectionGeneration, DuplicateDisposition,
    GameSessionAuthoritySnapshot, GameSessionState, IngressDecision,
    NormalizedSemanticIntentIdentity, RetainedBindingIdentity, RuntimeScopeRefV1,
    ScopeOwnershipGeneration, TerminalSemanticOutcome,
};
use crate::world_object_revert::RevertSchedulingCapability;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

/// MAP-LOAD-1: the World's base map, loaded once and shared by every Channel of the World
/// (ADR-0021 §4.1). No runtime holds one yet; MAP-CUTOVER-1 boots from it.
pub type WorldBaseHandle = Arc<crate::map::WorldBase>;

const DISPOSITION_COMMITTED: &str = "COMMITTED";
const DISPOSITION_NO_CHANGE: &str = "NO_CHANGE";
const DISPOSITION_OCCUPIED: &str = "OCCUPIED";
const DISPOSITION_BINDING_MISMATCH: &str = "BINDING_MISMATCH";
const DISPOSITION_STALE_STATE: &str = "STALE_STATE";
const DISPOSITION_REVISION_EXHAUSTED: &str = "REVISION_EXHAUSTED";
const LOCAL_OBJECT_TRANSITION_CAPABILITY: &str =
    "oteryn:runtime.capability.local-object-transition";
const REFERENCE_CONTENT_GENERATION_DOMAIN: &[u8] = b"OTERYN/CW4/REFERENCE_CONTENT_GENERATION/v1";
const EVENT_ORIGIN_BINDING_DOMAIN: &[u8] = b"OTERYN/CW4/D91_EVENT_ORIGIN_BINDING/v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceContentGeneration {
    semantic_identity: Box<str>,
}

impl ReferenceContentGeneration {
    pub fn from_content(
        content: &CanonicalReferencePlayableContent,
    ) -> Result<Self, WorldRuntimeError> {
        // Foundation retains one bounded Content-generation component. Hash the complete
        // Reference generation fence instead of aliasing it to package provenance: production
        // generation identity treats the Content Lock token as a distinct identity component.
        let provenance = content.package_manifest.package_provenance_digest()?;
        let mut preimage = Vec::new();
        append_reference_generation_component(&mut preimage, REFERENCE_CONTENT_GENERATION_DOMAIN)?;
        append_reference_generation_component(
            &mut preimage,
            content.profile_revision.as_str().as_bytes(),
        )?;
        append_reference_generation_component(
            &mut preimage,
            content.capability_profile.as_str().as_bytes(),
        )?;
        append_reference_generation_component(&mut preimage, provenance.as_str().as_bytes())?;
        append_reference_generation_component(
            &mut preimage,
            content
                .content_lock
                .revision_digest_token
                .as_str()
                .as_bytes(),
        )?;
        let world_bytes = content.world_id.as_bytes();
        append_reference_generation_component(&mut preimage, world_bytes.as_ref())?;
        append_reference_generation_component(
            &mut preimage,
            content.coordinate_frame.as_str().as_bytes(),
        )?;

        Ok(Self {
            semantic_identity: encode_reference_generation_digest(sha256_reference_generation(
                &preimage,
            )),
        })
    }

    /// Owner decision D91: the origin is part of the binding's identity. A placement that
    /// names event-origin transitions binds under this generation extended by its complete
    /// origin table, so the same content re-lowered with a different PLAYER_USE/EVENT split, or
    /// another owner, is a different generation for every operation, command, retained binding
    /// identity and revert record fenced on it. An empty table (every placement no encounter
    /// lowering targets, such as the native entry door) returns `self` unchanged, so no existing
    /// generation bytes move. `bind` validates the unextended generation against the scope's
    /// activation fence first.
    fn with_event_origins(
        &self,
        event_transitions: &BTreeMap<TransitionKey, TransitionEventOwner>,
    ) -> Result<Self, WorldRuntimeError> {
        if event_transitions.is_empty() {
            return Ok(self.clone());
        }
        let mut preimage = Vec::new();
        append_reference_generation_component(&mut preimage, EVENT_ORIGIN_BINDING_DOMAIN)?;
        append_reference_generation_component(&mut preimage, self.semantic_identity.as_bytes())?;
        for (transition, owner) in event_transitions {
            append_reference_generation_component(&mut preimage, transition.as_str().as_bytes())?;
            append_reference_generation_component(&mut preimage, owner.as_str().as_bytes())?;
        }
        Ok(Self {
            semantic_identity: encode_reference_generation_digest(sha256_reference_generation(
                &preimage,
            )),
        })
    }

    #[must_use]
    fn as_str(&self) -> &str {
        &self.semantic_identity
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeContentGenerationFence {
    scope: RuntimeScopeRefV1,
    scope_generation: ScopeOwnershipGeneration,
    content_generation: ReferenceContentGeneration,
}

impl ScopeContentGenerationFence {
    // This CW4 child has no production owner-composition authority. Keep construction
    // test-only until the current scope owner supplies this fence at a later allocated seam.
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub fn for_test(
        scope: RuntimeScopeRefV1,
        scope_generation: ScopeOwnershipGeneration,
        content_generation: ReferenceContentGeneration,
    ) -> Self {
        Self {
            scope,
            scope_generation,
            content_generation,
        }
    }

    /// The crate-visible, non-test constructor for the Channel activation owner (#162
    /// 5868482467, M2b). Built once, at Channel activation, directly from the exact scope
    /// being activated, its current ownership generation and the content generation of the
    /// content that activation is bringing up — never from any value a later `bind` call is
    /// validating against it (`validate_candidate` below still re-checks every candidate that
    /// tries to bind against this fixed fence). The caller is `node/serve.rs`'s boot sequence,
    /// which already holds the committed `RuntimeScopeRefV1`, `ScopeOwnershipGeneration` and
    /// the door's own `ReferenceContentGeneration` at the same point it builds the Channel
    /// runtime; nothing here derives any of the three from the door `LocalObjectRuntime` this
    /// fence is then used to bind.
    #[must_use]
    pub fn for_activation(
        scope: RuntimeScopeRefV1,
        scope_generation: ScopeOwnershipGeneration,
        content_generation: ReferenceContentGeneration,
    ) -> Self {
        Self {
            scope,
            scope_generation,
            content_generation,
        }
    }

    fn validate_candidate(
        &self,
        scope: RuntimeScopeRefV1,
        scope_generation: ScopeOwnershipGeneration,
        candidate_generation: &ReferenceContentGeneration,
    ) -> Result<(), WorldRuntimeError> {
        if self.scope != scope || self.scope_generation != scope_generation {
            return Err(WorldRuntimeError::InvalidBinding(
                "active Content-generation fence does not match runtime scope ownership",
            ));
        }
        if &self.content_generation != candidate_generation {
            return Err(WorldRuntimeError::InvalidBinding(
                "different Content generation cannot activate while runtime scope is live",
            ));
        }
        Ok(())
    }
}

/// Immutable, non-authoritative runtime projection of the one staged Reference Item.
///
/// This view carries no activation or controller capability. Its authoritative Item semantics
/// are decoded exactly once through the staged server artifact's typed identity index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonAuthoritativeReferenceItemRuntimeView {
    scope: RuntimeScopeRefV1,
    generation_identity: ReferencePlayableGenerationIdentity,
    authoritative_item: ReferenceServerItem,
}

impl NonAuthoritativeReferenceItemRuntimeView {
    pub fn bind(
        stage: NonAuthoritativeReferenceStage<'_>,
        scope: RuntimeScopeRefV1,
    ) -> Result<Self, WorldRuntimeError> {
        if scope.world_id() != stage.identity().world_id() {
            return Err(WorldRuntimeError::InvalidBinding(
                "Reference Item generation world does not match runtime scope",
            ));
        }

        let generation_identity = stage.identity().clone();
        let authoritative_item = stage
            .server_artifact()
            .lookup_server_item(generation_identity.item_identity())?
            .ok_or(WorldRuntimeError::InvalidBinding(
                "staged Reference Item identity is absent from its server artifact",
            ))?;

        Ok(Self {
            scope,
            generation_identity,
            authoritative_item,
        })
    }

    #[must_use]
    pub const fn scope(&self) -> RuntimeScopeRefV1 {
        self.scope
    }

    #[must_use]
    pub fn generation_identity(&self) -> &ReferencePlayableGenerationIdentity {
        &self.generation_identity
    }

    #[must_use]
    pub fn item_identity(&self) -> &TypedDefinitionRef {
        self.generation_identity.item_identity()
    }

    #[must_use]
    pub fn server_artifact_digest(&self) -> [u8; 32] {
        self.generation_identity.server_artifact_digest()
    }

    #[must_use]
    pub fn client_artifact_digest(&self) -> [u8; 32] {
        self.generation_identity.client_artifact_digest()
    }

    #[must_use]
    pub fn lookup_item(&self, identity: &TypedDefinitionRef) -> Option<&ReferenceServerItem> {
        (identity == self.item_identity()).then_some(&self.authoritative_item)
    }
}

fn append_reference_generation_component(
    preimage: &mut Vec<u8>,
    value: &[u8],
) -> Result<(), WorldRuntimeError> {
    let length = u32::try_from(value.len()).map_err(|_error| {
        WorldRuntimeError::InvalidBinding("Reference Content-generation component is too large")
    })?;
    preimage.extend_from_slice(&length.to_be_bytes());
    preimage.extend_from_slice(value);
    Ok(())
}

fn encode_reference_generation_digest(bytes: [u8; 32]) -> Box<str> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(64);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded.into_boxed_str()
}

fn sha256_reference_generation(input: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut state = [
        0x6a09e667_u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity(input.len().saturating_add(72));
    padded.extend_from_slice(input);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for block in padded.as_chunks::<64>().0 {
        let mut words = [0_u32; 64];
        for (index, chunk) in block.as_chunks::<4>().0.iter().enumerate() {
            words[index] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];

        for (&constant, &word) in K.iter().zip(words.iter()) {
            let sigma1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(sigma1)
                .wrapping_add(choose)
                .wrapping_add(constant)
                .wrapping_add(word);
            let sigma0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = sigma0.wrapping_add(majority);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    let mut output = [0_u8; 32];
    for (chunk, value) in output.as_chunks_mut::<4>().0.iter_mut().zip(state) {
        chunk.copy_from_slice(&value.to_be_bytes());
    }
    output
}

fn validate_reference_semantic_core(
    content: &CanonicalReferencePlayableContent,
) -> Result<(), WorldRuntimeError> {
    if !content.ordered_placements.is_empty() {
        return Err(WorldRuntimeError::InvalidBinding(
            "first CW4 child does not admit ordered placement claims",
        ));
    }

    let linked = link_reference_playable(ReferencePlayableContentSource {
        profile_revision: content.profile_revision.clone(),
        capability_profile: content.capability_profile.clone(),
        package_manifest: content.package_manifest.clone(),
        content_lock: content.content_lock.clone(),
        world_id: content.world_id,
        coordinate_frame: content.coordinate_frame.clone(),
        definitions: content.definitions.clone(),
        placements: Vec::new(),
        ordered_placements: Vec::new(),
        transitions: content.transitions.clone(),
    })?;

    if linked.profile_revision != content.profile_revision
        || linked.capability_profile != content.capability_profile
        || linked.package_manifest != content.package_manifest
        || linked.content_lock != content.content_lock
        || linked.world_id != content.world_id
        || linked.coordinate_frame != content.coordinate_frame
        || linked.definitions != content.definitions
        || linked.transitions != content.transitions
    {
        return Err(WorldRuntimeError::InvalidBinding(
            "Reference playable semantic core is not canonical linker output",
        ));
    }
    Ok(())
}

fn validate_synthetic_placement_evidence(
    placement: &crate::content::PlacementRef,
) -> Result<(), WorldRuntimeError> {
    if placement
        .address
        .evidence
        .disposition()
        .is_reference_promotable()
    {
        return Err(WorldRuntimeError::InvalidBinding(
            "synthetic CW4 placement cannot claim Reference target promotion",
        ));
    }

    for relation in [
        &placement.presentation_footprint,
        &placement.collision_footprint,
    ] {
        if let FootprintRelation::Qualified { evidence, .. } = relation
            && evidence.disposition().is_reference_promotable()
        {
            return Err(WorldRuntimeError::InvalidBinding(
                "synthetic CW4 placement cannot claim Reference target promotion",
            ));
        }
    }
    Ok(())
}

/// Builds the native entry-room door's synthetic, non-promotable `PlacementRef` (#162 A4-a,
/// M2b 5868482467) from the qualified door cell and binds its `LocalObjectRuntime` under
/// `fence`. `door_content` is the exact qualified, fully linked door content
/// (`NativeEntryContentPin::door` / `QualifiedNativeEntryRoom::door`); its own `placements` stay
/// empty (M2a DECISION_REQUIRED, r4120444680 — the accepted evidence manifest has no
/// `CONTENT_WORLD` case that can promote any placement claim honestly yet), so this placement is
/// injected into a local clone rather than authored through `link_reference_playable`, exactly
/// as the CW4 test fixtures elsewhere in this file inject their own synthetic placements. The
/// injected placement changes nothing `ReferenceContentGeneration::from_content` hashes, so it
/// binds under the same fence the caller already built from the unmutated `door_content`.
///
/// The sole caller is the Channel activation owner (`node/serve.rs`'s boot sequence, and the WP5
/// seam qualification harness building the same owners), once per Channel activation.
pub fn bind_native_entry_door(
    door_content: &CanonicalReferencePlayableContent,
    fence: &ScopeContentGenerationFence,
    scope: RuntimeScopeRefV1,
    scope_generation: ScopeOwnershipGeneration,
) -> Result<LocalObjectRuntime, WorldRuntimeError> {
    use crate::content::accepted as door_accepted;
    let invalid =
        |_| WorldRuntimeError::InvalidBinding("native entry door synthetic placement fixture");
    let evidence = crate::content::EvidenceBindingRef::new(
        crate::content::ProductionAtom::new("native entry door manifest revision", "manifest-r0")
            .map_err(invalid)?,
        ProductionKey::new("oteryn:cw4.native-entry-door-placement").map_err(invalid)?,
        crate::content::EvidenceDisposition::Unknown,
    );
    let members = vec![crate::content::FootprintCell {
        dx: 0,
        dy: 0,
        dz: 0,
    }];
    let placement_key = PlacementKey::new(door_accepted::DOOR_CELL.0).map_err(invalid)?;
    let placement = crate::content::PlacementRef {
        key: placement_key.clone(),
        map_revision: crate::content::MapRevisionRef::new(door_accepted::REVISIONS[1])
            .map_err(invalid)?,
        definition: crate::content::TypedDefinitionRef::new(
            DefinitionFamily::LocalObject,
            ProductionKey::new(door_accepted::DOOR_DEFINITION).map_err(invalid)?,
            crate::content::DefinitionRevisionRef::new(door_accepted::DEFINITION_REVISION)
                .map_err(invalid)?,
        ),
        address: crate::content::SpatialAddress {
            world_id: door_content.world_id,
            coordinate_frame: door_content.coordinate_frame.clone(),
            cell: LogicalCell {
                x: door_accepted::DOOR_CELL.1,
                y: door_accepted::DOOR_CELL.2,
                z: i32::from(door_accepted::DOOR_CELL.3),
            },
            evidence: evidence.clone(),
        },
        presentation_footprint: FootprintRelation::Qualified {
            members: members.clone(),
            evidence: evidence.clone(),
        },
        collision_footprint: FootprintRelation::Qualified { members, evidence },
        local_object_initial_state: Some(
            ProductionKey::new(door_accepted::DOOR_CLOSED_STATE).map_err(invalid)?,
        ),
        local_object_state_attributes: BTreeMap::new(),
        local_object_revert_after_ms: BTreeMap::new(),
        // D91: the door names no event origin, so both its edges stay PLAYER_USE.
        local_object_event_transitions: BTreeMap::new(),
    };
    let mut content = door_content.clone();
    content.placements = vec![placement];
    LocalObjectRuntime::bind(
        &content,
        fence,
        scope,
        scope_generation,
        &placement_key,
        1,
        &[
            TransitionKey::new(door_accepted::DOOR_OPEN_TRANSITION).map_err(invalid)?,
            TransitionKey::new(door_accepted::DOOR_CLOSE_TRANSITION).map_err(invalid)?,
        ],
    )
}

/// A D38 world-object operation on this runtime's pre-authored anchor, identified by the
/// authored `TransitionBinding` it invokes (docs/architecture/
/// OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md §4). `TRANSFORM(from,
/// to)`, `CREATE(def)`/`REMOVE(def)` (a fixed-footprint anchor toggling collision-Present/
/// collision-Absent) and `RETAG` (a same-collision-class rearm,
/// `LOCAL_OBJECT_RETAG_INTENT_FAMILY`) get no separate Rust variant here: each is exactly
/// "invoke one of this runtime's bound transitions", and `LocalObjectRuntime::bind` is what
/// fixes which transitions a given instance may invoke. `prepare()` derives the next blocking
/// footprint from the *target* state's own authored collision presence, so one execution path
/// serves all four named operations. Open/Close are the two-state special case of this same
/// mechanism, not a distinct code path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalObjectOperation(TransitionKey);

impl LocalObjectOperation {
    #[must_use]
    pub const fn new(transition: TransitionKey) -> Self {
        Self(transition)
    }

    #[must_use]
    fn transition_key(&self) -> &TransitionKey {
        &self.0
    }
}

/// USE-WIRE-V1 selection-kernel failure (#162 5868482467): `select_use_transition` found no
/// transition, or more than one, bound to the runtime's current state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UseSelectionError {
    NoCandidate,
    Ambiguous,
}

/// USE-WIRE-V1 (#162 5868482467) outcome of `LocalObjectRuntime::attempt_use`, one-to-one with
/// the wire `UseDisposition` the gameplay seam encodes, except this type carries the committed
/// state and revision instead of duplicating them into a disposition-only enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalObjectUseOutcome {
    Committed { state: ProductionKey, revision: u64 },
    NothingToUse,
    Occupied,
    StaleState,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalObjectCommand {
    command_ref: CommandRef,
    connection_generation: ConnectionGeneration,
    placement: PlacementKey,
    incarnation: u64,
    content_generation: ReferenceContentGeneration,
    operation: LocalObjectOperation,
    expected_revision: u64,
}

impl LocalObjectCommand {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        command_ref: CommandRef,
        connection_generation: ConnectionGeneration,
        placement: PlacementKey,
        incarnation: u64,
        content_generation: ReferenceContentGeneration,
        operation: LocalObjectOperation,
        expected_revision: u64,
    ) -> Self {
        Self {
            command_ref,
            connection_generation,
            placement,
            incarnation,
            content_generation,
            operation,
            expected_revision,
        }
    }

    #[must_use]
    pub const fn command_ref(&self) -> CommandRef {
        self.command_ref
    }
}

/// #162 §7: the target and delta of one scope-origin operation (`apply_scope_operation`) —
/// exactly what `prepare` needs, with no session, connection or `CommandId`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeLocalObjectOperation {
    placement: PlacementKey,
    incarnation: u64,
    content_generation: ReferenceContentGeneration,
    operation: LocalObjectOperation,
    expected_revision: u64,
    // Owner decision D91: the executing event's own owner, trusted execution evidence set by
    // the owning event's execution path (never client input); `apply_scope_operation` commits
    // only an event-origin transition bound to exactly this owner.
    owner: TransitionEventOwner,
}

impl ScopeLocalObjectOperation {
    #[must_use]
    pub const fn new(
        placement: PlacementKey,
        incarnation: u64,
        content_generation: ReferenceContentGeneration,
        operation: LocalObjectOperation,
        expected_revision: u64,
        owner: TransitionEventOwner,
    ) -> Self {
        Self {
            placement,
            incarnation,
            content_generation,
            operation,
            expected_revision,
            owner,
        }
    }

    #[must_use]
    pub fn transition_key(&self) -> &TransitionKey {
        self.operation.transition_key()
    }

    #[must_use]
    pub fn owner(&self) -> &TransitionEventOwner {
        &self.owner
    }
}

/// #162 §7: the post-operation state and revision of a `Publish` that `prepare` has already
/// computed, shown to `apply_scope_operation`'s staging step before anything commits.
#[derive(Debug)]
pub struct ScopePublish<'a> {
    pub next_state: &'a ProductionKey,
    pub next_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalObjectCommandResult {
    disposition: Box<str>,
    state: Box<str>,
    revision: u64,
    replayed: bool,
}

impl LocalObjectCommandResult {
    #[must_use]
    fn from_terminal(outcome: &TerminalSemanticOutcome, replayed: bool) -> Self {
        Self {
            disposition: outcome.disposition().into(),
            state: outcome.state().into(),
            revision: outcome.revision(),
            replayed,
        }
    }

    #[must_use]
    pub fn disposition(&self) -> &str {
        &self.disposition
    }

    #[must_use]
    pub fn state(&self) -> &str {
        &self.state
    }

    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub const fn replayed(&self) -> bool {
        self.replayed
    }
}

#[derive(Debug)]
pub enum WorldRuntimeError {
    Content(ContentError),
    CommandLifecycle(CommandLifecycleError),
    InvalidBinding(&'static str),
    SessionNotActive,
    StaleGameSession,
    StaleConnectionGeneration,
    StaleRuntimeScope,
    StaleScopeOwnershipGeneration,
    /// Owner decision D91: a scope operation's trusted owner is not the bound owner of the
    /// event-origin transition it names. Refused before `prepare`; nothing mutates.
    EventOwnerMismatch,
    IngressSequenceGap {
        expected: CommandId,
    },
    IngressCapacityExceeded {
        expected: CommandId,
    },
    CommandSpaceExhausted,
    PendingOriginal,
    ConflictChangedInput,
    OutcomeExpired,
    IngressClassificationMismatch,
    PendingContinuationMissing,
    InvalidSessionAuthority(&'static str),
}

impl Display for WorldRuntimeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Content(error) => write!(formatter, "{error}"),
            Self::CommandLifecycle(error) => write!(formatter, "{error}"),
            Self::InvalidBinding(reason) => {
                write!(formatter, "invalid CW4 local-object binding: {reason}")
            }
            Self::SessionNotActive => formatter.write_str("GameSession is not active"),
            Self::StaleGameSession => formatter.write_str("GameSession authority is stale"),
            Self::StaleConnectionGeneration => {
                formatter.write_str("connection generation is stale")
            }
            Self::StaleRuntimeScope => formatter.write_str("runtime scope is stale"),
            Self::EventOwnerMismatch => formatter
                .write_str("scope operation owner does not own the event-origin transition"),
            Self::StaleScopeOwnershipGeneration => {
                formatter.write_str("runtime scope ownership generation is stale")
            }
            Self::IngressSequenceGap { expected } => write!(
                formatter,
                "command sequence gap; expected CommandId {}",
                expected.get()
            ),
            Self::IngressCapacityExceeded { expected } => write!(
                formatter,
                "command ingress capacity exhausted at CommandId {}",
                expected.get()
            ),
            Self::CommandSpaceExhausted => formatter.write_str("CommandId space is exhausted"),
            Self::PendingOriginal => formatter.write_str("original command remains pending"),
            Self::ConflictChangedInput => {
                formatter.write_str("same CommandId has changed semantic input")
            }
            Self::OutcomeExpired => {
                formatter.write_str("terminal command outcome expired; reconcile state")
            }
            Self::IngressClassificationMismatch => formatter
                .write_str("Foundation duplicate classification is internally inconsistent"),
            Self::PendingContinuationMissing => {
                formatter.write_str("pending local-object continuation is no longer pending")
            }
            Self::InvalidSessionAuthority(reason) => {
                write!(formatter, "invalid current GameSession authority: {reason}")
            }
        }
    }
}

impl Error for WorldRuntimeError {}

impl From<ContentError> for WorldRuntimeError {
    fn from(error: ContentError) -> Self {
        Self::Content(error)
    }
}

impl From<CommandLifecycleError> for WorldRuntimeError {
    fn from(error: CommandLifecycleError) -> Self {
        Self::CommandLifecycle(error)
    }
}

#[derive(Debug)]
pub struct LocalObjectRuntime {
    scope: RuntimeScopeRefV1,
    scope_generation: ScopeOwnershipGeneration,
    content_generation: ReferenceContentGeneration,
    placement: PlacementKey,
    incarnation: u64,
    states: Vec<LocalObjectStateDefinition>,
    transitions: BTreeMap<TransitionKey, TransitionBinding>,
    state: ProductionKey,
    revision: u64,
    collision_cells: BTreeSet<LogicalCell>,
    blocking_cells: BTreeSet<LogicalCell>,
    // #162 §9 (design point 2): immutable after `bind`; attributes are a pure read of
    // `(state_attributes, state)` and revert durations are keyed per bound transition and
    // authored action.
    state_attributes: BTreeMap<ProductionKey, LocalObjectStateAttributes>,
    transition_revert_after_ms: BTreeMap<(TransitionKey, LoweredActionId), u64>,
    // #162 §7: the unique bound inverse `bind` validated for each timed forward transition.
    revert_inverses: BTreeMap<TransitionKey, TransitionKey>,
    // Owner decision D91: immutable after `bind`, read from the placement; a transition absent
    // here is `TransitionOrigin::PlayerUse`.
    event_transitions: BTreeMap<TransitionKey, TransitionEventOwner>,
}

impl LocalObjectRuntime {
    #[allow(clippy::too_many_arguments)]
    pub fn bind(
        content: &CanonicalReferencePlayableContent,
        active_content: &ScopeContentGenerationFence,
        scope: RuntimeScopeRefV1,
        scope_generation: ScopeOwnershipGeneration,
        placement_key: &PlacementKey,
        incarnation: u64,
        transition_keys: &[TransitionKey],
    ) -> Result<Self, WorldRuntimeError> {
        validate_reference_semantic_core(content)?;
        if incarnation == 0 {
            return Err(WorldRuntimeError::InvalidBinding(
                "runtime incarnation must be non-zero",
            ));
        }
        if content.profile_revision.as_str() != REFERENCE_PLAYABLE_CONTENT_PROFILE_ID
            || content.capability_profile.as_str() != REFERENCE_PLAYABLE_CAPABILITY_PROFILE
        {
            return Err(WorldRuntimeError::InvalidBinding(
                "Content profile is not the protected Reference playable successor",
            ));
        }
        if scope.world_id() != content.world_id {
            return Err(WorldRuntimeError::InvalidBinding(
                "runtime scope world differs from Content world",
            ));
        }
        let base_generation = ReferenceContentGeneration::from_content(content)?;
        active_content.validate_candidate(scope, scope_generation, &base_generation)?;

        let mut placements = content
            .placements
            .iter()
            .filter(|placement| &placement.key == placement_key);
        let placement = placements.next().ok_or(WorldRuntimeError::InvalidBinding(
            "PlacementKey is absent from active Content",
        ))?;
        if placements.next().is_some() {
            return Err(WorldRuntimeError::InvalidBinding(
                "PlacementKey is ambiguous in active Content",
            ));
        }
        if placement.address.world_id != content.world_id
            || placement.address.coordinate_frame != content.coordinate_frame
        {
            return Err(WorldRuntimeError::InvalidBinding(
                "placement spatial address is outside the active Content frame",
            ));
        }
        validate_synthetic_placement_evidence(placement)?;
        // Owner decision D91: this binding's identity includes its event-origin table.
        let content_generation =
            base_generation.with_event_origins(&placement.local_object_event_transitions)?;
        if placement.definition.family() != DefinitionFamily::LocalObject {
            return Err(WorldRuntimeError::InvalidBinding(
                "placement definition is not a local object",
            ));
        }

        let definition = content
            .definitions
            .iter()
            .find(|definition| definition.definition == placement.definition)
            .ok_or(WorldRuntimeError::InvalidBinding(
                "placement definition is absent from active Content",
            ))?;
        let states = match &definition.kind {
            ReferenceDefinitionKind::LocalObjectStates(states) if !states.is_empty() => {
                states.clone()
            }
            _ => {
                return Err(WorldRuntimeError::InvalidBinding(
                    "local object definition declares no states",
                ));
            }
        };

        // D38 W1/W2: generalized beyond the fixed Open/Close pair to an arbitrary, non-empty set
        // of pre-authored transitions (TRANSFORM/CREATE/REMOVE/RETAG are all just "a transition
        // this runtime may invoke" — see `LocalObjectOperation`). Each bound transition still gets
        // the same fail-closed structural checks the old two-key bind performed: it must target
        // this placement's own definition, its source/target states must be declared by that
        // definition's vocabulary, its capability must be the one this runtime profile supports,
        // and it must carry no unresolved policy guard. Unknown states, undeclared-state
        // references and cross-class RETAG pairs are additionally, and independently, rejected by
        // the CW3 linker for every transition in `content` (`validate_reference_semantic_core`
        // above re-runs it), not just the ones bound here.
        if transition_keys.is_empty() {
            return Err(WorldRuntimeError::InvalidBinding(
                "local-object runtime requires at least one bound D38 world-object operation",
            ));
        }
        let mut transitions = BTreeMap::new();
        for transition_key in transition_keys {
            let transition = unique_transition(content, transition_key)?.clone();
            if transition.definition != placement.definition {
                return Err(WorldRuntimeError::InvalidBinding(
                    "bound transition does not target the selected placement definition",
                ));
            }
            if !local_object_states_contain(&states, &transition.source_state)
                || !local_object_states_contain(&states, &transition.target_state)
            {
                return Err(WorldRuntimeError::InvalidBinding(
                    "bound transition references a state outside the local object's declared vocabulary",
                ));
            }
            if transition.owner_capability.capability_key.as_str()
                != LOCAL_OBJECT_TRANSITION_CAPABILITY
            {
                return Err(WorldRuntimeError::InvalidBinding(
                    "transition capability is not supported by this runtime profile",
                ));
            }
            if !transition.policy_guard_refs.is_empty() {
                return Err(WorldRuntimeError::InvalidBinding(
                    "first CW4 child cannot bypass unresolved policy guards",
                ));
            }
            if transitions
                .insert(transition.key.clone(), transition)
                .is_some()
            {
                return Err(WorldRuntimeError::InvalidBinding(
                    "duplicate TransitionKey requested for local-object runtime binding",
                ));
            }
        }

        // #162 §9 (design points 1/5): the linker never sees this injected placement, so its
        // attribute and revert tables are re-validated here, and every revert-bearing transition
        // must be bound at this placement with exactly one bound inverse (§7 Round 23/24). A
        // destination must resolve to exactly one placement in this scope's world and the
        // Content coordinate frame before `attributes()` can expose it (#1133 review).
        crate::content::validate_local_object_placement_attributes(
            placement,
            definition,
            &content.placements,
            &content.transitions,
            scope.world_id(),
            &content.coordinate_frame,
        )?;
        let mut revert_inverses = BTreeMap::new();
        for (transition_key, _action) in placement.local_object_revert_after_ms.keys() {
            let forward =
                transitions
                    .get(transition_key)
                    .ok_or(WorldRuntimeError::InvalidBinding(
                        "revert_after_ms names a transition this placement does not bind",
                    ))?;
            // §7: a timed transition must carry a recognized intent family, and its inverse must
            // carry the paired one (TRANSFORM↔TRANSFORM, CREATE↔REMOVE, RETAG↔RETAG, OPEN↔CLOSE).
            let paired_family =
                LocalObjectIntentFamily::from_key(&forward.normalized_intent_family)
                    .ok_or(WorldRuntimeError::InvalidBinding(
                        "revert_after_ms transition carries no recognized intent family",
                    ))?
                    .inverse();
            // Widened rule: a candidate inverse leaves the forward target and lands either on the
            // forward source itself or on a state whose *own* declared `attribute_variant_of` is
            // the forward source — never the reverse direction. Inert when no state declares one.
            let state_matches = transitions
                .values()
                .filter(|candidate| {
                    candidate.key != forward.key
                        && candidate.source_state == forward.target_state
                        && (candidate.target_state == forward.source_state
                            || states
                                .iter()
                                .find(|state| state.key == candidate.target_state)
                                .and_then(|state| state.attribute_variant_of.as_ref())
                                == Some(&forward.source_state))
                })
                .collect::<Vec<_>>();
            let mut inverses = state_matches.iter().filter(|candidate| {
                LocalObjectIntentFamily::from_key(&candidate.normalized_intent_family)
                    == Some(paired_family)
            });
            let Some(inverse) = inverses.next() else {
                return Err(WorldRuntimeError::InvalidBinding(
                    if state_matches.is_empty() {
                        "revert_after_ms transition has no bound inverse at this placement"
                    } else {
                        "revert_after_ms inverse does not carry the paired intent family"
                    },
                ));
            };
            if inverses.next().is_some() {
                return Err(WorldRuntimeError::InvalidBinding(
                    "revert_after_ms transition has an ambiguous bound inverse at this placement",
                ));
            }
            // D91: a timed forward and its validated inverse are both event-owned by one owner,
            // so neither is USE-selectable or session-invocable and only the owning event's
            // scope path, with the revert scheduler, commits them.
            let forward_owner = placement.local_object_event_transitions.get(&forward.key);
            let Some(forward_owner) = forward_owner else {
                return Err(WorldRuntimeError::InvalidBinding(
                    "revert_after_ms transition is not event-owned at this placement",
                ));
            };
            if placement.local_object_event_transitions.get(&inverse.key) != Some(forward_owner) {
                return Err(WorldRuntimeError::InvalidBinding(
                    "revert_after_ms inverse is not owned by its forward's event at this placement",
                ));
            }
            revert_inverses.insert(forward.key.clone(), inverse.key.clone());
        }

        let collision_cells = absolute_collision_cells(placement)?;
        // D38 W1b mechanical adaptation: the linker now requires every LocalObject placement to
        // carry an authored initial state validated against this same `states` vocabulary, so the
        // runtime reads it from the placement instead of deriving it from the OPEN transition.
        let initial_state = placement.local_object_initial_state.clone().ok_or(
            WorldRuntimeError::InvalidBinding(
                "placement lacks authored local object initial state",
            ),
        )?;
        // The initial state's own collision presence — not the OPEN/CLOSE operation identity —
        // decides whether the object starts blocking: an authored Absent (e.g. "open") start must
        // not block movement, and an authored Present (e.g. "closed") start must.
        let initial_collision = local_object_state_collision(&states, &initial_state).ok_or(
            WorldRuntimeError::InvalidBinding(
                "placement's authored local object initial state is not declared by the local object",
            ),
        )?;
        let initial_blocking = match initial_collision {
            LocalObjectCollisionPresence::Present => collision_cells.clone(),
            LocalObjectCollisionPresence::Absent => BTreeSet::new(),
        };
        Ok(Self {
            scope,
            scope_generation,
            content_generation,
            placement: placement.key.clone(),
            incarnation,
            state: initial_state,
            revision: 0,
            blocking_cells: initial_blocking,
            collision_cells,
            states,
            transitions,
            state_attributes: placement.local_object_state_attributes.clone(),
            transition_revert_after_ms: placement.local_object_revert_after_ms.clone(),
            revert_inverses,
            event_transitions: placement.local_object_event_transitions.clone(),
        })
    }

    /// #162 §9 (design point 2): the attributes this placement exposes in its current state —
    /// a pure read of the bind-time table, never separately tracked mutable state.
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "§9 accessor; its gameplay consumer is a later allocation"
        )
    )]
    #[must_use]
    pub fn attributes(&self) -> Option<&LocalObjectStateAttributes> {
        self.state_attributes.get(&self.state)
    }

    /// #162 §10.2: whether the current state is a synthesized absent state, for which the
    /// projection renders no object. Such a state is collision-`Absent` and carries no attributes
    /// (validated at link and at `bind`), so it neither blocks nor exposes a destination.
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "§10.2 accessor; the transport projection of absent states is a later allocation"
        )
    )]
    #[must_use]
    pub fn is_absent(&self) -> bool {
        self.states
            .iter()
            .any(|state| state.key == self.state && state.absent)
    }

    /// #162 §9 (design points 5/6): this placement's authored revert duration for one bound
    /// transition invoked by one authored action; `None` when that invocation carries none.
    #[cfg_attr(
        not(test),
        allow(
            dead_code,
            reason = "§9 accessor; the §7 revert driver's live scope owner is a later allocation"
        )
    )]
    #[must_use]
    pub fn revert_after_ms(
        &self,
        transition: &TransitionKey,
        action: &LoweredActionId,
    ) -> Option<u64> {
        self.transition_revert_after_ms
            .get(&(transition.clone(), action.clone()))
            .copied()
    }

    /// Owner decision D91: `transition`'s typed origin at this placement, fixed at `bind`.
    #[must_use]
    pub fn transition_origin(&self, transition: &TransitionKey) -> TransitionOrigin {
        self.event_transitions
            .get(transition)
            .map_or(TransitionOrigin::PlayerUse, |owner| {
                TransitionOrigin::Event(owner.clone())
            })
    }

    /// Owner decision D91: a scope operation may name only an event-origin transition bound to
    /// its own owner. Reads only; `apply_scope_operation` runs it before `prepare`, and the §7
    /// driver runs it before it mints an ordinal.
    pub fn check_scope_owner(
        &self,
        operation: &ScopeLocalObjectOperation,
    ) -> Result<(), WorldRuntimeError> {
        match self.event_transitions.get(operation.transition_key()) {
            None => Err(WorldRuntimeError::InvalidBinding(
                "player-use transition is not a scope operation",
            )),
            Some(bound) if bound != operation.owner() => Err(WorldRuntimeError::EventOwnerMismatch),
            Some(_) => Ok(()),
        }
    }

    /// The runtime scope and scope-ownership-generation fence of a scope operation: the caller's
    /// scope owner must be the one this runtime was bound under. Reads only; `apply_scope_operation`
    /// runs it first, and the §7 driver runs it before a Q2=b re-arm mints an ordinal.
    pub fn check_scope_fence(
        &self,
        scope: RuntimeScopeRefV1,
        scope_generation: ScopeOwnershipGeneration,
    ) -> Result<(), WorldRuntimeError> {
        if scope != self.scope {
            return Err(WorldRuntimeError::StaleRuntimeScope);
        }
        if scope_generation != self.scope_generation {
            return Err(WorldRuntimeError::StaleScopeOwnershipGeneration);
        }
        Ok(())
    }

    fn is_player_use(&self, transition: &TransitionKey) -> bool {
        !self.event_transitions.contains_key(transition)
    }

    /// The normalized intent family of a transition this runtime binds; `None` when it binds no such
    /// transition or its family is not a known one.
    #[must_use]
    pub fn transition_intent_family(
        &self,
        transition: &TransitionKey,
    ) -> Option<LocalObjectIntentFamily> {
        self.transitions
            .get(transition)
            .and_then(|bound| LocalObjectIntentFamily::from_key(&bound.normalized_intent_family))
    }

    /// #162 §7: the unique bound inverse `bind` validated for a timed forward transition.
    #[must_use]
    pub fn revert_inverse(&self, forward: &TransitionKey) -> Option<&TransitionKey> {
        self.revert_inverses.get(forward)
    }

    #[must_use]
    pub fn placement_key(&self) -> &PlacementKey {
        &self.placement
    }

    #[must_use]
    pub const fn incarnation(&self) -> u64 {
        self.incarnation
    }

    #[must_use]
    pub fn content_generation(&self) -> &ReferenceContentGeneration {
        &self.content_generation
    }

    #[must_use]
    pub fn state_key(&self) -> &ProductionKey {
        &self.state
    }

    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn collision_cells(&self) -> &BTreeSet<LogicalCell> {
        &self.collision_cells
    }

    #[must_use]
    pub fn blocking_cells(&self) -> &BTreeSet<LogicalCell> {
        &self.blocking_cells
    }

    /// USE-WIRE-V1 selection kernel (#162 5868482467): the client's `UseIntentV1` names no
    /// transition, only the target placement. The server selects the unique transition this
    /// runtime binds whose `source_state` is this runtime's *current* state. Zero candidates
    /// (nothing usable from here) and more than one candidate (an ambiguous authored binding)
    /// both fail closed; this method reads only, it never mutates and never picks a plausible
    /// candidate over an exact one.
    fn select_use_transition(&self) -> Result<TransitionKey, UseSelectionError> {
        // Owner decision D91: only `PlayerUse` edges are candidates. An event-owned edge (any
        // edge encounter lowering binds here, timed or not, forward, re-arm or revert inverse)
        // is never USE-selectable. `bind` requires every timed edge to be event-owned, so this
        // subsumes the #1144 rule that a `revert_after_ms` edge is not USE-selectable.
        let mut candidates = self
            .transitions
            .values()
            .filter(|transition| {
                transition.source_state == self.state && self.is_player_use(&transition.key)
            })
            .map(|transition| transition.key.clone());
        let first = candidates.next().ok_or(UseSelectionError::NoCandidate)?;
        if candidates.next().is_some() {
            return Err(UseSelectionError::Ambiguous);
        }
        Ok(first)
    }

    /// USE-WIRE-V1 (#162 5868482467): applies exactly the one server-selected transition (see
    /// `select_use_transition`) this runtime's current state binds. This is the seam-local,
    /// in-memory integration level `ComposedFreshAdmission::use_object` uses — the same level
    /// `ComposedFreshAdmission::step` already uses for Movement — and not the durable
    /// `CommandIngress`/`GameSessionAuthoritySnapshot` command lifecycle `apply`/
    /// `resume_pending` serve. `occupied_cells` carries only the cells the caller has already
    /// proven are currently occupied (WOBJ-RL-01: at most one use input per actor per Channel
    /// work cycle, so the caller supplies at most the acting actor's own current cell here).
    pub fn attempt_use(
        &mut self,
        expected_revision: u64,
        occupied_cells: &BTreeSet<LogicalCell>,
    ) -> Result<LocalObjectUseOutcome, WorldRuntimeError> {
        // Codex 5869579920: a stale caller's `expected_revision` is checked against the
        // current revision before selection ever runs. Selection reads the runtime's
        // *current* state (a different state than the one the stale caller last saw), so
        // running it first can report NOTHING_TO_USE/REJECTED/OCCUPIED computed from a state
        // the caller does not know about, instead of the STALE_STATE its evidence actually
        // calls for. A stale caller must always get STALE_STATE, regardless of what the
        // current state's own transition topology looks like.
        if expected_revision != self.revision {
            return Ok(LocalObjectUseOutcome::StaleState);
        }
        let transition_key = match self.select_use_transition() {
            Ok(key) => key,
            Err(UseSelectionError::NoCandidate) => return Ok(LocalObjectUseOutcome::NothingToUse),
            Err(UseSelectionError::Ambiguous) => return Ok(LocalObjectUseOutcome::Rejected),
        };
        let transition =
            self.transitions
                .get(&transition_key)
                .ok_or(WorldRuntimeError::InvalidBinding(
                    "selected use transition is no longer bound",
                ))?;
        // Guaranteed by `select_use_transition`'s own filter, not re-derived from client input.
        debug_assert_eq!(transition.source_state, self.state);
        let target_collision = local_object_state_collision(&self.states, &transition.target_state)
            .ok_or(WorldRuntimeError::InvalidBinding(
                "bound transition's target state is not declared by the local object",
            ))?;
        if target_collision == LocalObjectCollisionPresence::Present
            && !self.collision_cells.is_disjoint(occupied_cells)
        {
            return Ok(LocalObjectUseOutcome::Occupied);
        }
        let Some(next_revision) = self.revision.checked_add(1) else {
            return Ok(LocalObjectUseOutcome::Rejected);
        };
        let next_state = transition.target_state.clone();
        self.blocking_cells = match target_collision {
            LocalObjectCollisionPresence::Present => self.collision_cells.clone(),
            LocalObjectCollisionPresence::Absent => BTreeSet::new(),
        };
        self.state = next_state.clone();
        self.revision = next_revision;
        Ok(LocalObjectUseOutcome::Committed {
            state: next_state,
            revision: next_revision,
        })
    }

    fn transition_for(
        &self,
        operation: &LocalObjectOperation,
    ) -> Result<&TransitionBinding, WorldRuntimeError> {
        self.transitions
            .get(operation.transition_key())
            .ok_or(WorldRuntimeError::InvalidBinding(
                "command names a transition this local-object runtime does not bind",
            ))
    }

    /// #162 §7: whether this placement authors a `revert_after_ms` for `transition` under any
    /// action. Such a transition commits only through `apply_scope_operation` with the revert
    /// driver's capability. It is always event-owned (D91, enforced at `bind`), so the session
    /// and USE paths already refuse it.
    fn carries_revert(&self, transition: &TransitionKey) -> bool {
        self.transition_revert_after_ms
            .keys()
            .any(|(timed, _action)| timed == transition)
    }

    fn command_semantic_identity(
        &self,
        command: &LocalObjectCommand,
    ) -> Result<CommandSemanticIdentity, WorldRuntimeError> {
        // Owner decision D91: an event-owned transition (which includes every timed one) is not
        // session-invocable; it is refused exactly like a transition this runtime does not bind,
        // before ingress or mutation. This covers `apply` and `resume_pending`.
        if !self.is_player_use(command.operation.transition_key()) {
            return Err(WorldRuntimeError::InvalidBinding(
                "command names a transition this local-object runtime does not bind",
            ));
        }
        let transition = self.transition_for(&command.operation)?;
        Ok(CommandSemanticIdentity::new(
            NormalizedSemanticIntentIdentity::new(
                command.placement.as_str(),
                command.incarnation,
                transition.normalized_intent_family.as_str(),
                command.expected_revision,
            )?,
            RetainedBindingIdentity::new(
                command.content_generation.as_str(),
                transition.key.as_str(),
            )?,
        ))
    }

    pub fn apply<T: Copy + Eq>(
        &mut self,
        authority: &GameSessionAuthoritySnapshot<T>,
        command: &LocalObjectCommand,
        ingress: &mut CommandIngress,
        occupied_cells: &BTreeSet<LogicalCell>,
    ) -> Result<LocalObjectCommandResult, WorldRuntimeError> {
        self.validate_current_authority(authority, command)?;
        let semantic = self.command_semantic_identity(command)?;
        let command_id = command.command_ref.command_id();

        match ingress.reserve(command_id, semantic.clone()) {
            IngressDecision::Reserved(_) => {}
            IngressDecision::AlreadyReserved(_) => {
                return self.handle_duplicate(ingress, command_id, &semantic);
            }
            IngressDecision::SequenceGap { expected } => {
                return Err(WorldRuntimeError::IngressSequenceGap { expected });
            }
            IngressDecision::TooManyOutstanding { expected } => {
                return Err(WorldRuntimeError::IngressCapacityExceeded { expected });
            }
            IngressDecision::CommandSpaceExhausted => {
                return Err(WorldRuntimeError::CommandSpaceExhausted);
            }
        }

        self.terminalize_current(command, ingress, occupied_cells)
    }

    pub fn resume_pending<T: Copy + Eq>(
        &mut self,
        authority: &GameSessionAuthoritySnapshot<T>,
        command: &LocalObjectCommand,
        ingress: &mut CommandIngress,
        occupied_cells: &BTreeSet<LogicalCell>,
    ) -> Result<LocalObjectCommandResult, WorldRuntimeError> {
        self.validate_current_authority(authority, command)?;
        let semantic = self.command_semantic_identity(command)?;
        let command_id = command.command_ref.command_id();

        match ingress.classify_duplicate(command_id, &semantic) {
            DuplicateDisposition::PendingOriginal => {
                self.terminalize_current(command, ingress, occupied_cells)
            }
            DuplicateDisposition::ReplayRetainedOutcome(outcome) => {
                Ok(LocalObjectCommandResult::from_terminal(outcome, true))
            }
            DuplicateDisposition::ConflictChangedInput => {
                Err(WorldRuntimeError::ConflictChangedInput)
            }
            DuplicateDisposition::OutcomeExpired => Err(WorldRuntimeError::OutcomeExpired),
            DuplicateDisposition::NotDuplicate => {
                Err(WorldRuntimeError::PendingContinuationMissing)
            }
        }
    }

    fn terminalize_current(
        &mut self,
        command: &LocalObjectCommand,
        ingress: &mut CommandIngress,
        occupied_cells: &BTreeSet<LogicalCell>,
    ) -> Result<LocalObjectCommandResult, WorldRuntimeError> {
        let command_id = command.command_ref.command_id();
        let prepared = self.prepare(
            &command.placement,
            command.incarnation,
            &command.content_generation,
            &command.operation,
            command.expected_revision,
            occupied_cells,
        )?;
        let result = LocalObjectCommandResult::from_terminal(&prepared.outcome, false);
        let outcome = prepared.outcome;
        let mutation = prepared.mutation;
        ingress.terminalize(command_id, outcome, || mutation.commit(self))?;
        Ok(result)
    }

    fn validate_current_authority<T: Copy + Eq>(
        &self,
        authority: &GameSessionAuthoritySnapshot<T>,
        command: &LocalObjectCommand,
    ) -> Result<(), WorldRuntimeError> {
        if authority.session_state() != GameSessionState::Active {
            return Err(WorldRuntimeError::SessionNotActive);
        }
        if authority.current_game_session_id() != command.command_ref.game_session_id() {
            return Err(WorldRuntimeError::StaleGameSession);
        }

        let committed = authority.commit();
        if authority.current_transport().is_none() {
            return Err(WorldRuntimeError::InvalidSessionAuthority(
                "active GameSession has no current authenticated transport",
            ));
        }
        if authority.current_character_world_eligibility()
            != Some(CharacterWorldEligibilityClaimV1::new(
                committed.character_id(),
                committed.world_id(),
            ))
        {
            return Err(WorldRuntimeError::InvalidSessionAuthority(
                "current character/world eligibility does not match the committed GameSession",
            ));
        }
        let lease = authority.current_character_lease();
        if lease.character_id() != committed.character_id()
            || lease.generation() < committed.character_lease_generation()
            || (authority.current_game_session_id() == committed.game_session_id()
                && lease.generation() != committed.character_lease_generation())
        {
            return Err(WorldRuntimeError::InvalidSessionAuthority(
                "current character lease does not match Foundation authority",
            ));
        }
        if authority.current_connection_generation().get() < committed.connection_generation().get()
        {
            return Err(WorldRuntimeError::InvalidSessionAuthority(
                "current connection generation predates the committed GameSession",
            ));
        }
        if authority.current_scope_generation().get() < committed.scope_ownership_generation() {
            return Err(WorldRuntimeError::InvalidSessionAuthority(
                "current scope generation predates the committed GameSession",
            ));
        }

        if authority.current_connection_generation() != command.connection_generation {
            return Err(WorldRuntimeError::StaleConnectionGeneration);
        }
        if authority.current_runtime_scope() != self.scope {
            return Err(WorldRuntimeError::StaleRuntimeScope);
        }
        if authority.current_scope_generation() != self.scope_generation {
            return Err(WorldRuntimeError::StaleScopeOwnershipGeneration);
        }
        Ok(())
    }

    fn handle_duplicate(
        &self,
        ingress: &CommandIngress,
        command_id: CommandId,
        semantic: &CommandSemanticIdentity,
    ) -> Result<LocalObjectCommandResult, WorldRuntimeError> {
        match ingress.classify_duplicate(command_id, semantic) {
            DuplicateDisposition::ReplayRetainedOutcome(outcome) => {
                Ok(LocalObjectCommandResult::from_terminal(outcome, true))
            }
            DuplicateDisposition::PendingOriginal => Err(WorldRuntimeError::PendingOriginal),
            DuplicateDisposition::ConflictChangedInput => {
                Err(WorldRuntimeError::ConflictChangedInput)
            }
            DuplicateDisposition::OutcomeExpired => Err(WorldRuntimeError::OutcomeExpired),
            DuplicateDisposition::NotDuplicate => {
                Err(WorldRuntimeError::IngressClassificationMismatch)
            }
        }
    }

    /// #162 §7: a scope-origin, non-session operation — an encounter/server-event input or a
    /// due revert fired by the scope's own driver (`world_object_revert`). It never touches
    /// `CommandIngress` or `GameSessionAuthoritySnapshot`; it is fenced by the scope's own
    /// identity and generation and then reuses exactly `prepare` and `PreparedMutation::commit`.
    /// `stage_publish` runs only when `prepare` returned `Publish`, before `commit`; an error
    /// from it fails the whole operation with nothing committed (§7 Round 21: capacity is
    /// reserved only after `Publish`, in the same staged commit).
    ///
    /// Owner decision D91: only an event-owned transition bound to the operation's own owner is
    /// accepted. A `PlayerUse` edge, or another owner's edge (`EventOwnerMismatch`), is refused
    /// before `prepare` and any mutation. A transition carrying `revert_after_ms` at this placement is
    /// also refused, before any mutation, unless `capability` is presented (#1144 review). Only
    /// the §7 revert driver can construct one, so a timed transition never commits without its
    /// lifecycle record.
    pub fn apply_scope_operation<E>(
        &mut self,
        scope: RuntimeScopeRefV1,
        scope_generation: ScopeOwnershipGeneration,
        operation: &ScopeLocalObjectOperation,
        occupied_cells: &BTreeSet<LogicalCell>,
        capability: Option<&RevertSchedulingCapability>,
        stage_publish: impl FnOnce(&ScopePublish<'_>) -> Result<(), E>,
    ) -> Result<Result<TerminalSemanticOutcome, E>, WorldRuntimeError> {
        self.check_scope_fence(scope, scope_generation)?;
        self.check_scope_owner(operation)?;
        if capability.is_none() && self.carries_revert(operation.transition_key()) {
            return Err(WorldRuntimeError::InvalidBinding(
                "timed transition commits only through the revert scheduler",
            ));
        }
        let prepared = self.prepare(
            &operation.placement,
            operation.incarnation,
            &operation.content_generation,
            &operation.operation,
            operation.expected_revision,
            occupied_cells,
        )?;
        if let PreparedMutation::Publish {
            next_state,
            next_revision,
            ..
        } = &prepared.mutation
            && let Err(error) = stage_publish(&ScopePublish {
                next_state,
                next_revision: *next_revision,
            })
        {
            return Ok(Err(error));
        }
        prepared.mutation.commit(self);
        Ok(Ok(prepared.outcome))
    }

    fn prepare(
        &self,
        placement: &PlacementKey,
        incarnation: u64,
        content_generation: &ReferenceContentGeneration,
        operation: &LocalObjectOperation,
        expected_revision: u64,
        occupied_cells: &BTreeSet<LogicalCell>,
    ) -> Result<PreparedTerminal, WorldRuntimeError> {
        if placement != &self.placement
            || incarnation != self.incarnation
            || content_generation != &self.content_generation
        {
            return PreparedTerminal::unchanged(
                DISPOSITION_BINDING_MISMATCH,
                &self.state,
                self.revision,
            );
        }
        if expected_revision != self.revision {
            return PreparedTerminal::unchanged(
                DISPOSITION_STALE_STATE,
                &self.state,
                self.revision,
            );
        }

        let transition = self.transition_for(operation)?;
        if self.state == transition.target_state {
            return PreparedTerminal::unchanged(DISPOSITION_NO_CHANGE, &self.state, self.revision);
        }
        if self.state != transition.source_state {
            // Under the old fixed Open/Close pair this branch was unreachable: whichever of the
            // two states `self.state` held always matched one of the two known transitions' own
            // source or target. A runtime bound to more than two states, or more than one
            // transition out of the same state, makes "this operation's authored source state
            // does not match the current one" a normal, replay-safe outcome instead — the same
            // kind of stale precondition `expected_revision` already guards, just named by state
            // instead of by revision. Treat it identically: no mutation, no partial footprint.
            return PreparedTerminal::unchanged(
                DISPOSITION_STALE_STATE,
                &self.state,
                self.revision,
            );
        }

        // D38: the next blocking footprint comes from the *target* state's own authored
        // collision presence, not from which named operation was invoked. OCCUPIED is likewise
        // general: any transition landing on a collision-Present state onto an occupied footprint
        // is rejected, whether that transition is a CLOSE, a CREATE, a RETAG or a plain TRANSFORM.
        let target_collision = local_object_state_collision(&self.states, &transition.target_state)
            .ok_or(WorldRuntimeError::InvalidBinding(
                "bound transition's target state is not declared by the local object",
            ))?;
        if target_collision == LocalObjectCollisionPresence::Present
            && !self.collision_cells.is_disjoint(occupied_cells)
        {
            return PreparedTerminal::unchanged(DISPOSITION_OCCUPIED, &self.state, self.revision);
        }

        let Some(next_revision) = self.revision.checked_add(1) else {
            return PreparedTerminal::unchanged(
                DISPOSITION_REVISION_EXHAUSTED,
                &self.state,
                self.revision,
            );
        };
        let next_blocking = match target_collision {
            LocalObjectCollisionPresence::Present => self.collision_cells.clone(),
            LocalObjectCollisionPresence::Absent => BTreeSet::new(),
        };
        let outcome = TerminalSemanticOutcome::new(
            DISPOSITION_COMMITTED,
            transition.target_state.as_str(),
            next_revision,
        )?;
        Ok(PreparedTerminal {
            outcome,
            mutation: PreparedMutation::Publish {
                expected_state: self.state.clone(),
                expected_revision: self.revision,
                next_state: transition.target_state.clone(),
                next_revision,
                next_blocking,
            },
        })
    }
}

fn unique_transition<'a>(
    content: &'a CanonicalReferencePlayableContent,
    key: &TransitionKey,
) -> Result<&'a TransitionBinding, WorldRuntimeError> {
    let mut matches = content
        .transitions
        .iter()
        .filter(|transition| &transition.key == key);
    let transition = matches.next().ok_or(WorldRuntimeError::InvalidBinding(
        "selected TransitionKey is absent from active Content",
    ))?;
    if matches.next().is_some() {
        return Err(WorldRuntimeError::InvalidBinding(
            "selected TransitionKey is ambiguous in active Content",
        ));
    }
    Ok(transition)
}

fn local_object_states_contain(states: &[LocalObjectStateDefinition], key: &ProductionKey) -> bool {
    states.iter().any(|state| &state.key == key)
}

fn local_object_state_collision(
    states: &[LocalObjectStateDefinition],
    key: &ProductionKey,
) -> Option<LocalObjectCollisionPresence> {
    states
        .iter()
        .find(|state| &state.key == key)
        .map(|state| state.collision)
}

fn absolute_collision_cells(
    placement: &crate::content::PlacementRef,
) -> Result<BTreeSet<LogicalCell>, WorldRuntimeError> {
    let members = match &placement.collision_footprint {
        FootprintRelation::Qualified { members, .. } if !members.is_empty() => members,
        FootprintRelation::Qualified { .. } => {
            return Err(WorldRuntimeError::InvalidBinding(
                "local-object collision footprint is empty",
            ));
        }
        FootprintRelation::Unresolved(_) => {
            return Err(WorldRuntimeError::InvalidBinding(
                "local-object collision footprint is unresolved",
            ));
        }
    };

    let mut cells = BTreeSet::new();
    for member in members {
        let cell = LogicalCell {
            x: placement.address.cell.x.checked_add(member.dx).ok_or(
                WorldRuntimeError::InvalidBinding("collision footprint x coordinate overflow"),
            )?,
            y: placement.address.cell.y.checked_add(member.dy).ok_or(
                WorldRuntimeError::InvalidBinding("collision footprint y coordinate overflow"),
            )?,
            z: placement.address.cell.z.checked_add(member.dz).ok_or(
                WorldRuntimeError::InvalidBinding("collision footprint z coordinate overflow"),
            )?,
        };
        if !cells.insert(cell) {
            return Err(WorldRuntimeError::InvalidBinding(
                "local-object collision footprint contains duplicate absolute cell",
            ));
        }
    }
    Ok(cells)
}

#[derive(Debug)]
struct PreparedTerminal {
    outcome: TerminalSemanticOutcome,
    mutation: PreparedMutation,
}

impl PreparedTerminal {
    fn unchanged(
        disposition: &str,
        state: &ProductionKey,
        revision: u64,
    ) -> Result<Self, WorldRuntimeError> {
        Ok(Self {
            outcome: TerminalSemanticOutcome::new(disposition, state.as_str(), revision)?,
            mutation: PreparedMutation::None,
        })
    }
}

#[derive(Debug)]
enum PreparedMutation {
    None,
    Publish {
        expected_state: ProductionKey,
        expected_revision: u64,
        next_state: ProductionKey,
        next_revision: u64,
        next_blocking: BTreeSet<LogicalCell>,
    },
}

impl PreparedMutation {
    fn commit(self, runtime: &mut LocalObjectRuntime) {
        match self {
            Self::None => {}
            Self::Publish {
                expected_state,
                expected_revision,
                next_state,
                next_revision,
                next_blocking,
            } => {
                assert_eq!(
                    runtime.state, expected_state,
                    "CW4 fail-stop: local-object state changed after final validation"
                );
                assert_eq!(
                    runtime.revision, expected_revision,
                    "CW4 fail-stop: local-object revision changed after final validation"
                );
                runtime.state = next_state;
                runtime.revision = next_revision;
                runtime.blocking_cells = next_blocking;
            }
        }
    }
}

#[cfg(test)]
#[path = "world_runtime_tests.rs"]
mod tests;
// Read-only existing Map-owner fence validation. No grant construction/publication is added.
impl ScopeContentGenerationFence {
    pub fn qualifies_current_content(
        &self,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        content: &CanonicalReferencePlayableContent,
    ) -> Result<(), WorldRuntimeError> {
        let native = ReferenceContentGeneration::from_content(content)?;
        self.validate_candidate(scope, generation, &native)
    }
}

pub use crate::content::bosses::tentacles::{
    TENTACLE_COLLIDING_ITEM_IDS, TENTACLE_SOURCE_MAP_ROWS, TentacleSourceMapRow,
};
impl LocalObjectRuntime {
    pub fn tentacle_source_map_binding_matches(
        &self,
        content: &CanonicalReferencePlayableContent,
        op: &ScopeLocalObjectOperation,
        actor_key: &str,
        death: crate::foundation::MovementLocalPosition,
    ) -> Result<bool, WorldRuntimeError> {
        use crate::content::{ReferenceItemField, ReferenceItemStackClass};
        let Some((_, line, final_at, line_id, from_id, to_id)) =
            TENTACLE_SOURCE_MAP_ROWS.iter().find(|r| r.0 == actor_key)
        else {
            return Ok(false);
        };
        let current = line
            .iter()
            .position(|p| *p == (death.x, death.y, death.floor));
        let next = current.map_or(0, |i| i + 1);
        let (at, source_id, target_id, intent) = if let Some(at) = line.get(next) {
            (
                *at,
                *line_id,
                None,
                crate::content::LOCAL_OBJECT_REMOVE_INTENT_FAMILY,
            )
        } else {
            (
                *final_at,
                *from_id,
                Some(*to_id),
                crate::content::LOCAL_OBJECT_TRANSFORM_INTENT_FAMILY,
            )
        };
        let generation = ReferenceContentGeneration::from_content(content)?
            .with_event_origins(&self.event_transitions)?;
        if self.content_generation != generation || self.scope.world_id() != content.world_id {
            return Ok(false);
        }
        let Some(p) = content.placements.iter().find(|p| p.key == self.placement) else {
            return Ok(false);
        };
        if p.address.world_id != content.world_id
            || p.address.coordinate_frame != content.coordinate_frame
            || p.address.cell
                != (LogicalCell {
                    x: at.0,
                    y: at.1,
                    z: i32::from(at.2),
                })
        {
            return Ok(false);
        }
        let Some(t) = self.transitions.get(op.transition_key()) else {
            return Ok(false);
        };
        if t.definition != p.definition
            || t.normalized_intent_family.as_str() != intent
            || t.source_state.as_str() != format!("oteryn:item.tibia.i{source_id}")
        {
            return Ok(false);
        }
        if let Some(id) = target_id {
            if t.target_state.as_str() != format!("oteryn:item.tibia.i{id}") {
                return Ok(false);
            }
        } else if !self
            .states
            .iter()
            .find(|s| s.key == t.target_state)
            .is_some_and(|s| s.absent)
        {
            return Ok(false);
        }
        if self
            .transition_revert_after_ms
            .keys()
            .any(|(key, _)| key == op.transition_key())
        {
            return Ok(false);
        }
        for id in [Some(source_id), target_id].into_iter().flatten() {
            let expected_collision = if TENTACLE_COLLIDING_ITEM_IDS.contains(&id) {
                LocalObjectCollisionPresence::Present
            } else {
                LocalObjectCollisionPresence::Absent
            };
            if !self.states.iter().any(|s| {
                s.key.as_str() == format!("oteryn:item.tibia.i{id}")
                    && !s.absent
                    && s.collision == expected_collision
            }) {
                return Ok(false);
            }
            let Some(d) = content.definitions.iter().find(|d| {
                d.definition.family() == DefinitionFamily::Item
                    && d.definition.key().as_str() == format!("oteryn:item.tibia.i{id}")
                    && d.definition.revision().as_str() == "definition-r1"
            }) else {
                return Ok(false);
            };
            let ReferenceDefinitionKind::Item(item) = &d.kind else {
                return Ok(false);
            };
            let ReferenceItemField::Known(physical) = &item.semantics.physical else {
                return Ok(false);
            };
            if item.materializable
                || item.stack_class != ReferenceItemStackClass::NonStackable
                || physical.movable != ReferenceItemField::Known(false)
                || physical.pickupable != ReferenceItemField::Known(false)
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

impl crate::native_world_read::NativeWorldObjectRead for LocalObjectRuntime {
    type Error = WorldRuntimeError;

    fn qualify_entry(
        &self,
        room: &crate::content::QualifiedNativeEntryRoom,
        runtime: &crate::foundation::ChannelRuntimeV1,
    ) -> Result<bool, Self::Error> {
        self.check_scope_fence(
            crate::foundation::RuntimeScopeRefV1::Channel {
                world_id: runtime.binding().world_id(),
                channel_id: runtime.binding().channel_id(),
            },
            runtime.binding().scope_generation(),
        )?;
        let cells = room.movement_cells();
        let door = room.door();
        Ok(door.world_id == cells.scope().world_id
            && door.coordinate_frame == cells.scope().coordinate_frame
            && door.content_lock == cells.scope().content_lock
            && self.content_generation() == &ReferenceContentGeneration::from_content(door)?)
    }

    fn blocks_movement(&self, position: crate::foundation::MovementLocalPosition) -> bool {
        self.blocking_cells()
            .contains(&crate::content::LogicalCell {
                x: position.x,
                y: position.y,
                z: i32::from(position.floor),
            })
    }
}
