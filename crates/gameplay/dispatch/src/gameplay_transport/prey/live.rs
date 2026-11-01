use super::*;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use crate::gameplay_transport::actor_spell::ChannelSpellStates;

#[derive(Debug)]
pub(in crate::gameplay_transport) struct LivePrey {
    actor: ExactActorRef,
    session: GameSessionId,
    reply: wire::Reply,
    permanent: [bool; 3],
    spent: u32,
    pending: Option<([u8; 16], u32)>,
}
impl ChannelSpellStates {
    pub(in crate::gameplay_transport) fn house_prey_transfer_ready(
        &self, actor: ExactActorRef, source: GameSessionId, destination: GameSessionId,
    ) -> bool {
        !self.prey.iter().any(|state| state.session == destination)
            && self.prey.iter().filter(|state| state.actor == actor && state.session == source)
                .all(|state| state.spent == 0 && state.pending.is_none())
    }

    /// Preserve the same selected slots and clocks after the real native
    /// handoff; only the session lookup changes. Pending native writes must
    /// have settled under their original source before the handoff starts.
    pub(in crate::gameplay_transport) fn move_house_prey(
        &mut self, actor: ExactActorRef, source: GameSessionId, destination: GameSessionId,
    ) {
        for state in &mut self.prey {
            if state.actor == actor && state.session == source { state.session = destination; }
        }
    }

    pub(in crate::gameplay_transport) fn clear_prey(&mut self, session: GameSessionId) {
        self.prey.retain(|p| p.session != session);
    }

    #[cfg(test)]
    pub(in crate::gameplay_transport) fn house_prey_transfer_observation(
        &self, actor: ExactActorRef, session: GameSessionId,
    ) -> Option<(Vec<u8>, [bool;3], u32, bool)> {
        let state = self.prey.iter().find(|state| state.actor == actor && state.session == session)?;
        Some((wire::encode_reply(&state.reply).ok()?, state.permanent, state.spent, state.pending.is_some()))
    }

    pub(in crate::gameplay_transport) fn apply_prey_spell_damage(
        &self,
        runtime: &ChannelRuntimeV1,
        batch: &mut crate::spell::combat_batch::OwnerCombatBatch,
    ) {
        for effect in &mut batch.effects {
            if let crate::spell::combat_batch::OwnerCombatChange::Damage { magnitude, .. } =
                &mut effect.change
                && let Ok(snapshot) = runtime.companion_snapshot(effect.target)
            {
                let percent = self.prey_percent(
                    runtime,
                    batch.caster,
                    batch.command.game_session_id(),
                    &snapshot.state.policy.definition_key,
                    1,
                    batch.now_us.map_or(batch.now_ms * 1000, |at| at.get()),
                );
                *magnitude = super::damage(*magnitude, percent);
            }
        }
    }

    pub(in crate::gameplay_transport) fn load_prey(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        reply: wire::Reply,
        permanent: [bool; 3],
    ) {
        self.prey.retain(|p| runtime.player_control_facts(p.actor, p.session).is_ok()
            || runtime.house_player_control_facts(p.actor, p.session).is_ok());
        if self.get(runtime, actor, session).is_none() {
            return;
        }
        self.install_admitted_prey(actor, session, reply, permanent);
    }

    /// Only data prepared by the real source admission is installed here.
    /// House combat/Prey commands retain their existing unsupported gates.
    pub(in crate::gameplay_transport) fn load_house_prey(
        &mut self, runtime: &ChannelRuntimeV1, actor: ExactActorRef, session: GameSessionId,
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
        reply: wire::Reply, permanent: [bool; 3],
    ) -> Option<()> {
        self.house_state_mut(runtime, actor, session, fence)?;
        self.prey.retain(|p| runtime.player_control_facts(p.actor, p.session).is_ok()
            || runtime.house_player_control_facts(p.actor, p.session).is_ok());
        self.install_admitted_prey(actor, session, reply, permanent);
        Some(())
    }

    fn install_admitted_prey(&mut self, actor: ExactActorRef, session: GameSessionId,
        reply: wire::Reply, permanent: [bool; 3]) {
        if let Some(p) = self
            .prey
            .iter_mut()
            .find(|p| p.actor == actor && p.session == session)
        {
            if p.spent == 0 && p.pending.is_none() {
                p.reply = reply;
                p.permanent = permanent;
            }
        } else {
            self.prey.push(LivePrey {
                actor,
                session,
                reply,
                permanent,
                spent: 0,
                pending: None,
            });
        }
    }
    pub(in crate::gameplay_transport) fn prey_hunting_tick(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        seconds: u32,
    ) {
        if let Some(p) = self
            .prey
            .iter_mut()
            .find(|p| p.actor == actor && p.session == session)
        {
            p.spent = p.spent.saturating_add(seconds).min(7200);
            for slot in &mut p.reply.slots {
                if slot.state == 2 {
                    slot.hunting_seconds = slot.hunting_seconds.saturating_sub(seconds);
                }
            }
        }
    }
    pub(in crate::gameplay_transport) fn prey_percent(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        creature: &str,
        kind: u8,
        now: u64,
    ) -> u8 {
        if runtime.player_control_facts(actor, session).is_err() {
            return 0;
        }
        let premium = self
            .get(runtime, actor, session)
            .is_some_and(|s| s.hunting_premium_current(now));
        self.prey
            .iter()
            .find(|p| {
                p.actor == actor && p.session == session && p.pending.is_none() && p.spent < 300
            })
            .and_then(|p| {
                p.reply.slots.iter().find(|s| {
                    (s.slot != 2 || p.permanent[1] || premium)
                        && s.state == 2
                        && s.hunting_seconds > 0
                        && s.selected == creature
                        && s.bonus == kind
                })
            })
            .map_or(0, |s| s.percent)
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn flush_prey(&self, session: GameSessionId) -> bool {
        if !self
            .states
            .spell_states
            .lock()
            .await
            .prey
            .iter()
            .any(|p| p.session == session && (p.pending.is_some() || p.spent > 0))
        {
            return true;
        }
        if self.drain_kill_rewards(session).await
            != super::super::kill_reward::DrainOutcome::Drained
        {
            return false;
        }
        let Some(permit) = self.spell_lane_permit().await else {
            return false;
        };
        self.flush_prey_under_lane(session, &permit).await
    }

    pub(in crate::gameplay_transport) async fn flush_prey_under_lane(
        &self,
        session: GameSessionId,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) -> bool {
        if permit
            .check_channel(self.world_id, self.channel_id)
            .is_err()
        {
            return false;
        }
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return false;
        };
        // A bonus mutation must not reprice an already captured party reward. The lane
        // prevents new deaths until the mutation and its live projection are installed.
        if self
            .states
            .attack
            .lock()
            .await
            .kills()
            .blocks_prey(fence.character_id.as_bytes())
        {
            return false;
        }
        // First reconcile a retained attempt, then persist any hunting accumulated meanwhile.
        for _ in 0..2 {
            if !self.flush_prey_once(session).await {
                return false;
            }
            if !self
                .states
                .spell_states
                .lock()
                .await
                .prey
                .iter()
                .any(|p| p.session == session && (p.pending.is_some() || p.spent > 0))
            {
                return true;
            }
        }
        false
    }
    async fn flush_prey_once(&self, session: GameSessionId) -> bool {
        let pending = {
            let mut states = self.states.spell_states.lock().await;
            let Some(p) = states.prey.iter_mut().find(|p| p.session == session) else {
                return true;
            };
            if p.spent == 0 && p.pending.is_none() {
                return true;
            }
            if p.pending.is_none() {
                let Some(mut occurrence) = super::super::SecureIdentifiers::random::<16>() else {
                    return false;
                };
                occurrence[6] = (occurrence[6] & 15) | 0x70;
                occurrence[8] = (occurrence[8] & 63) | 0x80;
                p.pending = Some((occurrence, p.spent));
            }
            p.pending
        };
        let Some((occurrence, seconds)) = pending else {
            return true;
        };
        let Ok(current) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(self.root.clone())
                .current_session(session)
                .await
        else {
            return false;
        };
        let Some(item) = super::super::item_fence_of(current) else {
            return false;
        };
        let admitted = AdmittedSession {
            game_session_id: session,
            world_id: item.runtime_scope.world_id(),
            channel_id: self.channel_id,
            runtime_actor: None,
            first_entry: super::super::connection::FirstEntryOutcome::NotApplicable,
            controller: None,
            continuity: super::super::connection::SessionContinuity::FRESH,
            item_fence: Some(item),
        };
        let Ok((reply, permanent)) = self
            .prey_commit(admitted, occurrence, PreyMutation::Checkpoint(seconds))
            .await
        else {
            return false;
        };
        let mut states = self.states.spell_states.lock().await;
        if let Some(p) = states.prey.iter_mut().find(|p| p.session == session) {
            p.spent = p.spent.saturating_sub(seconds);
            p.pending = None;
            p.reply = reply;
            p.permanent = permanent;
            // Activity occurring during the DB await still belongs to the next checkpoint.
            for s in &mut p.reply.slots {
                if s.state == 2 {
                    s.hunting_seconds = s.hunting_seconds.saturating_sub(p.spent);
                }
            }
        }
        true
    }
}

impl LivePrey {
    pub(in crate::gameplay_transport) fn activity_matches(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        self.actor == actor && self.session == session
    }
    pub(in crate::gameplay_transport) fn activity_pending(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        self.activity_matches(actor, session) && self.pending.is_some()
    }
    pub(in crate::gameplay_transport) fn rebind_activity(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) {
        self.actor = actor;
        self.session = session;
    }
}
