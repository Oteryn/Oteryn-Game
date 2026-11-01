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
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, GameSessionState};
use oteryn_gameplay_characters::stamina::{HuntingActivityClock, HuntingClock};

#[derive(Debug)]
pub(super) struct LiveStamina {
    actor: ExactActorRef,
    session: GameSessionId,
    minutes: u16,
    durable: u16,
    clock: HuntingClock,
    experience_started: bool,
    prey_clock: HuntingActivityClock,
    pending_gain: Option<(BuildOccurrence, u16, u16)>,
    pending_debit: Option<(BuildOccurrence, u16, u16)>,
}

impl ChannelSpellStates {
    pub(in crate::gameplay_transport) fn house_stamina_transfer_ready(
        &self,
        actor: ExactActorRef,
        source: GameSessionId,
        destination: GameSessionId,
    ) -> bool {
        !self
            .hunting_stamina
            .iter()
            .any(|state| state.session == destination)
            && self
                .hunting_stamina
                .iter()
                .filter(|state| state.actor == actor && state.session == source)
                .all(|state| {
                    state.minutes == state.durable
                        && state.pending_gain.is_none()
                        && state.pending_debit.is_none()
                })
    }

    /// Caller retains native runtime/state ownership and has checked the
    /// successor handoff. Keep the exact HuntingClock and Prey activity clock.
    pub(in crate::gameplay_transport) fn move_house_stamina(
        &mut self,
        actor: ExactActorRef,
        source: GameSessionId,
        destination: GameSessionId,
    ) {
        for state in &mut self.hunting_stamina {
            if state.actor == actor && state.session == source {
                state.session = destination;
            }
        }
    }

    pub(super) fn load_stamina(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        minutes: u16,
    ) {
        self.load_stamina_at(runtime, actor, session, minutes, 0);
    }

    pub(super) fn load_stamina_at(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        minutes: u16,
        now: u64,
    ) {
        self.hunting_stamina.retain(|state| {
            runtime
                .player_control_facts(state.actor, state.session)
                .is_ok()
                || runtime
                    .house_player_control_facts(state.actor, state.session)
                    .is_ok()
        });
        if self.get(runtime, actor, session).is_some() {
            self.install_admitted_stamina(actor, session, minutes, now);
        }
    }

    /// Explicit House admission only; no generic Channel gameplay access is widened.
    pub(in crate::gameplay_transport) fn load_house_stamina(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
        minutes: u16,
        now: u64,
    ) -> Option<()> {
        let house = runtime.house_actor_state(actor, session).ok()?;
        if fence.game_session_id != session
            || house.scope != fence.runtime_scope
            || house.generation != fence.scope_ownership_generation
            || house.lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || house.lease.generation() != fence.character_lease_generation
            || !self
                .actors
                .iter()
                .any(|(a, s, _)| *a == actor && *s == session)
        {
            return None;
        }
        self.install_admitted_stamina(actor, session, minutes, now);
        Some(())
    }

    fn install_admitted_stamina(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        minutes: u16,
        now: u64,
    ) {
        if let Some(state) = self
            .hunting_stamina
            .iter_mut()
            .find(|s| s.actor == actor && s.session == session)
        {
            // Admission settlement may regenerate before the first hunting event.
            // A same-session reconnect must preserve live spending and replay state.
            if !state.experience_started
                && state.pending_gain.is_none()
                && state.pending_debit.is_none()
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
                clock: HuntingClock::new(now),
                experience_started: false,
                prey_clock: HuntingActivityClock::default(),
                pending_gain: None,
                pending_debit: None,
            });
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
            // Damage activity continues to drive Prey time, never character stamina.
            let (hunting_minutes, next) = state.prey_clock.activity(now, u16::MAX);
            state.prey_clock = next;
            self.prey_hunting_tick(actor, session, u32::from(hunting_minutes) * 60);
        }
    }
    /// Preview under the same runtime/state guards as the kill XP transaction.
    pub(super) fn stamina_after_experience(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        now: u64,
    ) -> Option<u16> {
        let state = self
            .hunting_stamina
            .iter()
            .find(|s| s.actor == actor && s.session == session)?;
        Some(state.minutes - state.clock.activity(now, state.minutes).0)
    }

    /// Publish only after the XP transaction commits or reconciles. Its original death time
    /// is already behind the new deadline on replay, so retries cannot spend twice.
    pub(super) fn hunting_experience(
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
            state.minutes -= spent;
            state.clock = next;
            state.experience_started |= spent > 0;
        }
    }

    pub(super) fn live_stamina(&self, actor: ExactActorRef, session: GameSessionId) -> Option<u16> {
        self.hunting_stamina
            .iter()
            .find(|s| s.actor == actor && s.session == session)
            .map(|s| s.minutes)
    }
    pub(super) fn gain_resting_stamina(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        let Some(state) = self
            .hunting_stamina
            .iter_mut()
            .find(|state| state.actor == actor && state.session == session)
        else {
            return false;
        };
        let next = state.minutes.saturating_add(1).min(2520);
        let changed = next != state.minutes;
        state.minutes = next;
        changed
    }
    pub(super) fn stamina_to_flush(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        self.hunting_stamina.iter().any(|s| {
            s.actor == actor
                && s.session == session
                && (s.minutes != s.durable || s.pending_gain.is_some() || s.pending_debit.is_some())
        })
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn save_resting_stamina(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> TrainingSave {
        if !self.flush_prey(session).await {
            return TrainingSave::Unknown;
        }
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
            || item.runtime_scope != runtime.binding().runtime_scope()
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
        if state.pending_gain.is_none() && state.minutes <= state.durable {
            return TrainingSave::NotApplicable;
        }
        let (occurrence, before, after) = match state.pending_gain {
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
                state.pending_gain = Some(pending);
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
            .commit_resting_stamina(
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
        state.pending_gain = None;
        TrainingSave::Saved
    }

    pub(super) async fn save_hunting_stamina(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> TrainingSave {
        if !self.flush_prey(session).await {
            return TrainingSave::Unknown;
        }
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
            || item.runtime_scope != runtime.binding().runtime_scope()
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
        if state.pending_debit.is_none() && state.minutes >= state.durable {
            return TrainingSave::NotApplicable;
        }
        let (occurrence, before, after) = match state.pending_debit {
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
                state.pending_debit = Some(pending);
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
        state.pending_debit = None;
        TrainingSave::Saved
    }
}

impl LiveStamina {
    pub(super) fn activity_matches(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        self.actor == actor && self.session == session
    }
    pub(super) fn activity_pending(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        self.activity_matches(actor, session)
            && (self.pending_gain.is_some() || self.pending_debit.is_some())
    }
    pub(super) fn rebind_activity(&mut self, actor: ExactActorRef, session: GameSessionId) {
        self.actor = actor;
        self.session = session;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::creature_bite::CreatureBiteVitals;
    use crate::gameplay_transport::actor_spell::tests as fixture;

    #[test]
    #[allow(clippy::expect_used)]
    fn house_handoff_keeps_stamina_minutes_and_the_same_hunting_clock() {
        let (runtime, actor, source) = fixture::runtime_with_player(144);
        let destination =
            GameSessionId::decode(&[0, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 198])
                .expect("destination");
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                actor,
                source,
                fixture::FACTS,
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(8_000_000),
            )
            .expect("player");
        states.load_stamina_at(&runtime, actor, source, 100, 8_000_000);
        let instants = [8_000_000, 60_000_000, 68_000_000, 128_000_000];
        let before = instants.map(|at| states.stamina_after_experience(actor, source, at));
        assert!(states.house_stamina_transfer_ready(actor, source, destination));
        states.move_house_stamina(actor, source, destination);
        assert_eq!(states.live_stamina(actor, source), None);
        assert_eq!(states.live_stamina(actor, destination), Some(100));
        assert_eq!(
            instants.map(|at| states.stamina_after_experience(actor, destination, at)),
            before
        );
        assert!(states.gain_resting_stamina(actor, destination));
        assert!(
            !states.house_stamina_transfer_ready(actor, destination, source),
            "uncheckpointed source gains cannot be stranded in a House"
        );
    }

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
        states.hunting_experience(&runtime, actor, session, 1_000_000);
        states.load_stamina(&runtime, actor, session, 841);
        assert_eq!(states.live_stamina(actor, session), Some(840));
        // This adapter also serves PvP swings; only its monster caller records activity.
        let mut attack = super::super::attack::ChannelAttackStates::default();
        let mut trained = Vec::new();
        let mut vitals = super::super::attack::player_block::PlayerBiteVitals {
            quest_sessions: None,
            charm: None,
            conditions: None,
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
    #[test]
    #[allow(clippy::expect_used)] // Content and actor setup must succeed in this unit fixture.
    fn hunting_stamina_no_kill_in_protection_zone_never_drains() {
        use crate::foundation::{ChannelContentPin, NodeId};
        use crate::map::boot::tests::{floor_items, floor_pins, teleport_bundle};
        use oteryn_simulation_determinism::SemanticTimeMicros;
        let (template, _, session) = fixture::runtime_with_player(147);
        let world = template.binding().world_id();
        let channel = template.binding().channel_id();
        let room = crate::content::qualify_native_entry_room(world).expect("room");
        let (bytes, pins) = teleport_bundle();
        let map = crate::map::boot::boot(
            &bytes,
            &floor_pins(pins),
            world,
            channel.expect("W02: pinned Channel fixture"),
            floor_items,
        )
        .expect("map");
        let cells = map.movement_cells(room.movement_cells()).expect("cells");
        let mut node = [147; 16];
        node[6] = 0x70;
        node[8] = 0x80;
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            room.compiled().server_digest(),
            room.compiled().client_digest(),
            map.frame_binding_digest(world),
            map.map_revision_digest(),
            (5, 1, 8),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel.expect("W02: pinned Channel fixture"),
            NodeId::decode(&node).expect("node"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .expect("runtime");
        let reservation = runtime.reserve_fresh_session(session).expect("reservation");
        let actor = runtime.commit_fresh_session(reservation).expect("actor");
        runtime
            .initialize_first_entry_position(actor)
            .expect("position");
        assert_eq!(
            super::super::actor_spell::qualified_protection_zone(&mut runtime, &cells, actor),
            Some(true)
        );
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                actor,
                session,
                fixture::FACTS,
                (0, 0),
                SemanticTimeMicros::from_micros(0),
            )
            .expect("state");
        states.load_stamina(&runtime, actor, session, 2520);
        for second in [1, 30, 61, 122, 3600] {
            states.tick_in_environment(
                &mut runtime,
                &cells,
                actor,
                session,
                SemanticTimeMicros::from_micros(second * 1_000_000),
            );
            assert_eq!(states.live_stamina(actor, session), Some(2520));
            assert!(!states.stamina_to_flush(actor, session));
        }
        // Monster damage activity must not consume stamina either.
        for second in [3601, 3631, 3661] {
            states.hunting_activity(&runtime, actor, session, second * 1_000_000);
            assert_eq!(states.live_stamina(actor, session), Some(2520));
        }
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn retained_resting_gain_does_not_hide_a_later_hunting_debit() {
        let (runtime, actor, session) = fixture::runtime_with_player(146);
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
        states.load_stamina(&runtime, actor, session, 100);
        assert!(states.gain_resting_stamina(actor, session));
        let state = states
            .hunting_stamina
            .iter_mut()
            .find(|state| state.actor == actor && state.session == session)
            .expect("stamina");
        let gain =
            BuildOccurrence::from_bytes([1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])
                .expect("occurrence");
        state.pending_gain = Some((gain, 100, 101));
        // One later hunting minute cancels the live delta but not the retained gain receipt.
        state.minutes = 100;
        assert_eq!(state.pending_gain, Some((gain, 100, 101)));
        assert!(state.pending_debit.is_none());
        assert!(states.stamina_to_flush(actor, session));
    }
}
