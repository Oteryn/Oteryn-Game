//! The forge dust balance and its immutable ledger (FORGE-1a, migration 0059; IMBUE-FORGE-0 §9,
//! stored as BANK-0 §3).
//!
//! Dust is a Character balance (0 to `dust_limit`, the limit 100-225, `IMBFORGE0-RL-08`). A
//! source calls [`spend_forge_dust_in_transaction`] or [`gain_forge_dust_in_transaction`] inside
//! its own fenced Character transaction, under its own cause occurrence, TransactionId, receipt
//! and DUR-03 admission: an entry is not a receipt and not a forge operation. The composition
//! decision's rule 1 covers the entry and the balance row (no `CharacterRevision` advance of its
//! own); the lock order is rule 4's (the caller holds the `character_root` lock), then the dust
//! row. Each call writes exactly one ledger entry and the balance row, and never commits: any
//! error leaves the caller's transaction to roll back. A spend above the balance is refused
//! before any write; a gain above the limit credits up to the limit and records the lost part.
//!
//! The causes are closed and reserved: `SPEND` for PROF-SHAPE-1b's `ProficiencyCause`, `GAIN`
//! for FORGE-CREATURE-1's dust on kill. Production wires neither yet. `CONVERT`, `LIMIT_RAISE`
//! and the forge causes are FORGE-1b's.

use super::DurabilityError;
use super::character_progression::{CurrentCharacterGameplayFence, uuid_text};
use super::item_mint_audit::check_uuid_v7;
use crate::domain::CharacterId;
use crate::domain::forge_dust::{DustLimit, ForgeDustBalance, ForgeDustError};
use sqlx::Row;
use sqlx::postgres::{PgConnection, PgRow};

pub(super) const KIND_GAIN: &str = "GAIN";
pub(super) const KIND_SPEND: &str = "SPEND";
const CAUSE_CREATURE_KILL: &str = "creature_kill";
const CAUSE_PROFICIENCY: &str = "proficiency";

/// The closed dust spending cause: one variant per admitted sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeDustSpendCause {
    /// Reserved for PROF-SHAPE-1b: a proficiency modification under its `ProficiencyCause`.
    Proficiency { occurrence: [u8; 16] },
}

/// The closed dust gain cause: one variant per admitted source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeDustGainCause {
    /// Reserved for FORGE-CREATURE-1: dust from a creature kill (IMBUE-FORGE-0 §11).
    CreatureKill { occurrence: [u8; 16] },
}

/// Identities the source fixes with its TransactionId before the first attempt and reuses on
/// every retry of the same occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgeDustWriteIds {
    pub entry_id: [u8; 16],
    pub transaction_id: [u8; 16],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeDustEntryKind {
    Gain,
    Spend,
}

/// One ledger entry: `amount` requested; a gain credits `amount - lost`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgeDustEntry {
    pub entry_id: [u8; 16],
    pub previous_entry_id: Option<[u8; 16]>,
    pub character_id: CharacterId,
    pub kind: ForgeDustEntryKind,
    pub cause_occurrence_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub amount: u32,
    pub lost: u32,
    pub before: ForgeDustBalance,
    pub after: ForgeDustBalance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeDustOutcome {
    /// Written in the caller's transaction; durable only when the caller commits.
    Written(ForgeDustEntry),
    /// The occurrence already has its entry with the same binding: the caller must return its
    /// own retained outcome and must not commit a second change.
    AlreadyWritten(ForgeDustEntry),
}

#[derive(Debug)]
pub enum ForgeDustWriteError {
    InvalidInput,
    /// The locked root is not the fenced Character in the fenced World.
    CharacterMismatch,
    /// A spend above the balance; nothing was written.
    InsufficientDust,
    /// The occurrence already has an entry with another binding.
    ConflictingOccurrence,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for ForgeDustWriteError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl From<sqlx::Error> for ForgeDustWriteError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}

impl std::fmt::Display for ForgeDustWriteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("invalid forge dust input"),
            Self::CharacterMismatch => formatter.write_str("forge dust Character does not match"),
            Self::InsufficientDust => formatter.write_str("insufficient forge dust"),
            Self::ConflictingOccurrence => {
                formatter.write_str("forge dust occurrence was reused with a different binding")
            }
            Self::Unavailable(error) => write!(formatter, "forge dust storage failed: {error:?}"),
        }
    }
}

impl std::error::Error for ForgeDustWriteError {}

type Result<T> = std::result::Result<T, ForgeDustWriteError>;

/// Debit `amount` dust inside the caller's fenced Character transaction (see the module
/// documentation). A spend above the balance is refused before any write.
pub async fn spend_forge_dust_in_transaction(
    connection: &mut PgConnection,
    fence: &CurrentCharacterGameplayFence,
    cause: ForgeDustSpendCause,
    amount: u32,
    ids: ForgeDustWriteIds,
) -> Result<ForgeDustOutcome> {
    let ForgeDustSpendCause::Proficiency { occurrence } = cause;
    write_entry(
        connection,
        fence,
        Write {
            kind: ForgeDustEntryKind::Spend,
            cause: CAUSE_PROFICIENCY,
            occurrence,
            amount,
            ids,
        },
    )
    .await
}

/// Credit `amount` dust inside the caller's fenced Character transaction (see the module
/// documentation), up to the limit; the lost part is recorded in the entry.
pub async fn gain_forge_dust_in_transaction(
    connection: &mut PgConnection,
    fence: &CurrentCharacterGameplayFence,
    cause: ForgeDustGainCause,
    amount: u32,
    ids: ForgeDustWriteIds,
) -> Result<ForgeDustOutcome> {
    let ForgeDustGainCause::CreatureKill { occurrence } = cause;
    write_entry(
        connection,
        fence,
        Write {
            kind: ForgeDustEntryKind::Gain,
            cause: CAUSE_CREATURE_KILL,
            occurrence,
            amount,
            ids,
        },
    )
    .await
}

/// The Character's committed dust balance; no row is balance 0 at the initial limit.
pub async fn read_forge_dust(
    connection: &mut PgConnection,
    character_id: CharacterId,
) -> std::result::Result<ForgeDustBalance, DurabilityError> {
    let row = sqlx::query(
        "SELECT balance, dust_limit FROM game_character_forge_dust \
          WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(character_id.as_bytes().as_slice())
    .fetch_optional(&mut *connection)
    .await?;
    row.map_or(Ok(ForgeDustBalance::default()), |row| {
        balance(&row, "balance", "dust_limit")
    })
}

struct Write {
    kind: ForgeDustEntryKind,
    cause: &'static str,
    occurrence: [u8; 16],
    amount: u32,
    ids: ForgeDustWriteIds,
}

async fn write_entry(
    connection: &mut PgConnection,
    fence: &CurrentCharacterGameplayFence,
    write: Write,
) -> Result<ForgeDustOutcome> {
    if write.amount == 0
        || check_uuid_v7(&write.occurrence).is_err()
        || check_uuid_v7(&write.ids.entry_id).is_err()
        || check_uuid_v7(&write.ids.transaction_id).is_err()
        || write.ids.entry_id == write.ids.transaction_id
    {
        return Err(ForgeDustWriteError::InvalidInput);
    }
    let character = fence.character_id.as_bytes().as_slice();

    // Rule 4: the source holds this lock already; taking it again proves the row is the fenced
    // Character in the fenced World, and orders the dust row after it.
    let root = sqlx::query(
        "SELECT world_id::text FROM game_character_roots \
          WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(character)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or(ForgeDustWriteError::CharacterMismatch)?;
    if uuid_text(&root.try_get::<String, _>("world_id")?)?
        != *fence.runtime_scope.world_id().as_bytes()
    {
        return Err(ForgeDustWriteError::CharacterMismatch);
    }

    let stored = sqlx::query(ENTRY_SELECT_BY_OCCURRENCE)
        .bind(character)
        .bind(write.cause)
        .bind(write.occurrence.as_slice())
        .fetch_optional(&mut *connection)
        .await?;
    if let Some(row) = stored {
        let entry = entry_from_row(&row)?;
        if entry.kind != write.kind
            || entry.amount != write.amount
            || entry.entry_id != write.ids.entry_id
            || entry.transaction_id != write.ids.transaction_id
        {
            return Err(ForgeDustWriteError::ConflictingOccurrence);
        }
        return Ok(ForgeDustOutcome::AlreadyWritten(entry));
    }

    let mut row = lock_balance_row(connection, character).await?;
    if row.is_none() {
        if write.kind == ForgeDustEntryKind::Spend {
            // No row is balance 0: refused before any write.
            return Err(ForgeDustWriteError::InsufficientDust);
        }
        // The zero row equals no row; a concurrent first gain waits on this insert.
        sqlx::query(
            "INSERT INTO game_character_forge_dust (character_id, balance, dust_limit) \
             VALUES (encode($1,'hex')::uuid, 0, 100) ON CONFLICT (character_id) DO NOTHING",
        )
        .bind(character)
        .execute(&mut *connection)
        .await?;
        row = lock_balance_row(connection, character).await?;
    }
    let (before, previous_entry_id) = row.ok_or(DurabilityError::InvalidStoredState)?;
    let (after, lost) = match write.kind {
        ForgeDustEntryKind::Gain => {
            let gain = before.gain(write.amount).map_err(plan_error)?;
            (gain.after, gain.lost)
        }
        ForgeDustEntryKind::Spend => (before.spend(write.amount).map_err(plan_error)?.after, 0),
    };

    sqlx::query(
        "INSERT INTO game_character_forge_dust_entries (entry_id, character_id, \
            previous_entry_id, kind, cause, cause_occurrence_id, transaction_id, amount, \
            lost_amount, balance_before, balance_after, dust_limit_before, dust_limit_after) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, \
            encode($3,'hex')::uuid, $4, $5, \
            encode($6,'hex')::uuid, encode($7,'hex')::uuid, $8, $9, $10, $11, $12, $13)",
    )
    .bind(write.ids.entry_id.as_slice())
    .bind(character)
    .bind(previous_entry_id.as_ref().map(|id| id.to_vec()))
    .bind(kind_text(write.kind))
    .bind(write.cause)
    .bind(write.occurrence.as_slice())
    .bind(write.ids.transaction_id.as_slice())
    .bind(i64::from(write.amount))
    .bind(i64::from(lost))
    .bind(i64::from(before.balance()))
    .bind(i64::from(after.balance()))
    .bind(limit_i16(before.limit()))
    .bind(limit_i16(after.limit()))
    .execute(&mut *connection)
    .await?;
    sqlx::query(
        "UPDATE game_character_forge_dust \
            SET balance = $2, dust_limit = $3, last_entry_id = encode($4,'hex')::uuid \
          WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(character)
    .bind(i64::from(after.balance()))
    .bind(limit_i16(after.limit()))
    .bind(write.ids.entry_id.as_slice())
    .execute(&mut *connection)
    .await?;

    Ok(ForgeDustOutcome::Written(ForgeDustEntry {
        entry_id: write.ids.entry_id,
        previous_entry_id,
        character_id: fence.character_id,
        kind: write.kind,
        cause_occurrence_id: write.occurrence,
        transaction_id: write.ids.transaction_id,
        amount: write.amount,
        lost,
        before,
        after,
    }))
}

const ENTRY_SELECT_BY_OCCURRENCE: &str = "\
SELECT entry_id::text, previous_entry_id::text, character_id::text, kind, \
       cause_occurrence_id::text, transaction_id::text, amount, lost_amount, balance_before, \
       balance_after, dust_limit_before, dust_limit_after \
  FROM game_character_forge_dust_entries \
 WHERE character_id = encode($1,'hex')::uuid AND cause = $2 \
   AND cause_occurrence_id = encode($3,'hex')::uuid";

/// The balance row FOR UPDATE, with its latest entry; `None` when the Character has no row.
async fn lock_balance_row(
    connection: &mut PgConnection,
    character: &[u8],
) -> Result<Option<(ForgeDustBalance, Option<[u8; 16]>)>> {
    let row = sqlx::query(
        "SELECT balance, dust_limit, last_entry_id::text FROM game_character_forge_dust \
          WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(character)
    .fetch_optional(&mut *connection)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let last = row
        .try_get::<Option<String>, _>("last_entry_id")?
        .map(|id| uuid_text(&id))
        .transpose()?;
    Ok(Some((balance(&row, "balance", "dust_limit")?, last)))
}

fn plan_error(error: ForgeDustError) -> ForgeDustWriteError {
    match error {
        ForgeDustError::InsufficientDust => ForgeDustWriteError::InsufficientDust,
        ForgeDustError::ZeroAmount => ForgeDustWriteError::InvalidInput,
        ForgeDustError::InvalidLimit | ForgeDustError::BalanceAboveLimit => {
            ForgeDustWriteError::Unavailable(DurabilityError::InvalidStoredState)
        }
    }
}

fn kind_text(kind: ForgeDustEntryKind) -> &'static str {
    match kind {
        ForgeDustEntryKind::Gain => KIND_GAIN,
        ForgeDustEntryKind::Spend => KIND_SPEND,
    }
}

fn limit_i16(limit: DustLimit) -> i16 {
    // 100-225 always fits.
    i16::try_from(limit.get()).unwrap_or(i16::MAX)
}

pub(super) fn balance(
    row: &PgRow,
    balance: &str,
    limit: &str,
) -> std::result::Result<ForgeDustBalance, DurabilityError> {
    let limit = u16::try_from(row.try_get::<i16, _>(limit)?)
        .ok()
        .and_then(|limit| DustLimit::new(limit).ok())
        .ok_or(DurabilityError::InvalidStoredState)?;
    let balance = u16::try_from(row.try_get::<i64, _>(balance)?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    ForgeDustBalance::new(balance, limit).map_err(|_| DurabilityError::InvalidStoredState)
}

/// Decode an entry row selected with the columns of `ENTRY_SELECT_BY_OCCURRENCE`.
pub(super) fn entry_from_row(row: &PgRow) -> std::result::Result<ForgeDustEntry, DurabilityError> {
    let kind = match row.try_get::<String, _>("kind")?.as_str() {
        KIND_GAIN => ForgeDustEntryKind::Gain,
        KIND_SPEND => ForgeDustEntryKind::Spend,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let amount = |column: &str| -> std::result::Result<u32, DurabilityError> {
        u32::try_from(row.try_get::<i64, _>(column)?)
            .map_err(|_| DurabilityError::InvalidStoredState)
    };
    Ok(ForgeDustEntry {
        entry_id: uuid_text(&row.try_get::<String, _>("entry_id")?)?,
        previous_entry_id: row
            .try_get::<Option<String>, _>("previous_entry_id")?
            .map(|id| uuid_text(&id))
            .transpose()?,
        character_id: CharacterId::from_bytes(uuid_text(
            &row.try_get::<String, _>("character_id")?,
        )?)
        .map_err(|_| DurabilityError::InvalidStoredState)?,
        kind,
        cause_occurrence_id: uuid_text(&row.try_get::<String, _>("cause_occurrence_id")?)?,
        transaction_id: uuid_text(&row.try_get::<String, _>("transaction_id")?)?,
        amount: amount("amount")?,
        lost: amount("lost_amount")?,
        before: balance(row, "balance_before", "dust_limit_before")?,
        after: balance(row, "balance_after", "dust_limit_after")?,
    })
}
