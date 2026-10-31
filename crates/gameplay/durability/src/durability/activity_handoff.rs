//! One native scope handoff owner shared by boss rooms and Soulpit.
//!
//! This owner consumes an existing control allocation. It never grants, assigns,
//! makes a scope ready, or treats the origin Channel as the private Instance.
//! A transaction terminalizes every source, issues fresh destination identities,
//! advances the existing Character/Account publishers and retains the group proof.
//! The runtime freezes and reserves physical actors before this asynchronous call;
//! only a committed result may install them. An uncertain commit retains the
//! reservation until `recover` or the exact command receipt resolves it.

use super::admission_authority_guards::AdmissionGuardStore;
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_last_position::{LastPositionCell, LastPositionRecord};
use super::character_progression::{assert_item_only_fence, assert_runtime_scope_fence};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::fresh_admission::FreshAdmissionStore;
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::{
    NodeIncarnationProof, assignment_scope_key, prove_current_incarnation,
};
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
};
use crate::foundation::{CharacterId, GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

mod entry;
mod transfer_recovery;
pub use transfer_recovery::ActivityTransferRecovery;
mod returning;
pub use returning::ActivityReturnPlacement;
mod departed_death;
pub use departed_death::DepartedActivityDeath;
pub(crate) use departed_death::validate_departed_activity_death_in_transaction;
mod retirement;
pub use retirement::RetiringActivity;
pub(crate) use retirement::read_retiring_activity_in_transaction;
pub mod boss_room;

/// Existing boss group storage ceiling, not a client-controlled allocation size.
pub const ACTIVITY_PARTICIPANTS_MAX: usize = 15;
pub type ActivityHandoffError = DurabilityError;
type Result<T> = std::result::Result<T, DurabilityError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityRefusal {
    InvalidInput,
    AuthorityRejected,
    NoCapacity,
    ContentChanged,
    ContentUnavailable,
    ReplayConflict,
    NotDue,
    AlreadyReturned,
    OwnerStillCurrent,
    PartyCount,
    Position,
    Level,
    Cooldown,
}

/// Current physical Instance interpretation, resolved from its native content pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityContentPin {
    pub coordinate_frame_digest: [u8; 32],
    pub map_revision_digest: [u8; 32],
    pub server_artifact_digest: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct ActivityParticipant {
    pub source: CurrentCharacterItemFence,
    /// Fresh server-issued identity, reserved in the destination native carrier.
    pub destination_game_session_id: GameSessionId,
    pub destination_transport: Option<crate::foundation::AuthenticatedTransportRefV1>,
    /// The authenticated source connection's exact negotiated selection. This
    /// is immutable handoff history; recovery never substitutes client support.
    pub selected_capabilities: Vec<u32>,
    /// Captured while the physical actor is frozen; synchronously persisted here.
    pub source_position: LastPositionRecord,
    pub source_position_write_sequence: u64,
    pub destination_cell: LastPositionCell,
    /// Content-authored return anchor in the original Channel. Its interpretation
    /// is retained independently from the Instance map and item-content digest.
    pub exit_position: LastPositionRecord,
}

#[derive(Debug, Clone)]
pub struct ActivityEnterRequest {
    pub handoff_id: [u8; 16],
    pub command_id: u64,
    pub initiating_character_id: CharacterId,
    pub activity_key: String,
    pub template_revision: String,
    pub content_revision: String,
    pub destination_scope: RuntimeScopeRefV1,
    pub destination_generation: ScopeOwnershipGeneration,
    pub destination_assignment_source_revision: u64,
    pub destination_content: ActivityContentPin,
    pub time_limit_ms: u64,
    /// Sorted by Character by this owner before locks or persistence.
    pub participants: Vec<ActivityParticipant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityParticipantReceipt {
    pub character_id: CharacterId,
    pub source_game_session_id: GameSessionId,
    pub destination_game_session_id: GameSessionId,
    pub destination_lease_generation: u64,
    pub destination_transport: Option<crate::foundation::AuthenticatedTransportRefV1>,
    pub destination_cell: LastPositionCell,
    pub exit_cell: LastPositionCell,
}

/// Retained historical outcome. It is not a current runtime authority witness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityEnterReceipt {
    pub handoff_id: [u8; 16],
    pub destination_scope: RuntimeScopeRefV1,
    pub destination_generation: ScopeOwnershipGeneration,
    pub deadline_unix_ms: i64,
    pub participants: Vec<ActivityParticipantReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityEnterOutcome {
    Entered(ActivityEnterReceipt),
    Replayed(ActivityEnterReceipt),
    Refused(ActivityRefusal),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityReturnReason {
    Exit,
    Timeout,
    Defeated,
    OwnerLost,
    Death {
        occurrence: [u8; 16],
    },
    Logout,
    /// Existing connection/serve/idle.rs elapsed timer plus its input permit and
    /// logout combat gate; a server occurrence, never a client command or loss.
    IdleLogout,
    /// Existing terminal-release owner: the exact ACTIVE transport ended.
    /// This reason is server-only and never represents a client logout.
    Abandoned {
        transport: crate::foundation::AuthenticatedTransportRefV1,
    },
    /// Existing D449 release after all recovery checks except selected support;
    /// the exact independently recorded loss remains authoritative.
    CapabilityMismatch {
        epoch: crate::foundation::ControlLossEpochRefV1,
    },
}
impl ActivityReturnReason {
    fn stored(self) -> i16 {
        match self {
            Self::Exit => 1,
            Self::Timeout => 2,
            Self::Defeated => 3,
            Self::OwnerLost => 4,
            Self::Death { .. } => 5,
            Self::Logout => 6,
            Self::IdleLogout => 7,
            Self::Abandoned { .. } => 8,
            Self::CapabilityMismatch { .. } => 9,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActivityReturnRequest {
    pub return_id: [u8; 16],
    pub handoff_id: [u8; 16],
    /// Some is a real client command; None is the server lifecycle occurrence
    /// identified by return_id. It never causes a synthetic CommandResult.
    pub command_id: Option<u64>,
    pub source: CurrentCharacterItemFence,
    pub destination_game_session_id: GameSessionId,
    pub destination_transport: Option<crate::foundation::AuthenticatedTransportRefV1>,
    pub destination_generation: ScopeOwnershipGeneration,
    pub reason: ActivityReturnReason,
    /// The retained exit cell/digests are authoritative. A live actor contributes
    /// only its current vitals/rest; recovery uses the retained entry snapshot.
    pub current_vitals: Option<super::character_last_position::LastVitals>,
    pub current_rest: Option<[u32; 4]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityReturnReceipt {
    pub return_id: [u8; 16],
    pub handoff_id: [u8; 16],
    pub character_id: CharacterId,
    pub source_game_session_id: GameSessionId,
    pub destination_game_session_id: GameSessionId,
    pub destination_scope: RuntimeScopeRefV1,
    pub destination_generation: ScopeOwnershipGeneration,
    pub destination_lease_generation: u64,
    pub destination_transport: Option<crate::foundation::AuthenticatedTransportRefV1>,
    pub exit_cell: LastPositionCell,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityReturnOutcome {
    Returned(ActivityReturnReceipt),
    Replayed(ActivityReturnReceipt),
    Refused(ActivityRefusal),
}

/// Recovery uses the same return receipt and origin binding. Its owner-loss
/// reason is independently proven against assignment and node history.
pub type ActivityRecoveryRequest = ActivityReturnRequest;
pub type ActivityRecoveryOutcome = ActivityReturnOutcome;

#[derive(Clone)]
pub struct ActivityHandoffOwner {
    root: DurabilityRoot,
}
impl ActivityHandoffOwner {
    pub fn new(root: DurabilityRoot) -> Self {
        Self { root }
    }

    pub async fn enter(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: ActivityEnterRequest,
    ) -> Result<ActivityEnterOutcome> {
        self.enter_inner(authority, node, request, false).await
    }

    pub async fn enter_boss_room(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: ActivityEnterRequest,
    ) -> Result<ActivityEnterOutcome> {
        self.enter_inner(authority, node, request, true).await
    }

    async fn enter_inner(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        mut request: ActivityEnterRequest,
        boss_room: bool,
    ) -> Result<ActivityEnterOutcome> {
        request
            .participants
            .sort_by_key(|p| *p.source.character_id.as_bytes());
        if !entry::valid_request(&request, boss_room) {
            return Ok(ActivityEnterOutcome::Refused(ActivityRefusal::InvalidInput));
        }
        let recovery = authority
            .record_for(&self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        let root = self.root.clone();
        let node = node.clone();
        self.root
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let result =
                        entry::enter_in_transaction(&mut tx, &root, &node, &request, boss_room)
                            .await?;
                    if !matches!(result, ActivityEnterOutcome::Refused(_)) {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await
    }

    pub async fn return_participant(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: ActivityReturnRequest,
    ) -> Result<ActivityReturnOutcome> {
        if request.reason == ActivityReturnReason::OwnerLost {
            return Ok(ActivityReturnOutcome::Refused(
                ActivityRefusal::InvalidInput,
            ));
        }
        self.return_inner(authority, node, request, false).await
    }

    pub async fn recover(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        mut request: ActivityRecoveryRequest,
    ) -> Result<ActivityRecoveryOutcome> {
        request.reason = ActivityReturnReason::OwnerLost;
        request.command_id = None;
        request.destination_transport = None;
        self.return_inner(authority, node, request, true).await
    }

    async fn return_inner(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: ActivityReturnRequest,
        recovering: bool,
    ) -> Result<ActivityReturnOutcome> {
        if request.command_id == Some(0)
            || matches!(request.reason, ActivityReturnReason::Abandoned { .. }
                if request.command_id.is_some() || request.destination_transport.is_none() || recovering)
            || matches!(request.reason, ActivityReturnReason::CapabilityMismatch { .. }
                if request.command_id.is_some() || request.destination_transport.is_some() || recovering)
            || (request.reason == ActivityReturnReason::IdleLogout
                && (request.command_id.is_some()
                    || request.destination_transport.is_none()
                    || recovering))
            || (request.command_id.is_none() && request.reason == ActivityReturnReason::Exit)
            || matches!(request.reason, ActivityReturnReason::Death { occurrence } if !uuid_v7(&occurrence) || request.command_id.is_some())
            || !uuid_v7(&request.return_id)
            || !uuid_v7(&request.handoff_id)
            || !matches!(
                request.source.runtime_scope,
                RuntimeScopeRefV1::Instance { .. }
            )
            || request.destination_game_session_id == request.source.game_session_id
            || request.source.character_lease_generation == u64::MAX
        {
            return Ok(ActivityReturnOutcome::Refused(
                ActivityRefusal::InvalidInput,
            ));
        }
        let recovery = authority
            .record_for(&self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        let root = self.root.clone();
        let node = node.clone();
        self.root
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let result = returning::return_in_transaction(
                        &mut tx, &root, &node, &request, recovering,
                    )
                    .await?;
                    if !matches!(result, ActivityReturnOutcome::Refused(_)) {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await
    }
}

fn uuid_v7(id: &[u8; 16]) -> bool {
    id[6] >> 4 == 7 && id[8] & 0xc0 == 0x80
}
fn revision_valid(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.as_bytes()[0].is_ascii_alphanumeric()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}
fn parse_u64(value: String) -> Result<u64> {
    value
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)
}
fn bytes16(value: Vec<u8>) -> Result<[u8; 16]> {
    value
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)
}
fn bytes32(value: Vec<u8>) -> Result<[u8; 32]> {
    value
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)
}
fn foundation_character(fence: CurrentCharacterItemFence) -> Result<CharacterId> {
    CharacterId::decode(fence.character_id.as_bytes())
        .map_err(|_| DurabilityError::InvalidStoredState)
}
async fn now_ms(tx: &mut Transaction<'_, Postgres>) -> Result<i64> {
    Ok(
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint")
            .fetch_one(&mut **tx)
            .await?,
    )
}

/// Publication history and session-identity history are separate from live mirrors.
async fn retained_participant(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    fence: CurrentCharacterItemFence,
) -> Result<Option<String>> {
    let character = foundation_character(fence)?;
    let admissions = FreshAdmissionStore::from_root(root.clone());
    if !admissions
        .committed_session_use(tx, character, fence.game_session_id)
        .await?
    {
        return Ok(None);
    }
    let guards = AdmissionGuardStore::from_root(root.clone());
    let Some(change) = guards
        .load_locked(tx, &AdmissionAuthorityGuardKeyV1::Character(character))
        .await?
    else {
        return Ok(None);
    };
    let AdmissionAuthorityGuardStateV1::Character { account_id, .. } = change.state else {
        return Ok(None);
    };
    for key in [
        AdmissionAuthorityGuardKeyV1::Account {
            account_id: account_id.clone(),
        },
        AdmissionAuthorityGuardKeyV1::Runtime(fence.runtime_scope),
    ] {
        if guards.load_locked(tx, &key).await?.is_none() {
            return Ok(None);
        }
    }
    Ok(Some(account_id))
}

fn hash_field(hash: &mut Sha256, value: &[u8]) {
    hash.update((value.len() as u64).to_be_bytes());
    hash.update(value);
}
