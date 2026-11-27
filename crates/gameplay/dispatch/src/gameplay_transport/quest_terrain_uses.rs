//! Native tool USE and exact replay observe their existing committed receipt.
use super::{ComposedFreshAdmission, connection::UseCommand};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_gameplay_quests::quest::authored_bindings::Trigger;

pub(super) async fn after_tool(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
) -> bool {
    let Some(catalogue) = owner.quest_catalogue.as_ref() else {
        return true;
    };
    let Some(bindings) = catalogue.authored_bindings() else {
        return true;
    };
    if !bindings
        .quests
        .iter()
        .flat_map(|quest| &quest.stages)
        .any(|stage| matches!(stage.trigger, Trigger::TerrainUse { .. }))
    {
        return true;
    }
    let Some(fence) = command.item_fence else {
        return false;
    };
    let Ok(ordinal) = CommandId::new(command.command_id) else {
        return false;
    };
    let observed = owner
        .root
        .read_quest_terrain_use(
            owner.character,
            owner.holder,
            fence,
            CommandRef::new(command.game_session_id, ordinal),
        )
        .await;
    let observation = match observed {
        Ok(Some(observation)) => observation,
        Ok(None) => return true,
        Err(error) => {
            super::operator_event(&format!(
                "quest_terrain_use_observation_refused reason={error:?}"
            ));
            owner.schedule_quest_refresh(command.game_session_id);
            return false;
        }
    };
    let keys = {
        let Ok(sessions) = owner.states.quest_sessions.lock() else {
            return false;
        };
        let Some(copy) = sessions
            .get(&command.game_session_id)
            .and_then(|state| state.copy.as_ref())
        else {
            return false;
        };
        bindings.terrain_use_candidates(
            catalogue,
            copy.tracks(),
            &observation.tool,
            &observation.placement,
            observation.result,
        )
    };
    super::quest_stage_events::commit_keys(owner, actor, command, keys).await
}
