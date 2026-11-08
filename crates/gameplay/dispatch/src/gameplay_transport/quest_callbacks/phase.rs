//! Advisory phase selection; the existing fenced writer rechecks every storage guard.
use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn select_step_phase(
        &self,
        selected: &mut Vec<(&Callback, u64)>,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
        cause: &QuestCause,
    ) {
        if selected.len() <= 1
            || selected.iter().enumerate().any(|(index, (left, _))| {
                selected[..index]
                    .iter()
                    .any(|(right, _)| !world_callbacks::phase::exclusive(left, right))
            })
        {
            return;
        }
        // A retained operation keeps its original callback after an acknowledged
        // durable write advances personal state but before map publication succeeds.
        if let Ok(records) = self.states.quest_callbacks.records.lock()
            && let Some(pending) = records.get(&session)
            && pending.actor == actor
            && pending.expected == entered
            && pending.cause == *cause
        {
            selected
                .retain(|(callback, key)| callback.key == pending.callback && *key == pending.key);
            return;
        }
        let Ok(sessions) = self.states.quest_sessions.lock() else {
            selected.clear();
            return;
        };
        let Some(copy) = sessions
            .get(&session)
            .and_then(|session| session.copy.as_ref())
        else {
            selected.clear();
            return;
        };
        selected.retain(|(callback, _)| world_callbacks::phase::matches(callback, copy.tracks()));
    }
}
