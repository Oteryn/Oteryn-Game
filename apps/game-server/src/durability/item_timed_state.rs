//! Timed-item rows and their write records (TIMED-RT-1a, migration 0054; decision
//! `TIMEDITEM0B-RUNTIME-CHARGES-AND-DURATION-V1` §4, §5.2, §6.3 and §12, TIMED-ITEM-0 §4).
//!
//! A hosting runtime commits a live item's checkpoint with
//! [`DurabilityRoot::commit_timed_checkpoint`] when the lane of `domain::timed_item` issues one.
//! It is one one-item transaction under the holder's current gameplay fence (the R7 P03 XP
//! writer's own: recovery fence, admission relation locks, the live FND-04 session, lease and
//! scope, the scope assignment held by the current node incarnation, and the locked live root at
//! the expected CharacterRevision), so a stale runtime writes nothing. The write is keyed by
//! (item, expected revision): it inserts the write record, then inserts the row at revision 1 or
//! moves it up by one. It does not advance the CharacterRevision. Lock order (§6.3): the fences
//! and `character_root`, the item's lifecycle row, the timed row. A replay of a committed key
//! returns its recorded result; a key committed by another write writes nothing.
//! [`find_timed_write`] is the lane's lookup of an ambiguous write (§5.2).
//!
//! [`DurabilityRoot::commit_timed_expiry`] (TIMED-RT-1b, migration 0058; §8, §12) is the held
//! item's expiry under the same fence: an expiry TRANSFORM (PRESERVE_INSTANCE, in place: the
//! item takes its decay target's definition, the row reset to the target's full values or spent,
//! one revision up) or, without a target, an expiry burn (the item RETIRED from its container
//! entry, the row left as it was). Each commits its record, its item change and its one
//! `timed_expiry` audit event in one transaction. A checkpoint never stores 0 ms: a value
//! reaching 0 is an expiry.
//!
//! The deadline causes and PutOut are typed for their records and are written by the
//! transactions that admit them (RT-1c).

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    uuid_text,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_mint::TypedDefinitionRef;
use super::item_mint_audit::{
    self as mint_audit, ITEM_LIFECYCLE_LIVE, OneItemStateV1, check_technical_text, check_uuid_v7,
};
use super::item_timed_state_audit::{
    OneItemTimedExpiryV1, OneItemTimedRowV1, TIMED_EXPIRY_REASON_CHARGES_EXHAUSTED,
    TIMED_EXPIRY_REASON_TIME_EXHAUSTED, TimedExpiryEventIdentity, encode_timed_expiry_event,
};
use super::item_transfer::definition_message;
use super::item_transfer_audit::{ITEM_LIFECYCLE_RETIRED, OneItemContainerEntryV1};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use crate::domain::timed_item::{ExpireReason, TimedValues};
use crate::foundation::RuntimeScopeRefV1;
use sqlx::Row;
use sqlx::postgres::{PgConnection, PgRow};

/// The closed `TimedItemCause` (§12). The item and expected revision are the record's key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimedItemCause {
    Checkpoint,
    Expire { reason: TimedExpireReason },
    SetDeadline,
    ClearDeadline,
    PutOut,
}

/// `Expire {reason}` (§8, §10.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimedExpireReason {
    TimeExhausted,
    ChargesExhausted,
    Deadline,
}

impl From<ExpireReason> for TimedExpireReason {
    fn from(reason: ExpireReason) -> Self {
        match reason {
            ExpireReason::TimeExhausted => Self::TimeExhausted,
            ExpireReason::ChargesExhausted => Self::ChargesExhausted,
        }
    }
}

/// The stored cause of a record: a [`TimedItemCause`] or the repair (`npc_repair`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimedWriteCause {
    Timed(TimedItemCause),
    NpcRepair,
}

impl TimedWriteCause {
    const fn code(self) -> i16 {
        match self {
            Self::Timed(TimedItemCause::Checkpoint) => 1,
            Self::Timed(TimedItemCause::Expire {
                reason: TimedExpireReason::TimeExhausted,
            }) => 2,
            Self::Timed(TimedItemCause::Expire {
                reason: TimedExpireReason::ChargesExhausted,
            }) => 3,
            Self::Timed(TimedItemCause::Expire {
                reason: TimedExpireReason::Deadline,
            }) => 4,
            Self::Timed(TimedItemCause::SetDeadline) => 5,
            Self::Timed(TimedItemCause::ClearDeadline) => 6,
            Self::Timed(TimedItemCause::PutOut) => 7,
            Self::NpcRepair => 8,
        }
    }

    const fn from_code(code: i16) -> Option<Self> {
        Some(match code {
            1 => Self::Timed(TimedItemCause::Checkpoint),
            2 => Self::Timed(TimedItemCause::Expire {
                reason: TimedExpireReason::TimeExhausted,
            }),
            3 => Self::Timed(TimedItemCause::Expire {
                reason: TimedExpireReason::ChargesExhausted,
            }),
            4 => Self::Timed(TimedItemCause::Expire {
                reason: TimedExpireReason::Deadline,
            }),
            5 => Self::Timed(TimedItemCause::SetDeadline),
            6 => Self::Timed(TimedItemCause::ClearDeadline),
            7 => Self::Timed(TimedItemCause::PutOut),
            8 => Self::NpcRepair,
            _ => return None,
        })
    }
}

/// The writer's content facts of an admitted timed definition (for an inactive form, of its
/// paired active form). The database pins them per definition revision at the first write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedDefinitionFacts {
    pub definition: TypedDefinitionRef,
    pub full: TimedValues,
    /// A lit `continuous` form (§10).
    pub lit_continuous: bool,
}

/// One checkpoint (§6.3): the live values of `item_instance_id`, keyed by `expected_revision`
/// (0 for an item without a row). The TransactionId is fixed by the lane and reused on retry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedCheckpointRequest {
    pub item_instance_id: [u8; 16],
    pub expected_revision: u64,
    pub transaction_id: [u8; 16],
    pub definition: TimedDefinitionFacts,
    pub values: TimedValues,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimedWriteOutcome {
    /// Written in the caller's transaction at this revision; durable only when it commits.
    Written { revision: u64 },
    /// The key already committed with this TransactionId: the caller commits nothing more.
    AlreadyCommitted { revision: u64 },
}

#[derive(Debug)]
pub enum TimedWriteError {
    InvalidInput,
    /// The holder's current gameplay fence does not hold; nothing was written.
    AuthorityRejected,
    /// The holder's root is not at the fence's expected CharacterRevision.
    CharacterRevisionMismatch,
    /// The holder does not hold the item, or the item is not live at the definition.
    NotHeld,
    /// The row is at another revision (§5.2 "Unexpected revision"); nothing was written.
    RevisionMismatch {
        current: Option<StoredTimedState>,
    },
    /// The values equal the row (a checkpoint of unchanged values writes nothing) or exceed it.
    NotAStoreableChange,
    /// The key was committed by another write; nothing was written.
    ConflictingWrite,
    /// An expiry burn of an item in the equipment `container` slot or with contents (§8: the
    /// slot is immutable; a container is never burned with its entries).
    NotBurnable,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for TimedWriteError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl From<sqlx::Error> for TimedWriteError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}

impl std::fmt::Display for TimedWriteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("invalid timed write input"),
            Self::AuthorityRejected => formatter.write_str("timed write authority rejected"),
            Self::CharacterRevisionMismatch => {
                formatter.write_str("timed write CharacterRevision mismatch")
            }
            Self::NotHeld => formatter.write_str("timed item is not held live by the holder"),
            Self::RevisionMismatch { .. } => formatter.write_str("timed row revision mismatch"),
            Self::NotAStoreableChange => formatter.write_str("timed values are not a change"),
            Self::ConflictingWrite => {
                formatter.write_str("timed write key was committed by another write")
            }
            Self::NotBurnable => formatter.write_str("timed item cannot be burned where it is"),
            Self::Unavailable(error) => write!(formatter, "timed write storage failed: {error:?}"),
        }
    }
}

impl std::error::Error for TimedWriteError {}

type Result<T> = std::result::Result<T, TimedWriteError>;

/// A stored row (TIMED-ITEM-0 §4). `values` is `None` for a spent row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredTimedState {
    pub revision: u64,
    pub values: Option<TimedValues>,
    pub deadline_at: Option<i64>,
}

/// A stored write record (§4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredTimedWrite {
    pub cause: TimedWriteCause,
    pub transaction_id: [u8; 16],
    pub after: Option<TimedValues>,
    pub deadline_after: Option<i64>,
    pub committed_at: i64,
}

/// The current row of an item, `None` without one (revision 0, full values).
pub async fn load_timed_state(
    connection: &mut PgConnection,
    item_instance_id: &[u8; 16],
) -> Result<Option<StoredTimedState>> {
    let row = sqlx::query(
        "SELECT charges, remaining_ms, deadline_at, state_revision::text \
           FROM game_item_timed_states WHERE item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(item_instance_id.as_slice())
    .fetch_optional(&mut *connection)
    .await?;
    Ok(row.map(|row| decode_state(&row)).transpose()?)
}

/// The record of (item, expected revision), if that key committed (§5.2 ambiguous lookup).
pub async fn find_timed_write(
    connection: &mut PgConnection,
    item_instance_id: &[u8; 16],
    expected_revision: u64,
) -> Result<Option<StoredTimedWrite>> {
    let row = sqlx::query(
        "SELECT cause, transaction_id::text, charges_after, remaining_ms_after, deadline_after, \
                committed_at \
           FROM game_item_timed_state_writes \
          WHERE item_instance_id = encode($1,'hex')::uuid \
            AND expected_revision = $2::text::numeric(20,0)",
    )
    .bind(item_instance_id.as_slice())
    .bind(expected_revision.to_string())
    .fetch_optional(&mut *connection)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(StoredTimedWrite {
        cause: TimedWriteCause::from_code(row.try_get("cause")?)
            .ok_or(DurabilityError::InvalidStoredState)?,
        transaction_id: uuid_text(&row.try_get::<String, _>("transaction_id")?)?,
        after: decode_values(&row, "charges_after", "remaining_ms_after")?,
        deadline_after: row.try_get("deadline_after")?,
        committed_at: row.try_get("committed_at")?,
    }))
}

impl DurabilityRoot {
    /// Commit a live item's checkpoint under its holder's current gameplay fence (see the module
    /// documentation). An exact replay of a committed key returns its result without
    /// reacquiring gameplay authority; every refusal writes nothing.
    pub async fn commit_timed_checkpoint(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: TimedCheckpointRequest,
    ) -> Result<TimedWriteOutcome> {
        validate_checkpoint(&request)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| TimedWriteError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    match replay(&mut tx, &request).await {
                        Ok(Some(outcome)) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            return Ok(Ok(outcome));
                        }
                        Ok(None) => {}
                        Err(TimedWriteError::Unavailable(error)) => return Err(error),
                        Err(error) => return Ok(Err(error)),
                    }
                    match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(_) => {}
                        Err(CharacterProgressionError::CharacterRevisionMismatch) => {
                            return Ok(Err(TimedWriteError::CharacterRevisionMismatch));
                        }
                        Err(CharacterProgressionError::Unavailable(error)) => return Err(error),
                        Err(_) => return Ok(Err(TimedWriteError::AuthorityRejected)),
                    }
                    match write_checkpoint(&mut tx, fence.character_id, &request).await {
                        Ok(outcome) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(outcome))
                        }
                        Err(TimedWriteError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

fn validate_checkpoint(request: &TimedCheckpointRequest) -> Result<()> {
    let definition = &request.definition.definition;
    if check_uuid_v7(&request.item_instance_id).is_err()
        || check_uuid_v7(&request.transaction_id).is_err()
        || !(1..=128).contains(&definition.family.len())
        || !(1..=512).contains(&definition.production_key.len())
        || !(1..=512).contains(&definition.revision_ref.len())
        || request.expected_revision == u64::MAX
        || !within(request.values, request.definition.full)
        // Decision §8: a value reaching 0 ms is an expiry, never a stored checkpoint.
        || request.values.remaining_ms() == Some(0)
    {
        return Err(TimedWriteError::InvalidInput);
    }
    Ok(())
}

/// The recorded result of an exact replay, a conflict for a key committed by another write, or
/// `None` for a new key.
async fn replay(
    connection: &mut PgConnection,
    request: &TimedCheckpointRequest,
) -> Result<Option<TimedWriteOutcome>> {
    let Some(stored) = find_timed_write(
        connection,
        &request.item_instance_id,
        request.expected_revision,
    )
    .await?
    else {
        return Ok(None);
    };
    if stored.cause == TimedWriteCause::Timed(TimedItemCause::Checkpoint)
        && stored.transaction_id == request.transaction_id
        && stored.after == Some(request.values)
    {
        Ok(Some(TimedWriteOutcome::AlreadyCommitted {
            revision: request.expected_revision + 1,
        }))
    } else {
        Err(TimedWriteError::ConflictingWrite)
    }
}

/// The checkpoint body, after the fence locked `holder`'s root in this transaction.
async fn write_checkpoint(
    connection: &mut PgConnection,
    holder: CharacterId,
    request: &TimedCheckpointRequest,
) -> Result<TimedWriteOutcome> {
    let full = request.definition.full;
    let definition = &request.definition.definition;
    let cause = TimedWriteCause::Timed(TimedItemCause::Checkpoint);
    if let Some(outcome) = replay(connection, request).await? {
        return Ok(outcome);
    }
    let item = sqlx::query(
        "SELECT i.definition_family, i.definition_production_key, i.definition_revision_ref, \
                i.lifecycle, \
                (EXISTS (SELECT 1 FROM game_item_container_slots s \
                          WHERE s.item_instance_id = i.item_instance_id \
                            AND s.character_id = encode($2,'hex')::uuid) \
                 OR EXISTS (SELECT 1 FROM game_item_container_entries e \
                             WHERE e.item_instance_id = i.item_instance_id \
                               AND e.character_id = encode($2,'hex')::uuid)) AS held \
           FROM game_item_instances i \
          WHERE i.item_instance_id = encode($1,'hex')::uuid FOR UPDATE OF i",
    )
    .bind(request.item_instance_id.as_slice())
    .bind(holder.as_bytes().as_slice())
    .fetch_optional(&mut *connection)
    .await?;
    let Some(item) = item else {
        return Err(TimedWriteError::NotHeld);
    };
    if item.try_get::<i16, _>("lifecycle")? != 1
        || !item.try_get::<bool, _>("held")?
        || item.try_get::<String, _>("definition_family")? != definition.family
        || item.try_get::<String, _>("definition_production_key")? != definition.production_key
        || item.try_get::<String, _>("definition_revision_ref")? != definition.revision_ref
    {
        return Err(TimedWriteError::NotHeld);
    }

    let current = sqlx::query(
        "SELECT charges, remaining_ms, deadline_at, state_revision::text \
           FROM game_item_timed_states WHERE item_instance_id = encode($1,'hex')::uuid \
           FOR UPDATE",
    )
    .bind(request.item_instance_id.as_slice())
    .fetch_optional(&mut *connection)
    .await?
    .map(|row| decode_state(&row))
    .transpose()?;
    let (revision, before, deadline_before) = match current {
        None => (0, Some(full), None),
        Some(state) => (state.revision, state.values, state.deadline_at),
    };
    if revision != request.expected_revision {
        return Err(TimedWriteError::RevisionMismatch { current });
    }
    let Some(before) = before else {
        return Err(TimedWriteError::NotHeld);
    };
    if deadline_before.is_some() || request.values == before || !within(request.values, before) {
        return Err(TimedWriteError::NotAStoreableChange);
    }

    sqlx::query(
        "INSERT INTO game_item_timed_state_writes (\
           item_instance_id, expected_revision, cause, transaction_id, holder_character_id, \
           definition_before_family, definition_before_production_key, \
           definition_before_revision_ref, definition_after_family, \
           definition_after_production_key, definition_after_revision_ref, full_charges, \
           full_remaining_ms, lit_continuous, charges_before, remaining_ms_before, \
           deadline_before, charges_after, remaining_ms_after, deadline_after) \
         VALUES (encode($1,'hex')::uuid, $2::text::numeric(20,0), $3, encode($4,'hex')::uuid, \
           encode($5,'hex')::uuid, $6, $7, $8, $6, $7, $8, $9, $10, $11, $12, $13, NULL, $14, \
           $15, NULL)",
    )
    .bind(request.item_instance_id.as_slice())
    .bind(request.expected_revision.to_string())
    .bind(cause.code())
    .bind(request.transaction_id.as_slice())
    .bind(holder.as_bytes().as_slice())
    .bind(&definition.family)
    .bind(&definition.production_key)
    .bind(&definition.revision_ref)
    .bind(charges_column(full))
    .bind(remaining_column(full)?)
    .bind(request.definition.lit_continuous)
    .bind(charges_column(before))
    .bind(remaining_column(before)?)
    .bind(charges_column(request.values))
    .bind(remaining_column(request.values)?)
    .execute(&mut *connection)
    .await?;
    let written = if revision == 0 {
        sqlx::query(
            "INSERT INTO game_item_timed_states (item_instance_id, charges, remaining_ms, \
               deadline_at, state_revision) \
             VALUES (encode($1,'hex')::uuid, $2, $3, NULL, 1)",
        )
        .bind(request.item_instance_id.as_slice())
        .bind(charges_column(request.values))
        .bind(remaining_column(request.values)?)
        .execute(&mut *connection)
        .await?
    } else {
        sqlx::query(
            "UPDATE game_item_timed_states \
                SET charges = $2, remaining_ms = $3, \
                    state_revision = state_revision + 1 \
              WHERE item_instance_id = encode($1,'hex')::uuid \
                AND state_revision = $4::text::numeric(20,0)",
        )
        .bind(request.item_instance_id.as_slice())
        .bind(charges_column(request.values))
        .bind(remaining_column(request.values)?)
        .bind(revision.to_string())
        .execute(&mut *connection)
        .await?
    };
    if written.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    Ok(TimedWriteOutcome::Written {
        revision: revision + 1,
    })
}

/// The decay target of an expiry TRANSFORM (§8: `transform{trigger: decay}`, otherwise the
/// item's `temporal.decay_target`) and its facts: `full` is `None` for a target that is not
/// timed, whose row is then spent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedExpiryTarget {
    pub definition: TypedDefinitionRef,
    pub full: Option<TimedValues>,
    pub lit_continuous: bool,
}

/// One expiry (§8) of a held item, keyed by (item, `expected_revision`). `target` `None` burns
/// the item. The TransactionId, the event id and the occurrence time are fixed by the lane and
/// reused on retry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedExpiryRequest {
    pub item_instance_id: [u8; 16],
    pub expected_revision: u64,
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub reason: ExpireReason,
    /// The facts of the expiring definition.
    pub definition: TimedDefinitionFacts,
    pub target: Option<TimedExpiryTarget>,
    pub occurred_at_unix_ms: i64,
    pub server_build_id: String,
}

impl TimedExpiryRequest {
    /// The row revision after the expiry: the transform writes one, the burn none.
    const fn revision_after(&self) -> u64 {
        if self.target.is_some() {
            self.expected_revision + 1
        } else {
            self.expected_revision
        }
    }

    fn cause(&self) -> TimedWriteCause {
        TimedWriteCause::Timed(TimedItemCause::Expire {
            reason: self.reason.into(),
        })
    }
}

impl DurabilityRoot {
    /// Commit a held item's expiry under its holder's current gameplay fence (see the module
    /// documentation). An exact replay of a committed key returns its result without
    /// reacquiring gameplay authority; every refusal writes nothing.
    pub async fn commit_timed_expiry(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: TimedExpiryRequest,
    ) -> Result<TimedWriteOutcome> {
        validate_expiry(&request)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| TimedWriteError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    match replay_expiry(&mut tx, &request).await {
                        Ok(Some(outcome)) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            return Ok(Ok(outcome));
                        }
                        Ok(None) => {}
                        Err(TimedWriteError::Unavailable(error)) => return Err(error),
                        Err(error) => return Ok(Err(error)),
                    }
                    match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(_) => {}
                        Err(CharacterProgressionError::CharacterRevisionMismatch) => {
                            return Ok(Err(TimedWriteError::CharacterRevisionMismatch));
                        }
                        Err(CharacterProgressionError::Unavailable(error)) => return Err(error),
                        Err(_) => return Ok(Err(TimedWriteError::AuthorityRejected)),
                    }
                    match write_expiry(&mut tx, &fence, &request).await {
                        Ok(outcome) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(outcome))
                        }
                        Err(TimedWriteError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

fn valid_definition(definition: &TypedDefinitionRef) -> bool {
    (1..=128).contains(&definition.family.len())
        && (1..=512).contains(&definition.production_key.len())
        && (1..=512).contains(&definition.revision_ref.len())
}

fn validate_expiry(request: &TimedExpiryRequest) -> Result<()> {
    let full = request.definition.full;
    let reason_value = match request.reason {
        ExpireReason::TimeExhausted => full.remaining_ms(),
        ExpireReason::ChargesExhausted => full.charges().map(u64::from),
    };
    let target_valid = request.target.as_ref().is_none_or(|target| {
        valid_definition(&target.definition)
            && target.definition != request.definition.definition
            && (target.full.is_some() || !target.lit_continuous)
            && target
                .full
                .is_none_or(|full| full.remaining_ms() != Some(0))
    });
    if check_uuid_v7(&request.item_instance_id).is_err()
        || check_uuid_v7(&request.transaction_id).is_err()
        || check_uuid_v7(&request.event_id).is_err()
        || request.transaction_id == request.event_id
        || check_technical_text(&request.server_build_id).is_err()
        || request.occurred_at_unix_ms <= 0
        || !valid_definition(&request.definition.definition)
        || request.expected_revision == u64::MAX
        || reason_value.is_none()
        || full.remaining_ms() == Some(0)
        || !target_valid
    {
        return Err(TimedWriteError::InvalidInput);
    }
    Ok(())
}

/// The recorded result of an exact expiry replay, a conflict for a key committed by another
/// write, or `None` for a new key.
async fn replay_expiry(
    connection: &mut PgConnection,
    request: &TimedExpiryRequest,
) -> Result<Option<TimedWriteOutcome>> {
    let Some(stored) = find_timed_write(
        connection,
        &request.item_instance_id,
        request.expected_revision,
    )
    .await?
    else {
        return Ok(None);
    };
    if stored.cause == request.cause() && stored.transaction_id == request.transaction_id {
        Ok(Some(TimedWriteOutcome::AlreadyCommitted {
            revision: request.revision_after(),
        }))
    } else {
        Err(TimedWriteError::ConflictingWrite)
    }
}

/// The expiry body, after the fence locked the holder's root in this transaction.
async fn write_expiry(
    connection: &mut PgConnection,
    fence: &CurrentCharacterGameplayFence,
    request: &TimedExpiryRequest,
) -> Result<TimedWriteOutcome> {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err(TimedWriteError::AuthorityRejected);
    };
    let holder = fence.character_id;
    let definition = &request.definition.definition;
    let full = request.definition.full;
    if let Some(outcome) = replay_expiry(connection, request).await? {
        return Ok(outcome);
    }
    let item = sqlx::query(
        "SELECT i.definition_family, i.definition_production_key, i.definition_revision_ref, \
                i.lifecycle, i.quantity, i.world_id::text, \
                EXISTS (SELECT 1 FROM game_item_container_slots s \
                         WHERE s.item_instance_id = i.item_instance_id \
                           AND s.character_id = encode($2,'hex')::uuid) AS in_slot, \
                e.parent_item_instance_id::text AS parent, \
                e.placement_ordinal::text AS ordinal, \
                (EXISTS (SELECT 1 FROM game_item_container_entries c \
                          WHERE c.parent_item_instance_id = i.item_instance_id) \
                 OR EXISTS (SELECT 1 FROM game_item_corpse_container_entries c \
                             WHERE c.parent_item_instance_id = i.item_instance_id)) \
                  AS has_contents \
           FROM game_item_instances i \
           LEFT JOIN game_item_container_entries e \
             ON e.item_instance_id = i.item_instance_id \
            AND e.character_id = encode($2,'hex')::uuid \
          WHERE i.item_instance_id = encode($1,'hex')::uuid FOR UPDATE OF i",
    )
    .bind(request.item_instance_id.as_slice())
    .bind(holder.as_bytes().as_slice())
    .fetch_optional(&mut *connection)
    .await?;
    let Some(item) = item else {
        return Err(TimedWriteError::NotHeld);
    };
    let in_slot: bool = item.try_get("in_slot")?;
    let entry = match item.try_get::<Option<String>, _>("parent")? {
        Some(parent) => Some(OneItemContainerEntryV1 {
            parent_item_instance_id: uuid_text(&parent)?.to_vec(),
            placement_ordinal: numeric_u64(&item, "ordinal")?,
        }),
        None => None,
    };
    let quantity = u32::try_from(item.try_get::<i64, _>("quantity")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if item.try_get::<i16, _>("lifecycle")? != 1
        || in_slot == entry.is_some()
        || uuid_text(&item.try_get::<String, _>("world_id")?)? != *world_id.as_bytes()
        || item.try_get::<String, _>("definition_family")? != definition.family
        || item.try_get::<String, _>("definition_production_key")? != definition.production_key
        || item.try_get::<String, _>("definition_revision_ref")? != definition.revision_ref
    {
        return Err(TimedWriteError::NotHeld);
    }
    if request.target.is_none() && (entry.is_none() || item.try_get::<bool, _>("has_contents")?) {
        return Err(TimedWriteError::NotBurnable);
    }

    let current = sqlx::query(
        "SELECT charges, remaining_ms, deadline_at, state_revision::text \
           FROM game_item_timed_states WHERE item_instance_id = encode($1,'hex')::uuid \
           FOR UPDATE",
    )
    .bind(request.item_instance_id.as_slice())
    .fetch_optional(&mut *connection)
    .await?
    .map(|row| decode_state(&row))
    .transpose()?;
    let (revision, before, deadline_before) = match current {
        None => (0, Some(full), None),
        Some(state) => (state.revision, state.values, state.deadline_at),
    };
    if revision != request.expected_revision {
        return Err(TimedWriteError::RevisionMismatch { current });
    }
    // A spent row never expires again; a row above the definition is not this definition's.
    let Some(before) = before else {
        return Err(TimedWriteError::NotHeld);
    };
    if deadline_before.is_some() || !within(before, full) {
        return Err(TimedWriteError::NotAStoreableChange);
    }

    let (after_definition, after_full, after_lit, after_values) = match &request.target {
        Some(target) => (
            &target.definition,
            target.full,
            target.lit_continuous,
            target.full,
        ),
        None => (
            definition,
            Some(full),
            request.definition.lit_continuous,
            Some(before),
        ),
    };
    let timed_row = |values: Option<TimedValues>, revision: u64| OneItemTimedRowV1 {
        charges: values.and_then(TimedValues::charges),
        remaining_ms: values.and_then(TimedValues::remaining_ms),
        revision,
    };
    let state = |definition: &TypedDefinitionRef, quantity: u32, lifecycle: u32| OneItemStateV1 {
        item_instance_id: request.item_instance_id.to_vec(),
        world_id: world_id.as_bytes().to_vec(),
        definition: Some(definition_message(definition)),
        quantity,
        lifecycle,
    };
    let (after_state, row_after) = match request.target {
        Some(_) => (
            state(after_definition, quantity, ITEM_LIFECYCLE_LIVE),
            timed_row(after_values, revision + 1),
        ),
        None => (
            state(definition, 0, ITEM_LIFECYCLE_RETIRED),
            timed_row(Some(before), revision),
        ),
    };
    let envelope = encode_timed_expiry_event(
        TimedExpiryEventIdentity {
            event_id: request.event_id,
            transaction_id: request.transaction_id,
            occurred_at_unix_ms: request.occurred_at_unix_ms,
            server_build_id: &request.server_build_id,
        },
        OneItemTimedExpiryV1 {
            before: Some(state(definition, quantity, ITEM_LIFECYCLE_LIVE)),
            after: Some(after_state),
            character_id: holder.as_bytes().to_vec(),
            world_id: world_id.as_bytes().to_vec(),
            channel_id: channel_id.as_bytes().to_vec(),
            runtime_scope_ownership_generation: fence.scope_ownership_generation.get(),
            container_entry: entry.clone(),
            equipment_container_slot: in_slot,
            reason: match request.reason {
                ExpireReason::TimeExhausted => TIMED_EXPIRY_REASON_TIME_EXHAUSTED,
                ExpireReason::ChargesExhausted => TIMED_EXPIRY_REASON_CHARGES_EXHAUSTED,
            },
            row_before: Some(timed_row(Some(before), revision)),
            row_after: Some(row_after),
        },
    )
    .map_err(|_| TimedWriteError::InvalidInput)?;

    // The record first: the item guard admits the in-place definition change only with it.
    sqlx::query(
        "INSERT INTO game_item_timed_state_writes (\
           item_instance_id, expected_revision, cause, transaction_id, holder_character_id, \
           definition_before_family, definition_before_production_key, \
           definition_before_revision_ref, definition_after_family, \
           definition_after_production_key, definition_after_revision_ref, full_charges, \
           full_remaining_ms, lit_continuous, charges_before, remaining_ms_before, \
           deadline_before, charges_after, remaining_ms_after, deadline_after, \
           before_full_charges, before_full_remaining_ms, before_lit_continuous, event_id) \
         VALUES (encode($1,'hex')::uuid, $2::text::numeric(20,0), $3, encode($4,'hex')::uuid, \
           encode($5,'hex')::uuid, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, NULL, \
           $17, $18, NULL, $19, $20, $21, encode($22,'hex')::uuid)",
    )
    .bind(request.item_instance_id.as_slice())
    .bind(revision.to_string())
    .bind(request.cause().code())
    .bind(request.transaction_id.as_slice())
    .bind(holder.as_bytes().as_slice())
    .bind(&definition.family)
    .bind(&definition.production_key)
    .bind(&definition.revision_ref)
    .bind(&after_definition.family)
    .bind(&after_definition.production_key)
    .bind(&after_definition.revision_ref)
    .bind(charges_of(after_full))
    .bind(remaining_of(after_full)?)
    .bind(after_lit)
    .bind(charges_column(before))
    .bind(remaining_column(before)?)
    .bind(charges_of(after_values))
    .bind(remaining_of(after_values)?)
    .bind(charges_column(full))
    .bind(remaining_column(full)?)
    .bind(request.definition.lit_continuous)
    .bind(request.event_id.as_slice())
    .execute(&mut *connection)
    .await?;

    let changed = if let Some(target) = &request.target {
        sqlx::query(
            "UPDATE game_item_instances \
                SET definition_family = $2, definition_production_key = $3, \
                    definition_revision_ref = $4, last_transaction_id = encode($5,'hex')::uuid \
              WHERE item_instance_id = encode($1,'hex')::uuid AND lifecycle = 1",
        )
        .bind(request.item_instance_id.as_slice())
        .bind(&target.definition.family)
        .bind(&target.definition.production_key)
        .bind(&target.definition.revision_ref)
        .bind(request.transaction_id.as_slice())
        .execute(&mut *connection)
        .await?
        .rows_affected()
            == 1
            && if revision == 0 {
                sqlx::query(
                    "INSERT INTO game_item_timed_states (item_instance_id, charges, \
                       remaining_ms, deadline_at, state_revision) \
                     VALUES (encode($1,'hex')::uuid, $2, $3, NULL, 1)",
                )
                .bind(request.item_instance_id.as_slice())
                .bind(charges_of(after_values))
                .bind(remaining_of(after_values)?)
                .execute(&mut *connection)
                .await?
            } else {
                sqlx::query(
                    "UPDATE game_item_timed_states \
                        SET charges = $2, remaining_ms = $3, \
                            state_revision = state_revision + 1 \
                      WHERE item_instance_id = encode($1,'hex')::uuid \
                        AND state_revision = $4::text::numeric(20,0)",
                )
                .bind(request.item_instance_id.as_slice())
                .bind(charges_of(after_values))
                .bind(remaining_of(after_values)?)
                .bind(revision.to_string())
                .execute(&mut *connection)
                .await?
            }
            .rows_affected()
                == 1
    } else {
        sqlx::query(
            "UPDATE game_item_instances \
                SET quantity = 0, lifecycle = 2, last_transaction_id = encode($2,'hex')::uuid \
              WHERE item_instance_id = encode($1,'hex')::uuid AND lifecycle = 1",
        )
        .bind(request.item_instance_id.as_slice())
        .bind(request.transaction_id.as_slice())
        .execute(&mut *connection)
        .await?
        .rows_affected()
            == 1
            && sqlx::query(
                "DELETE FROM game_item_container_entries \
                  WHERE item_instance_id = encode($1,'hex')::uuid \
                    AND character_id = encode($2,'hex')::uuid",
            )
            .bind(request.item_instance_id.as_slice())
            .bind(holder.as_bytes().as_slice())
            .execute(&mut *connection)
            .await?
            .rows_affected()
                == 1
    };
    if !changed {
        return Err(DurabilityError::InvalidStoredState.into());
    }

    sqlx::query(
        "INSERT INTO game_item_audit_outbox(event_id, transaction_id, transaction_ordinal, \
           transaction_count, event_type_id, schema_revision, retention_profile_id, \
           item_instance_id, occurred_at, expires_at, envelope, envelope_sha256, \
           publication_state) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, 1, $3, $4, $5, \
           encode($6,'hex')::uuid, $7, $7 + $8, $9, sha256($9), 1)",
    )
    .bind(request.event_id.as_slice())
    .bind(request.transaction_id.as_slice())
    .bind(i64::from(mint_audit::EVENT_TYPE_ID))
    .bind(i64::from(mint_audit::EVENT_SCHEMA_REVISION))
    .bind(mint_audit::RETENTION_PROFILE_ID)
    .bind(request.item_instance_id.as_slice())
    .bind(request.occurred_at_unix_ms)
    .bind(mint_audit::AUDIT_RETENTION_P90D_MS)
    .bind(envelope.as_slice())
    .execute(&mut *connection)
    .await?;
    Ok(TimedWriteOutcome::Written {
        revision: request.revision_after(),
    })
}

fn charges_of(values: Option<TimedValues>) -> Option<i32> {
    values.and_then(charges_column)
}

fn remaining_of(values: Option<TimedValues>) -> std::result::Result<Option<i64>, DurabilityError> {
    Ok(values.map(remaining_column).transpose()?.flatten())
}

/// `values` has exactly the components of `limit`, none above it.
fn within(values: TimedValues, limit: TimedValues) -> bool {
    let charges = match (values.charges(), limit.charges()) {
        (None, None) => true,
        (Some(value), Some(limit)) => value <= limit,
        _ => false,
    };
    let remaining = match (values.remaining_ms(), limit.remaining_ms()) {
        (None, None) => true,
        (Some(value), Some(limit)) => value <= limit,
        _ => false,
    };
    charges && remaining
}

fn charges_column(values: TimedValues) -> Option<i32> {
    values.charges().and_then(|value| i32::try_from(value).ok())
}

fn remaining_column(values: TimedValues) -> std::result::Result<Option<i64>, DurabilityError> {
    values
        .remaining_ms()
        .map(|value| i64::try_from(value).map_err(|_| DurabilityError::InvalidStoredState))
        .transpose()
}

fn decode_values(
    row: &PgRow,
    charges: &str,
    remaining_ms: &str,
) -> std::result::Result<Option<TimedValues>, DurabilityError> {
    let charges = row
        .try_get::<Option<i32>, _>(charges)?
        .map(u32::try_from)
        .transpose()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let remaining_ms = row
        .try_get::<Option<i64>, _>(remaining_ms)?
        .map(u64::try_from)
        .transpose()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if charges.is_none() && remaining_ms.is_none() {
        return Ok(None);
    }
    TimedValues::new(charges, remaining_ms)
        .map(Some)
        .map_err(|_| DurabilityError::InvalidStoredState)
}

fn decode_state(row: &PgRow) -> std::result::Result<StoredTimedState, DurabilityError> {
    Ok(StoredTimedState {
        revision: numeric_u64(row, "state_revision")?,
        values: decode_values(row, "charges", "remaining_ms")?,
        deadline_at: row.try_get("deadline_at")?,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn cause_codes_round_trip_and_are_closed() {
        for code in 1..=8 {
            let cause = TimedWriteCause::from_code(code).expect("admitted code");
            assert_eq!(cause.code(), code);
        }
        assert!(TimedWriteCause::from_code(0).is_none());
        assert!(TimedWriteCause::from_code(9).is_none());
    }

    #[test]
    fn values_must_have_the_definitions_components_and_stay_below_it() {
        let full = TimedValues::new(Some(5), Some(1_000)).expect("valid");
        assert!(within(
            TimedValues::new(Some(5), Some(0)).expect("valid"),
            full
        ));
        assert!(!within(
            TimedValues::new(Some(6), Some(1)).expect("valid"),
            full
        ));
        assert!(!within(
            TimedValues::new(Some(5), None).expect("valid"),
            full
        ));
        assert!(!within(
            TimedValues::new(None, Some(1)).expect("valid"),
            full
        ));
    }
}
