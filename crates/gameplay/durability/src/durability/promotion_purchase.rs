//! The permanent promotion effect; fee and Character receipt are owned by the participant.
use super::DurabilityError;
use super::character_promotion::read_durable_promotion;
use super::npc_purchase::{PurchaseRefusal, PurchaseService};
use crate::domain::CharacterId;
use sqlx::PgConnection;

pub(super) async fn quote(
    connection: &mut PgConnection,
    character: CharacterId,
    npc: &str,
    service: &PurchaseService,
    level: u32,
) -> Result<Result<(u64, Option<String>), PurchaseRefusal>, DurabilityError> {
    let promoted = read_durable_promotion(connection, character).await?;
    let key = npc
        .strip_prefix("oteryn:npc.")
        .map(|stem| format!("oteryn:service.promotion.{stem}"));
    if key.as_deref() != Some(service.key())
        || !oteryn_gameplay_progression::promotion::eligible(level, promoted)
    {
        return Ok(Err(PurchaseRefusal::NotEligible));
    }
    Ok(Ok((
        oteryn_gameplay_progression::promotion::PROMOTION_PRICE,
        None,
    )))
}

pub(super) async fn grant(
    connection: &mut PgConnection,
    character: CharacterId,
    occurrence: [u8; 16],
) -> Result<(), DurabilityError> {
    sqlx::query("INSERT INTO game_character_promotions(character_id,purchase_occurrence_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
        .bind(character.as_bytes().as_slice()).bind(occurrence.as_slice()).execute(connection).await?;
    Ok(())
}
