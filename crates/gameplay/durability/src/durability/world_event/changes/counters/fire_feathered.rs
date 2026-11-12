//! Native Guardian NPC completion joins the existing quest and counter receipt.
use super::{CounterKind, Credit, CreditAuthority, CreditEvent, CreditOutcome};
use crate::durability::{DurabilityError, quest_state::QuestCause};
use crate::foundation::{CharacterId, RuntimeScopeRefV1};
use oteryn_gameplay_quests::quest::{QuestTrackChange, fire_feathered_native};
use sqlx::{Postgres, Transaction};
#[allow(
    clippy::too_many_arguments,
    reason = "independent current authority and immutable quest occurrence are explicit"
)]
pub(crate) async fn credit_completed(
    tx: &mut Transaction<'_, Postgres>,
    authority: &CreditAuthority,
    scope: RuntimeScopeRefV1,
    character: CharacterId,
    epoch: u64,
    cause: QuestCause,
    changes: &[QuestTrackChange],
) -> Result<CreditOutcome, DurabilityError> {
    let QuestCause::Command(command) = cause else {
        return Ok(CreditOutcome::Refused);
    };
    if !fire_feathered_native::completed(changes) {
        return Ok(CreditOutcome::Refused);
    }
    let Some(day) = super::super::load(tx, scope.world_id(), epoch).await? else {
        return Ok(CreditOutcome::Refused);
    };
    if day.counter_stages[CounterKind::FireFeathered.index()] != 3 {
        return Ok(CreditOutcome::Refused);
    }
    let mut occurrence = scope.world_id().as_bytes().to_vec();
    occurrence.extend_from_slice(command.game_session_id().as_bytes());
    occurrence.extend_from_slice(&command.command_id().get().to_be_bytes());
    super::credit(
        tx,
        authority,
        &CreditEvent {
            source_kind: 4,
            occurrence,
            credits: vec![(CounterKind::FireFeathered, Credit::MissionComplete)],
            counter_generations: day.counter_generations,
            occurred_at_ms: 0,
            character: Some(character),
        },
    )
    .await
}
