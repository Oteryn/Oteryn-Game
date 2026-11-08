//! Reused world USE value participant (questpc7 653feac45 / questpc3 6146ee945).
use super::*;
use crate::durability::quest_state::QuestCause;

#[allow(
    clippy::too_many_arguments,
    reason = "same fenced Quest and complete MINT inputs as the retained participant"
)]
pub(crate) async fn give_world(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tuple: SelectedType2Tuple,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    callback: &oteryn_gameplay_quests::quest::world_callbacks::Callback,
    admission: QuestRewardAdmission<'_>,
) -> std::result::Result<bool, DurabilityError> {
    let (QuestCause::Use(command), Some(reward)) =
        (request.cause, callback.effects.item_reward.as_ref())
    else {
        return Ok(false);
    };
    if admission.weights.generation_digest() != admission.native.source_digest() {
        return Ok(false);
    }
    let Some(transaction) = mint(
        tx,
        tuple,
        node,
        fence,
        command,
        &callback.key,
        false,
        reward,
        admission,
    )
    .await?
    else {
        return Ok(false);
    };
    let plan = serde_json::json!({"key": callback.key, "quest": callback.quest,
        "storage": callback.storage, "take": [], "present": [], "give": reward,
        "outfit": null, "world_callback": callback});
    sqlx::query("INSERT INTO game_quest_reward_sources(character_id,cause_id,cause_ordinal,transition_key,plan,mint_transaction_id,grant_keys) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,encode($6,'hex')::uuid,ARRAY[]::text[])")
        .bind(fence.character_id.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice())
        .bind(command.command_id().get().to_string()).bind(&request.transition_key).bind(plan)
        .bind(transaction.as_slice()).execute(&mut **tx).await?;
    Ok(true)
}
