//! Canonical personal reward of the native Fire from the Earth credited-kill lane.
//!
//! Retained wiki spoiler 76221@887143, SHA256
//! 86c8bc596ec60a94a2784b26e06b6ce6dfd5d6cd86caf5be97743d1d31163a76,
//! lines 8/10: fifty fiery creatures killed on or inside the volcano grants
//! Fire from the Earth. The caller already verifies the native monster/volcano
//! binding, active counter stage, current authority and immutable death receipt.
//! Achievement catalogue: 62474@845084, staticdata id 256. Dragonling taming is
//! a separate opportunity and is not granted by this counter.
use crate::durability::{DurabilityError, account_achievement};
use crate::foundation::CharacterId;
use sqlx::{Postgres, Transaction};

/// Join the caller's receipt transaction after its Character lock and personal
/// counter increment. The account owner deduplicates already-held achievements;
/// receipt replay exits before reaching this observer.
pub(super) async fn observe_personal_credit(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    personal_count: u64,
    occurrence: &[u8],
) -> Result<(), DurabilityError> {
    if personal_count == 0 || !(16..=64).contains(&occurrence.len()) {
        return Err(DurabilityError::InvalidStoredState);
    }
    if personal_count >= 50 {
        account_achievement::grant_missing(
            tx,
            crate::domain::CharacterId::from_bytes(*character.as_bytes())
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            &["oteryn:achievement/fire_from_the_earth"],
            "world_change_fire_earth",
            occurrence,
        )
        .await?;
    }
    Ok(())
}
