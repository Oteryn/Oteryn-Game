//! Installed tier storage/read owner (IMBUE-FORGE-0 §8), not a forge writer.
//! Migration 0156 supplies the single schema and atomic storage audit; runtime
//! has SELECT only. A caller must separately qualify current authority,
//! item ownership/location/lifecycle, Content compatibility, class cap and imbuements.

use super::item_mint_audit::check_uuid_v7;
use crate::domain::forge::ItemTier;
use sqlx::Row;
use sqlx::postgres::PgConnection;

/// Installed by the normal GAME_MIGRATOR, never by a private test DDL path.
pub const FORGE_TIER_MIGRATION_VERSION: i64 = 156;

/// Raw stored data only: neither a live ownership witness nor forge permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredItemTier {
    pub item_instance_id: [u8; 16],
    pub tier: ItemTier,
}

#[derive(Debug)]
pub enum TierReadError {
    InvalidItemIdentity,
    UnknownItem,
    InvalidStoredState,
    Storage(sqlx::Error),
}

impl From<sqlx::Error> for TierReadError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error)
    }
}

/// Read one ItemInstance and its optional tier in one statement snapshot.
/// Unknown items fail closed; only an existing item without a row has tier zero.
/// No gameplay fence or root lock is acquired: this read cannot authorize a
/// mutation. A future writer must re-read under its actual current locks/fences.
pub async fn read_item_tier(
    connection: &mut PgConnection,
    item_instance_id: [u8; 16],
) -> Result<StoredItemTier, TierReadError> {
    check_uuid_v7(&item_instance_id).map_err(|_| TierReadError::InvalidItemIdentity)?;
    let row = sqlx::query(
        "SELECT uuid_send(i.item_instance_id) AS item_identity, t.tier, \
         t.item_instance_id IS NOT NULL AS tier_present \
         FROM game_item_instances i \
         LEFT JOIN game_item_tiers t ON t.item_instance_id = i.item_instance_id \
         WHERE i.item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(item_instance_id.as_slice())
    .fetch_optional(connection)
    .await?
    .ok_or(TierReadError::UnknownItem)?;
    let stored: Vec<u8> = row.try_get("item_identity")?;
    if stored.as_slice() != item_instance_id {
        return Err(TierReadError::InvalidStoredState);
    }
    let tier = match (
        row.try_get::<bool, _>("tier_present")?,
        row.try_get::<Option<i16>, _>("tier")?,
    ) {
        (false, None) => ItemTier::ZERO,
        (true, Some(value)) => u8::try_from(value)
            .ok()
            .filter(|value| *value != 0)
            .and_then(ItemTier::new)
            .ok_or(TierReadError::InvalidStoredState)?,
        _ => return Err(TierReadError::InvalidStoredState),
    };
    Ok(StoredItemTier {
        item_instance_id,
        tier,
    })
}
