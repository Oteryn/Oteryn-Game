//! DAILY_REWARD0 §3–5: candidate private storage and read/lock boundary only.
//!
//! No claim, grant, delivery, schema-installation or current-authority API lives
//! here. The eventual inseparable claim writer must lock native admission and
//! recovery evidence, fence the Character/session and lock its root, resolve the
//! current World reset and shrine/content facts, then lock Account+World jokers.
//! Claim/state/joker/ledger/lines and every available value sink commit together.
//! A request, stored epoch, receipt, arithmetic plan or row lock proves none of
//! that authority. Reconciliation also needs the native current recovery fence.

use super::DurabilityError;
use super::character_progression::{numeric_u64, uuid_text};
use super::item_mint::TypedDefinitionRef;
use crate::domain::daily_reward::RewardState;
use crate::domain::{AccountId, CharacterId, WorldId};
use sqlx::{Postgres, Row, Transaction};

/// Migration material, NOT executed by this module. CP must integrate the real
/// writer, grants, audit/cause registry and migration atomically before use.
/// No World reset table/epoch source, reward content or wildcard balance is
/// invented. Numeric epochs retain the native map-reset u64 representation.
pub const DAILY_REWARD_SCHEMA_CANDIDATE: &str = r#"
CREATE TABLE game_character_daily_rewards (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE RESTRICT,
 world_id UUID NOT NULL,
 lane_day SMALLINT NOT NULL CHECK(lane_day BETWEEN 1 AND 7),
 streak INTEGER NOT NULL CHECK(streak BETWEEN 0 AND 100000),
 last_claim_epoch NUMERIC(20,0) CHECK(last_claim_epoch BETWEEN 0 AND 18446744073709551615),
 CHECK((last_claim_epoch IS NULL) = (streak = 0)),
 PRIMARY KEY(character_id,world_id)
);
CREATE TABLE game_account_daily_reward_jokers (
 account_id UUID NOT NULL, world_id UUID NOT NULL,
 jokers SMALLINT NOT NULL CHECK(jokers BETWEEN 0 AND 3),
 last_grant_month DATE CHECK(EXTRACT(DAY FROM last_grant_month)=1),
 last_entry_id NUMERIC(20,0) NOT NULL CHECK(last_entry_id BETWEEN 0 AND 18446744073709551615),
 PRIMARY KEY(account_id,world_id)
);
CREATE TABLE game_account_daily_reward_joker_entries (
 account_id UUID NOT NULL, world_id UUID NOT NULL,
 entry_id NUMERIC(20,0) NOT NULL CHECK(entry_id BETWEEN 1 AND 18446744073709551615),
 transaction_id UUID NOT NULL,
 kind TEXT NOT NULL CHECK(kind IN ('GRANT','SPEND')),
 amount SMALLINT NOT NULL CHECK(amount BETWEEN 1 AND 3),
 balance_after SMALLINT NOT NULL CHECK(balance_after BETWEEN 0 AND 3),
 PRIMARY KEY(account_id,world_id,entry_id),
 FOREIGN KEY(account_id,world_id) REFERENCES game_account_daily_reward_jokers ON DELETE RESTRICT
);
CREATE TABLE game_character_daily_reward_claims (
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE RESTRICT,
 world_id UUID NOT NULL,
 reset_epoch NUMERIC(20,0) NOT NULL CHECK(reset_epoch BETWEEN 0 AND 18446744073709551615),
 transaction_id UUID NOT NULL UNIQUE,
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),
 lane_day SMALLINT NOT NULL CHECK(lane_day BETWEEN 1 AND 7),
 streak_before INTEGER NOT NULL CHECK(streak_before BETWEEN 0 AND 100000),
 streak_after INTEGER NOT NULL CHECK(streak_after BETWEEN 1 AND 100000),
 jokers_spent SMALLINT NOT NULL CHECK(jokers_spent BETWEEN 0 AND 3),
 premium BOOLEAN NOT NULL,
 PRIMARY KEY(character_id,world_id,reset_epoch)
);
CREATE TABLE game_character_daily_reward_lines (
 character_id UUID NOT NULL, world_id UUID NOT NULL, reset_epoch NUMERIC(20,0) NOT NULL,
 line INTEGER NOT NULL CHECK(line >= 0),
 kind TEXT NOT NULL CHECK(kind IN ('ITEM','PREY_WILDCARD','XP_BOOST')),
 definition_family TEXT, definition_key TEXT, definition_revision TEXT,
 quantity BIGINT NOT NULL CHECK(quantity > 0),
 status TEXT NOT NULL CHECK(status IN ('PENDING','DELIVERED')),
 delivering_transaction_id UUID,
 uncredited_excess BIGINT NOT NULL DEFAULT 0 CHECK(uncredited_excess BETWEEN 0 AND quantity),
 CHECK((kind='ITEM' AND definition_family IS NOT NULL AND definition_key IS NOT NULL
        AND definition_revision IS NOT NULL AND length(definition_family)>0
        AND length(definition_key)>0 AND length(definition_revision)>0)
    OR (kind<>'ITEM' AND definition_family IS NULL AND definition_key IS NULL AND definition_revision IS NULL)),
 CHECK((status='PENDING' AND delivering_transaction_id IS NULL AND uncredited_excess=0)
    OR (status='DELIVERED' AND delivering_transaction_id IS NOT NULL)),
 CHECK(kind='PREY_WILDCARD' OR uncredited_excess=0),
 PRIMARY KEY(character_id,world_id,reset_epoch,line),
 FOREIGN KEY(character_id,world_id,reset_epoch) REFERENCES game_character_daily_reward_claims ON DELETE RESTRICT
);
CREATE FUNCTION game_daily_reward_line_terminal() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF OLD.status='DELIVERED' THEN
  RAISE EXCEPTION 'daily reward line is terminal' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_daily_reward_line_terminal BEFORE UPDATE ON game_character_daily_reward_lines
 FOR EACH ROW EXECUTE FUNCTION game_daily_reward_line_terminal();
REVOKE ALL ON game_character_daily_rewards,game_account_daily_reward_jokers,
 game_account_daily_reward_joker_entries,game_character_daily_reward_claims,
 game_character_daily_reward_lines FROM PUBLIC,oteryn_game_runtime;
GRANT SELECT,INSERT ON game_character_daily_rewards,game_account_daily_reward_jokers,
 game_account_daily_reward_joker_entries,game_character_daily_reward_claims,
 game_character_daily_reward_lines TO oteryn_game_runtime;
GRANT UPDATE(lane_day,streak,last_claim_epoch) ON game_character_daily_rewards TO oteryn_game_runtime;
GRANT UPDATE(jokers,last_grant_month,last_entry_id) ON game_account_daily_reward_jokers TO oteryn_game_runtime;
GRANT UPDATE(status,delivering_transaction_id,uncredited_excess) ON game_character_daily_reward_lines TO oteryn_game_runtime;
"#;

/// A lookup key, never a proof that its World epoch is current.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClaimKey {
    pub character_id: CharacterId,
    pub world_id: WorldId,
    pub reset_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JokerState {
    pub jokers: u8,
    /// Stored UTC calendar month, `YYYY-MM-DD` (first day), not a grant proof.
    pub last_grant_month: Option<String>,
    pub last_entry_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewardPart {
    Item(TypedDefinitionRef),
    PreyWildcard,
    XpBoost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryState {
    Pending,
    /// A fully capped wildcard line can be terminal with zero credited value:
    /// quantity remains positive and uncredited_excess equals quantity.
    Delivered {
        transaction_id: [u8; 16],
        uncredited_excess: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewardLine {
    pub line: u32,
    pub part: RewardPart,
    pub quantity: u64,
    pub delivery: DeliveryState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimReceipt {
    pub key: ClaimKey,
    pub transaction_id: [u8; 16],
    pub request_binding: [u8; 32],
    pub lane_day: u8,
    pub streak_before: u32,
    pub streak_after: u32,
    pub jokers_spent: u8,
    /// Historical read, not current Premium evidence. Free remains required
    /// until the accepted PREM-3 owner qualifies the eventual claim writer.
    pub premium: bool,
    pub lines: Vec<RewardLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimLookup {
    Missing,
    /// Do not return another request's receipt or start a second grant.
    BindingMismatch,
    Replay(ClaimReceipt),
}

/// Raw caller-transaction read. Caller must separately qualify access/privacy.
pub async fn read_reward_state(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    world: WorldId,
) -> Result<Option<RewardState>, DurabilityError> {
    let row = sqlx::query("SELECT lane_day,streak,last_claim_epoch::text AS last_claim_epoch FROM game_character_daily_rewards WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    row.map(|row| {
        let epoch: Option<String> = row.try_get("last_claim_epoch")?;
        let state = RewardState {
            lane_day: u8::try_from(row.try_get::<i16, _>("lane_day")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            streak: u32::try_from(row.try_get::<i32, _>("streak")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            last_claim_epoch: epoch
                .map(|v| v.parse())
                .transpose()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        };
        state
            .validate()
            .map_err(|_| DurabilityError::InvalidStoredState)
    })
    .transpose()
}

/// Lock only an existing Account+World joker row, after the native Character
/// root lock. `None` needs initialization by the future atomic writer; this
/// function never manufactures zero balance or assumes a month was granted.
pub async fn lock_joker_state(
    tx: &mut Transaction<'_, Postgres>,
    account: AccountId,
    world: WorldId,
) -> Result<Option<JokerState>, DurabilityError> {
    let row=sqlx::query("SELECT jokers,last_grant_month::text AS last_grant_month,last_entry_id::text AS last_entry_id FROM game_account_daily_reward_jokers WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(account.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    row.map(|row| {
        let jokers = u8::try_from(row.try_get::<i16, _>("jokers")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if jokers > 3 {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(JokerState {
            jokers,
            last_grant_month: row.try_get("last_grant_month")?,
            last_entry_id: numeric_u64(&row, "last_entry_id")?,
        })
    })
    .transpose()
}

/// Historical receipt classification by exact SHA-256 request binding. The
/// eventual writer must freeze that digest from validated canonical picks and
/// planned outputs before its first attempt. No fresh claim is created here.
pub async fn read_claim(
    tx: &mut Transaction<'_, Postgres>,
    key: ClaimKey,
    request_binding: &[u8; 32],
) -> Result<ClaimLookup, DurabilityError> {
    let row=sqlx::query("SELECT transaction_id::text AS transaction_id,request_binding,lane_day,streak_before,streak_after,jokers_spent,premium FROM game_character_daily_reward_claims WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND reset_epoch=$3::text::numeric")
        .bind(key.character_id.as_bytes().as_slice()).bind(key.world_id.as_bytes().as_slice()).bind(key.reset_epoch.to_string()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(ClaimLookup::Missing);
    };
    let binding: [u8; 32] = row
        .try_get::<Vec<u8>, _>("request_binding")?
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if &binding != request_binding {
        return Ok(ClaimLookup::BindingMismatch);
    }
    let lane_day = u8::try_from(row.try_get::<i16, _>("lane_day")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let streak_before = u32::try_from(row.try_get::<i32, _>("streak_before")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let streak_after = u32::try_from(row.try_get::<i32, _>("streak_after")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let jokers_spent = u8::try_from(row.try_get::<i16, _>("jokers_spent")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if !(1..=7).contains(&lane_day)
        || streak_before > 100000
        || !(1..=100000).contains(&streak_after)
        || jokers_spent > 3
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let lines = load_lines(tx, key, false).await?;
    if lines.is_empty() {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(ClaimLookup::Replay(ClaimReceipt {
        key,
        transaction_id: uuid_text(&row.try_get::<String, _>("transaction_id")?)?,
        request_binding: binding,
        lane_day,
        streak_before,
        streak_after,
        jokers_spent,
        premium: row.try_get("premium")?,
        lines,
    }))
}

/// Lock pending lines in deterministic line order in the caller's existing
/// transaction, after native fence/root/joker locks. No SKIP LOCKED: a losing
/// caller waits and PostgreSQL rechecks status after the earlier writer commits.
/// Immutable claims need no UPDATE grant. The eventual writer must create the
/// complete line set with its claim and never append lines on replay. Hold these
/// locks through the atomic sink+terminal-status commit/rollback; locks alone
/// authorize no delivery or grant.
pub async fn lock_pending_lines(
    tx: &mut Transaction<'_, Postgres>,
    key: ClaimKey,
) -> Result<Vec<RewardLine>, DurabilityError> {
    load_lines(tx, key, true).await
}

async fn load_lines(
    tx: &mut Transaction<'_, Postgres>,
    key: ClaimKey,
    pending_only: bool,
) -> Result<Vec<RewardLine>, DurabilityError> {
    let suffix = if pending_only {
        " AND status='PENDING' ORDER BY line FOR UPDATE"
    } else {
        " ORDER BY line"
    };
    let query = format!(
        "SELECT line,kind,definition_family,definition_key,definition_revision,quantity::text AS quantity,status,delivering_transaction_id::text AS delivering_transaction_id,uncredited_excess::text AS uncredited_excess FROM game_character_daily_reward_lines WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND reset_epoch=$3::text::numeric{suffix}"
    );
    let rows = sqlx::query(sqlx::AssertSqlSafe(query))
        .bind(key.character_id.as_bytes().as_slice())
        .bind(key.world_id.as_bytes().as_slice())
        .bind(key.reset_epoch.to_string())
        .fetch_all(&mut **tx)
        .await?;
    rows.into_iter()
        .map(|row| {
            let family: Option<String> = row.try_get("definition_family")?;
            let definition: Option<String> = row.try_get("definition_key")?;
            let revision: Option<String> = row.try_get("definition_revision")?;
            let part = match (
                row.try_get::<String, _>("kind")?.as_str(),
                family,
                definition,
                revision,
            ) {
                ("ITEM", Some(family), Some(production_key), Some(revision_ref))
                    if !family.is_empty()
                        && !production_key.is_empty()
                        && !revision_ref.is_empty() =>
                {
                    RewardPart::Item(TypedDefinitionRef {
                        family,
                        production_key,
                        revision_ref,
                    })
                }
                ("PREY_WILDCARD", None, None, None) => RewardPart::PreyWildcard,
                ("XP_BOOST", None, None, None) => RewardPart::XpBoost,
                _ => return Err(DurabilityError::InvalidStoredState),
            };
            let quantity = numeric_u64(&row, "quantity")?;
            let excess = numeric_u64(&row, "uncredited_excess")?;
            if quantity == 0
                || excess > quantity
                || (excess != 0 && !matches!(part, RewardPart::PreyWildcard))
            {
                return Err(DurabilityError::InvalidStoredState);
            }
            let transaction: Option<String> = row.try_get("delivering_transaction_id")?;
            let delivery = match (row.try_get::<String, _>("status")?.as_str(), transaction) {
                ("PENDING", None) if excess == 0 => DeliveryState::Pending,
                ("DELIVERED", Some(transaction)) => DeliveryState::Delivered {
                    transaction_id: uuid_text(&transaction)?,
                    uncredited_excess: excess,
                },
                _ => return Err(DurabilityError::InvalidStoredState),
            };
            Ok(RewardLine {
                line: u32::try_from(row.try_get::<i32, _>("line")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                part,
                quantity,
                delivery,
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "daily_reward_tests.rs"]
mod tests;
