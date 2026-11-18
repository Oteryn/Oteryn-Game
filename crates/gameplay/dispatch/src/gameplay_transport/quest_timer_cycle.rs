//! Bounded original-actor quest consequences in the existing Channel clock owner.
use super::ComposedFreshAdmission;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_gameplay_quests::quest::rotten_blood::putrefactory;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn apply_putrefactory_tick(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        period: u64,
    ) -> bool {
        self.root
            .with_background_operation(self.apply_putrefactory_tick_inner(actor, session, period))
            .await
    }

    async fn apply_putrefactory_tick_inner(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        period: u64,
    ) -> bool {
        let Some(catalogue) = self
            .quest_catalogue
            .as_ref()
            .filter(|catalogue| catalogue.transition(putrefactory::PASSIVE_TICK).is_some())
        else {
            return true;
        };
        if self.bundle_map().is_none() {
            return true;
        }
        let in_zone = {
            let runtime = self.runtime.lock().await;
            runtime
                .read_actor_position(actor)
                .ok()
                .is_some_and(|snapshot| {
                    let at = snapshot.position();
                    putrefactory::accrues_charge_at(at.x, at.y, at.floor)
                })
        };
        if !in_zone {
            return true;
        }
        let fence = match self.current_quest_fence(session).await {
            Ok(Some(fence)) => fence,
            Ok(None) => return true,
            Err(()) => return false,
        };
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let runtime = self.runtime.lock().await;
        let Some(input) = crate::durability::quest_state::putrefactory::QualifiedPutrefactoryTick::from_current_actor(
            &runtime, actor, fence, period,
        ) else {
            return true;
        };
        match slot
            .commit_putrefactory_tick(
                self.root,
                self.character,
                self.holder,
                fence,
                input,
                std::sync::Arc::clone(catalogue),
            )
            .await
        {
            Ok(QuestTransitionOutcome::Committed(committed)) => {
                self.apply_committed_quest_copy(session, true, &committed);
                self.schedule_quest_refresh(session);
            }
            Ok(QuestTransitionOutcome::AlreadyCommitted(committed)) => {
                self.apply_committed_quest_copy(session, false, &committed);
                self.schedule_quest_refresh(session);
            }
            Ok(QuestTransitionOutcome::Refused(
                oteryn_gameplay_quests::quest::QuestRefusal::RechargeCoolingDown,
            )) => return false,
            Err(
                crate::durability::character_progression::CharacterProgressionError::Unavailable(_),
            ) => return false,
            _ => {}
        }
        true
    }

    pub(super) async fn drain_quest_timers(&self, actor: ExactActorRef, session: GameSessionId) {
        self.root
            .with_background_operation(self.drain_quest_timers_inner(actor, session))
            .await;
    }

    async fn drain_quest_timers_inner(&self, actor: ExactActorRef, session: GameSessionId) {
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
        for timer in due.into_iter().take(1) {
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
