use super::model::ContentError;
use super::production::{
    DurableMigrationClass, FirstProductionExpectation, FirstProductionRuntimeState,
    GenerationIdentity, StagedGeneration,
};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

mod sealed {
    pub trait Sealed {}
}

/// External control-plane authorization consumed by the CONTENT activation boundary.
///
/// The sealing trait is crate-private, so downstream crates cannot mint an authorization
/// implementation merely because they can construct valid content bytes. A future accepted
/// Game authority lane may implement this trait inside this crate.
pub trait ContentActivationAdmissionGuard {
    fn quiescent(&self) -> bool;
    fn live_scope_count(&self) -> usize;
}

pub trait AuthorizedContentGeneration: sealed::Sealed {
    type AdmissionGuard<'a>: ContentActivationAdmissionGuard
    where
        Self: 'a;

    fn acquire_admission_guard(&self) -> Self::AdmissionGuard<'_>;
    fn target_identity(&self) -> &GenerationIdentity;
    fn expected_current_identity(&self) -> Option<&GenerationIdentity>;
    fn expected_current_activation_sequence(&self) -> Option<u64>;
    fn activation_sequence(&self) -> u64;
    fn permits_last_known_good_fallback(&self) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentActivationError {
    Content(ContentError),
    CandidateAlreadyStaged,
    NoStagedCandidate,
    NotQuiescent,
    LiveScopesRemain(usize),
    AuthorizationTargetMismatch,
    ExpectedCurrentMismatch,
    NonMonotonicActivationSequence { attempted: u64, current_floor: u64 },
    MigrationBearingActivationRejected,
    LastKnownGoodFallbackNotAuthorized,
    LastKnownGoodReceiptMismatch,
}

impl Display for ContentActivationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Content(error) => write!(formatter, "{error}"),
            Self::CandidateAlreadyStaged => {
                formatter.write_str("a first-production content candidate is already staged")
            }
            Self::NoStagedCandidate => formatter.write_str("no content candidate is staged"),
            Self::NotQuiescent => {
                formatter.write_str("content activation requires an explicit quiescent boundary")
            }
            Self::LiveScopesRemain(count) => {
                write!(
                    formatter,
                    "content activation rejected while {count} live scopes remain"
                )
            }
            Self::AuthorizationTargetMismatch => {
                formatter.write_str("authorization target does not match the staged generation")
            }
            Self::ExpectedCurrentMismatch => {
                formatter.write_str("authorization expected-current generation identity or activation sequence is stale or wrong")
            }
            Self::NonMonotonicActivationSequence {
                attempted,
                current_floor,
            } => write!(
                formatter,
                "activation sequence must increase monotonically: {attempted} <= {current_floor}"
            ),
            Self::MigrationBearingActivationRejected => formatter.write_str(
                "migration-bearing content cannot activate in FIRST_PRODUCTION_CONTENT_PROFILE/v1",
            ),
            Self::LastKnownGoodFallbackNotAuthorized => formatter.write_str(
                "last-known-good fallback requires explicit external fallback authorization",
            ),
            Self::LastKnownGoodReceiptMismatch => formatter.write_str(
                "revalidated fallback bytes do not match the supplied last-known-good receipt",
            ),
        }
    }
}

impl Error for ContentActivationError {}

impl From<ContentError> for ContentActivationError {
    fn from(value: ContentError) -> Self {
        Self::Content(value)
    }
}

/// The authoritative runtime generation is intentionally non-cloneable.
///
/// ```compile_fail
/// fn require_clone<T: Clone>() {}
/// require_clone::<oteryn_game_server::content::ActiveGeneration>();
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct ActiveGeneration {
    identity: GenerationIdentity,
    activation_sequence: u64,
    runtime_state: FirstProductionRuntimeState,
}

impl ActiveGeneration {
    pub(crate) fn native_source_world(&self) -> Option<&[u8]> {
        self.runtime_state.native_source_world()
    }
    pub(crate) fn native_gameplay(&self) -> Option<&super::native_gameplay::NativeGameplayState> {
        self.runtime_state.native_gameplay()
    }
    pub fn identity(&self) -> &GenerationIdentity {
        &self.identity
    }

    pub const fn activation_sequence(&self) -> u64 {
        self.activation_sequence
    }

    pub fn server_record_count(&self) -> usize {
        self.runtime_state.server_record_count()
    }

    pub fn client_record_count(&self) -> usize {
        self.runtime_state.client_record_count()
    }

    pub fn server_definition_fields(&self, key: &str) -> Option<&[String]> {
        self.runtime_state.server_definition_fields(key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastKnownGoodReceipt {
    identity: GenerationIdentity,
    activation_sequence: u64,
}

impl LastKnownGoodReceipt {
    pub fn identity(&self) -> &GenerationIdentity {
        &self.identity
    }

    pub const fn activation_sequence(&self) -> u64 {
        self.activation_sequence
    }

    pub fn expectation(&self) -> FirstProductionExpectation {
        FirstProductionExpectation::from_generation_identity(&self.identity)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StagedCandidateKind {
    Primary,
    LastKnownGoodFallback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StagedCandidate {
    generation: StagedGeneration,
    kind: StagedCandidateKind,
}

#[derive(Debug, Default)]
pub struct ContentActivationController {
    active: Option<ActiveGeneration>,
    staged: Option<StagedCandidate>,
    last_known_good: Option<LastKnownGoodReceipt>,
    last_activation_sequence: u64,
    ready: bool,
}

impl ContentActivationController {
    pub const fn new() -> Self {
        Self {
            active: None,
            staged: None,
            last_known_good: None,
            last_activation_sequence: 0,
            ready: false,
        }
    }

    /// Reconstruct the process-local activation controller after restart.
    ///
    /// Restart is deliberately not ready and does not restore an active generation. The receipt
    /// is immutable evidence only and raises the monotonic activation-sequence floor.
    pub fn restart_with_receipt(receipt: Option<LastKnownGoodReceipt>) -> Self {
        let last_activation_sequence = receipt
            .as_ref()
            .map_or(0, LastKnownGoodReceipt::activation_sequence);
        Self {
            active: None,
            staged: None,
            last_known_good: receipt,
            last_activation_sequence,
            ready: false,
        }
    }

    pub const fn is_ready(&self) -> bool {
        self.ready
    }

    pub fn active(&self) -> Option<&ActiveGeneration> {
        self.active.as_ref()
    }

    pub fn staged_identity(&self) -> Option<&GenerationIdentity> {
        self.staged
            .as_ref()
            .map(|candidate| candidate.generation.identity())
    }

    pub fn last_known_good_receipt(&self) -> Option<&LastKnownGoodReceipt> {
        self.last_known_good.as_ref()
    }

    pub const fn last_activation_sequence(&self) -> u64 {
        self.last_activation_sequence
    }

    pub fn stage_primary(
        &mut self,
        server_bytes: &[u8],
        client_bytes: &[u8],
        expected: &FirstProductionExpectation,
    ) -> Result<&GenerationIdentity, ContentActivationError> {
        self.require_staging_slot()?;
        let generation = StagedGeneration::stage(server_bytes, client_bytes, expected)?;
        self.staged = Some(StagedCandidate {
            generation,
            kind: StagedCandidateKind::Primary,
        });
        self.staged_identity()
            .ok_or(ContentActivationError::NoStagedCandidate)
    }

    pub fn stage_last_known_good_fallback(
        &mut self,
        server_bytes: &[u8],
        client_bytes: &[u8],
        expected: &FirstProductionExpectation,
        receipt: &LastKnownGoodReceipt,
    ) -> Result<&GenerationIdentity, ContentActivationError> {
        self.require_staging_slot()?;
        let generation = StagedGeneration::stage(server_bytes, client_bytes, expected)?;
        if generation.identity() != receipt.identity() {
            return Err(ContentActivationError::LastKnownGoodReceiptMismatch);
        }
        self.staged = Some(StagedCandidate {
            generation,
            kind: StagedCandidateKind::LastKnownGoodFallback,
        });
        self.staged_identity()
            .ok_or(ContentActivationError::NoStagedCandidate)
    }

    pub fn discard_staged(&mut self) {
        self.staged = None;
    }

    pub fn activate<A: AuthorizedContentGeneration>(
        &mut self,
        authorization: &A,
    ) -> Result<&ActiveGeneration, ContentActivationError> {
        let candidate = self
            .staged
            .as_ref()
            .ok_or(ContentActivationError::NoStagedCandidate)?;

        // The externally owned guard must remain alive through every final authority check and
        // the non-fallible publication below. Its owning implementation is responsible for
        // holding new authoritative-scope admission while this value is alive.
        let admission_guard = authorization.acquire_admission_guard();
        if !admission_guard.quiescent() {
            return Err(ContentActivationError::NotQuiescent);
        }
        let live_scope_count = admission_guard.live_scope_count();
        if live_scope_count != 0 {
            return Err(ContentActivationError::LiveScopesRemain(live_scope_count));
        }
        if authorization.target_identity() != candidate.generation.identity() {
            return Err(ContentActivationError::AuthorizationTargetMismatch);
        }
        if !expected_current_matches(
            authorization.expected_current_identity(),
            authorization.expected_current_activation_sequence(),
            self.active.as_ref(),
        ) {
            return Err(ContentActivationError::ExpectedCurrentMismatch);
        }
        let activation_sequence = authorization.activation_sequence();
        if activation_sequence <= self.last_activation_sequence {
            return Err(ContentActivationError::NonMonotonicActivationSequence {
                attempted: activation_sequence,
                current_floor: self.last_activation_sequence,
            });
        }
        if candidate.generation.identity().migration_class()
            != DurableMigrationClass::CompatibleNoMigration
        {
            return Err(ContentActivationError::MigrationBearingActivationRejected);
        }
        if candidate.kind == StagedCandidateKind::LastKnownGoodFallback
            && !authorization.permits_last_known_good_fallback()
        {
            return Err(ContentActivationError::LastKnownGoodFallbackNotAuthorized);
        }

        // Exact commit point: all validation above is read-only. Only here is the complete staged
        // server+client generation removed from staging and published as the sole active pointer.
        let candidate = self
            .staged
            .take()
            .ok_or(ContentActivationError::NoStagedCandidate)?;
        let (identity, runtime_state) = candidate.generation.into_parts();
        let active = ActiveGeneration {
            identity: identity.clone(),
            activation_sequence,
            runtime_state,
        };
        self.active = Some(active);
        self.last_activation_sequence = activation_sequence;
        self.last_known_good = Some(LastKnownGoodReceipt {
            identity,
            activation_sequence,
        });
        self.ready = true;

        self.active
            .as_ref()
            .ok_or(ContentActivationError::NoStagedCandidate)
    }

    fn require_staging_slot(&self) -> Result<(), ContentActivationError> {
        if self.staged.is_some() {
            return Err(ContentActivationError::CandidateAlreadyStaged);
        }
        Ok(())
    }
}

fn expected_current_matches(
    expected_identity: Option<&GenerationIdentity>,
    expected_sequence: Option<u64>,
    current: Option<&ActiveGeneration>,
) -> bool {
    match (expected_identity, expected_sequence, current) {
        (None, None, None) => true,
        (Some(expected_identity), Some(expected_sequence), Some(current)) => {
            expected_identity == current.identity()
                && expected_sequence == current.activation_sequence()
        }
        _ => false,
    }
}

/// Boot-ordering proof for the native entry-room activation (#935 "Hold new admission, establish
/// real quiescence"). It is minted only by the node before its Channel runtime, gameplay listener
/// or control socket exists, so no authoritative scope or admission can be live while it is held.
/// The activation additionally requires an empty controller (explicit empty start).
#[derive(Debug)]
pub struct NodeBootQuiescence {
    _private: (),
}

impl NodeBootQuiescence {
    pub(crate) const fn before_channel_runtime() -> Self {
        Self { _private: () }
    }
}

/// One control-plane activation issuance for a Channel scope, as read from its current durable
/// row. It authorizes exact digests and one frame binding; it carries no Content bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeEntryActivationIssuance {
    pub world_id: crate::foundation::WorldId,
    pub activation_sequence: u64,
    pub server_artifact_digest: [u8; 32],
    pub client_artifact_digest: [u8; 32],
    pub frame_binding_digest: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeEntryActivationError {
    Qualification(super::ProjectError),
    Activation(ContentActivationError),
    WorldMismatch,
    DigestMismatch,
    FrameBindingMismatch,
}

impl Display for NativeEntryActivationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Qualification(error) => write!(formatter, "native entry qualification: {error}"),
            Self::Activation(error) => write!(formatter, "native entry activation: {error}"),
            Self::WorldMismatch => {
                formatter.write_str("activation issuance WorldId does not match the Channel scope")
            }
            Self::DigestMismatch => {
                formatter.write_str("qualified native entry pair does not match the issued digests")
            }
            Self::FrameBindingMismatch => formatter.write_str(
                "qualified native frame binding does not match the issued frame binding",
            ),
        }
    }
}

impl Error for NativeEntryActivationError {}

/// The exact active native entry generation a Channel runtime is created with (#935). It is
/// produced only by [`activate_native_entry_room`], so its identity, activation sequence and
/// source-qualified frame binding cannot be assembled or substituted independently.
///
/// ```compile_fail
/// fn require_clone<T: Clone>() {}
/// require_clone::<oteryn_game_server::content::NativeEntryContentPin>();
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct NativeEntryContentPin {
    qualified_room: super::QualifiedNativeEntryRoom,
    identity: GenerationIdentity,
    activation_sequence: u64,
    frame_binding: super::NativeEntryFrameBinding,
    entry_start: super::NativeEntryStart,
    map_revision_digest: [u8; 32],
    movement_cells: super::NativeEntryMovementCells,
    /// The one door's own genuine, fully linked Reference-profile content (M2b, 5868482467),
    /// carried through from the same qualified room this pin activates. The Channel activation
    /// owner binds the door's `LocalObjectRuntime` from this exact content.
    door: super::CanonicalReferencePlayableContent,
}

impl NativeEntryContentPin {
    /// Factual source qualification; active authority remains the single consumed pin.
    pub(crate) fn qualified_room(&self) -> &super::QualifiedNativeEntryRoom {
        &self.qualified_room
    }
    pub fn identity(&self) -> &GenerationIdentity {
        &self.identity
    }

    pub const fn activation_sequence(&self) -> u64 {
        self.activation_sequence
    }

    pub fn frame_binding(&self) -> &super::NativeEntryFrameBinding {
        &self.frame_binding
    }

    pub const fn entry_start(&self) -> super::NativeEntryStart {
        self.entry_start
    }

    /// The digest-level pin the Channel runtime is created with, the qualified cells the
    /// Channel's Movement reads, and the door's own qualified content the Channel's door
    /// `LocalObjectRuntime` binds from (M2b). Consuming the activation pin keeps one active
    /// generation per Channel creation.
    pub(crate) fn into_channel_parts(
        self,
    ) -> (
        crate::foundation::ChannelContentPin,
        super::NativeEntryMovementCells,
        super::CanonicalReferencePlayableContent,
    ) {
        let channel_pin = self.channel_pin();
        (channel_pin, self.movement_cells, self.door)
    }

    #[cfg(test)]
    pub(crate) fn into_channel_pin(self) -> crate::foundation::ChannelContentPin {
        self.channel_pin()
    }

    fn channel_pin(&self) -> crate::foundation::ChannelContentPin {
        crate::foundation::ChannelContentPin::from_activation(
            self.identity.world_id(),
            self.activation_sequence,
            self.identity.server_artifact_digest(),
            self.identity.client_artifact_digest(),
            self.frame_binding.digest(),
            self.map_revision_digest,
            (
                self.entry_start.x,
                self.entry_start.y,
                self.entry_start.floor,
            ),
        )
    }
}

struct NodeBootAuthorization<'a> {
    target: GenerationIdentity,
    activation_sequence: u64,
    _quiescence: &'a NodeBootQuiescence,
}

struct NodeBootAdmissionGuard;

impl ContentActivationAdmissionGuard for NodeBootAdmissionGuard {
    fn quiescent(&self) -> bool {
        true
    }

    fn live_scope_count(&self) -> usize {
        0
    }
}

impl sealed::Sealed for NodeBootAuthorization<'_> {}

impl AuthorizedContentGeneration for NodeBootAuthorization<'_> {
    type AdmissionGuard<'g>
        = NodeBootAdmissionGuard
    where
        Self: 'g;

    fn acquire_admission_guard(&self) -> Self::AdmissionGuard<'_> {
        NodeBootAdmissionGuard
    }

    fn target_identity(&self) -> &GenerationIdentity {
        &self.target
    }

    // A booting node holds no active generation: activation is an explicit empty start.
    fn expected_current_identity(&self) -> Option<&GenerationIdentity> {
        None
    }

    fn expected_current_activation_sequence(&self) -> Option<u64> {
        None
    }

    fn activation_sequence(&self) -> u64 {
        self.activation_sequence
    }

    fn permits_last_known_good_fallback(&self) -> bool {
        false
    }
}

/// Activates the committed native entry room for one Channel scope at node boot (#935).
///
/// The room is rebuilt from its genuine committed source and the scope WorldId, qualified natively
/// and compiled; its pair digests and frame binding must equal the current control-plane issuance
/// exactly. Only then is the pair staged and activated with the issuance's sequence. Any mismatch
/// refuses before staging, and a failed activation leaves no active generation.
pub fn activate_native_entry_room(
    controller: &mut ContentActivationController,
    quiescence: &NodeBootQuiescence,
    scope_world_id: crate::foundation::WorldId,
    issuance: &NativeEntryActivationIssuance,
) -> Result<NativeEntryContentPin, NativeEntryActivationError> {
    if issuance.world_id != scope_world_id {
        return Err(NativeEntryActivationError::WorldMismatch);
    }
    let room = super::qualify_native_entry_room(scope_world_id)
        .map_err(NativeEntryActivationError::Qualification)?;
    activate_qualified_native_entry_room(controller, quiescence, issuance, room)
}

/// Explicit candidate extension; it consumes the same independent control-plane issuance and
/// sealed activation guard as the baseline. Supplying valid bytes never supplies authority.
pub(crate) fn activate_native_entry_room_with_gameplay(
    controller: &mut ContentActivationController,
    quiescence: &NodeBootQuiescence,
    scope_world_id: crate::foundation::WorldId,
    issuance: &NativeEntryActivationIssuance,
    input: &super::native_gameplay::NativeGameplayInput,
) -> Result<NativeEntryContentPin, NativeEntryActivationError> {
    if issuance.world_id != scope_world_id {
        return Err(NativeEntryActivationError::WorldMismatch);
    }
    let room = match input.source_world.as_ref() {
        Some(source) => super::qualify_native_source_spell_world_with_gameplay(
            scope_world_id,
            input,
            &source.bytes,
        ),
        None => match input.native_map_profile {
            super::native_gameplay::NativeGameplayMapProfile::AcceptedEntryR1 => {
                super::qualify_native_entry_room_with_gameplay(scope_world_id, input)
            }
            super::native_gameplay::NativeGameplayMapProfile::SourceQualifiedSpellEntryR2 => {
                super::qualify_native_spell_entry_room_with_gameplay(scope_world_id, input)
            }
        },
    }
    .map_err(NativeEntryActivationError::Qualification)?;
    activate_qualified_native_entry_room(controller, quiescence, issuance, room)
}

fn activate_qualified_native_entry_room(
    controller: &mut ContentActivationController,
    quiescence: &NodeBootQuiescence,
    issuance: &NativeEntryActivationIssuance,
    room: super::QualifiedNativeEntryRoom,
) -> Result<NativeEntryContentPin, NativeEntryActivationError> {
    let compiled = room.compiled();
    if compiled.server_digest() != issuance.server_artifact_digest
        || compiled.client_digest() != issuance.client_artifact_digest
    {
        return Err(NativeEntryActivationError::DigestMismatch);
    }
    if room.frame_binding().digest() != issuance.frame_binding_digest {
        return Err(NativeEntryActivationError::FrameBindingMismatch);
    }
    let target = controller
        .stage_primary(
            &compiled.server_artifact,
            &compiled.client_artifact,
            compiled.expectation(),
        )
        .map_err(NativeEntryActivationError::Activation)?
        .clone();
    let authorization = NodeBootAuthorization {
        target,
        activation_sequence: issuance.activation_sequence,
        _quiescence: quiescence,
    };
    let result = controller.activate(&authorization);
    let active = match result {
        Ok(active) => active,
        Err(error) => {
            controller.discard_staged();
            return Err(NativeEntryActivationError::Activation(error));
        }
    };
    Ok(NativeEntryContentPin {
        qualified_room: room.clone(),
        identity: active.identity().clone(),
        activation_sequence: active.activation_sequence(),
        frame_binding: room.frame_binding().clone(),
        entry_start: room.entry_start(),
        map_revision_digest: room.map_revision_digest(),
        movement_cells: room.movement_cells().clone(),
        door: room.door().clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::production::{
        FirstProductionCompileTarget, compile_first_production, test_source,
    };
    use std::cell::Cell;

    #[derive(Debug, Clone)]
    struct TestAuthorization {
        target: GenerationIdentity,
        expected_current: Option<GenerationIdentity>,
        expected_current_sequence: Option<u64>,
        activation_sequence: u64,
        quiescent: bool,
        live_scopes: usize,
        permit_fallback: bool,
        hold_active: Cell<bool>,
        hold_observed_during_target_check: Cell<bool>,
    }

    struct TestAdmissionGuard<'a> {
        authorization: &'a TestAuthorization,
    }

    impl ContentActivationAdmissionGuard for TestAdmissionGuard<'_> {
        fn quiescent(&self) -> bool {
            self.authorization.quiescent
        }

        fn live_scope_count(&self) -> usize {
            self.authorization.live_scopes
        }
    }

    impl Drop for TestAdmissionGuard<'_> {
        fn drop(&mut self) {
            self.authorization.hold_active.set(false);
        }
    }

    impl sealed::Sealed for TestAuthorization {}

    impl AuthorizedContentGeneration for TestAuthorization {
        type AdmissionGuard<'a> = TestAdmissionGuard<'a>;

        fn acquire_admission_guard(&self) -> Self::AdmissionGuard<'_> {
            self.hold_active.set(true);
            TestAdmissionGuard {
                authorization: self,
            }
        }

        fn target_identity(&self) -> &GenerationIdentity {
            if self.hold_active.get() {
                self.hold_observed_during_target_check.set(true);
            }
            &self.target
        }

        fn expected_current_identity(&self) -> Option<&GenerationIdentity> {
            self.expected_current.as_ref()
        }

        fn expected_current_activation_sequence(&self) -> Option<u64> {
            self.expected_current_sequence
        }

        fn activation_sequence(&self) -> u64 {
            self.activation_sequence
        }

        fn permits_last_known_good_fallback(&self) -> bool {
            self.permit_fallback
        }
    }

    fn compiled(
        cell_count: usize,
    ) -> Result<crate::content::production::CompiledFirstProductionContent, ContentError> {
        let source = test_source(cell_count)?;
        compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)
    }

    fn authorization(
        target: &GenerationIdentity,
        expected_current: Option<(&GenerationIdentity, u64)>,
        sequence: u64,
    ) -> TestAuthorization {
        let (expected_current, expected_current_sequence) = match expected_current {
            Some((identity, sequence)) => (Some(identity.clone()), Some(sequence)),
            None => (None, None),
        };
        TestAuthorization {
            target: target.clone(),
            expected_current,
            expected_current_sequence,
            activation_sequence: sequence,
            quiescent: true,
            live_scopes: 0,
            permit_fallback: false,
            hold_active: Cell::new(false),
            hold_observed_during_target_check: Cell::new(false),
        }
    }

    #[test]
    fn restart_starts_not_ready_and_receipt_does_not_auto_activate() -> Result<(), Box<dyn Error>> {
        let compiled = compiled(3)?;
        let mut controller = ContentActivationController::new();
        let staged = controller
            .stage_primary(
                &compiled.server_artifact,
                &compiled.client_artifact,
                compiled.expectation(),
            )?
            .clone();
        controller.activate(&authorization(&staged, None, 1))?;
        let receipt = controller
            .last_known_good_receipt()
            .cloned()
            .ok_or("missing receipt")?;

        let mut restarted =
            ContentActivationController::restart_with_receipt(Some(receipt.clone()));
        assert!(!restarted.is_ready());
        assert!(restarted.active().is_none());
        assert!(restarted.staged_identity().is_none());
        assert_eq!(restarted.last_activation_sequence(), 1);

        let restaged = restarted
            .stage_last_known_good_fallback(
                &compiled.server_artifact,
                &compiled.client_artifact,
                &receipt.expectation(),
                &receipt,
            )?
            .clone();
        assert_eq!(&restaged, receipt.identity());
        assert!(!restarted.is_ready());
        assert!(restarted.active().is_none());
        Ok(())
    }

    #[test]
    fn second_staged_candidate_is_denied_without_changing_first() -> Result<(), Box<dyn Error>> {
        let first = compiled(3)?;
        let second = compiled(4)?;
        let mut controller = ContentActivationController::new();
        let first_identity = controller
            .stage_primary(
                &first.server_artifact,
                &first.client_artifact,
                first.expectation(),
            )?
            .clone();

        let result = controller.stage_primary(
            &second.server_artifact,
            &second.client_artifact,
            second.expectation(),
        );
        assert!(matches!(
            result,
            Err(ContentActivationError::CandidateAlreadyStaged)
        ));
        assert_eq!(controller.staged_identity(), Some(&first_identity));
        assert!(controller.active().is_none());
        Ok(())
    }

    #[test]
    fn activation_requires_quiescence_and_zero_live_scopes() -> Result<(), Box<dyn Error>> {
        let compiled = compiled(3)?;
        let mut controller = ContentActivationController::new();
        let identity = controller
            .stage_primary(
                &compiled.server_artifact,
                &compiled.client_artifact,
                compiled.expectation(),
            )?
            .clone();

        let mut auth = authorization(&identity, None, 1);
        auth.quiescent = false;
        assert!(matches!(
            controller.activate(&auth),
            Err(ContentActivationError::NotQuiescent)
        ));
        assert!(controller.active().is_none());
        assert_eq!(controller.staged_identity(), Some(&identity));

        auth.quiescent = true;
        auth.live_scopes = 1;
        assert!(matches!(
            controller.activate(&auth),
            Err(ContentActivationError::LiveScopesRemain(1))
        ));
        assert!(controller.active().is_none());
        assert_eq!(controller.staged_identity(), Some(&identity));
        Ok(())
    }

    #[test]
    fn stale_current_wrong_target_and_nonmonotonic_sequence_fail_precommit()
    -> Result<(), Box<dyn Error>> {
        let first = compiled(3)?;
        let second = compiled(4)?;
        let mut controller = ContentActivationController::new();
        let first_identity = controller
            .stage_primary(
                &first.server_artifact,
                &first.client_artifact,
                first.expectation(),
            )?
            .clone();
        controller.activate(&authorization(&first_identity, None, 10))?;
        let active_before_identity = controller
            .active()
            .map(ActiveGeneration::identity)
            .cloned()
            .ok_or("active missing")?;
        let active_before_sequence = controller
            .active()
            .map(ActiveGeneration::activation_sequence)
            .ok_or("active missing")?;

        controller.stage_primary(
            &second.server_artifact,
            &second.client_artifact,
            second.expectation(),
        )?;
        let second_identity = controller
            .staged_identity()
            .cloned()
            .ok_or("staged missing")?;

        let wrong_target = authorization(&first_identity, Some((&first_identity, 10)), 11);
        assert!(matches!(
            controller.activate(&wrong_target),
            Err(ContentActivationError::AuthorizationTargetMismatch)
        ));
        assert_eq!(
            controller.active().map(ActiveGeneration::identity),
            Some(&active_before_identity)
        );
        assert_eq!(
            controller
                .active()
                .map(ActiveGeneration::activation_sequence),
            Some(active_before_sequence)
        );

        let stale_expected = authorization(&second_identity, None, 11);
        assert!(matches!(
            controller.activate(&stale_expected),
            Err(ContentActivationError::ExpectedCurrentMismatch)
        ));
        assert_eq!(
            controller.active().map(ActiveGeneration::identity),
            Some(&active_before_identity)
        );
        assert_eq!(
            controller
                .active()
                .map(ActiveGeneration::activation_sequence),
            Some(active_before_sequence)
        );

        let stale_sequence = authorization(&second_identity, Some((&first_identity, 9)), 11);
        assert!(matches!(
            controller.activate(&stale_sequence),
            Err(ContentActivationError::ExpectedCurrentMismatch)
        ));
        assert_eq!(
            controller.active().map(ActiveGeneration::identity),
            Some(&active_before_identity)
        );
        assert_eq!(
            controller
                .active()
                .map(ActiveGeneration::activation_sequence),
            Some(active_before_sequence)
        );

        let nonmonotonic = authorization(&second_identity, Some((&first_identity, 10)), 10);
        assert!(matches!(
            controller.activate(&nonmonotonic),
            Err(ContentActivationError::NonMonotonicActivationSequence { .. })
        ));
        assert_eq!(
            controller.active().map(ActiveGeneration::identity),
            Some(&active_before_identity)
        );
        assert_eq!(
            controller
                .active()
                .map(ActiveGeneration::activation_sequence),
            Some(active_before_sequence)
        );
        assert_eq!(controller.staged_identity(), Some(&second_identity));
        Ok(())
    }

    #[test]
    fn commit_publishes_one_complete_generation_only() -> Result<(), Box<dyn Error>> {
        let compiled = compiled(3)?;
        let mut controller = ContentActivationController::new();
        let identity = controller
            .stage_primary(
                &compiled.server_artifact,
                &compiled.client_artifact,
                compiled.expectation(),
            )?
            .clone();

        let auth = authorization(&identity, None, 1);
        let active = controller.activate(&auth)?;
        assert_eq!(active.identity(), &identity);
        assert_eq!(active.activation_sequence(), 1);
        assert_eq!(active.server_record_count(), 22);
        assert_eq!(active.client_record_count(), 6);
        assert_eq!(
            active
                .server_definition_fields("oteryn:prod.creature")
                .and_then(|fields| fields.get(3))
                .map(String::as_str),
            Some("creature-policy-r1")
        );
        assert!(auth.hold_observed_during_target_check.get());
        assert!(!auth.hold_active.get());
        assert!(controller.staged_identity().is_none());
        assert!(controller.is_ready());
        assert_eq!(
            controller
                .last_known_good_receipt()
                .map(LastKnownGoodReceipt::identity),
            Some(&identity)
        );
        Ok(())
    }

    #[test]
    fn explicitly_authorized_last_known_good_fallback_revalidates_bytes_and_sequence()
    -> Result<(), Box<dyn Error>> {
        let first = compiled(3)?;
        let second = compiled(4)?;
        let mut controller = ContentActivationController::new();
        let first_identity = controller
            .stage_primary(
                &first.server_artifact,
                &first.client_artifact,
                first.expectation(),
            )?
            .clone();
        controller.activate(&authorization(&first_identity, None, 1))?;
        let old_receipt = controller
            .last_known_good_receipt()
            .cloned()
            .ok_or("missing first receipt")?;

        let second_identity = controller
            .stage_primary(
                &second.server_artifact,
                &second.client_artifact,
                second.expectation(),
            )?
            .clone();
        controller.activate(&authorization(
            &second_identity,
            Some((&first_identity, 1)),
            2,
        ))?;

        controller.stage_last_known_good_fallback(
            &first.server_artifact,
            &first.client_artifact,
            first.expectation(),
            &old_receipt,
        )?;
        let mut fallback = authorization(&first_identity, Some((&second_identity, 2)), 3);
        assert!(matches!(
            controller.activate(&fallback),
            Err(ContentActivationError::LastKnownGoodFallbackNotAuthorized)
        ));
        assert_eq!(
            controller.active().map(ActiveGeneration::identity),
            Some(&second_identity)
        );

        fallback.permit_fallback = true;
        let rolled_back = controller.activate(&fallback)?;
        assert_eq!(rolled_back.identity(), &first_identity);
        assert_eq!(rolled_back.activation_sequence(), 3);
        Ok(())
    }

    #[test]
    fn fallback_receipt_mismatch_and_corrupt_primary_fail_before_publication()
    -> Result<(), Box<dyn Error>> {
        let first = compiled(3)?;
        let second = compiled(4)?;
        let mut controller = ContentActivationController::new();
        let first_identity = controller
            .stage_primary(
                &first.server_artifact,
                &first.client_artifact,
                first.expectation(),
            )?
            .clone();
        controller.activate(&authorization(&first_identity, None, 1))?;
        let receipt = controller
            .last_known_good_receipt()
            .cloned()
            .ok_or("missing receipt")?;
        controller.discard_staged();

        let mismatch = controller.stage_last_known_good_fallback(
            &second.server_artifact,
            &second.client_artifact,
            second.expectation(),
            &receipt,
        );
        assert!(matches!(
            mismatch,
            Err(ContentActivationError::LastKnownGoodReceiptMismatch)
        ));
        assert!(controller.staged_identity().is_none());

        let mut corrupt = first.server_artifact.clone();
        let index = corrupt.len() - 33;
        corrupt[index] ^= 0x5a;
        let corrupt_result =
            controller.stage_primary(&corrupt, &first.client_artifact, first.expectation());
        assert!(matches!(
            corrupt_result,
            Err(ContentActivationError::Content(
                ContentError::IntegrityMismatch(_)
            ))
        ));
        assert!(controller.staged_identity().is_none());
        assert_eq!(
            controller.active().map(ActiveGeneration::identity),
            Some(&first_identity)
        );
        Ok(())
    }

    type NativeResult<T> = Result<T, Box<dyn Error>>;

    fn native_world(raw: u8) -> NativeResult<crate::foundation::WorldId> {
        let mut bytes = [0_u8; 16];
        bytes[15] = raw;
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        Ok(crate::foundation::WorldId::decode(&bytes)?)
    }

    fn native_issuance(
        world_id: crate::foundation::WorldId,
    ) -> NativeResult<(
        NativeEntryActivationIssuance,
        super::super::QualifiedNativeEntryRoom,
    )> {
        let room = super::super::qualify_native_entry_room(world_id)?;
        let issuance = NativeEntryActivationIssuance {
            world_id,
            activation_sequence: 7,
            server_artifact_digest: room.compiled().server_digest(),
            client_artifact_digest: room.compiled().client_digest(),
            frame_binding_digest: room.frame_binding().digest(),
        };
        Ok((issuance, room))
    }

    fn activate_native(
        controller: &mut ContentActivationController,
        world_id: crate::foundation::WorldId,
        issuance: &NativeEntryActivationIssuance,
    ) -> Result<NativeEntryContentPin, NativeEntryActivationError> {
        activate_native_entry_room(
            controller,
            &NodeBootQuiescence::before_channel_runtime(),
            world_id,
            issuance,
        )
    }

    #[test]
    fn native_entry_activation_pins_the_issued_generation_and_frame() -> NativeResult<()> {
        let world = native_world(1)?;
        let (issuance, room) = native_issuance(world)?;
        let mut controller = ContentActivationController::new();
        let pin = activate_native(&mut controller, world, &issuance)?;
        assert!(controller.is_ready());
        assert_eq!(
            controller.active().map(ActiveGeneration::identity),
            Some(pin.identity())
        );
        assert_eq!(pin.identity().world_id(), world);
        assert_eq!(pin.activation_sequence(), 7);
        assert_eq!(pin.frame_binding(), room.frame_binding());
        assert_eq!(
            pin.entry_start(),
            super::super::NativeEntryStart {
                x: 0,
                y: 0,
                floor: 0
            }
        );
        assert_eq!(
            pin.identity().server_artifact_digest(),
            issuance.server_artifact_digest
        );
        assert_eq!(
            pin.identity().client_artifact_digest(),
            issuance.client_artifact_digest
        );
        let channel = pin.into_channel_pin();
        assert_eq!(channel.world_id(), world);
        assert_eq!(channel.activation_sequence(), 7);
        assert_eq!(
            channel.server_artifact_digest(),
            issuance.server_artifact_digest
        );
        assert_eq!(
            channel.client_artifact_digest(),
            issuance.client_artifact_digest
        );
        assert_eq!(
            channel.frame_binding_digest(),
            issuance.frame_binding_digest
        );
        Ok(())
    }

    #[test]
    fn native_entry_frame_binding_is_bound_to_the_world() -> NativeResult<()> {
        let (first, first_room) = native_issuance(native_world(1)?)?;
        let (second, second_room) = native_issuance(native_world(2)?)?;
        assert_eq!(
            first_room.frame_binding().frame(),
            second_room.frame_binding().frame()
        );
        assert_ne!(first.frame_binding_digest, second.frame_binding_digest);
        assert_ne!(first.server_artifact_digest, second.server_artifact_digest);
        Ok(())
    }

    #[test]
    fn native_entry_activation_refuses_every_mismatch_before_staging() -> NativeResult<()> {
        let world = native_world(1)?;
        let other_world = native_world(2)?;
        let (issuance, _) = native_issuance(world)?;
        let (other, _) = native_issuance(other_world)?;
        let digest = NativeEntryActivationError::DigestMismatch;
        let frame = NativeEntryActivationError::FrameBindingMismatch;
        let cases = [
            (
                issuance.clone(),
                other_world,
                NativeEntryActivationError::WorldMismatch,
            ),
            (
                NativeEntryActivationIssuance {
                    server_artifact_digest: [9; 32],
                    ..issuance.clone()
                },
                world,
                digest.clone(),
            ),
            (
                NativeEntryActivationIssuance {
                    client_artifact_digest: [9; 32],
                    ..issuance.clone()
                },
                world,
                digest.clone(),
            ),
            (
                NativeEntryActivationIssuance {
                    frame_binding_digest: [9; 32],
                    ..issuance.clone()
                },
                world,
                frame.clone(),
            ),
            // Another World's genuine frame binding or pair cannot be substituted.
            (
                NativeEntryActivationIssuance {
                    frame_binding_digest: other.frame_binding_digest,
                    ..issuance.clone()
                },
                world,
                frame,
            ),
            (
                NativeEntryActivationIssuance {
                    server_artifact_digest: other.server_artifact_digest,
                    client_artifact_digest: other.client_artifact_digest,
                    ..issuance.clone()
                },
                world,
                digest,
            ),
        ];
        for (case, scope_world, expected) in cases {
            let mut controller = ContentActivationController::new();
            let Err(error) = activate_native(&mut controller, scope_world, &case) else {
                return Err(format!("{expected:?} case activated").into());
            };
            assert_eq!(error, expected);
            assert!(controller.active().is_none());
            assert!(controller.staged_identity().is_none());
            assert!(!controller.is_ready());
        }
        Ok(())
    }

    #[test]
    fn native_entry_activation_requires_a_newer_sequence_and_an_empty_start() -> NativeResult<()> {
        let world = native_world(1)?;
        let (issuance, _) = native_issuance(world)?;
        let mut controller = ContentActivationController::new();
        let zero = NativeEntryActivationIssuance {
            activation_sequence: 0,
            ..issuance.clone()
        };
        assert!(matches!(
            activate_native(&mut controller, world, &zero),
            Err(NativeEntryActivationError::Activation(
                ContentActivationError::NonMonotonicActivationSequence { .. }
            ))
        ));
        assert!(controller.active().is_none() && controller.staged_identity().is_none());

        let first = activate_native(&mut controller, world, &issuance)?;
        let newer = NativeEntryActivationIssuance {
            activation_sequence: 8,
            ..issuance.clone()
        };
        // A controller that already holds an active generation is not an empty start.
        assert!(matches!(
            activate_native(&mut controller, world, &newer),
            Err(NativeEntryActivationError::Activation(
                ContentActivationError::ExpectedCurrentMismatch
            ))
        ));
        assert!(controller.staged_identity().is_none());
        assert_eq!(
            controller.active().map(ActiveGeneration::identity),
            Some(first.identity())
        );
        assert_eq!(controller.last_activation_sequence(), 7);
        Ok(())
    }
}
