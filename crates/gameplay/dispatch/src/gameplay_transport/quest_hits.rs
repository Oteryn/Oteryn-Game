//! Deferred native-hit credit uses the same revision and session owners as native kills.
pub(super) mod queue;
use super::ComposedFreshAdmission;
use crate::durability::{
    character_progression::CharacterProgressionError, quest_state::QuestTransitionOutcome,
};
use std::sync::Arc;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn drain_native_quest_hits(&self) -> bool {
        let pending = {
            let attack = self.states.attack.lock().await;
            let Ok(cache) = attack.kills().1.lock() else {
                return false;
            };
            cache.hits.pending()
        };
        for pending in pending {
            let Some(catalogue) = &self.quest_catalogue else {
                return false;
            };
            let fence = match self.current_quest_fence(pending.session).await {
                Ok(Some(fence)) => Some(fence),
                Ok(None) => None,
                Err(()) => return false,
            };
            let mut accepted = false;
            if let Some(fence) = fence {
                let live = {
                    let runtime = self.runtime.lock().await;
                    runtime.owner_fence().is_ok()
                        && runtime.bound_attacker_lease(pending.actor, pending.session)
                            == Ok(pending.lease)
                };
                if live
                    && pending.current_for(catalogue)
                    && fence.character_id.as_bytes() == pending.lease.character_id().as_bytes()
                    && fence.character_lease_generation == pending.lease.generation()
                {
                    let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
                    let outcome = slot
                        .commit_native_quest_hit(
                            self.root,
                            self.character,
                            self.holder,
                            fence,
                            &pending.hit,
                            Arc::clone(catalogue),
                        )
                        .await;
                    drop(slot);
                    match outcome {
                        Ok(
                            QuestTransitionOutcome::Committed(_)
                            | QuestTransitionOutcome::AlreadyCommitted(_),
                        ) => accepted = true,
                        Ok(
                            QuestTransitionOutcome::Refused(_)
                            | QuestTransitionOutcome::ObligationClosed,
                        ) => {}
                        Err(
                            CharacterProgressionError::Unavailable(_)
                            | CharacterProgressionError::CharacterRevisionMismatch,
                        ) => {
                            self.schedule_quest_refresh(pending.session);
                            return false;
                        }
                        Err(error) => {
                            super::operator_event(&format!("quest_hit_refused reason={error:?}"))
                        }
                    }
                }
            }
            {
                let attack = self.states.attack.lock().await;
                let Ok(mut cache) = attack.kills().1.lock() else {
                    return false;
                };
                cache.hits.acknowledge(&pending);
            }
            if accepted {
                self.refresh_quest_session(pending.session, false).await;
            } else {
                self.schedule_quest_refresh(pending.session);
            }
        }
        let attack = self.states.attack.lock().await;
        let Ok(cache) = attack.kills().1.lock() else {
            return false;
        };
        !cache.hits.has_pending()
    }
}
