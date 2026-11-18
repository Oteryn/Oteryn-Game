//! Bounded original-actor quest consequences in the existing Channel clock owner.
use super::ComposedFreshAdmission;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{ExactActorRef, GameSessionId};
impl ComposedFreshAdmission<'_, '_, '_> {
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
        let mut runtime = self.runtime.lock().await;
        if runtime.owner_fence().is_err()
            || runtime.positioned_melee_census().ok().is_none_or(|rows| {
                !rows
                    .iter()
                    .any(|(a, _, s)| *a == actor && *s == Some(session))
            })
        {
            return;
        }
        if self
            .drain_personal_quest_return(&mut runtime, &mut slot, actor, session, fence, catalogue)
            .await
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

    #[allow(
        clippy::too_many_arguments,
        reason = "reuses the clock's exact runtime and Character revision guards"
    )]
    async fn drain_personal_quest_return(
        &self,
        runtime: &mut crate::foundation::ChannelRuntimeV1,
        slot: &mut crate::durability::character_revision_sequencer::RevisionSlot,
        actor: ExactActorRef,
        session: GameSessionId,
        fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
        catalogue: &std::sync::Arc<oteryn_gameplay_quests::quest::QuestStateCatalogue>,
    ) -> bool {
        let callbacks = self.callback_content();
        if !callbacks
            .iter()
            .any(|callback| callback.effects.personal_return.is_some())
        {
            return false;
        }
        let Some(map) = self.bundle_map() else {
            return false;
        };
        let revision = crate::map::overlay::map_revision(map.overlay.base());
        let Ok(Some(due)) = self
            .root
            .read_due_personal_return(self.character, self.holder, fence, revision)
            .await
        else {
            return false;
        };
        let Some(timer) = callbacks.iter().find_map(|callback| {
            (callback.personal_return_key().as_deref() == Some(due.transition.as_str()))
                .then_some(callback.effects.personal_return.as_ref())
                .flatten()
        }) else {
            return false;
        };
        let Ok(current) = runtime.read_actor_position(actor) else {
            return true;
        };
        let dest = crate::foundation::MovementLocalPosition {
            x: i32::from(timer.landing.x),
            y: i32::from(timer.landing.y),
            floor: -i16::from(timer.landing.floor),
        };
        let doors = crate::map::floor::BundleDoors(map.facts.doors());
        let proof = if current.position() == dest {
            None
        } else {
            let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport_facing(
                runtime,
                self.movement_cells(),
                actor,
                session,
                current,
                dest,
                false,
                &doors,
                None,
            ) else {
                return true;
            };
            Some(proof)
        };
        let request = QuestTransitionRequest {
            transition_key: due.transition.clone(),
            cause: QuestCause::ServerTimer(due.occurrence),
        };
        let committed = if due.already_committed {
            true
        } else {
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
                    QuestTransitionOutcome::Committed(committed)
                    | QuestTransitionOutcome::AlreadyCommitted(committed),
                ) => {
                    self.apply_committed_quest_copy(session, true, &committed);
                    self.schedule_quest_refresh(session);
                    true
                }
                Ok(QuestTransitionOutcome::Refused(
                    oteryn_gameplay_quests::quest::QuestRefusal::StageMismatch,
                )) => false,
                _ => return true,
            }
        };
        if committed {
            if let Some(proof) = proof {
                if runtime.commit_source_step(proof).is_err() {
                    return true;
                }
                self.states
                    .chat
                    .push_quest_feedback(session, &timer.failure_message);
            }
        }
        let _ = self
            .root
            .finish_personal_return(self.character, self.holder, fence, due)
            .await;
        true
    }
}
