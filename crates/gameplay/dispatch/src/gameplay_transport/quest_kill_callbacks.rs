//! Native death descendants reuse the fenced quest writer and captured contributor identities.
use super::{ComposedFreshAdmission, kill_reward::PendingKillSettlement};
use crate::durability::character_progression::ExperienceRewardOccurrence;
use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
use std::sync::Arc;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn credit_source_quest_kill(&self, entry: &PendingKillSettlement) -> bool {
        let Ok(bindings) = oteryn_gameplay_quests::quest::kill_callbacks::load() else {
            return false;
        };
        let bindings: Vec<_> = bindings
            .iter()
            .filter(|b| b.creature == entry.creature)
            .collect();
        if bindings.is_empty() {
            return true;
        }
        let Some(catalogue) = self.quest_catalogue.as_ref() else {
            return false;
        };
        let Some(recipients) = entry.task_board_recipients.get() else {
            return false;
        };
        for recipient in recipients {
            // Source forDamagingPlayers excludes shared-XP members who never damaged this death.
            if !entry.damage_shares.iter().any(|damage| {
                damage.total > 0
                    && damage.character.as_bytes() == recipient.fence.character_id.as_bytes()
                    && damage.session == Some(recipient.fence.game_session_id)
                    && damage.lease_generation == Some(recipient.fence.character_lease_generation)
            }) {
                continue;
            }
            let session = recipient.fence.game_session_id;
            let mut slot = self
                .revision_sequencer
                .acquire(recipient.fence.character_id)
                .await;
            let current = match self.current_quest_fence(session).await {
                Ok(Some(current)) => current,
                Ok(None) => continue,
                Err(_) => return false,
            };
            if current.character_id != recipient.fence.character_id
                || current.character_lease_generation != recipient.fence.character_lease_generation
                || current.runtime_scope != recipient.fence.runtime_scope
                || current.scope_ownership_generation != recipient.fence.scope_ownership_generation
            {
                continue;
            }
            let Ok(occurrence) = ExperienceRewardOccurrence::from_bytes(recipient.occurrence)
            else {
                return false;
            };
            for binding in &bindings {
                let request = QuestTransitionRequest {
                    transition_key: binding.transition.clone(),
                    cause: QuestCause::CreatureDeath(occurrence),
                };
                if slot
                    .commit_quest_transition(
                        self.root,
                        self.character,
                        self.holder,
                        current,
                        request,
                        Arc::clone(catalogue),
                    )
                    .await
                    .is_err()
                {
                    self.schedule_quest_refresh(session);
                    return false;
                }
            }
            drop(slot);
            self.refresh_quest_session(session, false).await;
        }
        true
    }
}
