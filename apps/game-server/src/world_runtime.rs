use crate::content::{
    CanonicalReferencePlayableContent, ContentError, DefinitionFamily, FootprintRelation,
    LogicalCell, PlacementKey, ProductionKey, ReferenceDefinitionKind, TransitionBinding,
    TransitionKey, REFERENCE_PLAYABLE_CAPABILITY_PROFILE, REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
};
use crate::foundation::{
    CommandId, CommandIngress, CommandLifecycleError, CommandRef, CommandSemanticIdentity,
    ConnectionGeneration, DuplicateDisposition, GameSessionAuthoritySnapshot, GameSessionState,
    IngressDecision, NormalizedSemanticIntentIdentity, RetainedBindingIdentity, RuntimeScopeRefV1,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReferenceContentGeneration {
    semantic_identity: Box<str>,
}

impl ReferenceContentGeneration {
    pub(crate) fn from_content(
        content: &CanonicalReferencePlayableContent,
    ) -> Result<Self, WorldRuntimeError> {
        let provenance = content.package_manifest.package_provenance_digest()?;
        let semantic_identity = format!(
            "{}|{}|{}|{}|{}|{}",
            content.profile_revision.as_str(),
            content.capability_profile.as_str(),
            content.package_manifest.package_key.as_str(),
            content.package_manifest.package_revision.as_str(),
            provenance.as_str(),
            content.content_lock.revision_digest_token.as_str(),
        );
        Ok(Self {
            semantic_identity: semantic_identity.into_boxed_str(),
        })
    }

    #[must_use]
    fn as_str(&self) -> &str {
        &self.semantic_identity
    }
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
            Self::IngressClassificationMismatch => {
                formatter.write_str("Foundation duplicate classification is internally inconsistent")
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
        scope: RuntimeScopeRefV1,
        scope_generation: ScopeOwnershipGeneration,
        placement_key: &PlacementKey,
        incarnation: u64,
        open_transition_key: &TransitionKey,
        close_transition_key: &TransitionKey,
    ) -> Result<Self, WorldRuntimeError> {
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
        if open_transition.owner_capability != close_transition.owner_capability {
            return Err(WorldRuntimeError::InvalidBinding(
                "OPEN/CLOSE transitions require different runtime capabilities",
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
        let content_generation = ReferenceContentGeneration::from_content(content)?;
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
            return PreparedTerminal::unchanged(
                DISPOSITION_NO_CHANGE,
                &self.state,
                self.revision,
            );
        }
        if self.state != transition.source_state {
            return Err(WorldRuntimeError::InvalidBinding(
                "runtime state escaped the bound two-state transition",
            ));
        }

        if command.operation == LocalObjectOperation::Close
            && !self.collision_cells.is_disjoint(occupied_cells)
        {
            return PreparedTerminal::unchanged(
                DISPOSITION_OCCUPIED,
                &self.state,
                self.revision,
            );
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
            x: placement
                .address
                .cell
                .x
                .checked_add(member.dx)
                .ok_or(WorldRuntimeError::InvalidBinding(
                    "collision footprint x coordinate overflow",
                ))?,
            y: placement
                .address
                .cell
                .y
                .checked_add(member.dy)
                .ok_or(WorldRuntimeError::InvalidBinding(
                    "collision footprint y coordinate overflow",
                ))?,
            z: placement
                .address
                .cell
                .z
                .checked_add(member.dz)
                .ok_or(WorldRuntimeError::InvalidBinding(
                    "collision footprint z coordinate overflow",
                ))?,
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
        ClientProjectionClass, ContentLockBinding, ContentLockEntry, CoordinateFrameRef,
        DefinitionRevisionRef, EvidenceBindingRef, EvidenceDisposition, FootprintCell,
        MapRevisionRef, OwnerCapabilityRequirement, PackageManifestBinding, PlacementRef,
        ProductionAtom, ReferenceDefinition, Sha256HexDigest, SpatialAddress, TypedDefinitionRef,
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
        let evidence = EvidenceBindingRef::new(
            ProductionAtom::new("reference manifest revision", "manifest-r1")?,
            ProductionKey::new(
                "oteryn:reference.case.ability_combat.light_healing.self_heal_semantics.v1",
            )?,
            EvidenceDisposition::Proven,
        );
        let coordinate_frame = CoordinateFrameRef::new("global-target-2026-07-28")?;
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

        let capability = OwnerCapabilityRequirement {
            capability_key: ProductionKey::new(
                "oteryn:runtime.capability.local-object-transition",
            )?,
        };
        Ok(CanonicalReferencePlayableContent {
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
            coordinate_frame,
            definitions: vec![ReferenceDefinition {
                definition: object_ref.clone(),
                kind: ReferenceDefinitionKind::LocalObjectStates(vec![
                    closed.clone(),
                    open.clone(),
                ]),
                client_projection: ClientProjectionClass::ClientSafe,
            }],
            placements: vec![placement(PLACEMENT_A)?, placement(PLACEMENT_B)?],
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
                    definition: object_ref,
                    source_state: open,
                    normalized_intent_family: ProductionKey::new(
                        "oteryn:reference.intent.local-object-close",
                    )?,
                    target_state: closed,
                    owner_capability: capability,
                    policy_guard_refs: vec![],
                },
            ],
        })
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
        let character_lease = CharacterLease::new(character, 1)
            .map_err(|_error| fixture_error("character lease"))?;
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
        LocalObjectRuntime::bind(
            content,
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
        let command_id = CommandId::new(id)
            .map_err(|_error: CommandIdError| fixture_error("command id"))?;
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
    fn open_close_then_replay_open_preserves_current_closed_state(
    ) -> Result<(), WorldRuntimeError> {
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
        assert_eq!(runtime.state_key().as_str(), "oteryn:reference.state.closed");
        assert_eq!(runtime.revision(), 2);
        Ok(())
    }

    #[test]
    fn later_command_cannot_commit_ahead_of_earlier_pending_across_objects(
    ) -> Result<(), WorldRuntimeError> {
        let content = synthetic_content("package-r1")?;
        let (authority, session, scope) = authority(20, 4, 1, 1)?;
        let runtime_a = runtime_for(&content, scope, PLACEMENT_A, 1)?;
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
        Ok(())
    }

    #[test]
    fn stale_scope_or_session_authority_fails_before_ingress_or_mutation(
    ) -> Result<(), WorldRuntimeError> {
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
            Some(
                CommandId::new(1)
                    .map_err(|_error: CommandIdError| fixture_error("command id"))?
            )
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
    fn same_placement_in_two_channels_has_independent_overlay_state(
    ) -> Result<(), WorldRuntimeError> {
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
        assert_eq!(runtime_b.state_key().as_str(), "oteryn:reference.state.closed");
        assert_eq!(runtime_b.revision(), 0);
        assert_eq!(runtime_b.blocking_cells(), &before_b);
        Ok(())
    }

    #[test]
    fn occupied_multicell_close_is_atomic_and_open_removes_only_own_contribution(
    ) -> Result<(), WorldRuntimeError> {
        let content = synthetic_content("package-r1")?;
        let (authority, session, scope) = authority(50, 8, 1, 1)?;
        let mut runtime_a = runtime_for(&content, scope, PLACEMENT_A, 1)?;
        let runtime_b = runtime_for(&content, scope, PLACEMENT_B, 1)?;
        let mut ingress = CommandIngress::new();

        let open = command(&runtime_a, session, 1, 1, LocalObjectOperation::Open, 0)?;
        runtime_a.apply(&authority, &open, &mut ingress, &BTreeSet::new())?;
        assert!(runtime_a.blocking_cells().is_empty());
        assert_eq!(runtime_b.blocking_cells().len(), 2);

        let occupied_cell = runtime_a
            .collision_cells()
            .iter()
            .next()
            .copied()
            .ok_or(WorldRuntimeError::InvalidBinding(
                "test fixture has no collision cell",
            ))?;
        let occupied = BTreeSet::from([occupied_cell]);
        let close = command(&runtime_a, session, 2, 1, LocalObjectOperation::Close, 1)?;
        let rejected = runtime_a.apply(&authority, &close, &mut ingress, &occupied)?;
        assert_eq!(rejected.disposition(), DISPOSITION_OCCUPIED);
        assert_eq!(runtime_a.state_key().as_str(), "oteryn:reference.state.open");
        assert!(runtime_a.blocking_cells().is_empty());

        let close_after_leave =
            command(&runtime_a, session, 3, 1, LocalObjectOperation::Close, 1)?;
        let committed =
            runtime_a.apply(&authority, &close_after_leave, &mut ingress, &BTreeSet::new())?;
        assert_eq!(committed.disposition(), DISPOSITION_COMMITTED);
        assert_eq!(runtime_a.blocking_cells(), runtime_a.collision_cells());
        assert_eq!(runtime_a.blocking_cells().len(), 2);
        Ok(())
    }

    #[test]
    fn changed_duplicate_conflicts_without_reexecuting_transition(
    ) -> Result<(), WorldRuntimeError> {
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
    fn different_content_generation_is_terminally_rejected_while_scope_stays_live(
    ) -> Result<(), WorldRuntimeError> {
        let content = synthetic_content("package-r1")?;
        let other_content = synthetic_content("package-r2")?;
        let (authority, session, scope) = authority(70, 10, 1, 1)?;
        let mut runtime = runtime_for(&content, scope, PLACEMENT_A, 1)?;
        let mut ingress = CommandIngress::new();
        let mut command = command(&runtime, session, 1, 1, LocalObjectOperation::Open, 0)?;
        command.content_generation = ReferenceContentGeneration::from_content(&other_content)?;

        let result = runtime.apply(&authority, &command, &mut ingress, &BTreeSet::new())?;
        assert_eq!(result.disposition(), DISPOSITION_BINDING_MISMATCH);
        assert_eq!(runtime.state_key().as_str(), "oteryn:reference.state.closed");
        assert_eq!(runtime.revision(), 0);
        assert_eq!(runtime.blocking_cells(), runtime.collision_cells());
        Ok(())
    }

    #[test]
    fn ingress_capacity_failure_leaves_gameplay_and_spatial_state_unchanged(
    ) -> Result<(), WorldRuntimeError> {
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
