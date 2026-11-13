//! A Task Board request is evidence only after its native owner commits acceptance.
use super::*;
use oteryn_gameplay_quests::quest::authored_bindings::task_board_use::TaskBoardUse;
use oteryn_gameplay_quests::quest::task_board::full::TaskBoardAction;

pub(super) async fn admit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    usage: &TaskBoardUse,
) -> StoredResult<bool> {
    let QuestCause::Command(command) = request.cause else {
        return Ok(false);
    };
    if command.game_session_id() != fence.game_session_id {
        return Ok(false);
    }
    // The primary key bounds this read to one immutable native receipt. Queries
    // and refusals have no committed revision and cannot authorize route progress.
    let action: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT action FROM game_task_board_receipts WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND cause_kind='command' AND cause_id=encode($3,'hex')::uuid AND cause_ordinal=$4::text::numeric AND refusal='null'::jsonb AND committed_character_revision IS NOT NULL",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
    .bind(command.game_session_id().as_bytes().as_slice())
    .bind(command.command_id().get().to_string())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(action) = action else {
        return Ok(false);
    };
    let action: TaskBoardAction =
        serde_json::from_value(action).map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(usage.programme.accepts(&action))
}
