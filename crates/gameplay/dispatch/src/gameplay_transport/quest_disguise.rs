//! Cosmetic projection consumes only a fenced Quest copy and this generation's Creature policy.
use super::{ComposedFreshAdmission, GameSessionId};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn refresh_quest_disguise(&self, session: GameSessionId) -> bool {
        let Some(native) = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            return false;
        };
        let creature = self.states.quest_sessions.lock().ok().and_then(|sessions| {
            sessions
                .get(&session)
                .and_then(|entry| entry.copy.as_ref())
                .and_then(|copy| copy.read_tracks(self.quest_catalogue.as_deref()?))
                .and_then(|tracks| {
                    oteryn_gameplay_quests::quest::world_callbacks::active_disguise(&tracks)
                        .ok()
                        .flatten()
                        .map(|rule| rule.creature.clone())
                })
        });
        let look = creature.as_ref().and_then(|key| {
            native
                .policies()
                .policies()
                .iter()
                .find(|policy| {
                    policy.definition_key == *key && policy.definition_revision == "definition-r1"
                })
                .and_then(|policy| policy.outfit_appearance())
        });
        let runtime = self.runtime.lock().await;
        let Some(actor) = runtime.positioned_actor_census().ok().and_then(|actors| {
            actors
                .into_iter()
                .find(|(_, _, present)| *present == Some(session))
                .map(|(actor, _, _)| actor)
        }) else {
            return false;
        };
        if runtime.player_control_facts(actor, session).is_err() {
            return false;
        }
        let mut states = self.states.spell_states.lock().await;
        if states.is_dead(actor) {
            return false;
        }
        if !states.spell_idle(&runtime, actor, session) {
            return true;
        }
        let Some(current) = states.get(&runtime, actor, session) else {
            return true;
        };
        match crate::spell::actor_conditions::stage_quest_disguise(
            current,
            look,
            self.owner_now().get() / 1000,
        ) {
            Ok(Some(next)) => {
                if !states.commit(&runtime, actor, session, next) {
                    return true;
                }
                states.owner_wake.notify_one();
                false
            }
            Ok(None) => false,
            Err(_) => true,
        }
    }
}
