//! Session-local stamina, saved by the existing actor checkpoint/logout/death path.
use super::{
    ComposedFreshAdmission, SecureIdentifiers,
    actor_spell::{ChannelSpellStates, TrainingSave},
};
use crate::durability::character_build::BuildOccurrence;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, GameSessionState, RuntimeScopeRefV1,
};
use oteryn_gameplay_characters::stamina::HuntingClock;

#[derive(Debug)]
pub(super) struct LiveStamina {
    actor: ExactActorRef,
    session: GameSessionId,
    minutes: u16,
    durable: u16,
    clock: HuntingClock,
    pending: Option<(BuildOccurrence, u16, u16)>,
}

impl ChannelSpellStates {
    pub(super) fn load_stamina(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        minutes: u16,
    ) {
        self.hunting_stamina.retain(|state| {
            runtime
                .player_control_facts(state.actor, state.session)
                .is_ok()
        });
        if self.get(runtime, actor, session).is_some() {
            if let Some(state) = self
                .hunting_stamina
                .iter_mut()
                .find(|s| s.actor == actor && s.session == session)
            {
                // Admission settlement may regenerate before the first hunting event.
                // A same-session reconnect must preserve live spending and replay state.
                if state.clock == HuntingClock::default()
                    && state.pending.is_none()
                    && state.minutes == state.durable
                {
                    state.minutes = minutes;
                    state.durable = minutes;
                }
            } else {
                self.hunting_stamina.push(LiveStamina {
                    actor,
                    session,
                    minutes,
                    durable: minutes,
                    clock: HuntingClock::default(),
                    pending: None,
                });
            }
        }
    }

    pub(super) fn hunting_activity(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        now: u64,
    ) {
        if runtime.owner_fence().is_err() || self.get(runtime, actor, session).is_none() {
            return;
        }
        if let Some(state) = self
            .hunting_stamina
            .iter_mut()
            .find(|s| s.actor == actor && s.session == session)
        {
            let (spent, next) = state.clock.activity(now, state.minutes);
            state.clock = next;
            state.minutes -= spent;
        }
    }
    pub(super) fn live_stamina(&self, actor: ExactActorRef, session: GameSessionId) -> Option<u16> {
        self.hunting_stamina
            .iter()
            .find(|s| s.actor == actor && s.session == session)
            .map(|s| s.minutes)
    }
    pub(super) fn stamina_to_flush(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        self.hunting_stamina.iter().any(|s| {
            s.actor == actor
                && s.session == session
                && (s.minutes < s.durable || s.pending.is_some())
        })
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn save_hunting_stamina(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> TrainingSave {
        {
            let states = self.states.spell_states.lock().await;
            if !states.stamina_to_flush(actor, session) {
                return TrainingSave::NotApplicable;
            }
        }
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let Ok(current) = store.current_session(session).await else {
            return TrainingSave::Unknown;
        };
        let Some(item) = super::item_fence_of(current) else {
            return TrainingSave::FencedOut;
        };
        if current.session_state() == GameSessionState::Terminal {
            return TrainingSave::FencedOut;
        }
        let mut slot = self.revision_sequencer.acquire(item.character_id).await;
        let Ok(root) = self
            .root
            .read_current_character(self.character, item.character_id)
            .await
        else {
            return TrainingSave::Unknown;
        };
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if runtime.owner_fence().is_err()
            || runtime.player_control_facts(actor, session).is_err()
            || item.runtime_scope
                != RuntimeScopeRefV1::channel(
                    runtime.binding().world_id(),
                    runtime.binding().channel_id(),
                )
            || item.scope_ownership_generation != runtime.binding().scope_generation()
        {
            return TrainingSave::FencedOut;
        }
        let Some(state) = states
            .hunting_stamina
            .iter_mut()
            .find(|s| s.actor == actor && s.session == session)
        else {
            return TrainingSave::NotApplicable;
        };
        let (occurrence, before, after) = match state.pending {
            Some(pending) => pending,
            None => {
                let Some(occurrence) = SecureIdentifiers::random::<16>().and_then(|mut bytes| {
                    bytes[6] = (bytes[6] & 15) | 0x70;
                    bytes[8] = (bytes[8] & 63) | 0x80;
                    BuildOccurrence::from_bytes(bytes).ok()
                }) else {
                    return TrainingSave::Unknown;
                };
                let pending = (occurrence, state.durable, state.minutes);
                state.pending = Some(pending);
                pending
            }
        };
        let fence = CurrentCharacterGameplayFence {
            character_id: item.character_id,
            game_session_id: session,
            connection_generation: item.connection_generation,
            character_lease_generation: item.character_lease_generation,
            runtime_scope: item.runtime_scope,
            scope_ownership_generation: item.scope_ownership_generation,
            expected_character_revision: root.revision,
        };
        let outcome = self
            .root
            .commit_hunting_stamina(
                self.character,
                self.holder,
                fence,
                occurrence,
                (before, after),
            )
            .await;
        slot.observe_loop_training();
        match outcome {
            Ok(_) => {}
            Err(
                CharacterProgressionError::AuthorityRejected
                | CharacterProgressionError::RespawnPending,
            ) => return TrainingSave::FencedOut,
            Err(_) => return TrainingSave::Unknown,
        }
        let Ok(fresh) = store.current_session(session).await else {
            return TrainingSave::Unknown;
        };
        if fresh.session_state() == GameSessionState::Terminal
            || super::item_fence_of(fresh) != Some(item)
        {
            return TrainingSave::FencedOut;
        }
        state.durable = after;
        state.pending = None;
        TrainingSave::Saved
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::creature_bite::CreatureBiteVitals;
    use crate::gameplay_transport::actor_spell::tests as fixture;

    #[test]
    #[allow(clippy::expect_used)]
    fn hunting_stamina_reconnect_preserves_spending_and_pvp_damage_is_not_hunting() {
        let (mut runtime, actor, session) = fixture::runtime_with_player(145);
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                actor,
                session,
                fixture::FACTS,
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .expect("player");
        states.load_stamina(&runtime, actor, session, 841);
        states.hunting_activity(&runtime, actor, session, 0);
        states.hunting_activity(&runtime, actor, session, 60_000_000);
        states.load_stamina(&runtime, actor, session, 841);
        assert_eq!(states.live_stamina(actor, session), Some(840));
        // This adapter also serves PvP swings; only its monster caller records activity.
        let mut attack = super::super::attack::ChannelAttackStates::default();
        let mut trained = Vec::new();
        let mut vitals = super::super::attack::player_block::PlayerBiteVitals {
            states: &mut states,
            attack: &mut attack,
            formula: None,
            trained: &mut trained,
        };
        for now in [90_000_000, 120_000_000, 150_000_000] {
            assert_eq!(
                vitals
                    .apply_creature_damage(
                        &mut runtime,
                        actor,
                        session,
                        1,
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now)
                    )
                    .expect("damage")
                    .damage
                    .applied,
                1
            );
        }
        assert_eq!(states.live_stamina(actor, session), Some(840));
    }
}
