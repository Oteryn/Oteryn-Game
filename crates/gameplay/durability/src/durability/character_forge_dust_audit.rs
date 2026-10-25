//! Read-only audit of one Character's forge dust ledger (FORGE-1a, migration 0059).
//!
//! [`audit_forge_dust_ledger`] walks the whole chain from its first entry and proves what the
//! commit-time guards prove one link at a time: one linear chain, each entry's before values
//! equal to its predecessor's after values, and the balance row equal to the latest entry. It
//! also returns the ledger totals, so value created and destroyed is reconciled against the
//! balance: `credited - spent = balance`. It is an integrity and investigation read, never on a
//! gameplay path; the chain length is bounded by `max_entries`.

use super::DurabilityError;
use super::character_forge_dust::{ForgeDustEntry, ForgeDustEntryKind, balance, entry_from_row};
use super::character_progression::uuid_text;
use crate::domain::CharacterId;
use crate::domain::forge_dust::ForgeDustBalance;
use sqlx::Row;
use sqlx::postgres::PgConnection;
use std::collections::HashMap;

/// A consistent ledger and its totals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgeDustLedgerAudit {
    pub balance: ForgeDustBalance,
    pub entries: u64,
    /// Dust credited by gains (requested minus lost).
    pub credited: u64,
    /// Dust lost by gains above the limit.
    pub lost: u64,
    pub spent: u64,
}

#[derive(Debug)]
pub enum ForgeDustAuditError {
    /// The chain is longer than `max_entries`.
    TooManyEntries,
    /// The stored ledger is not one consistent chain ending at the balance row.
    Inconsistent,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for ForgeDustAuditError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl From<sqlx::Error> for ForgeDustAuditError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}

impl std::fmt::Display for ForgeDustAuditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyEntries => {
                formatter.write_str("forge dust ledger exceeds the audit bound")
            }
            Self::Inconsistent => formatter.write_str("forge dust ledger is inconsistent"),
            Self::Unavailable(error) => {
                write!(formatter, "forge dust audit storage failed: {error:?}")
            }
        }
    }
}

impl std::error::Error for ForgeDustAuditError {}

pub async fn audit_forge_dust_ledger(
    connection: &mut PgConnection,
    character_id: CharacterId,
    max_entries: u32,
) -> Result<ForgeDustLedgerAudit, ForgeDustAuditError> {
    let character = character_id.as_bytes().as_slice();
    let row = sqlx::query(
        "SELECT balance, dust_limit, last_entry_id::text FROM game_character_forge_dust \
          WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(character)
    .fetch_optional(&mut *connection)
    .await?;
    let (stored, last) = match row {
        None => (ForgeDustBalance::default(), None),
        Some(row) => (
            balance(&row, "balance", "dust_limit")?,
            row.try_get::<Option<String>, _>("last_entry_id")?
                .map(|id| uuid_text(&id))
                .transpose()?,
        ),
    };
    let rows = sqlx::query(
        "SELECT entry_id::text, previous_entry_id::text, character_id::text, kind, \
                cause_occurrence_id::text, transaction_id::text, amount, lost_amount, \
                balance_before, balance_after, dust_limit_before, dust_limit_after \
           FROM game_character_forge_dust_entries \
          WHERE character_id = encode($1,'hex')::uuid LIMIT $2",
    )
    .bind(character)
    .bind(i64::from(max_entries) + 1)
    .fetch_all(&mut *connection)
    .await?;
    if rows.len() > usize::try_from(max_entries).unwrap_or(usize::MAX) {
        return Err(ForgeDustAuditError::TooManyEntries);
    }
    let mut first = None;
    let mut successors: HashMap<[u8; 16], ForgeDustEntry> = HashMap::new();
    for row in &rows {
        let entry = entry_from_row(row)?;
        let duplicate = match entry.previous_entry_id {
            None => first.replace(entry).is_some(),
            Some(previous) => successors.insert(previous, entry).is_some(),
        };
        if duplicate {
            return Err(ForgeDustAuditError::Inconsistent);
        }
    }

    let mut audit = ForgeDustLedgerAudit {
        balance: ForgeDustBalance::default(),
        entries: 0,
        credited: 0,
        lost: 0,
        spent: 0,
    };
    let mut latest = None;
    let mut next = first;
    while let Some(entry) = next {
        if entry.before != audit.balance {
            return Err(ForgeDustAuditError::Inconsistent);
        }
        let amount = u64::from(entry.amount);
        let lost = u64::from(entry.lost);
        let consistent = match entry.kind {
            ForgeDustEntryKind::Gain => {
                entry
                    .before
                    .gain(entry.amount)
                    .ok()
                    .map(|gain| (gain.after, gain.lost))
                    == Some((entry.after, entry.lost))
            }
            ForgeDustEntryKind::Spend => {
                entry.lost == 0
                    && entry
                        .before
                        .spend(entry.amount)
                        .ok()
                        .map(|spend| spend.after)
                        == Some(entry.after)
            }
            ForgeDustEntryKind::LimitRaise => {
                entry.lost == 0
                    && entry
                        .before
                        .raise_limit()
                        .ok()
                        .map(|raise| (raise.after, raise.amount))
                        == Some((entry.after, entry.amount))
            }
        };
        if !consistent {
            return Err(ForgeDustAuditError::Inconsistent);
        }
        match entry.kind {
            ForgeDustEntryKind::Gain => {
                audit.credited += amount - lost;
                audit.lost += lost;
            }
            ForgeDustEntryKind::Spend | ForgeDustEntryKind::LimitRaise => audit.spent += amount,
        }
        audit.entries += 1;
        audit.balance = entry.after;
        latest = Some(entry.entry_id);
        next = successors.remove(&entry.entry_id);
    }
    // Every entry is on the chain, the row names its end, and the totals reconcile.
    if !successors.is_empty()
        || latest != last
        || audit.balance != stored
        || audit.credited.checked_sub(audit.spent) != Some(u64::from(stored.balance()))
    {
        return Err(ForgeDustAuditError::Inconsistent);
    }
    Ok(audit)
}

// The closed native LIMIT_RAISE audit proposed by this private continuation packet.
use super::item_mint_audit::{
    self as mint, AuditError, EventEnvelopeV1, OneItemOperationV1, OneItemTransactionV1,
    TransactionEventRefV1, Type2EventTuple, check_uuid_v7, encode_bounded,
};
use crate::domain::forge_dust::DustLimit;
use prost::Message;
use sha2::{Digest, Sha256};

/// D1230 is the complete ruleset of this material-free operation. An update is a new
/// bound revision, never a reinterpretation of a retained occurrence.
pub const DUST_LIMIT_RULESET_REVISION: &str = "D1230";
/// Exact proposed closed shape: no item participant; two value effects (debit and limit).
pub const DUST_LIMIT_VALUE_LINES_MAX: u32 = 2;
pub const DUST_LIMIT_HISTORY_ROWS_MAX: i64 = 1_000;

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneForgeDustBalanceV1 {
    #[prost(uint32, tag = "1")]
    pub balance: u32,
    #[prost(uint32, tag = "2")]
    pub dust_limit: u32,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemForgeDustLimitV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub character_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub world_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub channel_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub occurrence_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub entry_id: Vec<u8>,
    #[prost(bytes = "vec", optional, tag = "6")]
    pub previous_entry_id: Option<Vec<u8>>,
    #[prost(message, optional, tag = "7")]
    pub before: Option<OneForgeDustBalanceV1>,
    #[prost(message, optional, tag = "8")]
    pub after: Option<OneForgeDustBalanceV1>,
    #[prost(uint32, tag = "9")]
    pub dust_burned: u32,
    /// Exact active server artifact, read independently under its activation lock.
    #[prost(bytes = "vec", tag = "10")]
    pub content_digest: Vec<u8>,
    #[prost(string, tag = "11")]
    pub ruleset_revision: String,
    #[prost(uint32, tag = "12")]
    pub simulation_profile_revision: u32,
    #[prost(bytes = "vec", tag = "13")]
    pub game_session_id: Vec<u8>,
    #[prost(uint64, tag = "14")]
    pub command_id: u64,
    #[prost(uint64, tag = "15")]
    pub connection_generation: u64,
    #[prost(uint64, tag = "16")]
    pub character_lease_generation: u64,
    #[prost(uint64, tag = "17")]
    pub scope_ownership_generation: u64,
    /// Observed under the root lock, unchanged by this non-item asset operation.
    #[prost(uint64, tag = "18")]
    pub character_revision: u64,
}

fn audit_balance(value: Option<&OneForgeDustBalanceV1>) -> Result<ForgeDustBalance, AuditError> {
    let value = value.ok_or(AuditError::InvalidInput)?;
    let limit = u16::try_from(value.dust_limit)
        .ok()
        .and_then(|n| DustLimit::new(n).ok())
        .ok_or(AuditError::InvalidInput)?;
    let amount = u16::try_from(value.balance).map_err(|_| AuditError::InvalidInput)?;
    ForgeDustBalance::new(amount, limit).map_err(|_| AuditError::InvalidInput)
}

pub fn check_forge_dust_limit(value: &OneItemForgeDustLimitV1) -> Result<(), AuditError> {
    for id in [
        &value.character_id,
        &value.world_id,
        &value.channel_id,
        &value.occurrence_id,
        &value.entry_id,
        &value.game_session_id,
    ] {
        check_uuid_v7(id)?;
    }
    if let Some(previous) = &value.previous_entry_id {
        check_uuid_v7(previous)?;
        if previous == &value.entry_id {
            return Err(AuditError::InvalidInput);
        }
    }
    let before = audit_balance(value.before.as_ref())?;
    let after = audit_balance(value.after.as_ref())?;
    let plan = before.raise_limit().map_err(|_| AuditError::InvalidInput)?;
    if plan.after != after || plan.amount != value.dust_burned
        || value.previous_entry_id.is_none() // a positive balance requires an earlier gain
        || value.content_digest.len() != 32 || value.content_digest.iter().all(|b| *b == 0)
        || value.ruleset_revision != DUST_LIMIT_RULESET_REVISION
        || value.simulation_profile_revision == 0
        || value.command_id == 0 || value.connection_generation == 0
        || value.character_lease_generation == 0 || value.scope_ownership_generation == 0
        || value.character_revision == 0 || value.world_id == value.channel_id
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

pub fn decode_forge_dust_limit_payload(wire: &[u8]) -> Result<OneItemForgeDustLimitV1, AuditError> {
    if wire.len() > mint::RL07_PAYLOAD_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let transaction = OneItemTransactionV1::decode(wire).map_err(|_| AuditError::InvalidInput)?;
    if transaction.interpretation_revision != mint::INTERPRETATION_REVISION
        || transaction.encode_to_vec() != wire
    {
        return Err(AuditError::InvalidInput);
    }
    let Some(OneItemOperationV1::ForgeDustLimit(value)) = transaction.operation else {
        return Err(AuditError::InvalidInput);
    };
    check_forge_dust_limit(&value)?;
    Ok(value)
}

pub fn decode_forge_dust_limit_envelope(
    wire: &[u8],
) -> Result<(EventEnvelopeV1, OneItemForgeDustLimitV1), AuditError> {
    let envelope = mint::decode_common_envelope(wire)?;
    let value = decode_forge_dust_limit_payload(&envelope.payload)?;
    mint::check_technical_text(&envelope.server_build_id)?;
    let transaction = envelope
        .transaction_event
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    if envelope.event_id == transaction.transaction_id
        || value.entry_id == envelope.event_id
        || value.entry_id == transaction.transaction_id
        || envelope.occurred_at_unix_ms > i64::MAX - mint::AUDIT_RETENTION_P90D_MS
    {
        return Err(AuditError::InvalidInput);
    }
    if envelope.world_id.as_deref() != Some(value.world_id.as_slice())
        || envelope.channel_id.as_deref() != Some(value.channel_id.as_slice())
        || envelope.game_session_id.as_deref() != Some(value.game_session_id.as_slice())
        || envelope.command_id != Some(value.command_id)
        || envelope.connection_generation != Some(value.connection_generation)
        || envelope.ruleset_revision.as_deref() != Some(value.ruleset_revision.as_str())
        || envelope.privacy_class != mint::RESTRICTED_PLAYER_LINKED
        || envelope.instance_id.is_some()
        || envelope.node_id.is_some()
        || envelope.runtime_order.is_some()
        || envelope.operation_id.is_some()
        || envelope.correlation_id.is_some()
        || envelope.causation.is_some()
        || envelope.analytics_actor.is_some()
        || envelope.protocol_major.is_some()
        || envelope.content_revision.is_some()
    {
        return Err(AuditError::InvalidInput);
    }
    Ok((envelope, value))
}

pub fn encode_forge_dust_limit_event(
    identity: mint::MintEventIdentity<'_>,
    value: OneItemForgeDustLimitV1,
    tuple: Type2EventTuple,
) -> Result<Vec<u8>, AuditError> {
    check_uuid_v7(&identity.event_id)?;
    check_uuid_v7(&identity.transaction_id)?;
    mint::check_technical_text(identity.server_build_id)?;
    if identity.event_id == identity.transaction_id
        || identity.occurred_at_unix_ms <= 0
        || identity.occurred_at_unix_ms > i64::MAX - mint::AUDIT_RETENTION_P90D_MS
    {
        return Err(AuditError::InvalidInput);
    }
    check_forge_dust_limit(&value)?;
    let mut envelope = EventEnvelopeV1 {
        envelope_revision: mint::ENVELOPE_REVISION,
        event_id: identity.event_id.to_vec(),
        event_type_id: mint::EVENT_TYPE_ID,
        event_schema_revision: tuple.schema_revision(),
        durability_class: mint::DURABLE_AUDIT,
        privacy_class: mint::RESTRICTED_PLAYER_LINKED,
        retention_profile_id: tuple.retention_profile_id().into(),
        occurred_at_unix_ms: identity.occurred_at_unix_ms,
        world_id: Some(value.world_id.clone()),
        channel_id: Some(value.channel_id.clone()),
        game_session_id: Some(value.game_session_id.clone()),
        connection_generation: Some(value.connection_generation),
        command_id: Some(value.command_id),
        ruleset_revision: Some(value.ruleset_revision.clone()),
        transaction_event: Some(TransactionEventRefV1 {
            transaction_id: identity.transaction_id.to_vec(),
            ordinal: 1,
            count: 1,
        }),
        server_build_id: identity.server_build_id.into(),
        ..EventEnvelopeV1::default()
    };
    envelope.payload = encode_bounded(
        &OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::ForgeDustLimit(value)),
        },
        mint::RL07_PAYLOAD_BYTES_MAX,
    )?;
    envelope.payload_sha256 = Sha256::digest(&envelope.payload).to_vec();
    let wire = encode_bounded(&envelope, mint::RL07_ENVELOPE_BYTES_MAX)?;
    if decode_forge_dust_limit_envelope(&wire)?.0 != envelope {
        return Err(AuditError::InvalidInput);
    }
    Ok(wire)
}
