//! Same-transaction evidence for Dreamer's retained Fire Bug, without progress writes.
use super::*;

pub(super) async fn retain(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &super::super::quest_state::QuestTransitionRequest,
    callback: &Callback,
    context: &QuestWorldContext,
    input: &Input,
) -> Result<(), DurabilityError> {
    if !oteryn_gameplay_quests::quest::dreamer_fire_bug::shared_input(callback) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let super::super::quest_state::QuestCause::Use(command) = request.cause else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let proof = serde_json::to_vec(&serde_json::json!({
        "recipe":callback.key,"source":[32849,32233,9,1],
        "input":input.before,"location":input.location,"context":context,
    }))
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_quest_shared_input_proofs(character_id,cause_id,cause_ordinal,transition_key,world_id,channel_id,item_instance_id,before_item,location,kind,proof,proof_sha256) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7::uuid,$8,$9,$10,$11,sha256($11))")
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(command.game_session_id().as_bytes().as_slice())
        .bind(command.command_id().get().to_string()).bind(&callback.key)
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(&input.item).bind(&input.before).bind(&input.location).bind(&input.kind).bind(proof)
        .execute(&mut **tx).await?;
    Ok(())
}
