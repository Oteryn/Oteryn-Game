use super::*;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::durability::prey::PreyCheckpoint;
use oteryn_gameplay_characters::stamina::HuntingActivityClock;
use oteryn_gameplay_progression::prey::difficulty::{PreyActivityKind, PreyDefenseMode};

#[derive(Debug, Default, Clone, Copy)]
struct SlotActivity {
    clock: HuntingActivityClock,
    spent: u32,
    expired_at: Option<u64>,
}

/// A committed combat fact. The caller supplies the canonical race from its current
/// creature snapshot; client commands never supply activity or semantic time.
pub(in crate::gameplay_transport) struct PreyActivity<'a> {
    pub creature: &'a str,
    pub kind: PreyActivityKind,
    pub now: u64,
}

#[derive(Debug)]
pub(in crate::gameplay_transport) struct LivePrey {
    actor: ExactActorRef,
    session: GameSessionId,
    reply: wire::Reply,
    permanent: [bool; 3],
    activity: [SlotActivity; 3],
    defense_mode: PreyDefenseMode,
    pending: Option<([u8; 16], Vec<PreyCheckpoint>)>,
}
impl ChannelSpellStates {
    pub(in crate::gameplay_transport) fn clear_prey(&mut self, session: GameSessionId) {
        self.prey.retain(|p| p.session != session);
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
        defense_mode: PreyDefenseMode,
    ) {
        self.prey
            .retain(|p| runtime.player_control_facts(p.actor, p.session).is_ok());
        if self.get(runtime, actor, session).is_none() {
            return;
        }
        if let Some(p) = self
            .prey
            .iter_mut()
            .find(|p| p.actor == actor && p.session == session)
        {
            if !p.has_spending() && p.pending.is_none() {
                p.install(reply, permanent, defense_mode);
            }
        } else {
            self.prey.push(LivePrey {
                actor,
                session,
                reply,
                permanent,
                activity: [SlotActivity::default(); 3],
                defense_mode,
                pending: None,
            });
        }
    }
    pub(in crate::gameplay_transport) fn prey_activity(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        event: PreyActivity<'_>,
    ) {
        if runtime.owner_fence().is_err() || self.get(runtime, actor, session).is_none() {
            return;
        }
        let premium = self.get(runtime, actor, session)
            .is_some_and(|state| state.hunting_premium_current(event.now));
        if let Some(p) = self
            .prey
            .iter_mut()
            .find(|p| p.actor == actor && p.session == session)
        {
            p.record_activity(event, premium);
        }
    }

    /// Snapshot the active chosen race for the principal's durable kill descendant.
    pub(in crate::gameplay_transport) fn prey_kill_checkpoint(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        creature: &str,
        now: u64,
    ) -> Option<PreyCheckpoint> {
        runtime.player_control_facts(actor, session).ok()?;
        let premium = self.get(runtime, actor, session)
            .is_some_and(|state| state.hunting_premium_current(now));
        let live = self.prey.iter().find(|p| p.actor == actor && p.session == session)?;
        live.kill_checkpoint(creature, now, premium)
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
                p.actor == actor && p.session == session && p.pending.is_none()
            })
            .and_then(|p| {
                p.reply.slots.iter().find(|s| {
                    (s.slot != 2 || p.permanent[1] || premium)
                        && s.state == 2
                        && s.hunting_seconds > 0
                        && s.selected == creature
                        && s.bonus == kind
                        && s.slot.checked_sub(1).and_then(|i| p.activity.get(usize::from(i)))
                            .is_some_and(|activity| activity.spent < 300)
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
            .any(|p| p.session == session && (p.pending.is_some() || p.has_spending()))
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
                .any(|p| p.session == session && (p.pending.is_some() || p.has_spending()))
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
            if !p.has_spending() && p.pending.is_none() {
                return true;
            }
            if p.pending.is_none() {
                let Some(mut occurrence) = super::super::SecureIdentifiers::random::<16>() else {
                    return false;
                };
                occurrence[6] = (occurrence[6] & 15) | 0x70;
                occurrence[8] = (occurrence[8] & 63) | 0x80;
                p.pending = Some((occurrence, p.checkpoints()));
            }
            p.pending.clone()
        };
        let Some((occurrence, checkpoints)) = pending else {
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
            .prey_commit(admitted, occurrence, PreyMutation::Checkpoint(checkpoints.clone()))
            .await
        else {
            return false;
        };
        let mut states = self.states.spell_states.lock().await;
        if let Some(p) = states.prey.iter_mut().find(|p| p.session == session) {
            // A refusal must retain the exact pending binding for reconciliation.
            if reply.status != 0 {
                return false;
            }
            for checkpoint in &checkpoints {
                if let Some(activity) = checkpoint.slot.checked_sub(1)
                    .and_then(|index| p.activity.get_mut(usize::from(index))) {
                    activity.spent = activity.spent.saturating_sub(checkpoint.seconds);
                }
            }
            p.pending = None;
            p.install(reply, permanent, p.defense_mode);
            // Activity occurring during the DB await still belongs to the next checkpoint.
            for s in &mut p.reply.slots {
                if s.state == 2 && let Some(activity) = s.slot.checked_sub(1)
                    .and_then(|index| p.activity.get(usize::from(index))) {
                    s.hunting_seconds = s.hunting_seconds.saturating_sub(activity.spent);
                }
            }
        }
        true
    }
}

impl LivePrey {
    fn kill_checkpoint(&self, creature: &str, now: u64, premium: bool) -> Option<PreyCheckpoint> {
        let slot = self.reply.slots.iter().find(|slot| {
            let at_expiry = slot.slot.checked_sub(1)
                .and_then(|index| self.activity.get(usize::from(index)))
                .and_then(|activity| activity.expired_at)
                .is_some_and(|expired| now <= expired);
            slot.state == 2 && slot.selected == creature
                && (slot.hunting_seconds > 0 || at_expiry)
                && (slot.slot != 2 || self.permanent[1] || premium)
        })?;
        Some(PreyCheckpoint {
            slot: slot.slot, creature: slot.selected.clone(), selection_epoch: slot.selection_epoch,
            seconds: 0, kills: 1, loot_gold: 0,
        })
    }

    fn has_spending(&self) -> bool {
        self.activity.iter().any(|activity| activity.spent > 0)
    }

    fn install(&mut self, reply: wire::Reply, permanent: [bool; 3], defense_mode: PreyDefenseMode) {
        for slot in &reply.slots {
            let unchanged = self.reply.slots.iter().any(|previous| {
                previous.slot == slot.slot && previous.selection_epoch == slot.selection_epoch
                    && previous.selected == slot.selected && previous.bonus == slot.bonus
            });
            if !unchanged && let Some(activity) = slot.slot.checked_sub(1)
                .and_then(|index| self.activity.get_mut(usize::from(index))) {
                *activity = SlotActivity::default();
            }
        }
        self.reply = reply;
        self.permanent = permanent;
        self.defense_mode = defense_mode;
    }

    fn record_activity(&mut self, event: PreyActivity<'_>, premium: bool) {
        for slot in &mut self.reply.slots {
            if slot.state != 2 || slot.hunting_seconds == 0 || slot.selected != event.creature
                || (slot.slot == 2 && !self.permanent[1] && !premium)
                || !self.defense_mode.consumes(slot.bonus, event.kind) {
                continue;
            }
            let Some(activity) = slot.slot.checked_sub(1)
                .and_then(|index| self.activity.get_mut(usize::from(index))) else { continue };
            // PREY-DIFF (2026-11-01): each selected race has the existing minute clock.
            // An unrelated race never refreshes it; idle and stale facts never catch up.
            let (minutes, next) = activity.clock.activity(event.now, u16::MAX);
            activity.clock = next;
            let seconds = (u32::from(minutes) * 60).min(slot.hunting_seconds);
            activity.spent += seconds;
            slot.hunting_seconds -= seconds;
            if seconds > 0 && slot.hunting_seconds == 0 { activity.expired_at = Some(event.now); }
        }
    }

    fn checkpoints(&self) -> Vec<PreyCheckpoint> {
        self.reply.slots.iter().filter_map(|slot| {
            let activity = self.activity.get(usize::from(slot.slot.checked_sub(1)?))?;
            (activity.spent > 0).then(|| PreyCheckpoint {
                slot: slot.slot,
                creature: slot.selected.clone(),
                selection_epoch: slot.selection_epoch,
                seconds: activity.spent,
                kills: 0,
                loot_gold: 0,
            })
        }).collect()
    }

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


#[cfg(test)]
#[path = "live_tests.rs"]
mod tests;
