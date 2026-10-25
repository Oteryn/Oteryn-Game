//! Locked-level blessing quote and held set, consuming the admitted content owner.
use super::DurabilityError;
use super::npc_purchase::{PurchaseRefusal, PurchaseService};
use crate::domain::CharacterId;
use oteryn_gameplay_content::content::blessing_catalogue::BlessingCatalogue;
use sqlx::PgConnection;

pub(super) async fn quote(
    connection: &mut PgConnection,
    character: CharacterId,
    npc: &str,
    service: &PurchaseService,
    level: u32,
) -> Result<Result<(u64, Option<String>), PurchaseRefusal>, DurabilityError> {
    let catalogue = BlessingCatalogue::load().map_err(|_| DurabilityError::InvalidStoredState)?;
    let Some(blessing) = catalogue.blessings.iter().find(|blessing| {
        blessing.service == service.key()
            && blessing.premium
            && blessing.npcs.iter().any(|key| key == npc)
    }) else {
        return Ok(Err(PurchaseRefusal::NotEligible));
    };
    let held: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_blessings WHERE character_id=encode($1,'hex')::uuid AND blessing_key=$2)")
        .bind(character.as_bytes().as_slice()).bind(&blessing.storage_key).fetch_one(connection).await?;
    if held {
        return Ok(Err(PurchaseRefusal::AlreadyHeld));
    }
    Ok(Ok((
        blessing.price(level),
        Some(blessing.storage_key.clone()),
    )))
}

pub(super) async fn grant(
    connection: &mut PgConnection,
    character: CharacterId,
    key: &str,
    occurrence: [u8; 16],
) -> Result<(), DurabilityError> {
    sqlx::query("INSERT INTO game_character_blessings(character_id,blessing_key,provenance) VALUES(encode($1,'hex')::uuid,$2,(encode($3,'hex')::uuid)::text)")
        .bind(character.as_bytes().as_slice()).bind(key).bind(occurrence.as_slice()).execute(connection).await?;
    Ok(())
}
