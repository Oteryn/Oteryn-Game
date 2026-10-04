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
        };
        if !consistent {
            return Err(ForgeDustAuditError::Inconsistent);
        }
        match entry.kind {
            ForgeDustEntryKind::Gain => {
                audit.credited += amount - lost;
                audit.lost += lost;
            }
            ForgeDustEntryKind::Spend => audit.spent += amount,
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
