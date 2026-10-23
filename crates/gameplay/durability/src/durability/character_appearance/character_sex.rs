//! Creation sex read under the caller's independently established Character fence.
use super::AppearanceWriteError;
use crate::domain::CharacterId;
use crate::domain::appearance::Sex;
use sqlx::PgConnection;

pub(super) async fn read(
    connection: &mut PgConnection,
    character: CharacterId,
) -> Result<Sex, AppearanceWriteError> {
    let sex: Option<i16> = sqlx::query_scalar(
        "SELECT sex FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_one(connection)
    .await
    .map_err(crate::durability::DurabilityError::from)?;
    match sex {
        Some(1) => Ok(Sex::Female),
        Some(2) => Ok(Sex::Male),
        _ => Err(AppearanceWriteError::InvalidInput),
    }
}
