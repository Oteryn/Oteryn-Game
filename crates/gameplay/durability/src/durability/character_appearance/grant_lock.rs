//! Serialize earned account grants across their existing source owners.
use crate::durability::DurabilityError;
use sqlx::{PgConnection, types::Uuid};

/// The caller derives this account from its already fenced Character root.
/// This lock adds no authority; it keeps equivalent grants from racing across source keys.
pub(crate) async fn lock_account_grants(
    tx: &mut PgConnection,
    account: Uuid,
) -> Result<(), DurabilityError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:appearance-account:' || $1::text,0))")
        .bind(account)
        .execute(tx)
        .await?;
    Ok(())
}

/// Read creation sex through the appearance owner after the caller's Character fence.
pub(crate) async fn item_action_sex(
    tx: &mut PgConnection,
    character: crate::domain::CharacterId,
) -> Result<crate::domain::appearance::Sex, DurabilityError> {
    super::character_sex::read(tx, character)
        .await
        .map_err(|error| match error {
            super::AppearanceWriteError::Unavailable(error) => error,
            _ => DurabilityError::InvalidStoredState,
        })
}
