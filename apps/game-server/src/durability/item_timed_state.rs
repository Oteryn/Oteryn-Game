//! Timed-item rows and their write records (TIMED-RT-1a, migration 0054; decision
//! `TIMEDITEM0B-RUNTIME-CHARGES-AND-DURATION-V1` §4, §5.2, §6.3 and §12, TIMED-ITEM-0 §4).
//!
//! A hosting runtime runs a live item's checkpoint ([`checkpoint_in_transaction`]) inside its
//! own one-item transaction, after its gameplay fence locked the holder's Character root (the
//! lane of `domain::timed_item` decides when). The write is keyed by (item, expected revision):
//! it inserts the write record, then inserts the row at revision 1 or moves it up by one, and
//! never commits, so any error leaves the caller's transaction to roll back. Lock order (§6.3):
//! the caller's fences and `character_root`, the item's lifecycle row, the timed row. A replay
//! of a committed key returns its recorded result; a key committed by another write writes
//! nothing. [`find_timed_write`] is the lane's lookup of an ambiguous write (§5.2).
//!
//! 0054 admits the checkpoint only. The other [`TimedItemCause`] variants are typed for their
//! records and are written by the transactions that admit their TRANSFORM, BURN and move lines.

use super::DurabilityError;
use super::character_progression::{numeric_u64, uuid_text};
use super::item_mint::TypedDefinitionRef;
use super::item_mint_audit::check_uuid_v7;
use crate::domain::CharacterId;
use crate::domain::timed_item::{ExpireReason, TimedValues};
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
            Self::NotHeld => formatter.write_str("timed item is not held live by the holder"),
            Self::RevisionMismatch { .. } => formatter.write_str("timed row revision mismatch"),
            Self::NotAStoreableChange => formatter.write_str("timed values are not a change"),
            Self::ConflictingWrite => {
                formatter.write_str("timed write key was committed by another write")
            }
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

/// Store a live item's checkpoint inside the caller's fenced one-item transaction (see the
/// module documentation). Every error requires the caller to roll back.
pub async fn checkpoint_in_transaction(
    connection: &mut PgConnection,
    holder: CharacterId,
    request: &TimedCheckpointRequest,
) -> Result<TimedWriteOutcome> {
    let full = request.definition.full;
    let definition = &request.definition.definition;
    if check_uuid_v7(&request.item_instance_id).is_err()
        || check_uuid_v7(&request.transaction_id).is_err()
        || !(1..=128).contains(&definition.family.len())
        || !(1..=512).contains(&definition.production_key.len())
        || !(1..=512).contains(&definition.revision_ref.len())
        || request.expected_revision == u64::MAX
        || !within(request.values, full)
    {
        return Err(TimedWriteError::InvalidInput);
    }
    let cause = TimedWriteCause::Timed(TimedItemCause::Checkpoint);
    if let Some(stored) = find_timed_write(
        connection,
        &request.item_instance_id,
        request.expected_revision,
    )
    .await?
    {
        return if stored.cause == cause
            && stored.transaction_id == request.transaction_id
            && stored.after == Some(request.values)
        {
            Ok(TimedWriteOutcome::AlreadyCommitted {
                revision: request.expected_revision + 1,
            })
        } else {
            Err(TimedWriteError::ConflictingWrite)
        };
    }

    // The caller holds this lock already; taking it again proves the holder's root is live.
    let root = sqlx::query(
        "SELECT 1 FROM game_character_roots \
          WHERE character_id = encode($1,'hex')::uuid AND lifecycle = 1 FOR UPDATE",
    )
    .bind(holder.as_bytes().as_slice())
    .fetch_optional(&mut *connection)
    .await?;
    if root.is_none() {
        return Err(TimedWriteError::NotHeld);
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
