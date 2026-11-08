//! M03 reads canonical Character facts inside the already fenced receipt transaction.
use super::super::quest_state::grizzly::GrizzlyPremiumGate;
use super::*;
use oteryn_gameplay_quests::quest::world_callbacks::Admission;

pub(super) async fn require<G: GrizzlyPremiumGate>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    requirements: &Admission,
    premium: &G,
) -> Result<bool, DurabilityError> {
    if requirements.promotion
        && !super::super::character_promotion::read_durable_promotion(tx, fence.character_id)
            .await?
    {
        return Ok(false);
    }
    if requirements.regular_blessings {
        let catalogue =
            oteryn_gameplay_content::content::blessing_catalogue::BlessingCatalogue::load()
                .map_err(|_| DurabilityError::InvalidStoredState)?;
        let keys: Vec<_> = catalogue
            .regular_blessings()
            .map(|blessing| blessing.storage_key.as_str())
            .collect();
        if keys.len() != 5 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_blessings WHERE character_id=encode($1,'hex')::uuid AND blessing_key=ANY($2::text[])")
            .bind(fence.character_id.as_bytes().as_slice()).bind(&keys).fetch_one(&mut **tx).await?;
        if count != i64::try_from(keys.len()).map_err(|_| DurabilityError::InvalidStoredState)? {
            return Ok(false);
        }
    }
    if requirements.premium {
        let account: sqlx::types::Uuid = sqlx::query_scalar(
            "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(fence.character_id.as_bytes().as_slice())
        .fetch_one(&mut **tx)
        .await?;
        if !premium.task_permitted(tx, *account.as_bytes()).await? {
            return Ok(false);
        }
    }
    Ok(true)
}
