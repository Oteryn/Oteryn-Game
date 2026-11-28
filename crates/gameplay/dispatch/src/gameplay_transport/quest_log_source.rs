//! Read the admitted copy; save-scope refresh reuses the canonical durability reader.

use super::quest_log::{QuestLogObservation, QuestLogSource};
use super::{AdmissionRetryBudget, ComposedFreshAdmission, ExactActorRef, GameSessionId};
use crate::durability::{DurabilityError, character_progression::CharacterProgressionError};
use std::future::Future;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn quest_log_observation(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        since: Option<u64>,
    ) -> QuestLogObservation {
        let world = {
            let runtime = self.runtime.lock().await;
            if !quest_reader(&runtime, actor, session) {
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
            // Commands and initial snapshots wait for a busy holder. The periodic
            // refresh can defer to its next tick instead of delaying the live loop.
            let retry = since
                .is_none()
                .then(|| AdmissionRetryBudget::for_command(self.root));
            let Ok(Some(epoch)) = quest_read(
                retry.as_ref(),
                "quest_log_epoch",
                || self.root.active_world_event_epoch(self.world_id),
                |error| Some(error),
            )
            .await
            else {
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
                let Ok(Some(fence)) = quest_read(
                    retry.as_ref(),
                    "quest_log_fence",
                    || self.current_quest_fence_durable(session),
                    |error| Some(error),
                )
                .await
                else {
                    return QuestLogObservation::Unavailable;
                };
                if fence.runtime_scope.world_id() != self.world_id {
                    return QuestLogObservation::Unavailable;
                }
                // Observation reloads only: it must not submit pending quest or XP
                // obligations as the admission/owner-cadence refresh can do.
                let Ok(copy) = quest_read(
                    retry.as_ref(),
                    "quest_log_copy",
                    || {
                        self.root
                            .read_character_quest_state(self.character, fence.character_id)
                    },
                    |error| match error {
                        CharacterProgressionError::Unavailable(source) => Some(source),
                        _ => None,
                    },
                )
                .await
                else {
                    return QuestLogObservation::Unavailable;
                };
                if !quest_reader(&*self.runtime.lock().await, actor, session) {
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
            if !quest_reader(&*self.runtime.lock().await, actor, session) {
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
                entry
                    .map_marks
                    .iter()
                    .cloned()
                    .map(|mut mark| {
                        mark.floor = mark.floor.saturating_abs();
                        mark
                    })
                    .collect()
            } else {
                Vec::new()
            },
            content: content.clone(),
            copy: std::sync::Arc::new(copy.clone()),
            version: entry.version,
        })
    }
}

async fn quest_read<T, E, Fut: Future<Output = Result<T, E>>>(
    retry: Option<&AdmissionRetryBudget<'_>>,
    stage: &'static str,
    mut read: impl FnMut() -> Fut,
    durability: impl Fn(&E) -> Option<&DurabilityError>,
) -> Result<T, E> {
    match retry {
        Some(retry) => retry.retry_holder(stage, read, durability).await,
        None => read().await,
    }
}

fn quest_reader(
    runtime: &crate::foundation::ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
) -> bool {
    runtime.player_control_facts(actor, session).is_ok()
        || runtime
            .house_player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
}
