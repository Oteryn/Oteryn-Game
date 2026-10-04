//! SCOPE-HANDOFF-1 durable house scope and entry handoff (migration 0074).
//!
//! Decision: `docs/architecture/reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md`
//! §4.1-§4.3 with its ADMIT-0 amendment; packet
//! `docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_PREMIUM_SOCIAL_PACKETS_2026-10-04.md` §2.3.
//!
//! * A house is a runtime scope of kind 2 keyed by its `HouseId`; the control plane assigns it
//!   to one GameNode exactly as it assigns a Channel ([`DurabilityRoot::assign_house_scope`]),
//!   under exact-house grants and the shared assignment writer high water.
//! * Entering a house is a recoverable two-step transition. [`DurabilityRoot::prepare_house_entry`]
//!   records a PREPARED handoff and reserves the entry tile while the source Channel session
//!   stays live. [`DurabilityRoot::commit_house_entry`] re-reads house access through
//!   `game_house_access` inside the commit transaction, then terminalizes the
//!   source session and admits a fresh house session in the same transaction. A PREPARED
//!   handoff found after a crash is aborted by [`DurabilityRoot::reconcile_house_entries`] of
//!   the node holding its origin Channel, so the Character stays in its source session; a
//!   COMMITTED one is final and replays. A house session is never replaced: a reconnect into a
//!   house is a fresh entry handoff.
//! * Only the entry direction exists. The exit into a Channel scope and the §4.3 fallback stay
//!   refused ([`HouseHandoffError::ExitNotAdmitted`]) until ADMIT-0 defines that admission.
//!
//! The house session is fenced by its own session row (GameSessionId, lease generation and
//! session generation) plus the house scope assignment (HOUSE-RUNTIME-0 §6.1); the admission
//! guards are not rebound here. Actor wiring is HOUSE-RUNTIME-1.

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::{
    AssignmentError, AssignmentRejection, ControlActor, NodeIncarnationProof, NodeRegistrationFact,
    OperationKey, prove_current_incarnation, require_history_matches_high_water,
    scope_key as channel_scope_key, writer_high_water,
};
use super::{
    DurabilityError, DurabilityRoot, V2_COMMITTED, V2_PREPARED, V2_STALE_TERMINAL,
    V2_TERMINAL_SESSION, fresh_admission,
};
use crate::foundation::{CharacterId, GameSessionId, RuntimeScopeRefV1, WorldId};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

/// HOUSERT0-RL-03: Characters inside one house, PREPARED entries included.
pub const HOUSE_SCOPE_CHARACTERS_MAX: u64 = 200;
/// Maximum UTF-8 bytes of a house key (migration 0025).
pub const HOUSE_KEY_MAX_BYTES: usize = 512;
/// Maximum bytes of an encoded HouseInterior spatial position (migration 0025).
pub const HOUSE_TILE_MAX_BYTES: usize = 128;

const HOUSE_KEY_PREFIX: &str = "oteryn:content.house.";
const HOUSE_INSTANCE_DOMAIN: &[u8] = b"oteryn:house-scope-instance:v1";
const SCOPE_KIND_HOUSE: i16 = 2;
const ASSIGNMENT_ASSIGNED: i16 = 1;
const ASSIGNMENT_REVOKED: i16 = 2;
const HANDOFF_PREPARED: i16 = 1;
const HANDOFF_COMMITTED: i16 = 2;
const HANDOFF_ENTRY: i16 = 1;
const COMMAND_VERSION: u8 = 1;
const COMMAND_MAX_BYTES: usize = 1_024;

/// Exact identity of one house: its World and its content house key (migration 0025).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HouseId {
    world_id: WorldId,
    house_key: String,
}

impl HouseId {
    /// `None` unless the key is `oteryn:content.house.[a-z0-9_]+` within 512 bytes.
    #[must_use]
    pub fn new(world_id: WorldId, house_key: &str) -> Option<Self> {
        let suffix = house_key.strip_prefix(HOUSE_KEY_PREFIX)?;
        let valid = house_key.len() <= HOUSE_KEY_MAX_BYTES
            && !suffix.is_empty()
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
        valid.then(|| Self {
            world_id,
            house_key: house_key.to_owned(),
        })
    }

    #[must_use]
    pub const fn world_id(&self) -> WorldId {
        self.world_id
    }

    #[must_use]
    pub fn house_key(&self) -> &str {
        &self.house_key
    }

    /// The tagged assignment key: `\x02 || world || house key`.
    #[must_use]
    pub fn scope_key(&self) -> Vec<u8> {
        let mut key = Vec::with_capacity(17 + self.house_key.len());
        key.push(0x02);
        key.extend_from_slice(self.world_id.as_bytes());
        key.extend_from_slice(self.house_key.as_bytes());
        key
    }

    /// The UUIDv7-shaped instance id of this house; equal to `game_house_scope_instance_id`.
    #[must_use]
    pub fn instance_id(&self) -> [u8; 16] {
        let digest = Sha256::new()
            .chain_update(HOUSE_INSTANCE_DOMAIN)
            .chain_update(self.world_id.as_bytes())
            .chain_update(self.house_key.as_bytes())
            .finalize();
        let mut id = [0_u8; 16];
        id.copy_from_slice(&digest[..16]);
        id[6] = (id[6] & 0x0f) | 0x70;
        id[8] = (id[8] & 0x3f) | 0x80;
        id
    }

    /// The runtime scope of a house session.
    pub fn runtime_scope(&self) -> Result<RuntimeScopeRefV1, DurabilityError> {
        RuntimeScopeRefV1::instance(self.world_id, self.instance_id())
            .map_err(|_| DurabilityError::InvalidStoredState)
    }
}

/// Typed refusals of a house entry (HOUSE-RUNTIME-0 §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseEntryRefusal {
    NoAccess,
    InCombat,
    Busy,
    HouseClosed,
    NoRoom,
}

impl HouseEntryRefusal {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoAccess => "NO_ACCESS",
            Self::InCombat => "IN_COMBAT",
            Self::Busy => "BUSY",
            Self::HouseClosed => "HOUSE_CLOSED",
            Self::NoRoom => "NO_ROOM",
        }
    }
}

#[derive(Debug)]
pub enum HouseHandoffError {
    InvalidInput,
    /// The session, scope assignment or node incarnation fence does not hold.
    AuthorityRejected,
    /// The exit into a Channel scope (and the §4.3 fallback) awaits ADMIT-0.
    ExitNotAdmitted,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for HouseHandoffError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl From<sqlx::Error> for HouseHandoffError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}

type Result<T, E = HouseHandoffError> = std::result::Result<T, E>;

/// House access roles `game_house_access` admits (OTERYN_GAME_HOUSE_RT_INBOX_PACKETS §1.2).
const HOUSE_ACCESS_ROLES: [&str; 3] = ["OWNER", "SUBOWNER", "GUEST"];

/// Direction of a handoff. Only `Entry` is admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseHandoffDirection {
    Entry,
    Exit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseEntryRequest {
    pub direction: HouseHandoffDirection,
    /// Caller-generated UUIDv7; the idempotency identity of the handoff.
    pub handoff_id: [u8; 16],
    pub character_id: CharacterId,
    pub source_game_session_id: GameSessionId,
    pub source_connection_generation: u64,
    pub source_character_lease_generation: u64,
    pub source_scope: RuntimeScopeRefV1,
    pub source_scope_ownership_generation: u64,
    pub house: HouseId,
    pub house_scope_ownership_generation: u64,
    pub acl_revision: u64,
    /// The guild revisions the pre-check used for a guild-entry grant, as
    /// `game_house_access` returns them; `None` for any other grant.
    pub guild_revisions: Option<serde_json::Value>,
    /// HouseInterior `spatial_position` encoding, 1..=128 bytes.
    pub reserved_tile: Vec<u8>,
    pub prepared_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HouseEntryCommit {
    pub handoff_id: [u8; 16],
    /// Caller-generated UUIDv7 for the house session.
    pub destination_game_session_id: GameSessionId,
    pub committed_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseHandoffState {
    Prepared,
    Committed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseHandoffRecord {
    pub handoff_id: [u8; 16],
    pub character_id: CharacterId,
    pub source_game_session_id: GameSessionId,
    pub house: HouseId,
    pub guild_revisions: Option<serde_json::Value>,
    pub reserved_tile: Vec<u8>,
    pub state: HouseHandoffState,
    pub destination_game_session_id: Option<GameSessionId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HousePrepareOutcome {
    Prepared(HouseHandoffRecord),
    /// The same handoff id was already recorded; nothing was written.
    Replayed(HouseHandoffRecord),
    Refused(HouseEntryRefusal),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseCommitOutcome {
    Committed(HouseHandoffRecord),
    /// Already committed; nothing was written.
    Replayed(HouseHandoffRecord),
    /// The port refused; the handoff was aborted and the source session stays live.
    Refused(HouseEntryRefusal),
    /// No such handoff (aborted, reconciled or never prepared).
    Absent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseAbortOutcome {
    Aborted,
    Absent,
    AlreadyCommitted(HouseHandoffRecord),
}

/// Result of a reconcile pass: aborted PREPARED handoff ids.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HouseReconcileReport {
    pub aborted: Vec<[u8; 16]>,
}

/// Control-plane house scope assignment command (Channel commands keep their own writer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseScopeAssignmentCommand {
    Assign {
        house: HouseId,
        target: NodeRegistrationFact,
    },
    Replace {
        house: HouseId,
        predecessor: HouseScopePredecessor,
        target: NodeRegistrationFact,
    },
    Revoke {
        house: HouseId,
        predecessor: HouseScopePredecessor,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HouseScopePredecessor {
    pub ownership_generation: u64,
    pub source_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseScopeAssignmentRequest {
    pub operation_key: OperationKey,
    pub actor: ControlActor,
    pub actor_role: String,
    pub command: HouseScopeAssignmentCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseScopeAssignment {
    pub house: HouseId,
    pub ownership_generation: u64,
    pub assigned: bool,
    pub holder: Option<NodeRegistrationFact>,
    pub source_revision: u64,
}

impl HouseScopeAssignment {
    #[must_use]
    pub const fn predecessor(&self) -> HouseScopePredecessor {
        HouseScopePredecessor {
            ownership_generation: self.ownership_generation,
            source_revision: self.source_revision,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseScopeAssignmentOutcome {
    Committed(HouseScopeAssignment),
    /// The operation key already committed this exact command; nothing was written.
    Replayed,
    Rejected(AssignmentRejection),
}

impl HouseScopeAssignmentCommand {
    const fn house(&self) -> &HouseId {
        match self {
            Self::Assign { house, .. }
            | Self::Replace { house, .. }
            | Self::Revoke { house, .. } => house,
        }
    }

    const fn kind(&self) -> u8 {
        match self {
            Self::Assign { .. } => 1,
            Self::Replace { .. } => 2,
            Self::Revoke { .. } => 3,
        }
    }
}

impl HouseScopeAssignmentRequest {
    /// Canonical command bytes retained in the receipt. The prefix (version, kind, operation
    /// key, actor length, actor) is the layout the receipt grant guard reads.
    fn encode(&self) -> std::result::Result<Vec<u8>, AssignmentError> {
        let actor = self.actor_role.as_bytes();
        if ControlActor::new(&self.actor_role).ok().as_ref() != Some(&self.actor) {
            return Err(AssignmentError::InvalidInput);
        }
        let house = self.command.house();
        let key_len =
            u16::try_from(house.house_key.len()).map_err(|_| AssignmentError::InvalidInput)?;
        let mut encoded = Vec::with_capacity(128 + house.house_key.len());
        encoded.push(COMMAND_VERSION);
        encoded.push(self.command.kind());
        encoded.extend_from_slice(self.operation_key.as_bytes());
        encoded.push(u8::try_from(actor.len()).map_err(|_| AssignmentError::InvalidInput)?);
        encoded.extend_from_slice(actor);
        encoded.push(0x02);
        encoded.extend_from_slice(house.world_id.as_bytes());
        encoded.extend_from_slice(&key_len.to_be_bytes());
        encoded.extend_from_slice(house.house_key.as_bytes());
        match &self.command {
            HouseScopeAssignmentCommand::Replace { predecessor, .. }
            | HouseScopeAssignmentCommand::Revoke { predecessor, .. } => {
                if predecessor.ownership_generation == 0 || predecessor.source_revision == 0 {
                    return Err(AssignmentError::InvalidInput);
                }
                encoded.extend_from_slice(&predecessor.ownership_generation.to_be_bytes());
                encoded.extend_from_slice(&predecessor.source_revision.to_be_bytes());
            }
            HouseScopeAssignmentCommand::Assign { .. } => {}
        }
        match &self.command {
            HouseScopeAssignmentCommand::Assign { target, .. }
            | HouseScopeAssignmentCommand::Replace { target, .. } => {
                if target.registration_revision() == 0 {
                    return Err(AssignmentError::InvalidInput);
                }
                encoded.extend_from_slice(target.node_id().as_bytes());
                encoded.extend_from_slice(&target.registration_revision().to_be_bytes());
            }
            HouseScopeAssignmentCommand::Revoke { .. } => {}
        }
        if encoded.len() > COMMAND_MAX_BYTES {
            return Err(AssignmentError::InvalidInput);
        }
        Ok(encoded)
    }
}

impl DurabilityRoot {
    /// Assign, replace or revoke the GameNode holding one house scope. Runs as the control
    /// role: the receipt and effect guards require an exact-house grant for `session_user`,
    /// which must also be the recorded actor.
    pub async fn assign_house_scope(
        &self,
        request: HouseScopeAssignmentRequest,
    ) -> std::result::Result<HouseScopeAssignmentOutcome, AssignmentError> {
        let command = request.encode()?;
        Ok(self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let outcome =
                        assign_house_scope_in_transaction(&mut tx, &request, &command).await?;
                    if matches!(outcome, HouseScopeAssignmentOutcome::Committed(_)) {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(outcome)
                })
            })
            .await?)
    }

    /// Record a PREPARED entry handoff and reserve its tile; the source session stays live.
    pub async fn prepare_house_entry(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: HouseEntryRequest,
    ) -> Result<HousePrepareOutcome> {
        validate_request(&request)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseHandoffError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    match prepare_house_entry_in_transaction(&mut tx, &node, &request).await {
                        Ok(outcome @ HousePrepareOutcome::Prepared(_)) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(outcome))
                        }
                        Ok(outcome) => Ok(Ok(outcome)),
                        Err(HouseHandoffError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }

    /// Commit a PREPARED entry: re-read house access, terminalize the
    /// source session and admit a fresh house session, in one transaction. `node` must hold
    /// the house scope.
    pub async fn commit_house_entry(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        commit: HouseEntryCommit,
    ) -> Result<HouseCommitOutcome> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseHandoffError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    match commit_house_entry_in_transaction(&mut tx, &node, &commit).await {
                        Ok(
                            outcome @ (HouseCommitOutcome::Committed(_)
                            | HouseCommitOutcome::Refused(_)),
                        ) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(outcome))
                        }
                        Ok(outcome) => Ok(Ok(outcome)),
                        Err(HouseHandoffError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }

    /// Abort a PREPARED entry, releasing its tile; the source session stays live.
    pub async fn abort_house_entry(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        handoff_id: [u8; 16],
    ) -> Result<HouseAbortOutcome> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseHandoffError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    match abort_house_entry_in_transaction(&mut tx, handoff_id).await {
                        Ok(outcome) => {
                            if outcome == HouseAbortOutcome::Aborted {
                                commit_semantic_transaction(tx, deadline).await?;
                            }
                            Ok(Ok(outcome))
                        }
                        Err(HouseHandoffError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }

    /// Restart recovery of the proving node: abort the PREPARED entries (of `character` only,
    /// when given) whose origin Channel the proven current incarnation of `node` holds at the
    /// source session's generation. Another node's PREPARED entry is never touched. A PREPARED
    /// row never committed, so its Character still holds its source session; a COMMITTED row is
    /// final and is left as retained history.
    pub async fn reconcile_house_entries(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        character: Option<CharacterId>,
    ) -> Result<HouseReconcileReport> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseHandoffError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    match reconcile_house_entries_in_transaction(&mut tx, &node, character).await {
                        Ok(report) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(report))
                        }
                        Err(HouseHandoffError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

/// The house writer step inside a transaction. Never commits.
pub(crate) async fn assign_house_scope_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    request: &HouseScopeAssignmentRequest,
    command: &[u8],
) -> std::result::Result<HouseScopeAssignmentOutcome, DurabilityError> {
    let house = request.command.house();
    let key = house.scope_key();
    let high_water = writer_high_water(tx, true).await?;
    require_history_matches_high_water(tx, high_water).await?;
    let stored: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT command FROM game_runtime_scope_assignment_receipts WHERE operation_key = $1",
    )
    .bind(request.operation_key.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(stored) = stored {
        return Ok(if stored == command {
            HouseScopeAssignmentOutcome::Replayed
        } else {
            HouseScopeAssignmentOutcome::Rejected(AssignmentRejection::OperationConflict)
        });
    }
    let granted: bool = sqlx::query_scalar(
        "SELECT $1 = session_user AND EXISTS (SELECT 1 FROM game_control_house_scope_grants \
         WHERE control_role = session_user AND world_id = encode($2, 'hex')::uuid \
           AND house_key = $3 AND operation = $4)",
    )
    .bind(request.actor_role.as_str())
    .bind(house.world_id.as_bytes().as_slice())
    .bind(house.house_key.as_str())
    .bind(i16::from(request.command.kind()))
    .fetch_one(&mut **tx)
    .await?;
    if !granted {
        return Ok(HouseScopeAssignmentOutcome::Rejected(
            AssignmentRejection::NotGranted,
        ));
    }
    let current = sqlx::query(
        "SELECT scope_kind, state, ownership_generation::text AS generation, \
                source_revision::text AS revision \
           FROM game_runtime_scope_assignments WHERE scope_key = $1 FOR UPDATE",
    )
    .bind(key.as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let current = match current {
        None => None,
        Some(row) => {
            if row.try_get::<i16, _>("scope_kind")? != SCOPE_KIND_HOUSE {
                return Err(DurabilityError::InvalidStoredState);
            }
            let predecessor = HouseScopePredecessor {
                ownership_generation: parse_u64(&row.try_get::<String, _>("generation")?)?,
                source_revision: parse_u64(&row.try_get::<String, _>("revision")?)?,
            };
            Some((predecessor, row.try_get::<i16, _>("state")?))
        }
    };
    let rejection = match (&request.command, current) {
        (HouseScopeAssignmentCommand::Assign { .. }, None) => None,
        (HouseScopeAssignmentCommand::Replace { predecessor, .. }, Some((current, _)))
            if *predecessor == current =>
        {
            None
        }
        // Like the Channel writer: only an assigned house scope can be revoked.
        (HouseScopeAssignmentCommand::Revoke { predecessor, .. }, Some((current, state)))
            if *predecessor == current =>
        {
            (state != ASSIGNMENT_ASSIGNED).then_some(AssignmentRejection::NotAssigned)
        }
        _ => Some(AssignmentRejection::PredecessorMismatch),
    };
    if let Some(rejection) = rejection {
        return Ok(HouseScopeAssignmentOutcome::Rejected(rejection));
    }
    let target = match &request.command {
        HouseScopeAssignmentCommand::Assign { target, .. }
        | HouseScopeAssignmentCommand::Replace { target, .. } => Some(*target),
        HouseScopeAssignmentCommand::Revoke { .. } => None,
    };
    if let Some(target) = target {
        let current_target: bool = sqlx::query_scalar(
            "SELECT game_node_lock_current_registration(encode($1, 'hex')::uuid, \
                                                        $2::text::numeric(20,0))",
        )
        .bind(target.node_id().as_bytes().as_slice())
        .bind(target.registration_revision().to_string())
        .fetch_one(&mut **tx)
        .await?;
        if !current_target {
            return Ok(HouseScopeAssignmentOutcome::Rejected(
                AssignmentRejection::TargetNotCurrent,
            ));
        }
    }
    let Some(source_revision) = high_water.checked_add(1) else {
        return Ok(HouseScopeAssignmentOutcome::Rejected(
            AssignmentRejection::SourceRevisionExhausted,
        ));
    };
    let Some(ownership_generation) = current.map_or(Some(1), |(current, _)| {
        current.ownership_generation.checked_add(1)
    }) else {
        return Ok(HouseScopeAssignmentOutcome::Rejected(
            AssignmentRejection::GenerationExhausted,
        ));
    };
    let state = if target.is_some() {
        ASSIGNMENT_ASSIGNED
    } else {
        ASSIGNMENT_REVOKED
    };
    let holder_node = target.map(|target| target.node_id().as_bytes().to_vec());
    let holder_revision = target.map(|target| target.registration_revision().to_string());
    let decision_identity = format!("runtime-scope-assignment:{source_revision}");
    let decided_at: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(&mut **tx)
            .await?;
    sqlx::query(
        "INSERT INTO game_runtime_scope_assignments \
         (scope_key, scope_kind, world_id, channel_id, house_key, ownership_generation, state, \
          holder_node_id, holder_registration_revision, source_revision, decision_identity, \
          operation_key, decided_at) \
         VALUES ($1, 2, encode($2, 'hex')::uuid, NULL, $3, $4::text::numeric(20,0), $5, \
                 encode($6::bytea, 'hex')::uuid, $7::text::numeric(20,0), \
                 $8::text::numeric(20,0), $9, $10, $11) \
         ON CONFLICT (scope_key) DO UPDATE SET ownership_generation = EXCLUDED.ownership_generation, \
             state = EXCLUDED.state, holder_node_id = EXCLUDED.holder_node_id, \
             holder_registration_revision = EXCLUDED.holder_registration_revision, \
             source_revision = EXCLUDED.source_revision, decision_identity = EXCLUDED.decision_identity, \
             operation_key = EXCLUDED.operation_key, decided_at = EXCLUDED.decided_at",
    )
    .bind(key.as_slice())
    .bind(house.world_id.as_bytes().as_slice())
    .bind(house.house_key.as_str())
    .bind(ownership_generation.to_string())
    .bind(state)
    .bind(holder_node.as_deref())
    .bind(holder_revision.as_deref())
    .bind(source_revision.to_string())
    .bind(&decision_identity)
    .bind(request.operation_key.as_bytes().as_slice())
    .bind(decided_at)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "UPDATE game_runtime_scope_assignment_writer \
         SET source_revision_high_water = $1::text::numeric(20,0) WHERE writer_id = 1",
    )
    .bind(source_revision.to_string())
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_runtime_scope_assignment_receipts \
         (operation_key, command, scope_key, ownership_generation, state, holder_node_id, \
          holder_registration_revision, source_revision, decision_identity, decided_at, \
          fenced_publication_revision) \
         VALUES ($1, $2, $3, $4::text::numeric(20,0), $5, encode($6::bytea, 'hex')::uuid, \
                 $7::text::numeric(20,0), $8::text::numeric(20,0), $9, $10, NULL)",
    )
    .bind(request.operation_key.as_bytes().as_slice())
    .bind(command)
    .bind(key.as_slice())
    .bind(ownership_generation.to_string())
    .bind(state)
    .bind(holder_node.as_deref())
    .bind(holder_revision.as_deref())
    .bind(source_revision.to_string())
    .bind(&decision_identity)
    .bind(decided_at)
    .execute(&mut **tx)
    .await?;
    Ok(HouseScopeAssignmentOutcome::Committed(
        HouseScopeAssignment {
            house: house.clone(),
            ownership_generation,
            assigned: target.is_some(),
            holder: target,
            source_revision,
        },
    ))
}

fn validate_request(request: &HouseEntryRequest) -> Result<()> {
    if request.direction != HouseHandoffDirection::Entry {
        return Err(HouseHandoffError::ExitNotAdmitted);
    }
    let RuntimeScopeRefV1::Channel { world_id, .. } = request.source_scope else {
        // An entry starts in a Channel scope; leaving a house is the refused exit.
        return Err(HouseHandoffError::ExitNotAdmitted);
    };
    let valid = is_uuid_v7(&request.handoff_id)
        && world_id == request.house.world_id
        && HouseId::new(request.house.world_id, &request.house.house_key).as_ref()
            == Some(&request.house)
        && (1..=HOUSE_TILE_MAX_BYTES).contains(&request.reserved_tile.len())
        && request.source_connection_generation >= 1
        && (1..u64::MAX).contains(&request.source_character_lease_generation)
        && request.source_scope_ownership_generation >= 1
        && request.house_scope_ownership_generation >= 1
        && request.prepared_at >= 0;
    if valid {
        Ok(())
    } else {
        Err(HouseHandoffError::InvalidInput)
    }
}

/// Prepare inside a transaction that asserted the recovery fence and took the admission
/// relation locks. Never commits; only `Prepared` wrote anything.
pub(crate) async fn prepare_house_entry_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    request: &HouseEntryRequest,
) -> Result<HousePrepareOutcome> {
    validate_request(request)?;
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = request.source_scope
    else {
        return Err(HouseHandoffError::ExitNotAdmitted);
    };
    if let Some((record, binding)) = load_handoff(tx, request.handoff_id, true).await? {
        return if binding == prepared_binding(request)
            && record.guild_revisions == request.guild_revisions
        {
            Ok(HousePrepareOutcome::Replayed(record))
        } else {
            Err(HouseHandoffError::InvalidInput)
        };
    }
    // The source Channel session, exactly at the caller's fence.
    let source = sqlx::query(
        "SELECT account_id::text AS account FROM game_durability_reconnect_sessions \
         WHERE game_session_id = encode($1,'hex')::uuid \
           AND character_id = encode($2,'hex')::uuid \
           AND world_id = encode($3,'hex')::uuid \
           AND runtime_scope_kind = 1 \
           AND runtime_scope_world_id = encode($3,'hex')::uuid \
           AND runtime_scope_channel_id = encode($4,'hex')::uuid \
           AND current_generation = $5::text::numeric(20,0) \
           AND character_lease_generation = $6::text::numeric(20,0) \
           AND scope_ownership_generation = $7::text::numeric(20,0) \
           AND session_state IN (1,2) FOR UPDATE",
    )
    .bind(request.source_game_session_id.as_bytes().as_slice())
    .bind(request.character_id.as_bytes().as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(request.source_connection_generation.to_string())
    .bind(request.source_character_lease_generation.to_string())
    .bind(request.source_scope_ownership_generation.to_string())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(source) = source else {
        return Err(HouseHandoffError::AuthorityRejected);
    };
    let account: String = source.try_get("account")?;
    // The source Channel is held by the proving node at the session's generation.
    if !origin_channel_held(
        tx,
        &channel_scope_key(world_id, channel_id),
        request.source_scope_ownership_generation,
        node,
    )
    .await?
    {
        return Err(HouseHandoffError::AuthorityRejected);
    }
    if !house_assigned(
        tx,
        &request.house,
        request.house_scope_ownership_generation,
        None,
    )
    .await?
    {
        return Ok(HousePrepareOutcome::Refused(HouseEntryRefusal::HouseClosed));
    }
    // SCOPE-HANDOFF-1 admits same-node entry only: a house held by another node is busy.
    if !house_assigned(
        tx,
        &request.house,
        request.house_scope_ownership_generation,
        Some(node),
    )
    .await?
    {
        return Ok(HousePrepareOutcome::Refused(HouseEntryRefusal::Busy));
    }
    let open: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_house_scope_handoffs \
          WHERE character_id = encode($1,'hex')::uuid AND state = 1)",
    )
    .bind(request.character_id.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    if open {
        return Ok(HousePrepareOutcome::Refused(HouseEntryRefusal::Busy));
    }
    let occupancy = sqlx::query(
        "SELECT (SELECT count(*) FROM game_durability_reconnect_sessions \
                  WHERE runtime_scope_world_id = encode($1,'hex')::uuid \
                    AND runtime_scope_house_key = $2 AND session_state IN (1,2)) \
              + (SELECT count(*) FROM game_house_scope_handoffs \
                  WHERE world_id = encode($1,'hex')::uuid AND house_key = $2 AND state = 1) \
                AS inside, \
                EXISTS (SELECT 1 FROM game_house_scope_handoffs \
                  WHERE world_id = encode($1,'hex')::uuid AND house_key = $2 AND state = 1 \
                    AND reserved_tile = $3) AS tile_taken",
    )
    .bind(request.house.world_id.as_bytes().as_slice())
    .bind(request.house.house_key.as_str())
    .bind(request.reserved_tile.as_slice())
    .fetch_one(&mut **tx)
    .await?;
    let inside: i64 = occupancy.try_get("inside")?;
    let tile_taken: bool = occupancy.try_get("tile_taken")?;
    if u64::try_from(inside).map_err(|_| DurabilityError::InvalidStoredState)?
        >= HOUSE_SCOPE_CHARACTERS_MAX
        || tile_taken
    {
        return Ok(HousePrepareOutcome::Refused(HouseEntryRefusal::NoRoom));
    }
    sqlx::query(
        "INSERT INTO game_house_scope_handoffs \
         (handoff_id, direction, character_id, account_id, world_id, origin_channel_id, \
          source_game_session_id, source_connection_generation, \
          source_character_lease_generation, source_scope_ownership_generation, house_key, \
          destination_scope_ownership_generation, acl_revision, reserved_tile, state, \
          prepared_at, guild_revisions) \
         VALUES (encode($1,'hex')::uuid, $2, encode($3,'hex')::uuid, $4::uuid, \
                 encode($5,'hex')::uuid, encode($6,'hex')::uuid, encode($7,'hex')::uuid, \
                 $8::text::numeric(20,0), $9::text::numeric(20,0), $10::text::numeric(20,0), \
                 $11, $12::text::numeric(20,0), $13::text::numeric(20,0), $14, $15, $16, \
                 $17::text::jsonb)",
    )
    .bind(request.handoff_id.as_slice())
    .bind(HANDOFF_ENTRY)
    .bind(request.character_id.as_bytes().as_slice())
    .bind(account)
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(request.source_game_session_id.as_bytes().as_slice())
    .bind(request.source_connection_generation.to_string())
    .bind(request.source_character_lease_generation.to_string())
    .bind(request.source_scope_ownership_generation.to_string())
    .bind(request.house.house_key.as_str())
    .bind(request.house_scope_ownership_generation.to_string())
    .bind(request.acl_revision.to_string())
    .bind(request.reserved_tile.as_slice())
    .bind(HANDOFF_PREPARED)
    .bind(request.prepared_at)
    .bind(
        request
            .guild_revisions
            .as_ref()
            .map(serde_json::Value::to_string),
    )
    .execute(&mut **tx)
    .await?;
    let (record, _) = load_handoff(tx, request.handoff_id, false)
        .await?
        .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(HousePrepareOutcome::Prepared(record))
}

/// Commit inside a transaction that asserted the recovery fence and took the admission
/// relation locks. Never commits; `Committed` and `Refused` wrote, the others did not.
pub(crate) async fn commit_house_entry_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    commit: &HouseEntryCommit,
) -> Result<HouseCommitOutcome> {
    if !is_uuid_v7(&commit.handoff_id) || commit.committed_at < 0 {
        return Err(HouseHandoffError::InvalidInput);
    }
    let Some(handoff) = sqlx::query(
        "SELECT state, account_id::text AS account, \
                uuid_send(origin_channel_id) AS origin, \
                source_connection_generation::text AS source_generation, \
                source_character_lease_generation::text AS source_lease, \
                source_scope_ownership_generation::text AS source_scope, \
                destination_scope_ownership_generation::text AS house_generation, prepared_at \
           FROM game_house_scope_handoffs WHERE handoff_id = encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(commit.handoff_id.as_slice())
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(HouseCommitOutcome::Absent);
    };
    let (record, _) = load_handoff(tx, commit.handoff_id, false)
        .await?
        .ok_or(DurabilityError::InvalidStoredState)?;
    if handoff.try_get::<i16, _>("state")? == HANDOFF_COMMITTED {
        return if record.destination_game_session_id == Some(commit.destination_game_session_id) {
            Ok(HouseCommitOutcome::Replayed(record))
        } else {
            Err(HouseHandoffError::InvalidInput)
        };
    }
    if commit.destination_game_session_id == record.source_game_session_id
        || commit.committed_at < handoff.try_get::<i64, _>("prepared_at")?
    {
        return Err(HouseHandoffError::InvalidInput);
    }
    let house_generation = parse_u64(&handoff.try_get::<String, _>("house_generation")?)?;
    let source_generation = handoff.try_get::<String, _>("source_generation")?;
    let source_lease = parse_u64(&handoff.try_get::<String, _>("source_lease")?)?;
    let source_scope = handoff.try_get::<String, _>("source_scope")?;
    let account: String = handoff.try_get("account")?;
    let origin: Vec<u8> = handoff.try_get("origin")?;

    // The proving node holds the house scope at the prepared generation.
    if !house_assigned(tx, &record.house, house_generation, Some(node)).await? {
        return Err(HouseHandoffError::AuthorityRejected);
    }
    // The origin Channel is still held by the proving node at the source session's generation:
    // a replaced or revoked origin assignment fails the handoff.
    let mut origin_key = vec![1_u8];
    origin_key.extend_from_slice(record.house.world_id.as_bytes());
    origin_key.extend_from_slice(&origin);
    if !origin_channel_held(tx, &origin_key, parse_u64(&source_scope)?, node).await? {
        return Err(HouseHandoffError::AuthorityRejected);
    }
    // The source session is still exactly the prepared one.
    let source = sqlx::query(
        "SELECT prepared_attempt_ref, original_grace_deadline \
           FROM game_durability_reconnect_sessions \
          WHERE game_session_id = encode($1,'hex')::uuid \
            AND character_id = encode($2,'hex')::uuid \
            AND runtime_scope_kind = 1 AND runtime_scope_channel_id = encode($3,'hex')::uuid \
            AND current_generation = $4::text::numeric(20,0) \
            AND character_lease_generation = $5::text::numeric(20,0) \
            AND scope_ownership_generation = $6::text::numeric(20,0) \
            AND session_state IN (1,2) FOR UPDATE",
    )
    .bind(record.source_game_session_id.as_bytes().as_slice())
    .bind(record.character_id.as_bytes().as_slice())
    .bind(origin.as_slice())
    .bind(&source_generation)
    .bind(source_lease.to_string())
    .bind(&source_scope)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(source) = source else {
        return Err(HouseHandoffError::AuthorityRejected);
    };

    // House access, re-read with its FOR SHARE locks (HOUSE-RUNTIME-0 §4.1): a concurrent
    // revocation either committed first and is seen here, or waits until this admission
    // commits and then finds the Character inside. Any difference from the pre-check refuses.
    let access = sqlx::query(
        "SELECT a.role, a.content_fenced, \
                a.acl_revision IS NOT DISTINCT FROM h.acl_revision AS acl_current, \
                a.guild_revisions IS NOT DISTINCT FROM h.guild_revisions AS guild_current \
           FROM game_house_scope_handoffs h \
          CROSS JOIN LATERAL game_house_access(h.world_id, h.house_key, h.character_id) a \
          WHERE h.handoff_id = encode($1,'hex')::uuid",
    )
    .bind(commit.handoff_id.as_slice())
    .fetch_all(&mut **tx)
    .await?;
    let refusal = match access.as_slice() {
        [] => Some(HouseEntryRefusal::NoAccess),
        [row] => {
            let role: Option<String> = row.try_get("role")?;
            if !role
                .as_deref()
                .is_some_and(|role| HOUSE_ACCESS_ROLES.contains(&role))
                || !row.try_get::<bool, _>("acl_current")?
                || !row.try_get::<bool, _>("guild_current")?
            {
                Some(HouseEntryRefusal::NoAccess)
            } else if row.try_get::<Option<bool>, _>("content_fenced")? != Some(false) {
                Some(HouseEntryRefusal::HouseClosed)
            } else {
                None
            }
        }
        _ => return Err(DurabilityError::InvalidStoredState.into()),
    };
    if let Some(refusal) = refusal {
        delete_prepared(tx, commit.handoff_id).await?;
        return Ok(HouseCommitOutcome::Refused(refusal));
    }

    let membership_revision = fresh_admission::unused_session_revision(
        tx,
        record.character_id,
        commit.destination_game_session_id,
    )
    .await?;
    if let Some(prepared_attempt_ref) =
        source.try_get::<Option<Vec<u8>>, _>("prepared_attempt_ref")?
    {
        let terminalized = sqlx::query(
            "UPDATE game_durability_reconnect_attempts SET state = $3 \
             WHERE game_session_id = encode($1, 'hex')::uuid \
               AND reconnect_attempt_ref = $2 AND state = $4",
        )
        .bind(record.source_game_session_id.as_bytes().as_slice())
        .bind(prepared_attempt_ref.as_slice())
        .bind(V2_STALE_TERMINAL)
        .bind(V2_PREPARED)
        .execute(&mut **tx)
        .await?;
        if terminalized.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState.into());
        }
    }
    sqlx::query(
        "UPDATE game_durability_reconnect_attempts SET state = $2 \
         WHERE game_session_id = encode($1, 'hex')::uuid AND state = $3",
    )
    .bind(record.source_game_session_id.as_bytes().as_slice())
    .bind(V2_STALE_TERMINAL)
    .bind(V2_COMMITTED)
    .execute(&mut **tx)
    .await?;
    let terminalized = sqlx::query(
        "UPDATE game_durability_reconnect_sessions \
         SET current_transport_ref = NULL, session_state = $2, prepared_attempt_ref = NULL \
         WHERE game_session_id = encode($1, 'hex')::uuid AND session_state IN (1, 2)",
    )
    .bind(record.source_game_session_id.as_bytes().as_slice())
    .bind(V2_TERMINAL_SESSION)
    .execute(&mut **tx)
    .await?;
    if terminalized.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    fresh_admission::commit_session_use(
        tx,
        record.character_id,
        commit.destination_game_session_id,
        commit.handoff_id,
        membership_revision,
    )
    .await?;
    let grace_deadline = source
        .try_get::<Option<i64>, _>("original_grace_deadline")?
        .unwrap_or(commit.committed_at);
    let destination_lease = source_lease
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    // The house session awaits its transport (HOUSE-RUNTIME-1 binds the actor): reconnectable,
    // first control-loss epoch, generation one, at the house scope generation.
    sqlx::query(
        "INSERT INTO game_durability_reconnect_sessions \
         (game_session_id, account_id, character_id, world_id, runtime_scope_kind, \
          runtime_scope_world_id, runtime_scope_channel_id, runtime_scope_instance_id, \
          runtime_scope_house_key, origin_channel_id, control_loss_epoch, \
          original_grace_deadline, predecessor_generation, character_lease_generation, \
          scope_ownership_generation, current_generation, current_transport_ref, session_state) \
         VALUES (encode($1,'hex')::uuid, $2::uuid, encode($3,'hex')::uuid, \
                 encode($4,'hex')::uuid, 2, encode($4,'hex')::uuid, NULL, encode($5,'hex')::uuid, \
                 $6, encode($7,'hex')::uuid, 1, $8, 1, $9::text::numeric(20,0), \
                 $10::text::numeric(20,0), 1, NULL, 1)",
    )
    .bind(commit.destination_game_session_id.as_bytes().as_slice())
    .bind(account)
    .bind(record.character_id.as_bytes().as_slice())
    .bind(record.house.world_id.as_bytes().as_slice())
    .bind(record.house.instance_id().as_slice())
    .bind(record.house.house_key.as_str())
    .bind(origin.as_slice())
    .bind(grace_deadline)
    .bind(destination_lease.to_string())
    .bind(house_generation.to_string())
    .execute(&mut **tx)
    .await?;
    let committed = sqlx::query(
        "UPDATE game_house_scope_handoffs \
         SET state = $2, destination_game_session_id = encode($3,'hex')::uuid, committed_at = $4 \
         WHERE handoff_id = encode($1,'hex')::uuid AND state = 1",
    )
    .bind(commit.handoff_id.as_slice())
    .bind(HANDOFF_COMMITTED)
    .bind(commit.destination_game_session_id.as_bytes().as_slice())
    .bind(commit.committed_at)
    .execute(&mut **tx)
    .await?;
    if committed.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    let (record, _) = load_handoff(tx, commit.handoff_id, false)
        .await?
        .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(HouseCommitOutcome::Committed(record))
}

/// Abort inside a fenced transaction. Never commits; only `Aborted` wrote.
pub(crate) async fn abort_house_entry_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    handoff_id: [u8; 16],
) -> Result<HouseAbortOutcome> {
    match load_handoff(tx, handoff_id, true).await? {
        None => Ok(HouseAbortOutcome::Absent),
        Some((record, _)) if record.state == HouseHandoffState::Committed => {
            Ok(HouseAbortOutcome::AlreadyCommitted(record))
        }
        Some(_) => {
            delete_prepared(tx, handoff_id).await?;
            Ok(HouseAbortOutcome::Aborted)
        }
    }
}

/// Reconcile inside a fenced transaction. Never commits.
pub(crate) async fn reconcile_house_entries_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    character: Option<CharacterId>,
) -> Result<HouseReconcileReport> {
    if !prove_current_incarnation(tx, node).await? {
        return Err(HouseHandoffError::AuthorityRejected);
    }
    let fact = node.fact();
    // The origin Channel assignment row is locked FOR SHARE, so a concurrent replace or revoke
    // of it waits for this transaction.
    let rows: Vec<Vec<u8>> = sqlx::query_scalar(
        "WITH owned AS ( \
           SELECT h.handoff_id FROM game_house_scope_handoffs h \
             JOIN game_runtime_scope_assignments a \
               ON a.scope_key = '\\x01'::bytea || uuid_send(h.world_id) \
                                || uuid_send(h.origin_channel_id) \
              AND a.scope_kind = 1 AND a.state = 1 \
              AND a.ownership_generation = h.source_scope_ownership_generation \
              AND a.holder_node_id = encode($2,'hex')::uuid \
              AND a.holder_registration_revision = $3::text::numeric(20,0) \
            WHERE h.state = 1 \
              AND ($1::bytea IS NULL OR h.character_id = encode($1,'hex')::uuid) \
              FOR SHARE OF a) \
         DELETE FROM game_house_scope_handoffs h USING owned o \
          WHERE h.handoff_id = o.handoff_id AND h.state = 1 \
          RETURNING uuid_send(h.handoff_id)",
    )
    .bind(character.map(|character| character.as_bytes().to_vec()))
    .bind(fact.node_id().as_bytes().as_slice())
    .bind(fact.registration_revision().to_string())
    .fetch_all(&mut **tx)
    .await?;
    let mut aborted = rows
        .into_iter()
        .map(|id| <[u8; 16]>::try_from(id).map_err(|_| DurabilityError::InvalidStoredState))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    aborted.sort_unstable();
    Ok(HouseReconcileReport { aborted })
}

async fn delete_prepared(tx: &mut Transaction<'_, Postgres>, handoff_id: [u8; 16]) -> Result<()> {
    let deleted = sqlx::query(
        "DELETE FROM game_house_scope_handoffs WHERE handoff_id = encode($1,'hex')::uuid AND state = 1",
    )
    .bind(handoff_id.as_slice())
    .execute(&mut **tx)
    .await?;
    if deleted.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    Ok(())
}

/// The Channel scope `scope_key` is ASSIGNED at `generation` to the proving node. Holds the
/// assignment row `FOR SHARE` until the transaction ends.
async fn origin_channel_held(
    tx: &mut Transaction<'_, Postgres>,
    scope_key: &[u8],
    generation: u64,
    node: &NodeIncarnationProof,
) -> std::result::Result<bool, DurabilityError> {
    let fact = node.fact();
    let held = sqlx::query(
        "SELECT 1 FROM game_runtime_scope_assignments \
          WHERE scope_key = $1 AND scope_kind = 1 AND state = 1 \
            AND ownership_generation = $2::text::numeric(20,0) \
            AND holder_node_id = encode($3,'hex')::uuid \
            AND holder_registration_revision = $4::text::numeric(20,0) FOR SHARE",
    )
    .bind(scope_key)
    .bind(generation.to_string())
    .bind(fact.node_id().as_bytes().as_slice())
    .bind(fact.registration_revision().to_string())
    .fetch_optional(&mut **tx)
    .await?
    .is_some();
    Ok(held && prove_current_incarnation(tx, node).await?)
}

/// The house scope is ASSIGNED at `generation` (to the proving node when `node` is given).
/// Holds the assignment row `FOR SHARE` until the transaction ends.
async fn house_assigned(
    tx: &mut Transaction<'_, Postgres>,
    house: &HouseId,
    generation: u64,
    node: Option<&NodeIncarnationProof>,
) -> std::result::Result<bool, DurabilityError> {
    let row = sqlx::query(
        "SELECT uuid_send(holder_node_id) AS node, holder_registration_revision::text AS revision \
           FROM game_runtime_scope_assignments \
          WHERE scope_key = $1 AND scope_kind = 2 AND world_id = encode($2,'hex')::uuid \
            AND house_key = $3 AND state = 1 AND ownership_generation = $4::text::numeric(20,0) \
          FOR SHARE",
    )
    .bind(house.scope_key().as_slice())
    .bind(house.world_id.as_bytes().as_slice())
    .bind(house.house_key.as_str())
    .bind(generation.to_string())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else {
        return Ok(false);
    };
    let Some(node) = node else {
        return Ok(true);
    };
    let fact = node.fact();
    let holder: Vec<u8> = row.try_get("node")?;
    let revision: String = row.try_get("revision")?;
    Ok(holder.as_slice() == fact.node_id().as_bytes()
        && parse_u64(&revision)? == fact.registration_revision()
        && prove_current_incarnation(tx, node).await?)
}

/// The immutable prepared binding of a handoff, for idempotent replay.
fn prepared_binding(request: &HouseEntryRequest) -> Vec<u8> {
    let mut binding = Vec::with_capacity(256);
    binding.extend_from_slice(request.character_id.as_bytes());
    binding.extend_from_slice(request.source_game_session_id.as_bytes());
    for value in [
        request.source_connection_generation,
        request.source_character_lease_generation,
        request.source_scope_ownership_generation,
        request.house_scope_ownership_generation,
        request.acl_revision,
    ] {
        binding.extend_from_slice(&value.to_be_bytes());
    }
    if let RuntimeScopeRefV1::Channel { channel_id, .. } = request.source_scope {
        binding.extend_from_slice(channel_id.as_bytes());
    }
    binding.extend_from_slice(&request.house.scope_key());
    binding.push(0);
    binding.extend_from_slice(&request.reserved_tile);
    binding.extend_from_slice(&request.prepared_at.to_be_bytes());
    binding
}

async fn load_handoff(
    tx: &mut Transaction<'_, Postgres>,
    handoff_id: [u8; 16],
    lock: bool,
) -> std::result::Result<Option<(HouseHandoffRecord, Vec<u8>)>, DurabilityError> {
    let sql = format!(
        "SELECT uuid_send(character_id) AS character, uuid_send(world_id) AS world, \
                uuid_send(origin_channel_id) AS origin, \
                uuid_send(source_game_session_id) AS source, \
                source_connection_generation::text AS source_generation, \
                source_character_lease_generation::text AS source_lease, \
                source_scope_ownership_generation::text AS source_scope, house_key, \
                destination_scope_ownership_generation::text AS house_generation, \
                acl_revision::text AS acl_revision, reserved_tile, state, prepared_at, \
                guild_revisions::text AS guild_revisions, \
                uuid_send(destination_game_session_id) AS destination \
           FROM game_house_scope_handoffs WHERE handoff_id = encode($1,'hex')::uuid{}",
        if lock { " FOR UPDATE" } else { "" }
    );
    let Some(row) = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(handoff_id.as_slice())
        .fetch_optional(&mut **tx)
        .await?
    else {
        return Ok(None);
    };
    let id16 = |bytes: Vec<u8>| {
        <[u8; 16]>::try_from(bytes).map_err(|_| DurabilityError::InvalidStoredState)
    };
    let world = WorldId::decode(&row.try_get::<Vec<u8>, _>("world")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let house_key: String = row.try_get("house_key")?;
    let house = HouseId::new(world, &house_key).ok_or(DurabilityError::InvalidStoredState)?;
    let character = CharacterId::decode(&row.try_get::<Vec<u8>, _>("character")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let source = GameSessionId::decode(&row.try_get::<Vec<u8>, _>("source")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let destination = row
        .try_get::<Option<Vec<u8>>, _>("destination")?
        .map(|bytes| GameSessionId::decode(&bytes).map_err(|_| DurabilityError::InvalidStoredState))
        .transpose()?;
    let state = match row.try_get::<i16, _>("state")? {
        HANDOFF_PREPARED => HouseHandoffState::Prepared,
        HANDOFF_COMMITTED => HouseHandoffState::Committed,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let reserved_tile: Vec<u8> = row.try_get("reserved_tile")?;
    let guild_revisions = row
        .try_get::<Option<String>, _>("guild_revisions")?
        .map(|text| {
            serde_json::from_str::<serde_json::Value>(&text)
                .map_err(|_| DurabilityError::InvalidStoredState)
        })
        .transpose()?;
    let mut binding = Vec::with_capacity(256);
    binding.extend_from_slice(character.as_bytes());
    binding.extend_from_slice(source.as_bytes());
    for column in [
        "source_generation",
        "source_lease",
        "source_scope",
        "house_generation",
        "acl_revision",
    ] {
        binding.extend_from_slice(&parse_u64(&row.try_get::<String, _>(column)?)?.to_be_bytes());
    }
    binding.extend_from_slice(&id16(row.try_get("origin")?)?);
    binding.extend_from_slice(&house.scope_key());
    binding.push(0);
    binding.extend_from_slice(&reserved_tile);
    binding.extend_from_slice(&row.try_get::<i64, _>("prepared_at")?.to_be_bytes());
    Ok(Some((
        HouseHandoffRecord {
            handoff_id,
            character_id: character,
            source_game_session_id: source,
            house,
            guild_revisions,
            reserved_tile,
            state,
            destination_game_session_id: destination,
        },
        binding,
    )))
}

fn is_uuid_v7(bytes: &[u8; 16]) -> bool {
    bytes[6] >> 4 == 7 && bytes[8] >> 6 == 2
}

fn parse_u64(text: &str) -> std::result::Result<u64, DurabilityError> {
    text.parse::<u64>()
        .map_err(|_| DurabilityError::InvalidStoredState)
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    fn world() -> std::result::Result<WorldId, Box<dyn std::error::Error>> {
        WorldId::decode(&[42, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 42])
            .map_err(|error| format!("{error:?}").into())
    }

    #[test]
    fn house_ids_follow_the_content_key_grammar() -> TestResult {
        let world = world()?;
        assert!(HouseId::new(world, "oteryn:content.house.thais_1").is_some());
        assert!(HouseId::new(world, "oteryn:content.house.").is_none());
        assert!(HouseId::new(world, "oteryn:content.house.Thais").is_none());
        assert!(HouseId::new(world, "oteryn:content.npc.thais").is_none());
        let long = format!("{HOUSE_KEY_PREFIX}{}", "a".repeat(HOUSE_KEY_MAX_BYTES));
        assert!(HouseId::new(world, &long).is_none());
        Ok(())
    }

    #[test]
    fn house_scope_identity_is_tagged_and_v7_shaped() -> TestResult {
        let world = world()?;
        let house = HouseId::new(world, "oteryn:content.house.thais_1").ok_or("house")?;
        let key = house.scope_key();
        assert_eq!(key[0], 0x02);
        assert_eq!(&key[1..17], world.as_bytes());
        let instance = house.instance_id();
        assert!(is_uuid_v7(&instance));
        assert_eq!(instance, house.instance_id());
        let other = HouseId::new(world, "oteryn:content.house.thais_2").ok_or("house")?;
        assert_ne!(instance, other.instance_id());
        assert!(house.runtime_scope().is_ok());
        Ok(())
    }

    /// Path-loaded test targets compile this module without a caller.
    #[test]
    fn house_scope_handoff_api_is_linked() {
        let _ = DurabilityRoot::assign_house_scope;
        let _ = DurabilityRoot::prepare_house_entry;
        let _ = DurabilityRoot::commit_house_entry;
        let _ = DurabilityRoot::abort_house_entry;
        let _ = DurabilityRoot::reconcile_house_entries;
        let _ = HouseId::world_id;
        let _ = HouseId::house_key;
        let _ = HouseScopeAssignment::predecessor;
        let _ = std::mem::size_of::<HouseCommitOutcome>();
        let _ = std::mem::size_of::<HouseAbortOutcome>();
        let _ = std::mem::size_of::<HouseReconcileReport>();
        assert_eq!(HOUSE_SCOPE_CHARACTERS_MAX, 200);
    }

    #[test]
    fn assignment_commands_encode_the_guarded_prefix() -> TestResult {
        let house = HouseId::new(world()?, "oteryn:content.house.thais_1").ok_or("house")?;
        let node = crate::foundation::NodeId::decode(&[
            1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 1,
        ])
        .map_err(|error| format!("{error:?}"))?;
        let target = NodeRegistrationFact::new(node, 1);
        let predecessor = HouseScopePredecessor {
            ownership_generation: 1,
            source_revision: 2,
        };
        let actor = "oteryn_control";
        let request = |command| -> std::result::Result<_, Box<dyn std::error::Error>> {
            Ok(HouseScopeAssignmentRequest {
                operation_key: OperationKey::from_bytes([7; 32]),
                actor: ControlActor::new(actor).map_err(|error| format!("{error:?}"))?,
                actor_role: actor.into(),
                command,
            })
        };
        let commands = [
            HouseScopeAssignmentCommand::Assign {
                house: house.clone(),
                target,
            },
            HouseScopeAssignmentCommand::Replace {
                house: house.clone(),
                predecessor,
                target,
            },
            HouseScopeAssignmentCommand::Revoke {
                house: house.clone(),
                predecessor,
            },
        ];
        for (kind, command) in (1_u8..).zip(commands) {
            let encoded = request(command)?
                .encode()
                .map_err(|error| format!("{error:?}"))?;
            // The receipt grant guard reads the kind, the actor length and the actor.
            assert_eq!(encoded[1], kind);
            assert_eq!(usize::from(encoded[34]), actor.len());
            assert_eq!(&encoded[35..35 + actor.len()], actor.as_bytes());
            assert_eq!(encoded[35 + actor.len()], 0x02);
        }
        let stale = HouseScopeAssignmentCommand::Revoke {
            house,
            predecessor: HouseScopePredecessor {
                ownership_generation: 0,
                source_revision: 2,
            },
        };
        assert!(matches!(
            request(stale)?.encode(),
            Err(AssignmentError::InvalidInput)
        ));
        Ok(())
    }

    #[test]
    fn only_house_access_roles_and_the_entry_direction_exist() {
        assert_eq!(HOUSE_ACCESS_ROLES, ["OWNER", "SUBOWNER", "GUEST"]);
        assert_ne!(HouseHandoffDirection::Entry, HouseHandoffDirection::Exit);
    }

    #[test]
    fn refusals_use_their_wire_names() {
        assert_eq!(HouseEntryRefusal::NoAccess.as_str(), "NO_ACCESS");
        assert_eq!(HouseEntryRefusal::InCombat.as_str(), "IN_COMBAT");
        assert_eq!(HouseEntryRefusal::Busy.as_str(), "BUSY");
        assert_eq!(HouseEntryRefusal::HouseClosed.as_str(), "HOUSE_CLOSED");
        assert_eq!(HouseEntryRefusal::NoRoom.as_str(), "NO_ROOM");
    }
}
