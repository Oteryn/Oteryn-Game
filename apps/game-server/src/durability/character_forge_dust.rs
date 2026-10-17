//! The forge dust balance and its immutable ledger (FORGE-1a, migration 0059; IMBUE-FORGE-0 §9,
//! stored as BANK-0 §3).
//!
//! Dust is a Character balance (0 to the exact D1230 `dust_limit`, 100+20*n). A
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
//! for FORGE-CREATURE-1's dust on kill. Production wires neither yet. The material-free
//! D1230 `LIMIT_RAISE` owner below reserves and commits its own closed receipt. `CONVERT` and
//! the remaining Forge causes still require their separate native admission.

use super::DurabilityError;
use super::character_progression::{CurrentCharacterGameplayFence, uuid_text};
use super::item_mint_audit::check_uuid_v7;
use crate::domain::CharacterId;
use crate::domain::forge_dust::{DustLimit, ForgeDustBalance, ForgeDustError};
use sqlx::Row;
use sqlx::postgres::{PgConnection, PgRow};

pub(super) const KIND_GAIN: &str = "GAIN";
pub(super) const KIND_SPEND: &str = "SPEND";
pub(super) const KIND_LIMIT_RAISE: &str = "LIMIT_RAISE";
pub(super) const CAUSE_DUST_LIMIT: &str = "forge_dust_limit";
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
    /// D1230: one debit and one +20 transition; only the native Forge receipt owner writes it.
    LimitRaise,
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
        // The general source API never admits this closed Forge kind. Its dedicated owner
        // below couples the ledger, exact terminal receipt, history and type-2 audit.
        ForgeDustEntryKind::LimitRaise => return Err(ForgeDustWriteError::InvalidInput),
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
        ForgeDustError::ZeroAmount | ForgeDustError::MaximumLimitReached => {
            ForgeDustWriteError::InvalidInput
        }
        ForgeDustError::InvalidLimit | ForgeDustError::BalanceAboveLimit => {
            ForgeDustWriteError::Unavailable(DurabilityError::InvalidStoredState)
        }
    }
}

fn kind_text(kind: ForgeDustEntryKind) -> &'static str {
    match kind {
        ForgeDustEntryKind::Gain => KIND_GAIN,
        ForgeDustEntryKind::Spend => KIND_SPEND,
        ForgeDustEntryKind::LimitRaise => KIND_LIMIT_RAISE,
    }
}

fn limit_i16(limit: DustLimit) -> i16 {
    // The exact D1230 stages, 100 through 4600, always fit.
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
        KIND_LIMIT_RAISE => ForgeDustEntryKind::LimitRaise,
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

// ---- D1230 native, material-free ForgeCause::DustLimit continuation.
// No runtime USE/window or Content recipe admission is manufactured here. The runtime must
// own the actual pending CommandRef and Forge interaction before calling this persistence
// owner. This API independently proves the live session/lease/incarnation/scope/Character,
// binds the current Content activation digest and the actual simulation profile, and then
// serializes the root before the dust row. No CharacterRevision is advanced.
use super::DurabilityRoot;
use super::character_authority::{
    ReconciledCharacterAuthority, SERVER_BUILD_ID, assert_recovery_fence,
};
use super::character_forge_dust_audit::{
    self as dust_audit, DUST_LIMIT_RULESET_REVISION, OneForgeDustBalanceV1, OneItemForgeDustLimitV1,
};
use super::db::{
    begin_semantic_transaction, begin_type2_transaction, commit_semantic_transaction,
    lock_admission_relations,
};
use super::item_mint_audit::{self as mint_audit, SelectedType2Tuple};
use super::item_transfer::{CurrentCharacterItemFence, character_item_fence_is_current, scope_of};
use super::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::CommandRef;
use sha2::{Digest, Sha256};

type LimitResult<T> = std::result::Result<T, ForgeDustLimitError>;
type NativeResult<T> = std::result::Result<T, DurabilityError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgeDustLimitRequest {
    pub occurrence_id: [u8; 16],
    /// A quote for precisely this unlocked stage. A concurrent stage change refuses; the
    /// writer never silently spends the next, more expensive quote.
    pub expected_upgrade_count: u16,
}

/// Process-local view of a retained reservation. Callers cannot construct it from rows or
/// substitute its identities/revision binding. Recreating it reads the actual reservation.
#[derive(Debug)]
pub struct ForgeDustLimitCandidate {
    frozen: FrozenDustLimit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FrozenDustLimit {
    character_id: CharacterId,
    request: ForgeDustLimitRequest,
    world_id: [u8; 16],
    channel_id: [u8; 16],
    transaction_id: [u8; 16],
    event_id: [u8; 16],
    entry_id: [u8; 16],
    occurred_at_unix_ms: i64,
    content_digest: [u8; 32],
    ruleset_revision: String,
    simulation_profile_revision: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeDustLimitDisposition {
    Raised,
    RevisionChanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgeDustLimitReceipt {
    pub disposition: ForgeDustLimitDisposition,
    pub character_id: CharacterId,
    pub occurrence_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub bound_content_digest: [u8; 32],
    pub bound_ruleset_revision: String,
    pub bound_simulation_profile_revision: u32,
    /// RevisionChanged has neither value/state line nor event/history row.
    pub entry: Option<ForgeDustEntry>,
    pub envelope_sha256: Option<[u8; 32]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgeDustLimitOutcome {
    pub receipt: ForgeDustLimitReceipt,
    pub replayed: bool,
}

#[derive(Debug)]
pub enum ForgeDustLimitError {
    InvalidInput,
    AuthorityRejected,
    MissingActiveContent,
    ConflictingOccurrence,
    InsufficientDust,
    MaximumLimitReached,
    StageChanged,
    CapacityExceeded,
    Unavailable(DurabilityError),
}
impl From<DurabilityError> for ForgeDustLimitError {
    fn from(value: DurabilityError) -> Self {
        Self::Unavailable(value)
    }
}
impl std::fmt::Display for ForgeDustLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("invalid Forge DustLimit intent"),
            Self::AuthorityRejected => f.write_str("Forge DustLimit current authority rejected"),
            Self::MissingActiveContent => {
                f.write_str("Forge DustLimit has no current Content activation")
            }
            Self::ConflictingOccurrence => {
                f.write_str("Forge DustLimit occurrence binding conflicts")
            }
            Self::InsufficientDust => f.write_str("insufficient dust for this limit upgrade"),
            Self::MaximumLimitReached => f.write_str("maximum dust limit already reached"),
            Self::StageChanged => f.write_str("dust limit changed since the quoted stage"),
            Self::CapacityExceeded => f.write_str("Forge DustLimit retry budget exhausted"),
            Self::Unavailable(error) => write!(f, "Forge DustLimit storage unavailable: {error:?}"),
        }
    }
}
impl std::error::Error for ForgeDustLimitError {}

impl DurabilityRoot {
    /// Reserve identities and the independently observed revision set. The Content identity
    /// is the current server artifact digest; the ruleset is this operation's D1230 policy;
    /// the simulation revision comes from the actual `active_profile()`, never request data.
    /// This spends nothing. It deliberately does not assert Forge-world-object admission.
    pub async fn freeze_forge_dust_limit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        request: ForgeDustLimitRequest,
    ) -> LimitResult<ForgeDustLimitCandidate> {
        if check_uuid_v7(&request.occurrence_id).is_err()
            || request.expected_upgrade_count > crate::domain::forge_dust::DUST_LIMIT_UPGRADES_MAX
        {
            return Err(ForgeDustLimitError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| ForgeDustLimitError::AuthorityRejected)?;
        let node = node.clone();
        let frozen = self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                lock_dust_limit_cause(&mut tx, fence.character_id, request.occurrence_id).await?;
                if let Some(frozen) = load_dust_limit_reservation(&mut tx, fence.character_id, request.occurrence_id).await? {
                    if frozen.request != request { return Ok(Err(ForgeDustLimitError::ConflictingOccurrence)); }
                    // A committed/rejected occurrence is historical even under a new CommandRef.
                    if load_dust_limit_receipt(&mut tx, &frozen).await?.is_some() {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(frozen));
                    }
                    if !lock_dust_limit_current(&mut tx, &node, &fence, command, frozen.world_id, frozen.channel_id).await? {
                        return Ok(Err(ForgeDustLimitError::AuthorityRejected));
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    return Ok(Ok(frozen));
                }
                let Ok((world, channel)) = scope_of(&fence) else { return Ok(Err(ForgeDustLimitError::AuthorityRejected)); };
                if !lock_dust_limit_current(&mut tx, &node, &fence, command, *world.as_bytes(), *channel.as_bytes()).await? {
                    return Ok(Err(ForgeDustLimitError::AuthorityRejected));
                }
                let Some(content_digest) = current_dust_limit_content(&mut tx, *world.as_bytes(), *channel.as_bytes()).await? else {
                    return Ok(Err(ForgeDustLimitError::MissingActiveContent));
                };
                let row = lock_limit_balance(&mut tx, fence.character_id).await?;
                let before = row.map_or(ForgeDustBalance::default(), |(balance, _)| balance);
                if let Err(error) = plan_dust_limit(before, request.expected_upgrade_count) {
                    return Ok(Err(error));
                }
                let row = sqlx::query("SELECT game_character_uuid_v7()::text AS transaction_id, game_character_uuid_v7()::text AS event_id, game_character_uuid_v7()::text AS entry_id, floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS occurred_at")
                    .fetch_one(&mut *tx).await?;
                let frozen = FrozenDustLimit {
                    character_id: fence.character_id, request, world_id: *world.as_bytes(), channel_id: *channel.as_bytes(),
                    transaction_id: uuid_text(row.try_get("transaction_id")?)?,
                    event_id: uuid_text(row.try_get("event_id")?)?,
                    entry_id: uuid_text(row.try_get("entry_id")?)?,
                    occurred_at_unix_ms: row.try_get("occurred_at")?, content_digest,
                    ruleset_revision: DUST_LIMIT_RULESET_REVISION.into(),
                    simulation_profile_revision: oteryn_simulation_determinism::active_profile().revision().get(),
                };
                insert_dust_limit_reservation(&mut tx, &frozen).await?;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(Ok(frozen))
            })
        }).await??;
        Ok(ForgeDustLimitCandidate { frozen })
    }

    /// Charge bounded retry work before a mutation attempt. Known terminal receipts cost no
    /// further work and remain replayable after the budget is exhausted. Reconciliation is a
    /// bounded read, so an ambiguous final attempt can always be resolved.
    async fn charge_forge_dust_limit_attempt(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        frozen: FrozenDustLimit,
    ) -> LimitResult<()> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| ForgeDustLimitError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_dust_limit_cause(&mut tx, frozen.character_id, frozen.request.occurrence_id).await?;
                if !matches_dust_limit_reservation(&mut tx, &frozen).await? {
                    return Ok(Err(ForgeDustLimitError::ConflictingOccurrence));
                }
                if load_dust_limit_receipt(&mut tx, &frozen).await?.is_none() {
                    let changed = sqlx::query("UPDATE game_forge_dust_limit_reservations SET work_units_used=work_units_used+1 WHERE character_id=encode($1,'hex')::uuid AND occurrence_id=encode($2,'hex')::uuid AND work_units_used<$3")
                        .bind(frozen.character_id.as_bytes().as_slice()).bind(frozen.request.occurrence_id.as_slice())
                        .bind(i16::from(mint_audit::RL08_RETRY_WORK_UNITS_MAX)).execute(&mut *tx).await?;
                    if changed.rows_affected() != 1 { return Ok(Err(ForgeDustLimitError::CapacityExceeded)); }
                }
                commit_semantic_transaction(tx, deadline).await?;
                Ok(Ok(()))
            })
        }).await?
    }

    pub async fn commit_forge_dust_limit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        candidate: &ForgeDustLimitCandidate,
    ) -> LimitResult<ForgeDustLimitOutcome> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| ForgeDustLimitError::AuthorityRejected)?;
        self.charge_forge_dust_limit_attempt(authority, candidate.frozen.clone())
            .await?;
        let frozen = candidate.frozen.clone();
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_type2_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                lock_dust_limit_cause(&mut tx, frozen.character_id, frozen.request.occurrence_id).await?;
                if !matches_dust_limit_reservation(&mut tx, &frozen).await? {
                    return Ok(Err(ForgeDustLimitError::ConflictingOccurrence));
                }
                if let Some(receipt) = load_dust_limit_receipt(&mut tx, &frozen).await? {
                    tx.commit(deadline).await?;
                    return Ok(Ok(ForgeDustLimitOutcome { receipt, replayed: true }));
                }
                if fence.character_id != frozen.character_id
                    || !lock_dust_limit_current(&mut tx, &node, &fence, command, frozen.world_id, frozen.channel_id).await? {
                    return Ok(Err(ForgeDustLimitError::AuthorityRejected));
                }
                let Some(active_digest) = current_dust_limit_content(&mut tx, frozen.world_id, frozen.channel_id).await? else {
                    return Ok(Err(ForgeDustLimitError::MissingActiveContent));
                };
                if active_digest != frozen.content_digest
                    || frozen.ruleset_revision != DUST_LIMIT_RULESET_REVISION
                    || frozen.simulation_profile_revision != oteryn_simulation_determinism::active_profile().revision().get() {
                    insert_dust_limit_receipt(&mut tx, &frozen, None, None).await?;
                    let receipt = load_dust_limit_receipt(&mut tx, &frozen).await?.ok_or(DurabilityError::InvalidStoredState)?;
                    tx.commit(deadline).await?;
                    return Ok(Ok(ForgeDustLimitOutcome { receipt, replayed: false }));
                }
                let Some((before, previous_entry_id)) = lock_limit_balance(&mut tx, frozen.character_id).await? else {
                    return Ok(Err(ForgeDustLimitError::InsufficientDust));
                };
                let plan = match plan_dust_limit(before, frozen.request.expected_upgrade_count) {
                    Ok(plan) => plan, Err(error) => return Ok(Err(error)),
                };
                let previous_entry_id = previous_entry_id.ok_or(DurabilityError::InvalidStoredState)?;
                let character_revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                    .bind(frozen.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                let character_revision: u64 = character_revision.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
                let entry = ForgeDustEntry {
                    entry_id: frozen.entry_id, previous_entry_id: Some(previous_entry_id), character_id: frozen.character_id,
                    kind: ForgeDustEntryKind::LimitRaise, cause_occurrence_id: frozen.request.occurrence_id,
                    transaction_id: frozen.transaction_id, amount: plan.amount, lost: 0, before, after: plan.after,
                };
                let tuple = tx.tuple();
                let message = dust_limit_message(&frozen, &fence, command, character_revision, &entry);
                let envelope = match dust_audit::encode_forge_dust_limit_event(mint_audit::MintEventIdentity {
                    event_id: frozen.event_id, transaction_id: frozen.transaction_id,
                    occurred_at_unix_ms: frozen.occurred_at_unix_ms, server_build_id: SERVER_BUILD_ID,
                }, message, tuple.tuple()) {
                    Ok(value) => value,
                    Err(mint_audit::AuditError::InvalidInput) => return Ok(Err(ForgeDustLimitError::InvalidInput)),
                    Err(mint_audit::AuditError::CapacityExceeded) => return Ok(Err(ForgeDustLimitError::CapacityExceeded)),
                };
                apply_dust_limit(&mut tx, &frozen, &entry, &envelope, tuple).await?;
                let receipt = load_dust_limit_receipt(&mut tx, &frozen).await?.ok_or(DurabilityError::InvalidStoredState)?;
                tx.commit(deadline).await?;
                Ok(Ok(ForgeDustLimitOutcome { receipt, replayed: false }))
            })
        }).await?
    }

    /// Receipt-first recovery. The same cause lock waits for any in-flight transaction; None
    /// proves absence after that wait. No live session/Content is reacquired and no dust moves.
    pub async fn reconcile_forge_dust_limit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        candidate: &ForgeDustLimitCandidate,
    ) -> LimitResult<Option<ForgeDustLimitReceipt>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| ForgeDustLimitError::AuthorityRejected)?;
        let frozen = candidate.frozen.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_dust_limit_cause(
                        &mut tx,
                        frozen.character_id,
                        frozen.request.occurrence_id,
                    )
                    .await?;
                    if !matches_dust_limit_reservation(&mut tx, &frozen).await? {
                        return Ok(Err(ForgeDustLimitError::ConflictingOccurrence));
                    }
                    let receipt = load_dust_limit_receipt(&mut tx, &frozen).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(receipt))
                })
            })
            .await?
    }
}

fn plan_dust_limit(
    before: ForgeDustBalance,
    expected: u16,
) -> LimitResult<crate::domain::forge_dust::DustLimitRaise> {
    if before.limit().upgrade_count() != expected {
        return Err(ForgeDustLimitError::StageChanged);
    }
    before.raise_limit().map_err(|error| match error {
        ForgeDustError::InsufficientDust => ForgeDustLimitError::InsufficientDust,
        ForgeDustError::MaximumLimitReached => ForgeDustLimitError::MaximumLimitReached,
        _ => ForgeDustLimitError::Unavailable(DurabilityError::InvalidStoredState),
    })
}

async fn lock_dust_limit_cause(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    occurrence: [u8; 16],
) -> NativeResult<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:forge-dust-limit:' || encode($1::bytea,'hex') || ':' || encode($2::bytea,'hex'),0))")
        .bind(character.as_bytes().as_slice()).bind(occurrence.as_slice()).execute(&mut **tx).await?;
    Ok(())
}

async fn lock_dust_limit_current(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    world: [u8; 16],
    channel: [u8; 16],
) -> NativeResult<bool> {
    // The same lock as the actual control-plane Content activation writer. It precedes the
    // root lock just as the existing Equipment owner, so activation cannot race the binding.
    sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended('oteryn:content-activation:' || encode($1::bytea,'hex')::uuid::text || ':' || encode($2::bytea,'hex')::uuid::text,0))")
        .bind(world.as_slice()).bind(channel.as_slice()).execute(&mut **tx).await?;
    character_item_fence_is_current(tx, node, fence, command, fence.character_id, world, channel)
        .await
}

async fn current_dust_limit_content(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    world: [u8; 16],
    channel: [u8; 16],
) -> NativeResult<Option<[u8; 32]>> {
    let bytes: Option<Vec<u8>> = sqlx::query_scalar("SELECT server_artifact_digest FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid ORDER BY activation_sequence DESC LIMIT 1")
        .bind(world.as_slice()).bind(channel.as_slice()).fetch_optional(&mut **tx).await?;
    bytes
        .map(|value| {
            let digest: [u8; 32] = value
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if digest == [0; 32] {
                return Err(DurabilityError::InvalidStoredState);
            }
            Ok(digest)
        })
        .transpose()
}

async fn lock_limit_balance(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
) -> NativeResult<Option<(ForgeDustBalance, Option<[u8; 16]>)>> {
    lock_balance_row(tx, character.as_bytes().as_slice())
        .await
        .map_err(|error| match error {
            ForgeDustWriteError::Unavailable(error) => error,
            _ => DurabilityError::InvalidStoredState,
        })
}

async fn load_dust_limit_reservation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    occurrence: [u8; 16],
) -> NativeResult<Option<FrozenDustLimit>> {
    let row = sqlx::query("SELECT world_id::text, channel_id::text, transaction_id::text, event_id::text, entry_id::text, occurred_at, content_digest, ruleset_revision, simulation_profile_revision, expected_upgrade_count FROM game_forge_dust_limit_reservations WHERE character_id=encode($1,'hex')::uuid AND occurrence_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(character.as_bytes().as_slice()).bind(occurrence.as_slice()).fetch_optional(&mut **tx).await?;
    row.map(|row| {
        Ok(FrozenDustLimit {
            character_id: character,
            request: ForgeDustLimitRequest {
                occurrence_id: occurrence,
                expected_upgrade_count: u16::try_from(
                    row.try_get::<i16, _>("expected_upgrade_count")?,
                )
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            },
            world_id: uuid_text(row.try_get("world_id")?)?,
            channel_id: uuid_text(row.try_get("channel_id")?)?,
            transaction_id: uuid_text(row.try_get("transaction_id")?)?,
            event_id: uuid_text(row.try_get("event_id")?)?,
            entry_id: uuid_text(row.try_get("entry_id")?)?,
            occurred_at_unix_ms: row.try_get("occurred_at")?,
            content_digest: row
                .try_get::<Vec<u8>, _>("content_digest")?
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            ruleset_revision: row.try_get("ruleset_revision")?,
            simulation_profile_revision: u32::try_from(
                row.try_get::<i64, _>("simulation_profile_revision")?,
            )
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        })
    })
    .transpose()
}

async fn matches_dust_limit_reservation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenDustLimit,
) -> NativeResult<bool> {
    Ok(
        load_dust_limit_reservation(tx, frozen.character_id, frozen.request.occurrence_id)
            .await?
            .as_ref()
            == Some(frozen),
    )
}

async fn insert_dust_limit_reservation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenDustLimit,
) -> NativeResult<()> {
    sqlx::query("INSERT INTO game_forge_dust_limit_reservations(character_id, occurrence_id, world_id, channel_id, transaction_id, event_id, entry_id, occurred_at, content_digest, ruleset_revision, simulation_profile_revision, expected_upgrade_count) VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8,$9,$10,$11,$12)")
        .bind(frozen.character_id.as_bytes().as_slice()).bind(frozen.request.occurrence_id.as_slice()).bind(frozen.world_id.as_slice()).bind(frozen.channel_id.as_slice())
        .bind(frozen.transaction_id.as_slice()).bind(frozen.event_id.as_slice()).bind(frozen.entry_id.as_slice()).bind(frozen.occurred_at_unix_ms)
        .bind(frozen.content_digest.as_slice()).bind(&frozen.ruleset_revision).bind(i64::from(frozen.simulation_profile_revision))
        .bind(i16::try_from(frozen.request.expected_upgrade_count).map_err(|_| DurabilityError::InvalidStoredState)?)
        .execute(&mut **tx).await?;
    Ok(())
}

async fn load_dust_limit_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenDustLimit,
) -> NativeResult<Option<ForgeDustLimitReceipt>> {
    let row = sqlx::query("SELECT outcome, previous_entry_id::text, amount, balance_before, balance_after, dust_limit_before, dust_limit_after, envelope_sha256 FROM game_forge_dust_limit_receipts WHERE character_id=encode($1,'hex')::uuid AND occurrence_id=encode($2,'hex')::uuid")
        .bind(frozen.character_id.as_bytes().as_slice()).bind(frozen.request.occurrence_id.as_slice()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let disposition = match row.try_get::<i16, _>("outcome")? {
        1 => ForgeDustLimitDisposition::Raised,
        2 => ForgeDustLimitDisposition::RevisionChanged,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let envelope_sha256 = row
        .try_get::<Option<Vec<u8>>, _>("envelope_sha256")?
        .map(|value| {
            value
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)
        })
        .transpose()?;
    let entry = if disposition == ForgeDustLimitDisposition::Raised {
        let entry = ForgeDustEntry {
            entry_id: frozen.entry_id,
            character_id: frozen.character_id,
            previous_entry_id: Some(uuid_text(row.try_get("previous_entry_id")?)?),
            kind: ForgeDustEntryKind::LimitRaise,
            cause_occurrence_id: frozen.request.occurrence_id,
            transaction_id: frozen.transaction_id,
            amount: u32::try_from(row.try_get::<i64, _>("amount")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            lost: 0,
            before: balance(&row, "balance_before", "dust_limit_before")?,
            after: balance(&row, "balance_after", "dust_limit_after")?,
        };
        let plan = entry
            .before
            .raise_limit()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if plan.after != entry.after || plan.amount != entry.amount || envelope_sha256.is_none() {
            return Err(DurabilityError::InvalidStoredState);
        }
        Some(entry)
    } else {
        if envelope_sha256.is_some() {
            return Err(DurabilityError::InvalidStoredState);
        }
        None
    };
    Ok(Some(ForgeDustLimitReceipt {
        disposition,
        character_id: frozen.character_id,
        occurrence_id: frozen.request.occurrence_id,
        transaction_id: frozen.transaction_id,
        event_id: frozen.event_id,
        occurred_at_unix_ms: frozen.occurred_at_unix_ms,
        bound_content_digest: frozen.content_digest,
        bound_ruleset_revision: frozen.ruleset_revision.clone(),
        bound_simulation_profile_revision: frozen.simulation_profile_revision,
        entry,
        envelope_sha256,
    }))
}

fn dust_limit_message(
    frozen: &FrozenDustLimit,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    character_revision: u64,
    entry: &ForgeDustEntry,
) -> OneItemForgeDustLimitV1 {
    let state = |value: ForgeDustBalance| OneForgeDustBalanceV1 {
        balance: u32::from(value.balance()),
        dust_limit: u32::from(value.limit().get()),
    };
    OneItemForgeDustLimitV1 {
        character_id: frozen.character_id.as_bytes().to_vec(),
        world_id: frozen.world_id.to_vec(),
        channel_id: frozen.channel_id.to_vec(),
        occurrence_id: frozen.request.occurrence_id.to_vec(),
        entry_id: frozen.entry_id.to_vec(),
        previous_entry_id: entry.previous_entry_id.map(|id| id.to_vec()),
        before: Some(state(entry.before)),
        after: Some(state(entry.after)),
        dust_burned: entry.amount,
        content_digest: frozen.content_digest.to_vec(),
        ruleset_revision: frozen.ruleset_revision.clone(),
        simulation_profile_revision: frozen.simulation_profile_revision,
        game_session_id: command.game_session_id().as_bytes().to_vec(),
        command_id: command.command_id().get(),
        connection_generation: fence.connection_generation.get(),
        character_lease_generation: fence.character_lease_generation,
        scope_ownership_generation: fence.scope_ownership_generation.get(),
        character_revision,
    }
}

async fn apply_dust_limit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenDustLimit,
    entry: &ForgeDustEntry,
    envelope: &[u8],
    tuple: SelectedType2Tuple,
) -> NativeResult<()> {
    let previous = entry
        .previous_entry_id
        .ok_or(DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_character_forge_dust_entries(entry_id,character_id,previous_entry_id,kind,cause,cause_occurrence_id,transaction_id,amount,lost_amount,balance_before,balance_after,dust_limit_before,dust_limit_after) VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$11,$12,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,0,$7,$8,$9,$10)")
        .bind(frozen.entry_id.as_slice()).bind(frozen.character_id.as_bytes().as_slice()).bind(previous.as_slice())
        .bind(frozen.request.occurrence_id.as_slice()).bind(frozen.transaction_id.as_slice()).bind(i64::from(entry.amount))
        .bind(i64::from(entry.before.balance())).bind(i64::from(entry.after.balance())).bind(limit_i16(entry.before.limit())).bind(limit_i16(entry.after.limit())).bind(KIND_LIMIT_RAISE).bind(CAUSE_DUST_LIMIT)
        .execute(&mut **tx).await?;
    let changed = sqlx::query("UPDATE game_character_forge_dust SET balance=$2,dust_limit=$3,last_entry_id=encode($4,'hex')::uuid WHERE character_id=encode($1,'hex')::uuid AND balance=$5 AND dust_limit=$6 AND last_entry_id=encode($7,'hex')::uuid")
        .bind(frozen.character_id.as_bytes().as_slice()).bind(i64::from(entry.after.balance())).bind(limit_i16(entry.after.limit())).bind(frozen.entry_id.as_slice())
        .bind(i64::from(entry.before.balance())).bind(limit_i16(entry.before.limit())).bind(previous.as_slice()).execute(&mut **tx).await?;
    if changed.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,$3,$4,$5,NULL,$6,$6+$7,$8,sha256($8),1)")
        .bind(frozen.event_id.as_slice()).bind(frozen.transaction_id.as_slice()).bind(i64::from(mint_audit::EVENT_TYPE_ID))
        .bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(frozen.occurred_at_unix_ms)
        .bind(mint_audit::AUDIT_RETENTION_P90D_MS).bind(envelope).execute(&mut **tx).await?;
    let envelope_sha256: [u8; 32] = Sha256::digest(envelope).into();
    insert_dust_limit_receipt(tx, frozen, Some(entry), Some(envelope_sha256)).await?;
    sqlx::query("INSERT INTO game_forge_dust_limit_history(character_id,occurrence_id,history_ordinal) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,COALESCE((SELECT history_ordinal FROM game_forge_dust_limit_history WHERE character_id=encode($1,'hex')::uuid ORDER BY history_ordinal DESC LIMIT 1),0)+1")
        .bind(frozen.character_id.as_bytes().as_slice()).bind(frozen.request.occurrence_id.as_slice()).execute(&mut **tx).await?;
    // The index bounds the retained read to at most 1001 rows; receipts and ledger survive.
    sqlx::query("DELETE FROM game_forge_dust_limit_history WHERE character_id=encode($1,'hex')::uuid AND history_ordinal IN (SELECT history_ordinal FROM game_forge_dust_limit_history WHERE character_id=encode($1,'hex')::uuid ORDER BY history_ordinal DESC OFFSET $2)")
        .bind(frozen.character_id.as_bytes().as_slice()).bind(dust_audit::DUST_LIMIT_HISTORY_ROWS_MAX).execute(&mut **tx).await?;
    Ok(())
}

async fn insert_dust_limit_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenDustLimit,
    entry: Option<&ForgeDustEntry>,
    envelope_sha256: Option<[u8; 32]>,
) -> NativeResult<()> {
    sqlx::query("INSERT INTO game_forge_dust_limit_receipts(character_id,occurrence_id,transaction_id,event_id,entry_id,outcome,previous_entry_id,amount,balance_before,balance_after,dust_limit_before,dust_limit_after,envelope_sha256) VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,encode($7,'hex')::uuid,$8,$9,$10,$11,$12,$13)")
        .bind(frozen.character_id.as_bytes().as_slice()).bind(frozen.request.occurrence_id.as_slice()).bind(frozen.transaction_id.as_slice()).bind(frozen.event_id.as_slice()).bind(frozen.entry_id.as_slice())
        .bind(if entry.is_some() { 1i16 } else { 2i16 }).bind(entry.and_then(|e| e.previous_entry_id.map(|id| id.to_vec())))
        .bind(entry.map(|e| i64::from(e.amount))).bind(entry.map(|e| i64::from(e.before.balance()))).bind(entry.map(|e| i64::from(e.after.balance())))
        .bind(entry.map(|e| limit_i16(e.before.limit()))).bind(entry.map(|e| limit_i16(e.after.limit()))).bind(envelope_sha256.map(|hash| hash.to_vec()))
        .execute(&mut **tx).await?;
    Ok(())
}
