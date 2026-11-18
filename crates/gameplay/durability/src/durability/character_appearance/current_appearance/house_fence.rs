//! Narrow observation adapter. No Character appearance write accepts this House proof.
use super::*;
use crate::durability::character_progression::FencedCharacterRoot;

pub(super) async fn observe_house_root(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CurrentCharacterGameplayFence,
    node: &NodeIncarnationProof,
    origin_channel: [u8; 16],
) -> Result<Result<FencedCharacterRoot, CharacterProgressionError>, DurabilityError> {
    let Some(house) = crate::durability::house_hireling::fence::observe_house_custody_fence(
        tx,
        node,
        &fence.item_fence(),
        origin_channel,
    )
    .await?
    else {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    };
    // Use the native access owner for both personal Houses and guildhalls.
    let access: bool = sqlx::query_scalar(
        "SELECT COALESCE(count(*) = 1 AND bool_and(content_fenced IS FALSE AND role IN ('OWNER','SUBOWNER','GUEST')), false) \
         FROM game_house_access(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid)",
    )
    .bind(house.world_id().as_bytes().as_slice())
    .bind(house.house_key())
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await
    .map_err(|error| {
        // A property change after the snapshot began must be reobserved.
        if error.as_database_error().and_then(|error| error.code()).as_deref() == Some("40001") {
            DurabilityError::Database(crate::durability::SqlxFailureClass::RetryableStatement)
        } else {
            error.into()
        }
    })?;
    if !access {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    }
    let row = sqlx::query(
        "SELECT character_revision::text,profile_revision,ruleset_revision,content_revision \
         FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    let revision = numeric_u64(&row, "character_revision")?;
    if revision != fence.expected_character_revision.get() {
        return Ok(Err(CharacterProgressionError::CharacterRevisionMismatch));
    }
    Ok(Ok(FencedCharacterRoot {
        revision,
        profile_revision: row.try_get("profile_revision")?,
        ruleset_revision: row.try_get("ruleset_revision")?,
        content_revision: row.try_get("content_revision")?,
    }))
}
