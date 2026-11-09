//! Read the admitted copy; save-scope refresh reuses the canonical durability reader.

use super::quest_log::{QuestLogObservation, QuestLogSource};
use super::{ComposedFreshAdmission, ExactActorRef, GameSessionId};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn quest_log_observation(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        since: Option<u64>,
    ) -> QuestLogObservation {
        let world = {
            let runtime = self.runtime.lock().await;
            if runtime.player_control_facts(actor, session).is_err() {
                return QuestLogObservation::Unavailable;
            }
            runtime.binding().runtime_scope().world_id()
        };
        let Some(content) = self.quest_log_content.as_ref() else {
            return QuestLogObservation::Unavailable;
        };
        let scoped = {
            let Ok(sessions) = self.states.quest_sessions.lock() else {
                return QuestLogObservation::Unavailable;
            };
            sessions
                .get(&session)
                .and_then(|entry| entry.copy.as_ref())
                .is_some_and(|copy| copy.has_save_event_scope())
        };
        if scoped {
            let Ok(Some(epoch)) = self.root.active_world_event_epoch(self.world_id).await else {
                return QuestLogObservation::Unavailable;
            };
            let refresh_version = {
                let Ok(sessions) = self.states.quest_sessions.lock() else {
                    return QuestLogObservation::Unavailable;
                };
                let Some(entry) = sessions.get(&session) else {
                    return QuestLogObservation::Unavailable;
                };
                let Some(copy) = entry.copy.as_ref() else {
                    return QuestLogObservation::Unavailable;
                };
                copy.save_event_scope_needs_refresh(Some(epoch))
                    .then_some(entry.version)
            };
            if let Some(version) = refresh_version {
                let Ok(Some(fence)) = self.current_quest_fence(session).await else {
                    return QuestLogObservation::Unavailable;
                };
                if fence.runtime_scope.world_id() != self.world_id {
                    return QuestLogObservation::Unavailable;
                }
                // Observation reloads only: it must not submit pending quest or XP
                // obligations as the admission/owner-cadence refresh can do.
                let Ok(copy) = self
                    .root
                    .read_character_quest_state(self.character, fence.character_id)
                    .await
                else {
                    return QuestLogObservation::Unavailable;
                };
                if self
                    .runtime
                    .lock()
                    .await
                    .player_control_facts(actor, session)
                    .is_err()
                {
                    return QuestLogObservation::Unavailable;
                }
                let Ok(mut sessions) = self.states.quest_sessions.lock() else {
                    return QuestLogObservation::Unavailable;
                };
                let Some(entry) = sessions.get_mut(&session) else {
                    return QuestLogObservation::Unavailable;
                };
                // Do not replace an intervening receipt or admission refresh.
                if entry.version != version {
                    return QuestLogObservation::Unavailable;
                }
                let Some(next) = entry.version.checked_add(1) else {
                    return QuestLogObservation::Unavailable;
                };
                entry.copy = Some(copy);
                entry.version = next;
            }
            if self
                .runtime
                .lock()
                .await
                .player_control_facts(actor, session)
                .is_err()
            {
                return QuestLogObservation::Unavailable;
            }
        }
        let Ok(sessions) = self.states.quest_sessions.lock() else {
            return QuestLogObservation::Unavailable;
        };
        let Some(entry) = sessions.get(&session) else {
            return QuestLogObservation::Unavailable;
        };
        let Some(copy) = entry.copy.as_ref() else {
            return QuestLogObservation::Unavailable;
        };
        if since == Some(entry.version) {
            return QuestLogObservation::Unchanged;
        }
        QuestLogObservation::Changed(QuestLogSource {
            map_marks: if entry.map_marks_world == Some(world) {
                entry.map_marks.clone()
            } else {
                Vec::new()
            },
            content: content.clone(),
            copy: std::sync::Arc::new(copy.clone()),
            version: entry.version,
        })
    }
}
