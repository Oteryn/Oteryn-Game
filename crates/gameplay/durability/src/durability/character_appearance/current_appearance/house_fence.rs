//! Narrow observation adapter. No Character appearance write accepts this House proof.
use super::*;
use crate::durability::character_progression::FencedCharacterRoot;

pub(super) async fn assert_read_fence(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CurrentCharacterGameplayFence,
    node: &NodeIncarnationProof,
    scope: ReadScope,
) -> Result<Result<FencedCharacterRoot, CharacterProgressionError>, DurabilityError> {
    let ReadScope::House { origin_channel } = scope else {
        return assert_gameplay_fence(tx, fence, node).await;
    };
    if !crate::durability::house_hireling::fence::assert_house_inventory_fence(
        tx, node, &fence.item_fence(), origin_channel,
    ).await? {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    }
    let row = sqlx::query(
        "SELECT character_revision::text,profile_revision,ruleset_revision,content_revision \
         FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1",
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
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
