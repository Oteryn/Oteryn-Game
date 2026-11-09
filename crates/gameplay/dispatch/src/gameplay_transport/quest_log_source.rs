//! Read the admitted session copy; journal queries never create another durable loader.

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
