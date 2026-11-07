use super::super::character_appearance::AppearanceReadError;
use super::*;
/// Pure projection of a live personal contract. No account ledger or permanent ownership write.
pub(crate) async fn earned(
    connection: &mut sqlx::PgConnection,
    character: CharacterId,
    grant_keys: &[String],
) -> Result<Vec<(String, String)>, AppearanceReadError> {
    let mount:Option<i32>=sqlx::query_scalar("SELECT mount_id FROM game_horse_rental_receipts WHERE character_id=encode($1,'hex')::uuid AND expires_seconds>=floor(extract(epoch FROM public.game_daily_effect_now()))::bigint ORDER BY issued_seconds DESC,transaction_id DESC LIMIT 1")
  .bind(character.as_bytes().as_slice()).fetch_optional(connection).await?;
    let Some(mount) = mount else {
        return Ok(Vec::new());
    };
    let horse = rules::Horse::from_source_id(
        u16::try_from(mount).map_err(|_| AppearanceReadError::InvalidStoredState)?,
    )
    .ok_or(AppearanceReadError::InvalidStoredState)?;
    Ok(if grant_keys.iter().any(|k| k == horse.grant()) {
        vec![(horse.grant().to_owned(), rules::PROVENANCE.to_owned())]
    } else {
        Vec::new()
    })
}
