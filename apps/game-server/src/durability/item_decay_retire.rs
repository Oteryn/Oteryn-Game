//! DUR-03 `DECAY_RETIRE` of a decayed corpse (child D3-6; decision
//! `D3-CORPSE-CONTAINER-LOOT-WINDOW-DECAY-V1` §4.6/§4.7, D135/D136; DUR-03
//! §39.1/§39.4).
//!
//! A corpse decays at the durable absolute deadline
//! `materialized_at + 60 s` of its own `CORPSE_MATERIALIZATION` MINT receipt
//! (never `occurred_at`). Decay retires every live entry of its container and
//! then the corpse itself; nothing drops to Ground. It is N+1 separate
//! one-item logical transactions, never one multi-item transaction: one step
//! per live entry, then one step for the corpse, admitted only once no live
//! entry remains under it. Each step fits the default `DUR03-RL-01`/`-RL-06`
//! one-item shape and commits its own admitted `CorpseDecay` audit event.
//!
//! Lifecycle of one step, keyed by the retired `ItemInstanceId` (one retire
//! per item, forever — the corpse's step is thereby keyed by its unique
//! `CORPSE_MATERIALIZATION` receipt):
//! 1. [`DurabilityRoot::freeze_decay_retire`] checks the current owner fence
//!    and that the step is admissible now (deadline reached by the database
//!    clock, the item live in its corpse location, and for the corpse step
//!    no live entry left), then reserves TransactionId, EventId, the trusted
//!    timestamp, the exact event bytes and a zero DUR03-RL-08 budget under
//!    that fence's ownership generation, in its own transaction. A refusal
//!    writes nothing. A later generation (restart, handoff) reserves afresh:
//!    the former generation can no longer commit, so at most one receipt per
//!    item can ever exist.
//! 2. [`DurabilityRoot::commit_decay_retire`] re-checks the fence and the
//!    admission under the retired item's row lock and commits the item's
//!    retirement, its location removal, the receipt and the audit event
//!    together; the migration 0015 deferred guard re-proves the deadline with
//!    `clock_timestamp()` at commit, so the decay is never early.
//! 3. [`DurabilityRoot::reconcile_decay_retire`] reads the receipt after an
//!    unknown outcome.
//!
//! There is no in-memory progress state. [`DurabilityRoot::read_corpse_decay_schedule`]
//! is the Ground-only, `lifecycle = 1`-filtered recovery query: every live
//! corpse of a scope with its deadline, so a new owner reschedules exactly
//! the corpses still on Ground — a partly drained corpse looks like a fresh
//! one — and [`DurabilityRoot::retire_decayed_corpse`] issues the remaining
//! steps from durable state alone. An entry picked up before decay reaches it
//! is no longer in the corpse and is never retired a second time.

use super::character_authority::{
    ReconciledCharacterAuthority, SERVER_BUILD_ID, assert_recovery_fence,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_decay_retire_audit::{
    self as audit, DecayRetireEventIdentity, OneItemCorpseDecayV1, OneItemDecayRetireV1,
};
use super::item_mint::{CORPSE_MATERIALIZATION_PURPOSE_KEY, uuid_text};
use super::item_mint_audit::{
    self as mint_audit, AuditError, ITEM_LIFECYCLE_LIVE, OneItemGroundV1, OneItemStateV1,
    OneItemTypedDefinitionRevisionV1, RL08_RETRY_WORK_UNITS_MAX, check_uuid_v7,
};
use super::item_transfer_audit::{ITEM_LIFECYCLE_RETIRED, OneItemCorpseSourceV1};
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation, scope_key};
use super::{DurabilityError, DurabilityRoot};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};
use sha2::{Digest, Sha256};
use sqlx::Row;

type Result<T> = std::result::Result<T, DecayRetireError>;
type Pass<T> = std::result::Result<std::result::Result<T, DecayRetireError>, DurabilityError>;
const EVENT_TYPE_ID: i64 = mint_audit::EVENT_TYPE_ID as i64;
const EVENT_SCHEMA_REVISION: i64 = mint_audit::EVENT_SCHEMA_REVISION as i64;

/// D135 (`decay_at = materialized_at + 60 s`) and `COMBAT01-CORPSES-PER-SCOPE`
/// (the recovery query never returns more live corpses than one scope may
/// hold) are the owner timer's own decay-family values.
pub use crate::foundation::owner_timer::{COMBAT01_CORPSES_PER_SCOPE, CORPSE_DECAY_AFTER_MS};
/// `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX` (D3 §4.2): at most this many
/// entry steps precede one corpse step.
const CORPSE_CONTAINER_ENTRIES_MAX: usize = 16;

/// The durable decay deadline of `materialized_at`, or `None` on overflow.
#[must_use]
pub const fn corpse_decay_at_unix_ms(materialized_at_unix_ms: i64) -> Option<i64> {
    materialized_at_unix_ms.checked_add(CORPSE_DECAY_AFTER_MS)
}

/// The current owner authority of the corpse's scope, supplied by the Channel
/// owner that drains the decay timer. It may be a later generation than the
/// corpse's own (a restart or handoff resumes decay).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorpseDecayFence {
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    pub scope_ownership_generation: ScopeOwnershipGeneration,
}

/// One of the N+1 steps: `item_instance_id` is a live entry of the corpse,
/// or the corpse itself for its final step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecayRetireStep {
    pub corpse_item_instance_id: [u8; 16],
    pub item_instance_id: [u8; 16],
}

impl DecayRetireStep {
    /// The final step: the now-empty corpse itself.
    #[must_use]
    pub const fn corpse(corpse_item_instance_id: [u8; 16]) -> Self {
        Self {
            corpse_item_instance_id,
            item_instance_id: corpse_item_instance_id,
        }
    }

    /// One live entry of the corpse's container.
    #[must_use]
    pub const fn entry(corpse_item_instance_id: [u8; 16], item_instance_id: [u8; 16]) -> Self {
        Self {
            corpse_item_instance_id,
            item_instance_id,
        }
    }

    #[must_use]
    pub fn is_corpse_step(&self) -> bool {
        self.item_instance_id == self.corpse_item_instance_id
    }
}

/// One live corpse of a scope and its durable decay deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorpseDecayDeadline {
    pub corpse_item_instance_id: [u8; 16],
    pub materialized_at_unix_ms: i64,
    pub decay_at_unix_ms: i64,
}

/// The recovery query's result: every live corpse of the scope (at most
/// `COMBAT01-CORPSES-PER-SCOPE`), oldest first, with the database clock read
/// in the same statement so the owner can map each deadline onto its own
/// monotonic timer clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpseDecaySchedule {
    pub database_now_unix_ms: i64,
    pub corpses: Vec<CorpseDecayDeadline>,
}

/// Frozen step of one logical `DECAY_RETIRE`, a process-local view of its
/// durable reservation. It is deliberately not `Clone`.
#[derive(Debug)]
pub struct DecayRetireCandidate {
    step: DecayRetireStep,
    fence_generation: u64,
    transaction_id: [u8; 16],
    event_id: [u8; 16],
    occurred_at_unix_ms: i64,
    deadline_unix_ms: i64,
    envelope: Vec<u8>,
    work_units_used: u8,
}

impl DecayRetireCandidate {
    #[must_use]
    pub const fn step(&self) -> DecayRetireStep {
        self.step
    }
    #[must_use]
    pub const fn transaction_id(&self) -> &[u8; 16] {
        &self.transaction_id
    }
    #[must_use]
    pub const fn event_id(&self) -> &[u8; 16] {
        &self.event_id
    }
    #[must_use]
    pub const fn occurred_at_unix_ms(&self) -> i64 {
        self.occurred_at_unix_ms
    }
    #[must_use]
    pub const fn deadline_unix_ms(&self) -> i64 {
        self.deadline_unix_ms
    }
    /// Exact immutable EventEnvelope bytes (event type 2, operation tag 5).
    #[must_use]
    pub fn envelope(&self) -> &[u8] {
        &self.envelope
    }
    #[must_use]
    pub const fn work_units_used(&self) -> u8 {
        self.work_units_used
    }
}

/// Terminal committed result of one step. It never changes after commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedDecayRetire {
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub item_instance_id: [u8; 16],
    pub corpse_item_instance_id: [u8; 16],
    pub deadline_unix_ms: i64,
    pub occurred_at_unix_ms: i64,
    pub envelope_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecayRetireOutcome {
    Committed(CommittedDecayRetire),
    /// The item already retired; this is its original terminal result.
    AlreadyCommitted(CommittedDecayRetire),
}

impl DecayRetireOutcome {
    #[must_use]
    pub fn into_committed(self) -> CommittedDecayRetire {
        match self {
            Self::Committed(result) | Self::AlreadyCommitted(result) => result,
        }
    }
}

/// Typed refusals; each writes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecayRetireRefusal {
    /// The named corpse has no `CORPSE_MATERIALIZATION` receipt or is no
    /// longer live on Ground.
    NotACorpse,
    /// The database clock is still before `materialized_at + 60 s`.
    NotYetDue,
    /// The item is no longer a live entry of this corpse (for example it was
    /// legitimately picked up before decay reached it): never retired.
    NotInCorpse,
    /// The corpse's own step while a live entry still remains under it.
    EntriesRemain,
}

#[derive(Debug)]
pub enum DecayRetireError {
    /// Malformed input; registered `INVALID_INPUT`.
    InvalidInput,
    /// A registered DUR-03 ceiling (including RL-08) was exceeded.
    CapacityExceeded,
    /// The recovery, runtime-scope or node-incarnation fence rejected the
    /// step, or the corpse is not in the fenced scope.
    AuthorityRejected,
    Refused(DecayRetireRefusal),
    /// The item already retired for another corpse (integrity CONFLICT).
    ConflictingCause,
    /// A frozen identity is already bound elsewhere, or the reservation moved.
    ConflictingCandidate,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for DecayRetireError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl From<AuditError> for DecayRetireError {
    fn from(error: AuditError) -> Self {
        match error {
            AuditError::InvalidInput => Self::InvalidInput,
            AuditError::CapacityExceeded => Self::CapacityExceeded,
        }
    }
}

impl std::fmt::Display for DecayRetireError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("invalid DECAY_RETIRE input"),
            Self::CapacityExceeded => formatter.write_str("DECAY_RETIRE ceiling exceeded"),
            Self::AuthorityRejected => formatter.write_str("DECAY_RETIRE authority rejected"),
            Self::Refused(refusal) => write!(formatter, "DECAY_RETIRE refused: {refusal:?}"),
            Self::ConflictingCause => formatter.write_str("DECAY_RETIRE cause conflict"),
            Self::ConflictingCandidate => formatter.write_str("DECAY_RETIRE identity conflict"),
            Self::Unavailable(error) => write!(formatter, "DECAY_RETIRE unavailable: {error}"),
        }
    }
}

impl std::error::Error for DecayRetireError {}

/// The corpse's N+1 committed steps, in commit order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpseDecayReport {
    pub entries: Vec<CommittedDecayRetire>,
    pub corpse: CommittedDecayRetire,
}

/// The durable reservation row of one (item, fence generation).
struct Reservation {
    corpse_item_instance_id: [u8; 16],
    fence_generation: u64,
    transaction_id: [u8; 16],
    event_id: [u8; 16],
    occurred_at_unix_ms: i64,
    deadline_unix_ms: i64,
    quantity_before: u32,
    placement_ordinal: Option<u64>,
    envelope: Vec<u8>,
    fence_node_id: [u8; 16],
    fence_registration_revision: u64,
    work_units_used: u8,
}

/// The authoritative before-state admitting one step now.
struct Admitted {
    world_id: [u8; 16],
    channel_id: [u8; 16],
    definition: OneItemTypedDefinitionRevisionV1,
    quantity_before: u32,
    /// `None` for the corpse's own step.
    placement_ordinal: Option<u64>,
    corpse_ground: OneItemGroundV1,
    deadline_unix_ms: i64,
    database_now_unix_ms: i64,
}

impl DurabilityRoot {
    /// Validate the step and reserve its logical `DECAY_RETIRE` under the
    /// current owner fence, or resume this generation's reservation with the
    /// same identities and budget. An item that already retired resumes its
    /// committed reservation regardless of the fence, so the original result
    /// stays reachable. A refusal writes nothing; this retires nothing.
    pub async fn freeze_decay_retire(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CorpseDecayFence,
        step: DecayRetireStep,
    ) -> Result<DecayRetireCandidate> {
        check_uuid_v7(&step.corpse_item_instance_id)?;
        check_uuid_v7(&step.item_instance_id)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DecayRetireError::AuthorityRejected)?;
        let node = node.clone();
        let reservation = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_cause(&mut tx, step.item_instance_id).await?;
                    if let Some(receipt) = load_receipt(&mut tx, step.item_instance_id).await? {
                        let committed = decode_receipt(&receipt)?;
                        if committed.corpse_item_instance_id != step.corpse_item_instance_id {
                            return Ok(Err(DecayRetireError::ConflictingCause));
                        }
                        let generation: String =
                            receipt.try_get("fence_scope_ownership_generation")?;
                        let Some(row) =
                            load_reservation(&mut tx, step.item_instance_id, &generation).await?
                        else {
                            return Err(DurabilityError::InvalidStoredState);
                        };
                        let reservation = decode_reservation(&row)?;
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(reservation));
                    }
                    let generation = fence.scope_ownership_generation.get().to_string();
                    if let Some(row) =
                        load_reservation(&mut tx, step.item_instance_id, &generation).await?
                    {
                        let reservation = decode_reservation(&row)?;
                        if reservation.corpse_item_instance_id != step.corpse_item_instance_id {
                            return Ok(Err(DecayRetireError::ConflictingCause));
                        }
                        if !fence_matches(&reservation, &node)
                            || !fence_is_live(&mut tx, &fence, &node).await?
                        {
                            return Ok(Err(DecayRetireError::AuthorityRejected));
                        }
                        if let Err(error) = admit(&mut tx, &fence, step).await? {
                            return Ok(Err(error));
                        }
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(reservation));
                    }

                    if !fence_is_live(&mut tx, &fence, &node).await? {
                        return Ok(Err(DecayRetireError::AuthorityRejected));
                    }
                    let admitted = match admit(&mut tx, &fence, step).await? {
                        Ok(admitted) => admitted,
                        Err(error) => return Ok(Err(error)),
                    };
                    let row = sqlx::query(
                        "SELECT game_character_uuid_v7()::text AS transaction_id, \
                                game_character_uuid_v7()::text AS event_id",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    let transaction_id = uuid_text(row.try_get("transaction_id")?)?;
                    let event_id = uuid_text(row.try_get("event_id")?)?;
                    // The trusted timestamp is the admission's own database
                    // clock reading, already at or after the deadline.
                    let occurred_at_unix_ms = admitted.database_now_unix_ms;
                    let envelope = match audit::encode_decay_retire_event(
                        DecayRetireEventIdentity {
                            event_id,
                            transaction_id,
                            occurred_at_unix_ms,
                            server_build_id: SERVER_BUILD_ID,
                        },
                        decay_message(step, &fence, &admitted),
                    ) {
                        Ok(envelope) => envelope,
                        Err(error) => return Ok(Err(error.into())),
                    };
                    let fact = node.fact();
                    let reservation = Reservation {
                        corpse_item_instance_id: step.corpse_item_instance_id,
                        fence_generation: fence.scope_ownership_generation.get(),
                        transaction_id,
                        event_id,
                        occurred_at_unix_ms,
                        deadline_unix_ms: admitted.deadline_unix_ms,
                        quantity_before: admitted.quantity_before,
                        placement_ordinal: admitted.placement_ordinal,
                        envelope,
                        fence_node_id: *fact.node_id().as_bytes(),
                        fence_registration_revision: fact.registration_revision(),
                        work_units_used: 0,
                    };
                    insert_reservation(&mut tx, step, &admitted, &reservation).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(reservation))
                })
            })
            .await??;
        Ok(DecayRetireCandidate {
            step,
            fence_generation: reservation.fence_generation,
            transaction_id: reservation.transaction_id,
            event_id: reservation.event_id,
            occurred_at_unix_ms: reservation.occurred_at_unix_ms,
            deadline_unix_ms: reservation.deadline_unix_ms,
            envelope: reservation.envelope,
            work_units_used: reservation.work_units_used,
        })
    }

    /// Durably charge one DUR03-RL-08 work unit to the reservation in its own
    /// committed transaction before the pass does any work.
    async fn charge_decay_retire_work_unit(
        &self,
        recovery: CharacterRecoveryFenceV1,
        candidate: &mut DecayRetireCandidate,
    ) -> Result<()> {
        if candidate.work_units_used >= RL08_RETRY_WORK_UNITS_MAX {
            return Err(DecayRetireError::CapacityExceeded);
        }
        let item = candidate.step.item_instance_id;
        let generation = candidate.fence_generation.to_string();
        let transaction_id = candidate.transaction_id;
        let charged = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let used = sqlx::query(
                        "UPDATE game_item_decay_retire_reservations \
                            SET work_units_used = work_units_used + 1 \
                          WHERE item_instance_id = encode($1,'hex')::uuid \
                            AND fence_scope_ownership_generation = $2::text::numeric(20,0) \
                            AND transaction_id = encode($3,'hex')::uuid \
                            AND work_units_used < $4 RETURNING work_units_used",
                    )
                    .bind(item.as_slice())
                    .bind(&generation)
                    .bind(transaction_id.as_slice())
                    .bind(i16::from(RL08_RETRY_WORK_UNITS_MAX))
                    .fetch_optional(&mut *tx)
                    .await?;
                    let Some(used) = used else {
                        let reserved = sqlx::query(
                            "SELECT 1 FROM game_item_decay_retire_reservations \
                              WHERE item_instance_id = encode($1,'hex')::uuid \
                                AND fence_scope_ownership_generation = $2::text::numeric(20,0) \
                                AND transaction_id = encode($3,'hex')::uuid",
                        )
                        .bind(item.as_slice())
                        .bind(&generation)
                        .bind(transaction_id.as_slice())
                        .fetch_optional(&mut *tx)
                        .await?;
                        return Ok(Err(if reserved.is_some() {
                            DecayRetireError::CapacityExceeded
                        } else {
                            DecayRetireError::ConflictingCandidate
                        }));
                    };
                    let used = u8::try_from(used.try_get::<i16, _>("work_units_used")?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(used))
                })
            })
            .await?;
        match charged {
            Ok(used) => {
                candidate.work_units_used = used;
                Ok(())
            }
            Err(DecayRetireError::CapacityExceeded) => {
                candidate.work_units_used = RL08_RETRY_WORK_UNITS_MAX;
                Err(DecayRetireError::CapacityExceeded)
            }
            Err(error) => Err(error),
        }
    }

    /// Commit one frozen step. An item that already retired returns its
    /// original terminal result without reacquiring owner authority. A new
    /// retirement commits only under the reservation's live fence and with
    /// the authoritative before-state still admitting it now.
    pub async fn commit_decay_retire(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CorpseDecayFence,
        candidate: &mut DecayRetireCandidate,
    ) -> Result<DecayRetireOutcome> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DecayRetireError::AuthorityRejected)?;
        self.charge_decay_retire_work_unit(recovery.clone(), candidate)
            .await?;
        let step = candidate.step;
        let generation = candidate.fence_generation;
        let transaction_id = candidate.transaction_id;
        let event_id = candidate.event_id;
        let occurred_at_unix_ms = candidate.occurred_at_unix_ms;
        let envelope = candidate.envelope.clone();
        let node = node.clone();

        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    lock_cause(&mut tx, step.item_instance_id).await?;

                    if let Some(row) = load_receipt(&mut tx, step.item_instance_id).await? {
                        let committed = decode_receipt(&row)?;
                        if committed.corpse_item_instance_id != step.corpse_item_instance_id {
                            return Ok(Err(DecayRetireError::ConflictingCause));
                        }
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(DecayRetireOutcome::AlreadyCommitted(committed)));
                    }

                    if generation != fence.scope_ownership_generation.get() {
                        return Ok(Err(DecayRetireError::AuthorityRejected));
                    }
                    let Some(row) =
                        load_reservation(&mut tx, step.item_instance_id, &generation.to_string())
                            .await?
                    else {
                        return Ok(Err(DecayRetireError::ConflictingCandidate));
                    };
                    let reservation = decode_reservation(&row)?;
                    if reservation.transaction_id != transaction_id
                        || reservation.event_id != event_id
                        || reservation.occurred_at_unix_ms != occurred_at_unix_ms
                        || reservation.corpse_item_instance_id != step.corpse_item_instance_id
                        || reservation.envelope != envelope
                    {
                        return Ok(Err(DecayRetireError::ConflictingCandidate));
                    }
                    if identity_reused(&mut tx, transaction_id, event_id).await? {
                        return Ok(Err(DecayRetireError::ConflictingCandidate));
                    }
                    if !fence_matches(&reservation, &node)
                        || !fence_is_live(&mut tx, &fence, &node).await?
                    {
                        return Ok(Err(DecayRetireError::AuthorityRejected));
                    }
                    let admitted = match admit(&mut tx, &fence, step).await? {
                        Ok(admitted) => admitted,
                        Err(error) => return Ok(Err(error)),
                    };
                    // The frozen event bytes describe exactly this before-state.
                    if admitted.quantity_before != reservation.quantity_before
                        || admitted.placement_ordinal != reservation.placement_ordinal
                        || admitted.deadline_unix_ms != reservation.deadline_unix_ms
                    {
                        return Ok(Err(DecayRetireError::ConflictingCandidate));
                    }
                    let committed = apply_retire(&mut tx, step, &admitted, &reservation).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(DecayRetireOutcome::Committed(committed)))
                })
            })
            .await?
    }

    /// Resolve an unknown outcome. The cause lock waits for any in-flight
    /// attempt, so `None` proves nothing retired this item yet. Never
    /// reacquires owner authority.
    pub async fn reconcile_decay_retire(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        candidate: &mut DecayRetireCandidate,
    ) -> Result<Option<CommittedDecayRetire>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DecayRetireError::AuthorityRejected)?;
        self.charge_decay_retire_work_unit(recovery.clone(), candidate)
            .await?;
        let step = candidate.step;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_cause(&mut tx, step.item_instance_id).await?;
                    let Some(row) = load_receipt(&mut tx, step.item_instance_id).await? else {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(None));
                    };
                    let committed = decode_receipt(&row)?;
                    if committed.corpse_item_instance_id != step.corpse_item_instance_id {
                        return Ok(Err(DecayRetireError::ConflictingCause));
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(Some(committed)))
                })
            })
            .await?
    }

    /// D135 recovery query, run at scope (re)admission and after a corpse
    /// MINT commits: every corpse still live on Ground in this scope (live
    /// `game_item_ground_locations` joined to a `CORPSE_MATERIALIZATION`
    /// receipt, `lifecycle = 1`; never a `Container` row and never a retired
    /// corpse) with its durable deadline, oldest first. A partly drained
    /// corpse appears exactly like a fresh one. Reads only; grants nothing.
    pub async fn read_corpse_decay_schedule(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        world_id: WorldId,
        channel_id: ChannelId,
    ) -> Result<CorpseDecaySchedule> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DecayRetireError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let database_now_unix_ms: i64 = sqlx::query_scalar(
                        "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    let rows = sqlx::query(
                        "SELECT g.item_instance_id::text, r.materialized_at \
                           FROM game_item_ground_locations g \
                           JOIN game_item_mint_receipts r ON r.item_instance_id = g.item_instance_id \
                           JOIN game_item_instances i ON i.item_instance_id = g.item_instance_id \
                          WHERE g.world_id = encode($1,'hex')::uuid \
                            AND g.channel_id = encode($2,'hex')::uuid \
                            AND r.loot_purpose_key = $3 AND r.draw_ordinal = 0 \
                            AND i.lifecycle = 1 \
                          ORDER BY r.materialized_at, g.item_instance_id \
                          LIMIT $4",
                    )
                    .bind(world_id.as_bytes().as_slice())
                    .bind(channel_id.as_bytes().as_slice())
                    .bind(CORPSE_MATERIALIZATION_PURPOSE_KEY)
                    .bind(i64::try_from(COMBAT01_CORPSES_PER_SCOPE + 1).unwrap_or(i64::MAX))
                    .fetch_all(&mut *tx)
                    .await?;
                    if rows.len() > COMBAT01_CORPSES_PER_SCOPE {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let mut corpses = Vec::with_capacity(rows.len());
                    for row in &rows {
                        let materialized_at_unix_ms: Option<i64> =
                            row.try_get("materialized_at")?;
                        let materialized_at_unix_ms =
                            materialized_at_unix_ms.ok_or(DurabilityError::InvalidStoredState)?;
                        corpses.push(CorpseDecayDeadline {
                            corpse_item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
                            materialized_at_unix_ms,
                            decay_at_unix_ms: corpse_decay_at_unix_ms(materialized_at_unix_ms)
                                .ok_or(DurabilityError::InvalidStoredState)?,
                        });
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(CorpseDecaySchedule {
                        database_now_unix_ms,
                        corpses,
                    }))
                })
            })
            .await?
    }

    /// Drain one decayed corpse from durable state alone: one step per entry
    /// still live in its container (in ordinal order, each its own freeze and
    /// commit), then the corpse's own step. An entry that is no longer in the
    /// corpse by the time its step runs (picked up) is skipped, never retired.
    /// Resuming after a partial drain simply finds fewer live entries.
    pub async fn retire_decayed_corpse(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CorpseDecayFence,
        corpse_item_instance_id: [u8; 16],
    ) -> Result<CorpseDecayReport> {
        check_uuid_v7(&corpse_item_instance_id)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DecayRetireError::AuthorityRejected)?;
        let live_entries = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let rows = sqlx::query(
                        "SELECT item_instance_id::text FROM game_item_corpse_container_entries \
                          WHERE parent_item_instance_id = encode($1,'hex')::uuid \
                          ORDER BY placement_ordinal LIMIT $2",
                    )
                    .bind(corpse_item_instance_id.as_slice())
                    .bind(i64::try_from(CORPSE_CONTAINER_ENTRIES_MAX + 1).unwrap_or(i64::MAX))
                    .fetch_all(&mut *tx)
                    .await?;
                    if rows.len() > CORPSE_CONTAINER_ENTRIES_MAX {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let entries = rows
                        .iter()
                        .map(|row| uuid_text(row.try_get("item_instance_id")?))
                        .collect::<std::result::Result<Vec<_>, DurabilityError>>()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok::<_, DecayRetireError>(entries))
                })
            })
            .await??;
        let mut entries = Vec::with_capacity(live_entries.len());
        for item in live_entries {
            let step = DecayRetireStep::entry(corpse_item_instance_id, item);
            match self.retire_step(authority, node, fence, step).await {
                Ok(committed) => entries.push(committed),
                Err(DecayRetireError::Refused(DecayRetireRefusal::NotInCorpse)) => {}
                Err(error) => return Err(error),
            }
        }
        let corpse = self
            .retire_step(
                authority,
                node,
                fence,
                DecayRetireStep::corpse(corpse_item_instance_id),
            )
            .await?;
        Ok(CorpseDecayReport { entries, corpse })
    }

    async fn retire_step(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CorpseDecayFence,
        step: DecayRetireStep,
    ) -> Result<CommittedDecayRetire> {
        let mut candidate = self
            .freeze_decay_retire(authority, node, fence, step)
            .await?;
        Ok(self
            .commit_decay_retire(authority, node, fence, &mut candidate)
            .await?
            .into_committed())
    }
}

fn decay_message(
    step: DecayRetireStep,
    fence: &CorpseDecayFence,
    admitted: &Admitted,
) -> OneItemDecayRetireV1 {
    let state = |quantity, lifecycle| OneItemStateV1 {
        item_instance_id: step.item_instance_id.to_vec(),
        world_id: admitted.world_id.to_vec(),
        definition: Some(admitted.definition.clone()),
        quantity,
        lifecycle,
    };
    let entry = admitted
        .placement_ordinal
        .map(|placement_ordinal| OneItemCorpseSourceV1 {
            corpse_item_instance_id: step.corpse_item_instance_id.to_vec(),
            placement_ordinal,
            corpse_ground: Some(admitted.corpse_ground.clone()),
        });
    OneItemDecayRetireV1 {
        before: Some(state(admitted.quantity_before, ITEM_LIFECYCLE_LIVE)),
        after: Some(state(0, ITEM_LIFECYCLE_RETIRED)),
        ground: entry.is_none().then(|| admitted.corpse_ground.clone()),
        corpse_entry: entry,
        cause: Some(OneItemCorpseDecayV1 {
            corpse_item_instance_id: step.corpse_item_instance_id.to_vec(),
            deadline_unix_ms: u64::try_from(admitted.deadline_unix_ms).unwrap_or(0),
        }),
        runtime_scope_ownership_generation: fence.scope_ownership_generation.get(),
    }
}

/// Serializes attempts and reconciliation of one item's retirement. The
/// receipt's primary key, not this lock, is the uniqueness authority.
async fn lock_cause(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    item_instance_id: [u8; 16],
) -> std::result::Result<(), DurabilityError> {
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
         'oteryn:item-decay:' || encode($1, 'hex'), 0))",
    )
    .bind(item_instance_id.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// The reservation's fence names exactly this node incarnation.
fn fence_matches(reservation: &Reservation, node: &NodeIncarnationProof) -> bool {
    let fact = node.fact();
    reservation.fence_node_id == *fact.node_id().as_bytes()
        && reservation.fence_registration_revision == fact.registration_revision()
}

/// The fenced generation is the current assignment of the fenced scope, held
/// by `node`, whose incarnation is current.
async fn fence_is_live(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CorpseDecayFence,
    node: &NodeIncarnationProof,
) -> std::result::Result<bool, DurabilityError> {
    let fact = node.fact();
    let assignment = sqlx::query(
        "SELECT 1 FROM game_runtime_scope_assignments \
         WHERE scope_key = $1 AND world_id = encode($2,'hex')::uuid \
           AND channel_id = encode($3,'hex')::uuid AND state = 1 \
           AND ownership_generation = $4::text::numeric(20,0) \
           AND holder_node_id = encode($5,'hex')::uuid \
           AND holder_registration_revision = $6::text::numeric(20,0) \
         FOR SHARE",
    )
    .bind(scope_key(fence.world_id, fence.channel_id).as_slice())
    .bind(fence.world_id.as_bytes().as_slice())
    .bind(fence.channel_id.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .bind(fact.node_id().as_bytes().as_slice())
    .bind(fact.registration_revision().to_string())
    .fetch_optional(&mut **tx)
    .await?;
    Ok(assignment.is_some() && prove_current_incarnation(tx, node).await?)
}

/// The authoritative before-state admitting `step` now: the retired item is
/// locked first in its own statement (so a concurrent pickup's committed
/// removal is seen by the reads after it), the corpse is live on Ground in
/// the fenced scope, the database clock has reached its deadline, the item
/// is a live entry of that corpse (an entry's step) or no live entry remains
/// (the corpse's step). Writes nothing.
async fn admit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CorpseDecayFence,
    step: DecayRetireStep,
) -> Pass<Admitted> {
    let item = sqlx::query(
        "SELECT world_id::text, definition_family, definition_production_key, \
                definition_revision_ref, quantity \
           FROM game_item_instances \
          WHERE item_instance_id = encode($1,'hex')::uuid AND lifecycle = 1 FOR UPDATE",
    )
    .bind(step.item_instance_id.as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(item) = item else {
        return Ok(Err(DecayRetireError::Refused(if step.is_corpse_step() {
            DecayRetireRefusal::NotACorpse
        } else {
            DecayRetireRefusal::NotInCorpse
        })));
    };
    let corpse = sqlx::query(
        "SELECT r.materialized_at, g.world_id::text, g.channel_id::text, \
                g.runtime_scope_ownership_generation::text, g.spatial_position, g.corpse_ref, \
                g.map_revision, g.content_revision, g.native_room_placement_context, \
                floor(extract(epoch FROM clock_timestamp())*1000)::bigint AS database_now \
           FROM game_item_mint_receipts r \
           JOIN game_item_ground_locations g ON g.item_instance_id = r.item_instance_id \
           JOIN game_item_instances ci ON ci.item_instance_id = r.item_instance_id \
          WHERE r.item_instance_id = encode($1,'hex')::uuid \
            AND r.loot_purpose_key = $2 AND r.draw_ordinal = 0 AND ci.lifecycle = 1",
    )
    .bind(step.corpse_item_instance_id.as_slice())
    .bind(CORPSE_MATERIALIZATION_PURPOSE_KEY)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(corpse) = corpse else {
        return Ok(Err(DecayRetireError::Refused(
            DecayRetireRefusal::NotACorpse,
        )));
    };
    let world_id = uuid_text(corpse.try_get("world_id")?)?;
    let channel_id = uuid_text(corpse.try_get("channel_id")?)?;
    if world_id != *fence.world_id.as_bytes() || channel_id != *fence.channel_id.as_bytes() {
        return Ok(Err(DecayRetireError::AuthorityRejected));
    }
    if uuid_text(item.try_get("world_id")?)? != world_id {
        return Err(DurabilityError::InvalidStoredState);
    }
    let materialized_at: Option<i64> = corpse.try_get("materialized_at")?;
    let deadline_unix_ms = materialized_at
        .and_then(corpse_decay_at_unix_ms)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let database_now_unix_ms: i64 = corpse.try_get("database_now")?;
    if database_now_unix_ms < deadline_unix_ms {
        return Ok(Err(DecayRetireError::Refused(
            DecayRetireRefusal::NotYetDue,
        )));
    }
    let placement_ordinal = if step.is_corpse_step() {
        let remaining: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_corpse_container_entries \
              WHERE parent_item_instance_id = encode($1,'hex')::uuid",
        )
        .bind(step.corpse_item_instance_id.as_slice())
        .fetch_one(&mut **tx)
        .await?;
        if remaining != 0 {
            return Ok(Err(DecayRetireError::Refused(
                DecayRetireRefusal::EntriesRemain,
            )));
        }
        None
    } else {
        let ordinal: Option<String> = sqlx::query_scalar(
            "SELECT placement_ordinal::text FROM game_item_corpse_container_entries \
              WHERE item_instance_id = encode($1,'hex')::uuid \
                AND parent_item_instance_id = encode($2,'hex')::uuid",
        )
        .bind(step.item_instance_id.as_slice())
        .bind(step.corpse_item_instance_id.as_slice())
        .fetch_optional(&mut **tx)
        .await?;
        let Some(ordinal) = ordinal else {
            return Ok(Err(DecayRetireError::Refused(
                DecayRetireRefusal::NotInCorpse,
            )));
        };
        Some(
            ordinal
                .parse()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        )
    };
    let quantity = u32::try_from(item.try_get::<i64, _>("quantity")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(Ok(Admitted {
        world_id,
        channel_id,
        definition: OneItemTypedDefinitionRevisionV1 {
            family: item.try_get("definition_family")?,
            production_key: item.try_get("definition_production_key")?,
            revision_ref: item.try_get("definition_revision_ref")?,
        },
        quantity_before: quantity,
        placement_ordinal,
        corpse_ground: OneItemGroundV1 {
            world_id: world_id.to_vec(),
            channel_id: channel_id.to_vec(),
            spatial_position: corpse.try_get("spatial_position")?,
            corpse_ref: corpse.try_get("corpse_ref")?,
            map_revision: corpse.try_get("map_revision")?,
            content_revision: corpse.try_get("content_revision")?,
            native_room_placement_context: corpse.try_get("native_room_placement_context")?,
            runtime_scope_ownership_generation: corpse
                .try_get::<String, _>("runtime_scope_ownership_generation")?
                .parse()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        },
        deadline_unix_ms,
        database_now_unix_ms,
    }))
}

/// Remove the one location, retire the item, and write the receipt and the
/// audit event, all in the caller's transaction.
async fn apply_retire(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    step: DecayRetireStep,
    admitted: &Admitted,
    reservation: &Reservation,
) -> std::result::Result<CommittedDecayRetire, DurabilityError> {
    let item = step.item_instance_id.as_slice();
    let removed = match admitted.placement_ordinal {
        None => {
            sqlx::query(
                "DELETE FROM game_item_ground_locations \
                  WHERE item_instance_id = encode($1,'hex')::uuid",
            )
            .bind(item)
            .execute(&mut **tx)
            .await?
        }
        Some(ordinal) => {
            sqlx::query(
                "DELETE FROM game_item_corpse_container_entries \
                  WHERE item_instance_id = encode($1,'hex')::uuid \
                    AND parent_item_instance_id = encode($2,'hex')::uuid \
                    AND placement_ordinal = $3::text::numeric(20,0)",
            )
            .bind(item)
            .bind(step.corpse_item_instance_id.as_slice())
            .bind(ordinal.to_string())
            .execute(&mut **tx)
            .await?
        }
    };
    if removed.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let retired = sqlx::query(
        "UPDATE game_item_instances SET quantity = 0, lifecycle = $2, \
                last_transaction_id = encode($3,'hex')::uuid \
          WHERE item_instance_id = encode($1,'hex')::uuid AND lifecycle = 1 AND quantity = $4",
    )
    .bind(item)
    .bind(i16::try_from(ITEM_LIFECYCLE_RETIRED).map_err(|_| DurabilityError::InvalidStoredState)?)
    .bind(reservation.transaction_id.as_slice())
    .bind(i64::from(admitted.quantity_before))
    .execute(&mut **tx)
    .await?;
    if retired.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let envelope_sha256: [u8; 32] = Sha256::digest(&reservation.envelope).into();
    sqlx::query(
        "INSERT INTO game_item_decay_retire_receipts(item_instance_id, corpse_item_instance_id, \
           world_id, channel_id, fence_scope_ownership_generation, transaction_id, event_id, \
           quantity_before, placement_ordinal, deadline, occurred_at, envelope_sha256, \
           committed_at) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
           encode($4,'hex')::uuid, $5::text::numeric(20,0), encode($6,'hex')::uuid, \
           encode($7,'hex')::uuid, $8, $9::text::numeric(20,0), $10, $11, $12, \
           floor(extract(epoch FROM statement_timestamp())*1000)::bigint)",
    )
    .bind(item)
    .bind(step.corpse_item_instance_id.as_slice())
    .bind(admitted.world_id.as_slice())
    .bind(admitted.channel_id.as_slice())
    .bind(reservation.fence_generation.to_string())
    .bind(reservation.transaction_id.as_slice())
    .bind(reservation.event_id.as_slice())
    .bind(i64::from(admitted.quantity_before))
    .bind(
        admitted
            .placement_ordinal
            .map(|ordinal| ordinal.to_string()),
    )
    .bind(admitted.deadline_unix_ms)
    .bind(reservation.occurred_at_unix_ms)
    .bind(envelope_sha256.as_slice())
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_item_audit_outbox(event_id, transaction_id, transaction_ordinal, \
           transaction_count, event_type_id, schema_revision, retention_profile_id, \
           item_instance_id, occurred_at, expires_at, envelope, envelope_sha256, \
           publication_state) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, 1, $3, $4, $5, \
           encode($6,'hex')::uuid, $7, $7 + $8, $9, sha256($9), 1)",
    )
    .bind(reservation.event_id.as_slice())
    .bind(reservation.transaction_id.as_slice())
    .bind(EVENT_TYPE_ID)
    .bind(EVENT_SCHEMA_REVISION)
    .bind(mint_audit::RETENTION_PROFILE_ID)
    .bind(item)
    .bind(reservation.occurred_at_unix_ms)
    .bind(mint_audit::AUDIT_RETENTION_P90D_MS)
    .bind(reservation.envelope.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(CommittedDecayRetire {
        transaction_id: reservation.transaction_id,
        event_id: reservation.event_id,
        item_instance_id: step.item_instance_id,
        corpse_item_instance_id: step.corpse_item_instance_id,
        deadline_unix_ms: admitted.deadline_unix_ms,
        occurred_at_unix_ms: reservation.occurred_at_unix_ms,
        envelope_sha256,
    })
}

async fn load_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    item_instance_id: [u8; 16],
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "SELECT item_instance_id::text, corpse_item_instance_id::text, \
                fence_scope_ownership_generation::text, transaction_id::text, \
                event_id::text, deadline, occurred_at, envelope_sha256 \
           FROM game_item_decay_retire_receipts \
          WHERE item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(item_instance_id.as_slice())
    .fetch_optional(&mut **tx)
    .await?)
}

fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CommittedDecayRetire, DurabilityError> {
    let digest: Vec<u8> = row.try_get("envelope_sha256")?;
    Ok(CommittedDecayRetire {
        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
        event_id: uuid_text(row.try_get("event_id")?)?,
        item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
        corpse_item_instance_id: uuid_text(row.try_get("corpse_item_instance_id")?)?,
        deadline_unix_ms: row.try_get("deadline")?,
        occurred_at_unix_ms: row.try_get("occurred_at")?,
        envelope_sha256: digest
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    })
}

async fn load_reservation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    item_instance_id: [u8; 16],
    generation: &str,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "SELECT corpse_item_instance_id::text, fence_scope_ownership_generation::text, \
                transaction_id::text, event_id::text, occurred_at, deadline, quantity_before, \
                placement_ordinal::text, envelope, fence_holder_node_id::text, \
                fence_holder_registration_revision::text, work_units_used \
           FROM game_item_decay_retire_reservations \
          WHERE item_instance_id = encode($1,'hex')::uuid \
            AND fence_scope_ownership_generation = $2::text::numeric(20,0)",
    )
    .bind(item_instance_id.as_slice())
    .bind(generation)
    .fetch_optional(&mut **tx)
    .await?)
}

fn decode_reservation(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<Reservation, DurabilityError> {
    let invalid = |_| DurabilityError::InvalidStoredState;
    let ordinal: Option<String> = row.try_get("placement_ordinal")?;
    Ok(Reservation {
        corpse_item_instance_id: uuid_text(row.try_get("corpse_item_instance_id")?)?,
        fence_generation: row
            .try_get::<String, _>("fence_scope_ownership_generation")?
            .parse()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
        event_id: uuid_text(row.try_get("event_id")?)?,
        occurred_at_unix_ms: row.try_get("occurred_at")?,
        deadline_unix_ms: row.try_get("deadline")?,
        quantity_before: u32::try_from(row.try_get::<i64, _>("quantity_before")?)
            .map_err(invalid)?,
        placement_ordinal: ordinal
            .map(|value| value.parse())
            .transpose()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        envelope: row.try_get("envelope")?,
        fence_node_id: uuid_text(row.try_get("fence_holder_node_id")?)?,
        fence_registration_revision: row
            .try_get::<String, _>("fence_holder_registration_revision")?
            .parse()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        work_units_used: u8::try_from(row.try_get::<i16, _>("work_units_used")?)
            .map_err(invalid)?,
    })
}

async fn insert_reservation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    step: DecayRetireStep,
    admitted: &Admitted,
    reservation: &Reservation,
) -> std::result::Result<(), DurabilityError> {
    sqlx::query(
        "INSERT INTO game_item_decay_retire_reservations(item_instance_id, \
           fence_scope_ownership_generation, corpse_item_instance_id, world_id, channel_id, \
           transaction_id, event_id, quantity_before, placement_ordinal, deadline, occurred_at, \
           envelope, fence_holder_node_id, fence_holder_registration_revision, \
           work_units_used, reserved_at) \
         VALUES (encode($1,'hex')::uuid, $2::text::numeric(20,0), encode($3,'hex')::uuid, \
           encode($4,'hex')::uuid, encode($5,'hex')::uuid, encode($6,'hex')::uuid, \
           encode($7,'hex')::uuid, $8, $9::text::numeric(20,0), $10, $11, $12, \
           encode($13,'hex')::uuid, $14::text::numeric(20,0), 0, \
           floor(extract(epoch FROM statement_timestamp())*1000)::bigint)",
    )
    .bind(step.item_instance_id.as_slice())
    .bind(reservation.fence_generation.to_string())
    .bind(step.corpse_item_instance_id.as_slice())
    .bind(admitted.world_id.as_slice())
    .bind(admitted.channel_id.as_slice())
    .bind(reservation.transaction_id.as_slice())
    .bind(reservation.event_id.as_slice())
    .bind(i64::from(reservation.quantity_before))
    .bind(
        reservation
            .placement_ordinal
            .map(|ordinal| ordinal.to_string()),
    )
    .bind(reservation.deadline_unix_ms)
    .bind(reservation.occurred_at_unix_ms)
    .bind(reservation.envelope.as_slice())
    .bind(reservation.fence_node_id.as_slice())
    .bind(reservation.fence_registration_revision.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// The frozen identities already belong to a committed item transaction or
/// audit event: a different logical transaction must never reuse them.
async fn identity_reused(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    transaction_id: [u8; 16],
    event_id: [u8; 16],
) -> std::result::Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_item_decay_retire_receipts \
                         WHERE transaction_id = encode($1,'hex')::uuid \
                            OR event_id = encode($2,'hex')::uuid) \
             OR EXISTS (SELECT 1 FROM game_item_mint_receipts \
                         WHERE transaction_id = encode($1,'hex')::uuid \
                            OR event_id = encode($2,'hex')::uuid) \
             OR EXISTS (SELECT 1 FROM game_item_transfer_receipts \
                         WHERE transaction_id = encode($1,'hex')::uuid \
                            OR event_id = encode($2,'hex')::uuid) \
             OR EXISTS (SELECT 1 FROM game_item_audit_outbox \
                         WHERE transaction_id = encode($1,'hex')::uuid \
                            OR event_id = encode($2,'hex')::uuid)",
    )
    .bind(transaction_id.as_slice())
    .bind(event_id.as_slice())
    .fetch_one(&mut **tx)
    .await?)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    fn id(seed: u8) -> [u8; 16] {
        let mut bytes = [0_u8; 16];
        bytes[5] = 1;
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = seed;
        bytes
    }

    fn admitted(ordinal: Option<u64>) -> Admitted {
        Admitted {
            world_id: id(1),
            channel_id: id(2),
            definition: OneItemTypedDefinitionRevisionV1 {
                family: "ItemType".into(),
                production_key: "fixture:corpse-loot".into(),
                revision_ref: "rev-1".into(),
            },
            quantity_before: 3,
            placement_ordinal: ordinal,
            corpse_ground: OneItemGroundV1 {
                world_id: id(1).to_vec(),
                channel_id: id(2).to_vec(),
                spatial_position: vec![9, 9, 0],
                corpse_ref: id(3).to_vec(),
                map_revision: "map-1".into(),
                content_revision: "content-1".into(),
                native_room_placement_context: id(6).to_vec(),
                runtime_scope_ownership_generation: 1,
            },
            deadline_unix_ms: 1_790_000_060_000,
            database_now_unix_ms: 1_790_000_060_000,
        }
    }

    fn fence() -> CorpseDecayFence {
        CorpseDecayFence {
            world_id: WorldId::decode(&id(1)).unwrap(),
            channel_id: ChannelId::decode(&id(2)).unwrap(),
            scope_ownership_generation: ScopeOwnershipGeneration::new(2).unwrap(),
        }
    }

    #[test]
    fn the_deadline_is_materialized_at_plus_sixty_seconds() {
        assert_eq!(corpse_decay_at_unix_ms(0), Some(60_000));
        assert_eq!(
            corpse_decay_at_unix_ms(1_790_000_000_000),
            Some(1_790_000_060_000)
        );
        assert_eq!(corpse_decay_at_unix_ms(i64::MAX), None);
        assert_eq!(
            u64::try_from(CORPSE_DECAY_AFTER_MS).unwrap(),
            audit::CORPSE_DECAY_AFTER_MS
        );
    }

    #[test]
    fn step_kinds_are_the_corpse_itself_or_one_of_its_entries() {
        assert!(DecayRetireStep::corpse(id(60)).is_corpse_step());
        assert!(!DecayRetireStep::entry(id(60), id(9)).is_corpse_step());
    }

    #[test]
    fn each_step_message_is_one_admitted_item_with_its_own_location() {
        let entry = decay_message(
            DecayRetireStep::entry(id(60), id(9)),
            &fence(),
            &admitted(Some(4)),
        );
        assert_eq!(
            audit::check_decay_retire(&entry),
            Ok(audit::DecayRetireShape::Entry)
        );
        assert!(entry.ground.is_none());
        assert_eq!(entry.corpse_entry.as_ref().unwrap().placement_ordinal, 4);
        assert_eq!(entry.runtime_scope_ownership_generation, 2);
        let corpse = decay_message(DecayRetireStep::corpse(id(60)), &fence(), &admitted(None));
        assert_eq!(
            audit::check_decay_retire(&corpse),
            Ok(audit::DecayRetireShape::Corpse)
        );
        assert!(corpse.corpse_entry.is_none());
        for message in [entry, corpse] {
            let cause = message.cause.as_ref().unwrap();
            assert_eq!(cause.corpse_item_instance_id, id(60).to_vec());
            assert_eq!(cause.deadline_unix_ms, 1_790_000_060_000);
            assert_eq!(message.after.as_ref().unwrap().quantity, 0);
            assert_eq!(
                message.after.as_ref().unwrap().lifecycle,
                ITEM_LIFECYCLE_RETIRED
            );
        }
        // An entry step that claims to be the corpse's own is not admissible.
        let forged = decay_message(
            DecayRetireStep::entry(id(60), id(9)),
            &fence(),
            &admitted(None),
        );
        assert!(audit::check_decay_retire(&forged).is_err());
    }

    #[test]
    fn outcomes_expose_their_terminal_result() {
        let committed = CommittedDecayRetire {
            transaction_id: id(40),
            event_id: id(30),
            item_instance_id: id(9),
            corpse_item_instance_id: id(60),
            deadline_unix_ms: 60_000,
            occurred_at_unix_ms: 60_000,
            envelope_sha256: [0; 32],
        };
        assert_eq!(
            DecayRetireOutcome::AlreadyCommitted(committed.clone()).into_committed(),
            committed
        );
    }
}
