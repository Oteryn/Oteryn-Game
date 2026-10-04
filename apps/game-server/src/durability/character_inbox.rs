//! The CharacterInbox read helpers and the typed delivery error (INBOX-1a, migration 0076;
//! MARKET-0 §5, §7 and §8, SOCIAL-MAP-PACKETS-1 §1.12).
//!
//! The Inbox is one item location per (Character, World), without a channel. Items arrive only
//! through `game_character_inbox_deliver`, a SECURITY DEFINER function that no runtime role may
//! execute: each caller (HOUSE-1b, MARKET-1) calls it from its own SECURITY DEFINER function and
//! audits the delivery in its own event from the returned `(character_id, ordinal)`. The
//! function raises one typed SQLSTATE per refusal, which [`CharacterInboxDeliveryError`]
//! classifies for the callers and their tests. This module has no writer.

use super::DurabilityError;
use super::character_progression::{numeric_u64, uuid_text};
use sqlx::Row;
use sqlx::postgres::PgConnection;

/// The SQLSTATEs `game_character_inbox_deliver` and the Inbox guards raise (migration 0076).
pub const SQLSTATE_ITEM_HAS_CONTENTS: &str = "OTI01";
pub const SQLSTATE_WORLD_MISMATCH: &str = "OTI02";
pub const SQLSTATE_ITEM_IN_ANOTHER_LOCATION: &str = "OTI03";
pub const SQLSTATE_UNKNOWN_CAUSE_KIND: &str = "OTI04";
pub const SQLSTATE_ALREADY_DELIVERED: &str = "OTI05";

/// One refused or failed delivery. A refusal aborts the caller's transaction; nothing of it
/// commits.
#[derive(Debug)]
pub enum CharacterInboxDeliveryError {
    /// The item holds other items; only a whole item without contents is delivered.
    ItemHasContents,
    /// The Character, the item or the call name different Worlds, or the Character has no root.
    WorldMismatch,
    /// At commit the item was still in another location (the exclusivity guard).
    ItemInAnotherLocation,
    /// The cause kind has no row in `game_character_inbox_cause_kinds`.
    UnknownCauseKind,
    /// The same (item, cause kind, cause ref) delivery record already exists.
    AlreadyDelivered,
    Unavailable(DurabilityError),
}

impl CharacterInboxDeliveryError {
    /// The typed refusal for a SQLSTATE, or `None` when the code is not an Inbox refusal.
    pub fn from_sqlstate(code: &str) -> Option<Self> {
        match code {
            SQLSTATE_ITEM_HAS_CONTENTS => Some(Self::ItemHasContents),
            SQLSTATE_WORLD_MISMATCH => Some(Self::WorldMismatch),
            SQLSTATE_ITEM_IN_ANOTHER_LOCATION => Some(Self::ItemInAnotherLocation),
            SQLSTATE_UNKNOWN_CAUSE_KIND => Some(Self::UnknownCauseKind),
            SQLSTATE_ALREADY_DELIVERED => Some(Self::AlreadyDelivered),
            _ => None,
        }
    }
}

impl From<DurabilityError> for CharacterInboxDeliveryError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl From<sqlx::Error> for CharacterInboxDeliveryError {
    fn from(error: sqlx::Error) -> Self {
        let refusal = error
            .as_database_error()
            .and_then(|database| database.code())
            .and_then(|code| Self::from_sqlstate(&code));
        refusal.unwrap_or_else(|| Self::Unavailable(error.into()))
    }
}

impl std::fmt::Display for CharacterInboxDeliveryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ItemHasContents => formatter.write_str("CharacterInbox item has contents"),
            Self::WorldMismatch => formatter.write_str("CharacterInbox delivery World mismatch"),
            Self::ItemInAnotherLocation => {
                formatter.write_str("CharacterInbox item is still in another location")
            }
            Self::UnknownCauseKind => formatter.write_str("CharacterInbox cause kind is unknown"),
            Self::AlreadyDelivered => {
                formatter.write_str("CharacterInbox item was already delivered for this cause")
            }
            Self::Unavailable(error) => {
                write!(formatter, "CharacterInbox storage failed: {error:?}")
            }
        }
    }
}

impl std::error::Error for CharacterInboxDeliveryError {}

/// One Inbox item of a Character, in delivery order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterInboxEntry {
    pub item_instance_id: [u8; 16],
    pub world_id: [u8; 16],
    pub ordinal: u64,
}

/// The Character's Inbox counter: `committed` equals the entry count (INBOX-1a has no
/// reservations) and `next_ordinal` is the next ordinal a delivery takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterInboxCounter {
    pub committed: u64,
    pub next_ordinal: u64,
}

/// The delivery record of one Inbox placement, its proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterInboxDelivery {
    pub item_instance_id: [u8; 16],
    pub cause_kind: String,
    pub cause_ref: String,
    pub world_id: [u8; 16],
    pub character_id: [u8; 16],
    pub ordinal: u64,
}

/// The Character's Inbox items, ordered by ordinal.
pub async fn read_character_inbox(
    connection: &mut PgConnection,
    character_id: &[u8; 16],
) -> Result<Vec<CharacterInboxEntry>, DurabilityError> {
    let rows = sqlx::query(
        "SELECT item_instance_id::text AS item_instance_id, world_id::text AS world_id, \
                ordinal::text AS ordinal \
           FROM game_item_character_inbox_locations \
          WHERE character_id = encode($1,'hex')::uuid \
          ORDER BY ordinal",
    )
    .bind(character_id.as_slice())
    .fetch_all(connection)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(CharacterInboxEntry {
                item_instance_id: uuid_text(&row.try_get::<String, _>("item_instance_id")?)?,
                world_id: uuid_text(&row.try_get::<String, _>("world_id")?)?,
                ordinal: numeric_u64(row, "ordinal")?,
            })
        })
        .collect()
}

/// The Character's Inbox counter, or `None` before its first delivery.
pub async fn read_character_inbox_counter(
    connection: &mut PgConnection,
    character_id: &[u8; 16],
) -> Result<Option<CharacterInboxCounter>, DurabilityError> {
    let row = sqlx::query(
        "SELECT committed::text AS committed, next_ordinal::text AS next_ordinal \
           FROM game_character_inbox_counters \
          WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(character_id.as_slice())
    .fetch_optional(connection)
    .await?;
    row.map(|row| {
        Ok(CharacterInboxCounter {
            committed: numeric_u64(&row, "committed")?,
            next_ordinal: numeric_u64(&row, "next_ordinal")?,
        })
    })
    .transpose()
}

/// The delivery records of one item, ordered by ordinal.
pub async fn read_character_inbox_deliveries(
    connection: &mut PgConnection,
    item_instance_id: &[u8; 16],
) -> Result<Vec<CharacterInboxDelivery>, DurabilityError> {
    let rows = sqlx::query(
        "SELECT item_instance_id::text AS item_instance_id, cause_kind, cause_ref, \
                world_id::text AS world_id, character_id::text AS character_id, \
                ordinal::text AS ordinal \
           FROM game_character_inbox_deliveries \
          WHERE item_instance_id = encode($1,'hex')::uuid \
          ORDER BY ordinal",
    )
    .bind(item_instance_id.as_slice())
    .fetch_all(connection)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(CharacterInboxDelivery {
                item_instance_id: uuid_text(&row.try_get::<String, _>("item_instance_id")?)?,
                cause_kind: row.try_get("cause_kind")?,
                cause_ref: row.try_get("cause_ref")?,
                world_id: uuid_text(&row.try_get::<String, _>("world_id")?)?,
                character_id: uuid_text(&row.try_get::<String, _>("character_id")?)?,
                ordinal: numeric_u64(row, "ordinal")?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::CharacterInboxDeliveryError as E;

    #[test]
    fn every_inbox_sqlstate_has_one_typed_refusal() {
        assert!(matches!(
            E::from_sqlstate("OTI01"),
            Some(E::ItemHasContents)
        ));
        assert!(matches!(E::from_sqlstate("OTI02"), Some(E::WorldMismatch)));
        assert!(matches!(
            E::from_sqlstate("OTI03"),
            Some(E::ItemInAnotherLocation)
        ));
        assert!(matches!(
            E::from_sqlstate("OTI04"),
            Some(E::UnknownCauseKind)
        ));
        assert!(matches!(
            E::from_sqlstate("OTI05"),
            Some(E::AlreadyDelivered)
        ));
        assert!(E::from_sqlstate("23503").is_none());
        assert!(E::from_sqlstate("23514").is_none());
    }
}
