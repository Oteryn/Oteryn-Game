use crate::content::{
    CanonicalReferencePlayableContent, ContentError, DefinitionFamily, FootprintRelation,
    LogicalCell, NonAuthoritativeReferenceStage, PlacementKey, ProductionKey,
    REFERENCE_PLAYABLE_CAPABILITY_PROFILE, REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
    ReferenceDefinitionKind, ReferencePlayableContentSource, ReferencePlayableGenerationIdentity,
    ReferenceServerItem, TransitionBinding, TransitionKey, TypedDefinitionRef,
    link_reference_playable,
};
use crate::foundation::{
    CharacterWorldEligibilityClaimV1, CommandId, CommandIngress, CommandLifecycleError, CommandRef,
    CommandSemanticIdentity, ConnectionGeneration, DuplicateDisposition,
    GameSessionAuthoritySnapshot, GameSessionState, IngressDecision,
    NormalizedSemanticIntentIdentity, RetainedBindingIdentity, RuntimeScopeRefV1,
    ScopeOwnershipGeneration, TerminalSemanticOutcome,
};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

const DISPOSITION_COMMITTED: &str = "COMMITTED";
const DISPOSITION_NO_CHANGE: &str = "NO_CHANGE";
const DISPOSITION_OCCUPIED: &str = "OCCUPIED";
const DISPOSITION_BINDING_MISMATCH: &str = "BINDING_MISMATCH";
const DISPOSITION_STALE_STATE: &str = "STALE_STATE";
const DISPOSITION_REVISION_EXHAUSTED: &str = "REVISION_EXHAUSTED";
const LOCAL_OBJECT_TRANSITION_CAPABILITY: &str =
    "oteryn:runtime.capability.local-object-transition";
const LOCAL_OBJECT_OPEN_INTENT_FAMILY: &str = "oteryn:reference.intent.local-object-open";
const LOCAL_OBJECT_CLOSE_INTENT_FAMILY: &str = "oteryn:reference.intent.local-object-close";
const REFERENCE_CONTENT_GENERATION_DOMAIN: &[u8] = b"OTERYN/CW4/REFERENCE_CONTENT_GENERATION/v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReferenceContentGeneration {
    semantic_identity: Box<str>,
}

impl ReferenceContentGeneration {
    pub(crate) fn from_content(
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

    #[must_use]
    fn as_str(&self) -> &str {
        &self.semantic_identity
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScopeContentGenerationFence {
    scope: RuntimeScopeRefV1,
    scope_generation: ScopeOwnershipGeneration,
    content_generation: ReferenceContentGeneration,
}

impl ScopeContentGenerationFence {
    // This CW4 child has no production owner-composition authority. Keep construction
    // test-only until the current scope owner supplies this fence at a later allocated seam.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn for_test(
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
pub(crate) struct NonAuthoritativeReferenceItemRuntimeView {
    scope: RuntimeScopeRefV1,
    generation_identity: ReferencePlayableGenerationIdentity,
    authoritative_item: ReferenceServerItem,
}

impl NonAuthoritativeReferenceItemRuntimeView {
    pub(crate) fn bind(
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
    pub(crate) const fn scope(&self) -> RuntimeScopeRefV1 {
        self.scope
    }

    #[must_use]
    pub(crate) fn generation_identity(&self) -> &ReferencePlayableGenerationIdentity {
        &self.generation_identity
    }

    #[must_use]
    pub(crate) fn item_identity(&self) -> &TypedDefinitionRef {
        self.generation_identity.item_identity()
    }

    #[must_use]
    pub(crate) fn server_artifact_digest(&self) -> [u8; 32] {
        self.generation_identity.server_artifact_digest()
    }

    #[must_use]
    pub(crate) fn client_artifact_digest(&self) -> [u8; 32] {
        self.generation_identity.client_artifact_digest()
    }

    #[must_use]
    pub(crate) fn lookup_item(
        &self,
        identity: &TypedDefinitionRef,
    ) -> Option<&ReferenceServerItem> {
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

    for block in padded.chunks_exact(64) {
        let mut words = [0_u32; 64];
        for (index, chunk) in block.chunks_exact(4).enumerate() {
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
    for (chunk, value) in output.chunks_exact_mut(4).zip(state) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalObjectOperation {
    Open,
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalObjectCommand {
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
    pub(crate) const fn new(
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
    pub(crate) const fn command_ref(&self) -> CommandRef {
        self.command_ref
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalObjectCommandResult {
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
    pub(crate) fn disposition(&self) -> &str {
        &self.disposition
    }

    #[must_use]
    pub(crate) fn state(&self) -> &str {
        &self.state
    }

    #[must_use]
    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub(crate) const fn replayed(&self) -> bool {
        self.replayed
    }
}

#[derive(Debug)]
pub(crate) enum WorldRuntimeError {
    Content(ContentError),
    CommandLifecycle(CommandLifecycleError),
    InvalidBinding(&'static str),
    SessionNotActive,
    StaleGameSession,
    StaleConnectionGeneration,
    StaleRuntimeScope,
    StaleScopeOwnershipGeneration,
    IngressSequenceGap { expected: CommandId },
    IngressCapacityExceeded { expected: CommandId },
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
pub(crate) struct LocalObjectRuntime {
    scope: RuntimeScopeRefV1,
    scope_generation: ScopeOwnershipGeneration,
    content_generation: ReferenceContentGeneration,
    placement: PlacementKey,
    incarnation: u64,
    open_transition: TransitionBinding,
    close_transition: TransitionBinding,
    state: ProductionKey,
    revision: u64,
    collision_cells: BTreeSet<LogicalCell>,
    blocking_cells: BTreeSet<LogicalCell>,
}

impl LocalObjectRuntime {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn bind(
        content: &CanonicalReferencePlayableContent,
        active_content: &ScopeContentGenerationFence,
        scope: RuntimeScopeRefV1,
        scope_generation: ScopeOwnershipGeneration,
        placement_key: &PlacementKey,
        incarnation: u64,
        open_transition_key: &TransitionKey,
        close_transition_key: &TransitionKey,
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
        let content_generation = ReferenceContentGeneration::from_content(content)?;
        active_content.validate_candidate(scope, scope_generation, &content_generation)?;

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
            ReferenceDefinitionKind::LocalObjectStates(states) if states.len() == 2 => states,
            _ => {
                return Err(WorldRuntimeError::InvalidBinding(
                    "first CW4 child requires exactly two local-object states",
                ));
            }
        };

        let open_transition = unique_transition(content, open_transition_key)?.clone();
        let close_transition = unique_transition(content, close_transition_key)?.clone();
        if open_transition.definition != placement.definition
            || close_transition.definition != placement.definition
        {
            return Err(WorldRuntimeError::InvalidBinding(
                "OPEN/CLOSE transitions do not bind the selected placement definition",
            ));
        }
        if open_transition.source_state == open_transition.target_state
            || close_transition.source_state != open_transition.target_state
            || close_transition.target_state != open_transition.source_state
        {
            return Err(WorldRuntimeError::InvalidBinding(
                "OPEN/CLOSE transitions are not an exact two-state inverse pair",
            ));
        }
        if !states.contains(&open_transition.source_state)
            || !states.contains(&open_transition.target_state)
        {
            return Err(WorldRuntimeError::InvalidBinding(
                "OPEN/CLOSE transition states are not declared by the local object",
            ));
        }
        if open_transition.normalized_intent_family == close_transition.normalized_intent_family {
            return Err(WorldRuntimeError::InvalidBinding(
                "OPEN and CLOSE must have distinct normalized intent families",
            ));
        }
        if open_transition.normalized_intent_family.as_str() != LOCAL_OBJECT_OPEN_INTENT_FAMILY
            || close_transition.normalized_intent_family.as_str()
                != LOCAL_OBJECT_CLOSE_INTENT_FAMILY
        {
            return Err(WorldRuntimeError::InvalidBinding(
                "OPEN/CLOSE transition intent families do not match runtime operations",
            ));
        }
        if open_transition.owner_capability.capability_key.as_str()
            != LOCAL_OBJECT_TRANSITION_CAPABILITY
            || close_transition.owner_capability.capability_key.as_str()
                != LOCAL_OBJECT_TRANSITION_CAPABILITY
        {
            return Err(WorldRuntimeError::InvalidBinding(
                "OPEN/CLOSE transition capability is not supported by this runtime profile",
            ));
        }
        if !open_transition.policy_guard_refs.is_empty()
            || !close_transition.policy_guard_refs.is_empty()
        {
            return Err(WorldRuntimeError::InvalidBinding(
                "first CW4 child cannot bypass unresolved policy guards",
            ));
        }

        let collision_cells = absolute_collision_cells(placement)?;
        Ok(Self {
            scope,
            scope_generation,
            content_generation,
            placement: placement.key.clone(),
            incarnation,
            state: open_transition.source_state.clone(),
            revision: 0,
            blocking_cells: collision_cells.clone(),
            collision_cells,
            open_transition,
            close_transition,
        })
    }

    #[must_use]
    pub(crate) fn placement_key(&self) -> &PlacementKey {
        &self.placement
    }

    #[must_use]
    pub(crate) const fn incarnation(&self) -> u64 {
        self.incarnation
    }

    #[must_use]
    pub(crate) fn content_generation(&self) -> &ReferenceContentGeneration {
        &self.content_generation
    }

    #[must_use]
    pub(crate) fn state_key(&self) -> &ProductionKey {
        &self.state
    }

    #[must_use]
    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub(crate) fn collision_cells(&self) -> &BTreeSet<LogicalCell> {
        &self.collision_cells
    }

    #[must_use]
    pub(crate) fn blocking_cells(&self) -> &BTreeSet<LogicalCell> {
        &self.blocking_cells
    }

    #[must_use]
    fn transition_for(&self, operation: LocalObjectOperation) -> &TransitionBinding {
        match operation {
            LocalObjectOperation::Open => &self.open_transition,
            LocalObjectOperation::Close => &self.close_transition,
        }
    }

    fn command_semantic_identity(
        &self,
        command: &LocalObjectCommand,
    ) -> Result<CommandSemanticIdentity, WorldRuntimeError> {
        let transition = self.transition_for(command.operation);
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

    pub(crate) fn apply<T: Copy + Eq>(
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

    pub(crate) fn resume_pending<T: Copy + Eq>(
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
        let prepared = self.prepare(command, occupied_cells)?;
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

    fn prepare(
        &self,
        command: &LocalObjectCommand,
        occupied_cells: &BTreeSet<LogicalCell>,
    ) -> Result<PreparedTerminal, WorldRuntimeError> {
        if command.placement != self.placement
            || command.incarnation != self.incarnation
            || command.content_generation != self.content_generation
        {
            return PreparedTerminal::unchanged(
                DISPOSITION_BINDING_MISMATCH,
                &self.state,
                self.revision,
            );
        }
        if command.expected_revision != self.revision {
            return PreparedTerminal::unchanged(
                DISPOSITION_STALE_STATE,
                &self.state,
                self.revision,
            );
        }

        let transition = self.transition_for(command.operation);
        if self.state == transition.target_state {
            return PreparedTerminal::unchanged(DISPOSITION_NO_CHANGE, &self.state, self.revision);
        }
        if self.state != transition.source_state {
            return Err(WorldRuntimeError::InvalidBinding(
                "runtime state escaped the bound two-state transition",
            ));
        }

        if command.operation == LocalObjectOperation::Close
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
        let next_blocking = match command.operation {
            LocalObjectOperation::Open => BTreeSet::new(),
            LocalObjectOperation::Close => self.collision_cells.clone(),
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
mod tests {
    use super::*;
    use crate::content::{
        CW2_B1_VASE_KEY, CW2_B1_VASE_REVISION, CanonicalProjectDocuments, ClientProjectionClass,
        ContentActivationController, ContentLockBinding, ContentLockEntry, CoordinateFrameRef,
        DefinitionRevisionRef, EvidenceBindingRef, EvidenceDisposition, FootprintCell,
        MapRevisionRef, OwnerCapabilityRequirement, PackageManifestBinding, PlacementRef,
        ProductionAtom, ProjectDraft, ProjectEvidenceLimits, ReferenceDefinition,
        ReferenceItemDestination, ReferenceItemPhysicalClass, ReferenceItemStackClass,
        Sha256HexDigest, TypedDefinitionRef, compile_reference_playable,
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
        "../../../docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
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

    fn canonical_b1_vase_content()
    -> Result<CanonicalReferencePlayableContent, Box<dyn std::error::Error>> {
        let imported = protected_cw2_b1_vase_import(B1_EVIDENCE)?;
        let limits = reference_project_limits();
        Ok(CanonicalProjectDocuments::from_draft(
            ProjectDraft {
                project_revision: "project-r1".to_owned(),
                package_key: "oteryn:content.world-project".to_owned(),
                semantic_schema_version: "reference-schema-v1".to_owned(),
                licensing_metadata: "PENDING".to_owned(),
                world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
                coordinate_frame: "global-target-2026-07-28".to_owned(),
                records: vec![imported.record],
                imports: vec![imported.batch],
                metadata: Vec::new(),
            },
            limits,
        )?
        .into_snapshot(limits)?
        .parse(limits)?
        .link()?)
    }

    #[test]
    fn protected_b1_vase_stage_binds_exact_runtime_item_view_without_controller_authority()
    -> Result<(), Box<dyn std::error::Error>> {
        let linked = canonical_b1_vase_content()?;
        let compiled = compile_reference_playable(&linked)?;
        let staged = stage_reference_playable(
            &compiled.server_artifact,
            &compiled.client_artifact,
            compiled.expectation(),
        )?;
        let exact_generation = staged.identity().clone();
        let exact_item = exact_generation.item_identity().clone();
        let channel = ChannelId::decode(&uuid_v7(91))?;
        let scope = RuntimeScopeRefV1::channel(linked.world_id, channel);
        let controller = ContentActivationController::new();

        let view = NonAuthoritativeReferenceItemRuntimeView::bind(staged, scope)?;

        assert_eq!(view.scope(), scope);
        assert_eq!(view.generation_identity(), &exact_generation);
        assert_eq!(view.item_identity(), &exact_item);
        assert_eq!(view.item_identity().family(), DefinitionFamily::Item);
        assert_eq!(view.item_identity().key().as_str(), CW2_B1_VASE_KEY);
        assert_eq!(
            view.item_identity().revision().as_str(),
            CW2_B1_VASE_REVISION
        );
        assert_eq!(view.server_artifact_digest(), compiled.server_digest());
        assert_eq!(view.client_artifact_digest(), compiled.client_digest());

        let item = view
            .lookup_item(&exact_item)
            .ok_or(fixture_error("runtime Item lookup"))?;
        assert_eq!(item.physical_class, ReferenceItemPhysicalClass::Physical);
        assert!(item.materializable);
        assert_eq!(item.stack_class, ReferenceItemStackClass::NonStackable);
        assert_eq!(
            item.legal_destinations,
            [ReferenceItemDestination::CharacterInventory]
        );

        let unknown = TypedDefinitionRef::new(
            DefinitionFamily::Item,
            ProductionKey::new("oteryn:item.decor.unknown")?,
            DefinitionRevisionRef::new("definition-r1")?,
        );
        assert_eq!(view.lookup_item(&unknown), None);
        assert!(controller.staged_identity().is_none());
        assert!(controller.active().is_none());
        assert!(!controller.is_ready());
        Ok(())
    }

    #[test]
    fn reference_item_runtime_view_rejects_a_different_world()
    -> Result<(), Box<dyn std::error::Error>> {
        let linked = canonical_b1_vase_content()?;
        let compiled = compile_reference_playable(&linked)?;
        let staged = stage_reference_playable(
            &compiled.server_artifact,
            &compiled.client_artifact,
            compiled.expectation(),
        )?;
        let other_world = decode_world(99)?;
        assert_ne!(other_world, linked.world_id);
        let channel = ChannelId::decode(&uuid_v7(92))?;

        assert!(matches!(
            NonAuthoritativeReferenceItemRuntimeView::bind(
                staged,
                RuntimeScopeRefV1::channel(other_world, channel),
            ),
            Err(WorldRuntimeError::InvalidBinding(
                "Reference Item generation world does not match runtime scope"
            ))
        ));
        Ok(())
    }

    fn synthetic_content(
        package_revision_value: &str,
    ) -> Result<CanonicalReferencePlayableContent, WorldRuntimeError> {
        let world_id = decode_world(1)?;
        let package_key = ProductionKey::new("oteryn:content.cw4-local-object")?;
        let package_revision =
            ProductionAtom::new("cw4 test package revision", package_revision_value)?;
        let package_manifest = PackageManifestBinding::new(
            package_key.clone(),
            package_revision.clone(),
            ProductionAtom::new("cw4 test schema", "schema-v1")?,
            ProductionAtom::new("cw4 test license", "license:project-owned-v1")?,
            Sha256HexDigest::new(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )?,
        );
        let provenance = package_manifest.package_provenance_digest()?;
        let content_lock = ContentLockBinding {
            revision_digest_token: ProductionAtom::new(
                "cw4 test content lock",
                &format!("lock:{package_revision_value}"),
            )?,
            entries: vec![ContentLockEntry::exact(
                package_key,
                package_revision,
                provenance,
            )],
        };

        let object_ref = TypedDefinitionRef::new(
            DefinitionFamily::LocalObject,
            ProductionKey::new("oteryn:reference.object.cw4-local")?,
            DefinitionRevisionRef::new("definition-r1")?,
        );
        let closed = ProductionKey::new("oteryn:reference.state.closed")?;
        let open = ProductionKey::new("oteryn:reference.state.open")?;
        let coordinate_frame = CoordinateFrameRef::new("global-target-2026-07-28")?;
        let capability = OwnerCapabilityRequirement {
            capability_key: ProductionKey::new(
                "oteryn:runtime.capability.local-object-transition",
            )?,
        };

        let mut canonical = link_reference_playable(ReferencePlayableContentSource {
            profile_revision: ProductionAtom::new(
                "cw4 test profile",
                REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
            )?,
            capability_profile: ProductionAtom::new(
                "cw4 test capability profile",
                REFERENCE_PLAYABLE_CAPABILITY_PROFILE,
            )?,
            package_manifest,
            content_lock,
            world_id,
            coordinate_frame: coordinate_frame.clone(),
            definitions: vec![ReferenceDefinition {
                definition: object_ref.clone(),
                kind: ReferenceDefinitionKind::LocalObjectStates(vec![
                    closed.clone(),
                    open.clone(),
                ]),
                client_projection: ClientProjectionClass::ClientSafe,
            }],
            placements: vec![],
            ordered_placements: vec![],
            transitions: vec![
                TransitionBinding {
                    key: TransitionKey::new(OPEN_TRANSITION)?,
                    definition: object_ref.clone(),
                    source_state: closed.clone(),
                    normalized_intent_family: ProductionKey::new(
                        "oteryn:reference.intent.local-object-open",
                    )?,
                    target_state: open.clone(),
                    owner_capability: capability.clone(),
                    policy_guard_refs: vec![],
                },
                TransitionBinding {
                    key: TransitionKey::new(CLOSE_TRANSITION)?,
                    definition: object_ref.clone(),
                    source_state: open,
                    normalized_intent_family: ProductionKey::new(
                        "oteryn:reference.intent.local-object-close",
                    )?,
                    target_state: closed,
                    owner_capability: capability,
                    policy_guard_refs: vec![],
                },
            ],
        })?;

        // Protected CW3 currently has no accepted CONTENT_WORLD target-sensitive
        // evidence binding. The CW4 component is therefore a synthetic in-process
        // runtime proof only: its placement witness is deliberately unpromoted
        // and must never be interpreted as Reference target authority.
        let evidence = EvidenceBindingRef::new(
            ProductionAtom::new("reference manifest revision", "manifest-r0")?,
            ProductionKey::new("oteryn:cw4.local-object-placement")?,
            EvidenceDisposition::Unknown,
        );
        let collision_members = vec![
            FootprintCell {
                dx: 0,
                dy: 0,
                dz: 0,
            },
            FootprintCell {
                dx: 1,
                dy: 0,
                dz: 0,
            },
        ];
        let placement = |key: &str| -> Result<PlacementRef, WorldRuntimeError> {
            Ok(PlacementRef {
                key: PlacementKey::new(key)?,
                map_revision: MapRevisionRef::new("map-r1")?,
                definition: object_ref.clone(),
                address: crate::content::SpatialAddress {
                    world_id,
                    coordinate_frame: coordinate_frame.clone(),
                    cell: LogicalCell {
                        x: 100,
                        y: 200,
                        z: 7,
                    },
                    evidence: evidence.clone(),
                },
                presentation_footprint: FootprintRelation::Qualified {
                    members: collision_members.clone(),
                    evidence: evidence.clone(),
                },
                collision_footprint: FootprintRelation::Qualified {
                    members: collision_members.clone(),
                    evidence: evidence.clone(),
                },
            })
        };
        canonical.placements = vec![placement(PLACEMENT_A)?, placement(PLACEMENT_B)?];
        Ok(canonical)
    }

    fn authority(
        session_seed: u8,
        channel_seed: u8,
        scope_generation: u64,
        connection_generation: u64,
    ) -> Result<
        (
            GameSessionAuthoritySnapshot<u64>,
            crate::foundation::GameSessionId,
            RuntimeScopeRefV1,
        ),
        WorldRuntimeError,
    > {
        let world = decode_world(1)?;
        let channel = ChannelId::decode(&uuid_v7(channel_seed))
            .map_err(|_error: FoundationProtocolError| fixture_error("channel"))?;
        let character = CharacterId::decode(&uuid_v7(session_seed.wrapping_add(80)))
            .map_err(|_error: FoundationProtocolError| fixture_error("character"))?;
        let session = GameSessionId::decode(&uuid_v7(session_seed))
            .map_err(|_error: FoundationProtocolError| fixture_error("session"))?;
        let mut nonce = [0_u8; 32];
        nonce[31] = session_seed.max(1);
        let facts = FreshAdmissionFacts::new(nonce, character, world, channel, 1, scope_generation)
            .map_err(|_error| fixture_error("fresh admission"))?;
        let commit = FreshAdmissionCommit::from_facts(session, facts, 99_u64)
            .map_err(|_error| fixture_error("fresh commit"))?;
        let scope = RuntimeScopeRefV1::channel(world, channel);
        let connection_generation = ConnectionGeneration::new(connection_generation)
            .map_err(|_error: GenerationError| fixture_error("connection generation"))?;
        let character_lease =
            CharacterLease::new(character, 1).map_err(|_error| fixture_error("character lease"))?;
        let scope_generation = ScopeOwnershipGeneration::new(scope_generation)
            .map_err(|_error: GenerationError| fixture_error("scope generation"))?;
        let snapshot = GameSessionAuthoritySnapshot::new(
            commit,
            GameSessionState::Active,
            connection_generation,
            Some(99_u64),
            character_lease,
            scope_generation,
        );
        Ok((snapshot, session, scope))
    }

    fn runtime_for(
        content: &CanonicalReferencePlayableContent,
        scope: RuntimeScopeRefV1,
        placement: &str,
        scope_generation: u64,
    ) -> Result<LocalObjectRuntime, WorldRuntimeError> {
        let scope_generation = ScopeOwnershipGeneration::new(scope_generation)
            .map_err(|_error: GenerationError| fixture_error("scope generation"))?;
        let active_content = ScopeContentGenerationFence::for_test(
            scope,
            scope_generation,
            ReferenceContentGeneration::from_content(content)?,
        );
        LocalObjectRuntime::bind(
            content,
            &active_content,
            scope,
            scope_generation,
            &PlacementKey::new(placement)?,
            1,
            &TransitionKey::new(OPEN_TRANSITION)?,
            &TransitionKey::new(CLOSE_TRANSITION)?,
        )
    }

    fn command(
        runtime: &LocalObjectRuntime,
        session: GameSessionId,
        id: u64,
        connection_generation: u64,
        operation: LocalObjectOperation,
        expected_revision: u64,
    ) -> Result<LocalObjectCommand, WorldRuntimeError> {
        let command_id =
            CommandId::new(id).map_err(|_error: CommandIdError| fixture_error("command id"))?;
        let connection_generation = ConnectionGeneration::new(connection_generation)
            .map_err(|_error: GenerationError| fixture_error("connection generation"))?;
        Ok(LocalObjectCommand::new(
            CommandRef::new(session, command_id),
            connection_generation,
            runtime.placement_key().clone(),
            runtime.incarnation(),
            runtime.content_generation().clone(),
            operation,
            expected_revision,
        ))
    }

    #[test]
    fn open_close_then_replay_open_preserves_current_closed_state() -> Result<(), WorldRuntimeError>
    {
        let content = synthetic_content("package-r1")?;
        let (authority_a, session_a, scope) = authority(10, 3, 1, 1)?;
        let (authority_b, session_b, _) = authority(11, 3, 1, 1)?;
        let mut runtime = runtime_for(&content, scope, PLACEMENT_A, 1)?;
        let mut ingress_a = CommandIngress::new();
        let mut ingress_b = CommandIngress::new();
        let empty = BTreeSet::new();

        let open = command(&runtime, session_a, 1, 1, LocalObjectOperation::Open, 0)?;
        let opened = runtime.apply(&authority_a, &open, &mut ingress_a, &empty)?;
        assert_eq!(opened.disposition(), DISPOSITION_COMMITTED);
        assert_eq!(opened.revision(), 1);
        assert!(runtime.blocking_cells().is_empty());

        let close = command(&runtime, session_b, 1, 1, LocalObjectOperation::Close, 1)?;
        let closed = runtime.apply(&authority_b, &close, &mut ingress_b, &empty)?;
        assert_eq!(closed.disposition(), DISPOSITION_COMMITTED);
        assert_eq!(closed.revision(), 2);
        assert_eq!(runtime.blocking_cells(), runtime.collision_cells());

        let (stale_authority_a, _, _) = authority(10, 3, 1, 2)?;
        assert!(matches!(
            runtime.apply(&stale_authority_a, &open, &mut ingress_a, &empty),
            Err(WorldRuntimeError::StaleConnectionGeneration)
        ));
        assert_eq!(runtime.revision(), 2);

        let replay = runtime.apply(&authority_a, &open, &mut ingress_a, &empty)?;
        assert!(replay.replayed());
        assert_eq!(replay.disposition(), DISPOSITION_COMMITTED);
        assert_eq!(replay.state(), "oteryn:reference.state.open");
        assert_eq!(replay.revision(), 1);
        assert_eq!(
            runtime.state_key().as_str(),
            "oteryn:reference.state.closed"
        );
        assert_eq!(runtime.revision(), 2);
        Ok(())
    }

    #[test]
    fn later_command_cannot_commit_ahead_of_earlier_pending_across_objects()
    -> Result<(), WorldRuntimeError> {
        let content = synthetic_content("package-r1")?;
        let (authority, session, scope) = authority(20, 4, 1, 1)?;
        let mut runtime_a = runtime_for(&content, scope, PLACEMENT_A, 1)?;
        let mut runtime_b = runtime_for(&content, scope, PLACEMENT_B, 1)?;
        let mut ingress = CommandIngress::new();
        let first = command(&runtime_a, session, 1, 1, LocalObjectOperation::Open, 0)?;
        let first_semantic = runtime_a.command_semantic_identity(&first)?;
        assert_eq!(
            ingress.reserve(first.command_ref().command_id(), first_semantic),
            IngressDecision::Reserved(first.command_ref().command_id())
        );

        let second = command(&runtime_b, session, 2, 1, LocalObjectOperation::Open, 0)?;
        let before = runtime_b.blocking_cells().clone();
        assert!(matches!(
            runtime_b.apply(&authority, &second, &mut ingress, &BTreeSet::new()),
            Err(WorldRuntimeError::CommandLifecycle(
                CommandLifecycleError::TerminalOutOfOrder { .. }
            ))
        ));
        assert_eq!(runtime_b.revision(), 0);
        assert_eq!(runtime_b.blocking_cells(), &before);

        assert!(matches!(
            runtime_b.apply(&authority, &second, &mut ingress, &BTreeSet::new()),
            Err(WorldRuntimeError::PendingOriginal)
        ));
        assert_eq!(runtime_b.revision(), 0);

        let first_result =
            runtime_a.resume_pending(&authority, &first, &mut ingress, &BTreeSet::new())?;
        assert_eq!(first_result.disposition(), DISPOSITION_COMMITTED);
        assert_eq!(runtime_a.revision(), 1);

        let second_result =
            runtime_b.resume_pending(&authority, &second, &mut ingress, &BTreeSet::new())?;
        assert_eq!(second_result.disposition(), DISPOSITION_COMMITTED);
        assert_eq!(runtime_b.revision(), 1);
        assert!(runtime_b.blocking_cells().is_empty());
        assert_eq!(ingress.outstanding(), 0);
        Ok(())
    }

    #[test]
    fn swapped_open_close_intents_fail_closed_before_runtime_creation()
    -> Result<(), WorldRuntimeError> {
        let content = synthetic_content("package-r1")?;
        let (_authority, _session, scope) = authority(22, 4, 1, 1)?;
        let scope_generation = ScopeOwnershipGeneration::new(1)
            .map_err(|_error: GenerationError| fixture_error("scope generation"))?;
        let active_content = ScopeContentGenerationFence::for_test(
            scope,
            scope_generation,
            ReferenceContentGeneration::from_content(&content)?,
        );
        let error = match LocalObjectRuntime::bind(
            &content,
            &active_content,
            scope,
            scope_generation,
            &PlacementKey::new(PLACEMENT_A)?,
            1,
            &TransitionKey::new(CLOSE_TRANSITION)?,
            &TransitionKey::new(OPEN_TRANSITION)?,
        ) {
            Ok(_runtime) => {
                return Err(fixture_error(
                    "swapped OPEN/CLOSE semantic intents unexpectedly bound",
                ));
            }
            Err(error) => error,
        };
        assert!(matches!(
            error,
            WorldRuntimeError::InvalidBinding(
                "OPEN/CLOSE transition intent families do not match runtime operations"
            )
        ));
        Ok(())
    }

    #[test]
    fn invalid_active_authority_is_rejected_before_ingress_or_mutation()
    -> Result<(), WorldRuntimeError> {
        let content = synthetic_content("package-r1")?;
        let (authority, session, scope) = authority(23, 4, 1, 1)?;
        let invalid = GameSessionAuthoritySnapshot::from_current_facts(
            authority.commit(),
            GameSessionState::Active,
            authority.current_connection_generation(),
            None,
            authority.current_character_lease(),
            authority.current_character_world_eligibility(),
            authority.current_runtime_scope(),
            authority.current_scope_generation(),
        )
        .map_err(|_error| fixture_error("invalid current authority fixture"))?;
        let mut runtime = runtime_for(&content, scope, PLACEMENT_A, 1)?;
        let command = command(&runtime, session, 1, 1, LocalObjectOperation::Open, 0)?;
        let before_state = runtime.state_key().clone();
        let before_blocking = runtime.blocking_cells().clone();
        let mut ingress = CommandIngress::new();

        assert!(matches!(
            runtime.apply(&invalid, &command, &mut ingress, &BTreeSet::new()),
            Err(WorldRuntimeError::InvalidSessionAuthority(
                "active GameSession has no current authenticated transport"
            ))
        ));
        assert_eq!(ingress.outstanding(), 0);
        assert_eq!(runtime.state_key(), &before_state);
        assert_eq!(runtime.revision(), 0);
        assert_eq!(runtime.blocking_cells(), &before_blocking);
        Ok(())
    }

    #[test]
    fn authored_capability_cannot_grant_runtime_authority() -> Result<(), WorldRuntimeError> {
        let mut content = synthetic_content("package-r1")?;
        let unsupported = OwnerCapabilityRequirement {
            capability_key: ProductionKey::new("oteryn:runtime.capability.unowned-transition")?,
        };
        for transition in &mut content.transitions {
            transition.owner_capability = unsupported.clone();
        }
        let (_authority, _session, scope) = authority(21, 4, 1, 1)?;
        assert!(matches!(
            runtime_for(&content, scope, PLACEMENT_A, 1),
            Err(WorldRuntimeError::InvalidBinding(
                "OPEN/CLOSE transition capability is not supported by this runtime profile"
            ))
        ));
        Ok(())
    }

    #[test]
    fn content_generation_binding_stays_within_foundation_component_bound()
    -> Result<(), WorldRuntimeError> {
        let content = synthetic_content("package-r1")?;
        let generation = ReferenceContentGeneration::from_content(&content)?;
        let mut changed_lock = content.clone();
        changed_lock.content_lock.revision_digest_token =
            ProductionAtom::new("cw4 test content lock", "lock:changed")?;
        let changed_generation = ReferenceContentGeneration::from_content(&changed_lock)?;
        assert_eq!(generation.as_str().len(), 64);
        assert_eq!(changed_generation.as_str().len(), 64);
        assert_ne!(generation, changed_generation);
        Ok(())
    }

    #[test]
    fn live_scope_rejects_different_content_generation_before_second_runtime_creation()
    -> Result<(), WorldRuntimeError> {
        let content_a = synthetic_content("package-r1")?;
        let content_b = synthetic_content("package-r2")?;
        let (_authority, _session, scope) = authority(24, 4, 1, 1)?;
        let scope_generation = ScopeOwnershipGeneration::new(1)
            .map_err(|_error: GenerationError| fixture_error("scope generation"))?;
        let active_content = ScopeContentGenerationFence::for_test(
            scope,
            scope_generation,
            ReferenceContentGeneration::from_content(&content_a)?,
        );

        let runtime_a = LocalObjectRuntime::bind(
            &content_a,
            &active_content,
            scope,
            scope_generation,
            &PlacementKey::new(PLACEMENT_A)?,
            1,
            &TransitionKey::new(OPEN_TRANSITION)?,
            &TransitionKey::new(CLOSE_TRANSITION)?,
        )?;
        let before_state = runtime_a.state_key().clone();
        let before_blocking = runtime_a.blocking_cells().clone();

        let error = match LocalObjectRuntime::bind(
            &content_b,
            &active_content,
            scope,
            scope_generation,
            &PlacementKey::new(PLACEMENT_B)?,
            1,
            &TransitionKey::new(OPEN_TRANSITION)?,
            &TransitionKey::new(CLOSE_TRANSITION)?,
        ) {
            Ok(_runtime) => {
                return Err(fixture_error(
                    "second Content generation unexpectedly activated in live scope",
                ));
            }
            Err(error) => error,
        };
        assert!(matches!(
            error,
            WorldRuntimeError::InvalidBinding(
                "different Content generation cannot activate while runtime scope is live"
            )
        ));
        assert_eq!(runtime_a.state_key(), &before_state);
        assert_eq!(runtime_a.revision(), 0);
        assert_eq!(runtime_a.blocking_cells(), &before_blocking);
        Ok(())
    }

    #[test]
    fn forged_proven_target_evidence_is_rejected_before_runtime_creation()
    -> Result<(), WorldRuntimeError> {
        let mut content = synthetic_content("package-r1")?;
        let forged = EvidenceBindingRef::new(
            ProductionAtom::new("reference manifest revision", "manifest-r1")?,
            ProductionKey::new(
                "oteryn:reference.case.ability_combat.light_healing.self_heal_semantics.v1",
            )?,
            EvidenceDisposition::Proven,
        );
        let placement = content
            .placements
            .first_mut()
            .ok_or(WorldRuntimeError::InvalidBinding(
                "CW4 test fixture has no placement",
            ))?;
        placement.address.evidence = forged;

        let (_authority, _session, scope) = authority(25, 4, 1, 1)?;
        assert!(matches!(
            runtime_for(&content, scope, PLACEMENT_A, 1),
            Err(WorldRuntimeError::InvalidBinding(
                "synthetic CW4 placement cannot claim Reference target promotion"
            ))
        ));
        Ok(())
    }

    #[test]
    fn invalid_content_lock_is_rejected_by_cw3_linker_before_runtime_creation()
    -> Result<(), WorldRuntimeError> {
        let mut content = synthetic_content("package-r1")?;
        let entry =
            content
                .content_lock
                .entries
                .first_mut()
                .ok_or(WorldRuntimeError::InvalidBinding(
                    "CW4 test fixture has no Content Lock entry",
                ))?;
        entry.package_revision =
            ProductionAtom::new("cw4 test package revision", "package-corrupted")?;

        let (_authority, _session, scope) = authority(26, 4, 1, 1)?;
        assert!(matches!(
            runtime_for(&content, scope, PLACEMENT_A, 1),
            Err(WorldRuntimeError::Content(ContentError::InvalidArtifact(
                "reference-playable Content Lock does not bind exact root package provenance"
            )))
        ));
        Ok(())
    }

    #[test]
    fn stale_scope_or_session_authority_fails_before_ingress_or_mutation()
    -> Result<(), WorldRuntimeError> {
        let content = synthetic_content("package-r1")?;
        let (_authority, session, scope) = authority(30, 5, 1, 1)?;
        let mut runtime = runtime_for(&content, scope, PLACEMENT_A, 1)?;
        let command = command(&runtime, session, 1, 1, LocalObjectOperation::Open, 0)?;
        let mut ingress = CommandIngress::new();

        let (stale_scope, _, _) = authority(30, 5, 2, 1)?;
        assert!(matches!(
            runtime.apply(&stale_scope, &command, &mut ingress, &BTreeSet::new()),
            Err(WorldRuntimeError::StaleScopeOwnershipGeneration)
        ));
        assert_eq!(
            ingress.next_command_id(),
            Some(CommandId::new(1).map_err(|_error: CommandIdError| fixture_error("command id"))?)
        );
        assert_eq!(runtime.revision(), 0);

        let (wrong_session_authority, _, _) = authority(31, 5, 1, 1)?;
        assert!(matches!(
            runtime.apply(
                &wrong_session_authority,
                &command,
                &mut ingress,
                &BTreeSet::new()
            ),
            Err(WorldRuntimeError::StaleGameSession)
        ));
        assert_eq!(runtime.revision(), 0);
        Ok(())
    }

    #[test]
    fn same_placement_in_two_channels_has_independent_overlay_state()
    -> Result<(), WorldRuntimeError> {
        let content = synthetic_content("package-r1")?;
        let (authority_a, session_a, scope_a) = authority(40, 6, 1, 1)?;
        let (_authority_b, _session_b, scope_b) = authority(41, 7, 1, 1)?;
        let mut runtime_a = runtime_for(&content, scope_a, PLACEMENT_A, 1)?;
        let runtime_b = runtime_for(&content, scope_b, PLACEMENT_A, 1)?;
        let before_b = runtime_b.blocking_cells().clone();
        let mut ingress_a = CommandIngress::new();
        let open = command(&runtime_a, session_a, 1, 1, LocalObjectOperation::Open, 0)?;

        runtime_a.apply(&authority_a, &open, &mut ingress_a, &BTreeSet::new())?;
        assert!(runtime_a.blocking_cells().is_empty());
        assert_eq!(
            runtime_b.state_key().as_str(),
            "oteryn:reference.state.closed"
        );
        assert_eq!(runtime_b.revision(), 0);
        assert_eq!(runtime_b.blocking_cells(), &before_b);
        Ok(())
    }

    #[test]
    fn occupied_multicell_close_is_atomic_and_open_removes_only_own_contribution()
    -> Result<(), WorldRuntimeError> {
        let content = synthetic_content("package-r1")?;
        let (authority, session, scope) = authority(50, 8, 1, 1)?;
        let mut runtime_a = runtime_for(&content, scope, PLACEMENT_A, 1)?;
        let runtime_b = runtime_for(&content, scope, PLACEMENT_B, 1)?;
        let mut ingress = CommandIngress::new();

        let open = command(&runtime_a, session, 1, 1, LocalObjectOperation::Open, 0)?;
        runtime_a.apply(&authority, &open, &mut ingress, &BTreeSet::new())?;
        assert!(runtime_a.blocking_cells().is_empty());
        assert_eq!(runtime_b.blocking_cells().len(), 2);

        let occupied_cell = runtime_a.collision_cells().iter().next().copied().ok_or(
            WorldRuntimeError::InvalidBinding("test fixture has no collision cell"),
        )?;
        let occupied = BTreeSet::from([occupied_cell]);
        let close = command(&runtime_a, session, 2, 1, LocalObjectOperation::Close, 1)?;
        let rejected = runtime_a.apply(&authority, &close, &mut ingress, &occupied)?;
        assert_eq!(rejected.disposition(), DISPOSITION_OCCUPIED);
        assert_eq!(
            runtime_a.state_key().as_str(),
            "oteryn:reference.state.open"
        );
        assert!(runtime_a.blocking_cells().is_empty());

        let close_after_leave = command(&runtime_a, session, 3, 1, LocalObjectOperation::Close, 1)?;
        let committed = runtime_a.apply(
            &authority,
            &close_after_leave,
            &mut ingress,
            &BTreeSet::new(),
        )?;
        assert_eq!(committed.disposition(), DISPOSITION_COMMITTED);
        assert_eq!(runtime_a.blocking_cells(), runtime_a.collision_cells());
        assert_eq!(runtime_a.blocking_cells().len(), 2);
        Ok(())
    }

    #[test]
    fn changed_duplicate_conflicts_without_reexecuting_transition() -> Result<(), WorldRuntimeError>
    {
        let content = synthetic_content("package-r1")?;
        let (authority, session, scope) = authority(60, 9, 1, 1)?;
        let mut runtime = runtime_for(&content, scope, PLACEMENT_A, 1)?;
        let mut ingress = CommandIngress::new();
        let open = command(&runtime, session, 1, 1, LocalObjectOperation::Open, 0)?;
        runtime.apply(&authority, &open, &mut ingress, &BTreeSet::new())?;

        let changed = command(&runtime, session, 1, 1, LocalObjectOperation::Close, 1)?;
        assert!(matches!(
            runtime.apply(&authority, &changed, &mut ingress, &BTreeSet::new()),
            Err(WorldRuntimeError::ConflictChangedInput)
        ));
        assert_eq!(runtime.state_key().as_str(), "oteryn:reference.state.open");
        assert_eq!(runtime.revision(), 1);
        Ok(())
    }

    #[test]
    fn changed_content_generation_on_duplicate_conflicts_without_replay()
    -> Result<(), WorldRuntimeError> {
        let content = synthetic_content("package-r1")?;
        let mut changed_content = synthetic_content("package-r1")?;
        changed_content.content_lock.revision_digest_token =
            ProductionAtom::new("cw4 test content lock", "lock:changed-duplicate")?;
        let (authority, session, scope) = authority(69, 10, 1, 1)?;
        let mut runtime = runtime_for(&content, scope, PLACEMENT_A, 1)?;
        let mut ingress = CommandIngress::new();
        let first = command(&runtime, session, 1, 1, LocalObjectOperation::Open, 0)?;
        runtime.apply(&authority, &first, &mut ingress, &BTreeSet::new())?;

        let mut changed = first.clone();
        changed.content_generation = ReferenceContentGeneration::from_content(&changed_content)?;
        assert!(matches!(
            runtime.apply(&authority, &changed, &mut ingress, &BTreeSet::new()),
            Err(WorldRuntimeError::ConflictChangedInput)
        ));
        assert_eq!(runtime.state_key().as_str(), "oteryn:reference.state.open");
        assert_eq!(runtime.revision(), 1);
        assert!(runtime.blocking_cells().is_empty());
        Ok(())
    }

    #[test]
    fn different_content_generation_is_terminally_rejected_while_scope_stays_live()
    -> Result<(), WorldRuntimeError> {
        let content = synthetic_content("package-r1")?;
        let mut other_content = synthetic_content("package-r1")?;
        other_content.content_lock.revision_digest_token =
            ProductionAtom::new("cw4 test content lock", "lock:other-generation")?;
        let (authority, session, scope) = authority(70, 10, 1, 1)?;
        let mut runtime = runtime_for(&content, scope, PLACEMENT_A, 1)?;
        let mut ingress = CommandIngress::new();
        let mut command = command(&runtime, session, 1, 1, LocalObjectOperation::Open, 0)?;
        command.content_generation = ReferenceContentGeneration::from_content(&other_content)?;

        let result = runtime.apply(&authority, &command, &mut ingress, &BTreeSet::new())?;
        assert_eq!(result.disposition(), DISPOSITION_BINDING_MISMATCH);
        assert_eq!(
            runtime.state_key().as_str(),
            "oteryn:reference.state.closed"
        );
        assert_eq!(runtime.revision(), 0);
        assert_eq!(runtime.blocking_cells(), runtime.collision_cells());
        Ok(())
    }

    #[test]
    fn ingress_capacity_failure_leaves_gameplay_and_spatial_state_unchanged()
    -> Result<(), WorldRuntimeError> {
        let content = synthetic_content("package-r1")?;
        let (authority, session, scope) = authority(71, 11, 1, 1)?;
        let mut runtime = runtime_for(&content, scope, PLACEMENT_A, 1)?;
        let mut ingress = CommandIngress::new();
        let count = u64::try_from(MAX_OUTSTANDING_COMMANDS)
            .map_err(|_error| fixture_error("outstanding command count"))?;
        for raw in 1..=count {
            let command_id = CommandId::new(raw)
                .map_err(|_error: CommandIdError| fixture_error("command id"))?;
            let semantic = CommandSemanticIdentity::new(
                NormalizedSemanticIntentIdentity::new(
                    "oteryn:capacity.placement",
                    raw,
                    "oteryn:capacity.intent",
                    0,
                )?,
                RetainedBindingIdentity::new(
                    "oteryn:capacity.generation",
                    "oteryn:capacity.transition",
                )?,
            );
            assert_eq!(
                ingress.reserve(command_id, semantic),
                IngressDecision::Reserved(command_id)
            );
        }
        let next = count
            .checked_add(1)
            .ok_or(WorldRuntimeError::InvalidBinding(
                "test command count overflow",
            ))?;
        let open = command(&runtime, session, next, 1, LocalObjectOperation::Open, 0)?;
        let before = runtime.blocking_cells().clone();
        assert!(matches!(
            runtime.apply(&authority, &open, &mut ingress, &BTreeSet::new()),
            Err(WorldRuntimeError::IngressCapacityExceeded { .. })
        ));
        assert_eq!(runtime.revision(), 0);
        assert_eq!(runtime.blocking_cells(), &before);
        Ok(())
    }
}
