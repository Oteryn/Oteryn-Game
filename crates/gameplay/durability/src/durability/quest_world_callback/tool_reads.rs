//! Read-only source custody participates in the same fenced quest receipt.
use super::{Input, QuestWorldToolContext};
use crate::durability::{
    DurabilityError,
    character_progression::CurrentCharacterGameplayFence,
    quest_state::{QuestCause, QuestTransitionRequest},
};
use oteryn_gameplay_quests::quest::world_callbacks::Callback;

pub(super) async fn record(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    callback: &Callback,
    context: &QuestWorldToolContext,
    input: &Input,
) -> Result<bool, DurabilityError> {
    let Some(tool) = &context.selected_tool else {
        return Ok(false);
    };
    let QuestCause::Use(command) = request.cause else {
        return Ok(false);
    };
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    let proof=serde_json::to_vec(&serde_json::json!({"callback":callback,"context":context,"before":input.before,"location":input.location,"kind":input.kind}))
        .map_err(|_|DurabilityError::InvalidStoredState)?;
    Ok(sqlx::query("INSERT INTO game_quest_world_tool_reads(character_id,cause_id,cause_ordinal,transition_key,item_instance_id,world_id,channel_id,scope_generation,proof) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8::text::numeric,$9) ON CONFLICT DO NOTHING")
        .bind(fence.character_id.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice())
        .bind(command.command_id().get().to_string()).bind(&callback.key).bind(tool.source.as_slice())
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(fence.scope_ownership_generation.get().to_string()).bind(proof).execute(&mut **tx).await?.rows_affected()==1)
}
