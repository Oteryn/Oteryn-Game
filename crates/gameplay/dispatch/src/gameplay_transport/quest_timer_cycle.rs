//! Bounded original-actor quest consequences in the existing Channel clock owner.
use super::ComposedFreshAdmission;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{ExactActorRef, GameSessionId};
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn drain_quest_timers(&self, actor: ExactActorRef, session: GameSessionId) {
        let Some(catalogue) = self.quest_catalogue.as_ref() else {
            return;
        };
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return;
        };
        // Match the admitted command lock order: Character revision slot, then runtime.
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let runtime = self.runtime.lock().await;
        if runtime.owner_fence().is_err()
            || runtime.positioned_melee_census().ok().is_none_or(|rows| {
                !rows
                    .iter()
                    .any(|(a, _, s)| *a == actor && *s == Some(session))
            })
        {
            return;
        }
        let due = match self
            .root
            .read_due_quest_timers(
                self.character,
                self.holder,
                fence,
                actor.placement_identity(),
            )
            .await
        {
            Ok(due) => due,
            Err(error) => {
                #[cfg(test)]
                eprintln!("QUEST_TIMER read error: {error:?}");
                #[cfg(not(test))]
                let _ = error;
                return;
            }
        };
        for timer in due {
            if timer.actor_placement != actor.placement_identity() {
                continue;
            }
            let request = QuestTransitionRequest {
                transition_key: timer.transition,
                cause: QuestCause::ServerTimer(timer.occurrence),
            };
            match slot
                .commit_quest_transition(
                    self.root,
                    self.character,
                    self.holder,
                    fence,
                    request,
                    std::sync::Arc::clone(catalogue),
                )
                .await
            {
                Ok(
                    QuestTransitionOutcome::Committed(_)
                    | QuestTransitionOutcome::AlreadyCommitted(_),
                ) => {
                    self.states.loop_ground.invalidate();
                    self.schedule_quest_refresh(session);
                }
                Ok(_) => {}
                Err(error) => {
                    #[cfg(test)]
                    eprintln!("QUEST_TIMER commit error: {error:?}");
                    #[cfg(not(test))]
                    let _ = error;
                    return;
                }
            }
        }
    }
}
