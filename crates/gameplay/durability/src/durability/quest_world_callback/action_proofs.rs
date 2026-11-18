//! Same-transaction evidence for physical world actions without a journal delta.
use super::{Callback, CurrentCharacterGameplayFence, DurabilityError, QuestWorldToolContext};
use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
use crate::foundation::RuntimeScopeRefV1;
use sqlx::{Postgres, Transaction};

pub(in crate::durability) async fn record(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    callback: &Callback,
    context: &QuestWorldToolContext,
    binding: &[u8],
) -> Result<(), DurabilityError> {
    let (
        QuestCause::Use(command) | QuestCause::Step(command),
        RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        },
    ) = (request.cause, fence.runtime_scope)
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let plan = serde_json::to_value(callback).map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_quest_world_action_proofs
        (character_id,cause_id,cause_ordinal,transition_key,world_id,channel_id,scope_generation,request_binding,callback,source_face)
        VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,
            encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7::text::numeric,$8,$9,$10)")
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(command.game_session_id().as_bytes().as_slice())
        .bind(command.command_id().get().to_string())
        .bind(&request.transition_key)
        .bind(world_id.as_bytes().as_slice())
        .bind(channel_id.as_bytes().as_slice())
        .bind(fence.scope_ownership_generation.get().to_string())
        .bind(binding)
        .bind(plan)
        .bind(context.ground_supply.as_ref().map(|face|i32::from(face.before)))
        .execute(&mut **tx).await?;
    Ok(())
}
