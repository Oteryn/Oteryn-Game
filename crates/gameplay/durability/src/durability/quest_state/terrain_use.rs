//! Observe an existing native action; never repeat its terrain or Item writes.
use super::*;
use oteryn_gameplay_quests::quest::authored_bindings::terrain_use::TerrainUse;

pub(super) async fn admit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    quest: &str,
    usage: &TerrainUse,
) -> StoredResult<bool> {
    let QuestCause::Command(command) = request.cause else {
        return Ok(false);
    };
    if command.game_session_id() != fence.game_session_id {
        return Ok(false);
    }
    let Some(channel) = fence.runtime_scope.channel_id() else {
        return Ok(false);
    };
    let world = fence.runtime_scope.world_id();
    // Definition alternatives are exact pairs, never a cross product of key/revision lists.
    // A native action can advance at most one stage in this quest. Exact replay of the
    // same stage has already returned its immutable quest receipt before this predicate.
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_terrain_tool_receipts r \
         JOIN game_terrain_tool_transforms t USING(world_id,channel_id,placement,reset_epoch) \
         WHERE r.character_id=encode($1,'hex')::uuid \
           AND r.game_session_id=encode($2,'hex')::uuid AND r.command_id=$3::text::numeric \
           AND r.world_id=encode($4,'hex')::uuid AND r.channel_id=encode($5,'hex')::uuid \
           AND r.placement=$6 AND r.transformed_item_id=$7 \
           AND r.tool_definition_family='Item' \
           AND EXISTS(SELECT 1 FROM jsonb_to_recordset($8) AS tool(key TEXT,revision TEXT) \
                      WHERE tool.key=r.tool_definition_production_key \
                        AND tool.revision=r.tool_definition_revision_ref) \
           AND t.content_generation=(SELECT server_artifact_digest FROM game_content_activations a \
               WHERE a.world_id=r.world_id AND a.channel_id=r.channel_id \
               ORDER BY activation_sequence DESC LIMIT 1) \
           AND NOT EXISTS(SELECT 1 FROM game_character_quest_receipts q \
               WHERE q.character_id=r.character_id AND q.quest_key=$9 AND q.cause_kind='command' \
                 AND q.cause_id=r.game_session_id AND q.cause_ordinal=r.command_id))",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(command.game_session_id().as_bytes().as_slice())
    .bind(command.command_id().get().to_string())
    .bind(world.as_bytes().as_slice())
    .bind(channel.as_bytes().as_slice())
    .bind(&usage.placement)
    .bind(i64::from(usage.result))
    .bind(serde_json::to_value(&usage.tools).map_err(|_| DurabilityError::InvalidStoredState)?)
    .bind(quest)
    .fetch_one(&mut **tx)
    .await?)
}
