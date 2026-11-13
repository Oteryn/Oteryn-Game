//! Observe an accepted native programme after its receipt commits.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use crate::foundation::ExactActorRef;
use oteryn_gameplay_quests::quest::authored_bindings::Trigger;

pub(super) async fn after_acceptance(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
) {
    let Some(catalogue) = owner.quest_catalogue.as_ref() else {
        return;
    };
    let Some(bindings) = catalogue.authored_bindings() else {
        return;
    };
    let keys = {
        let Ok(sessions) = owner.states.quest_sessions.lock() else {
            return;
        };
        let Some(copy) = sessions
            .get(&command.game_session_id)
            .and_then(|state| state.copy.as_ref())
        else {
            return;
        };
        bindings
            .quests
            .iter()
            .flat_map(|quest| &quest.stages)
            .filter_map(|stage| {
                let Trigger::TaskBoardUse { usage } = &stage.trigger else {
                    return None;
                };
                usage
                    .storage
                    .iter()
                    .all(|guard| {
                        catalogue.track(&guard.track).is_some_and(|track| {
                            let value = copy
                                .tracks()
                                .get(&guard.track)
                                .copied()
                                .unwrap_or(track.initial);
                            (guard.minimum..=guard.maximum).contains(&value)
                        })
                    })
                    .then(|| usage.key.clone())
            })
            .collect()
    };
    // Root independently verifies this exact command's immutable native receipt.
    super::super::quest_stage_events::commit_keys(owner, actor, command, keys).await;
}
