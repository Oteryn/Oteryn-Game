//! Retain the existing actor-owned attack state across a physical activity move.
//! Extraction precedes the native freeze, so ordinary pruning cannot discard a
//! frozen source actor's live deadlines while its durable outcome is uncertain.
use super::*;
use crate::foundation::{ActivityTransferMember, ActivityTransferredActor, CharacterLease};

#[derive(Debug)]
struct HeldAttackState {
    source_actor: ExactActorRef,
    source_session: GameSessionId,
    destination_session: GameSessionId,
    source_lease: CharacterLease,
    destination_lease: CharacterLease,
    attack: Option<ActorAttack>,
    step_clock: Option<StepClock>,
}

#[derive(Debug)]
#[must_use]
pub(in crate::gameplay_transport) struct PreparedActivityAttackStates {
    players: Vec<HeldAttackState>,
}

/// All mapping/conflict checks precede installation of either spell or attack
/// state. The caller holds the destination attack guard until this token commits.
#[must_use]
pub(in crate::gameplay_transport) struct ActivityAttackCommit<'a> {
    destination: &'a mut ChannelAttackStates,
    held: &'a mut PreparedActivityAttackStates,
    moved: &'a [ActivityTransferredActor],
}

impl ActivityAttackCommit<'_> {
    pub(in crate::gameplay_transport) fn commit(self) {
        for (mut player, next) in self.held.players.drain(..).zip(self.moved) {
            if let Some(mut attack) = player.attack.take() {
                attack.actor = next.destination_actor;
                attack.session = next.destination_session;
                // The shared owner clock is unchanged. clear_target preserves
                // AttackState's swing history, sequence and in-fight deadline.
                attack.clear_target();
                attack.target_set_at = SemanticTimeMicros::from_micros(0);
                attack.next_ordinal = 0;
                attack.weapon = None;
                attack.weapon_epoch = attack.weapon_epoch.wrapping_add(1);
                if let Some(activity) = &mut attack.shared_experience_activity
                    && activity.character == *player.source_lease.character_id().as_bytes()
                    && activity.lease_generation == player.source_lease.generation()
                {
                    // Rebind the proven existing observation, never renew its
                    // timestamp or promote an older unqualified lease's event.
                    activity.lease_generation = next.destination_lease.generation();
                }
                self.destination.entries.push(attack);
            }
            if let Some(clock) = player.step_clock.take() {
                self.destination
                    .step_clocks
                    .push((next.destination_actor, clock));
            }
        }
    }
}

impl ChannelAttackStates {
    pub(in crate::gameplay_transport) fn prepare_activity_attacks(
        &mut self,
        runtime: &ChannelRuntimeV1,
        destination: &mut Self,
        members: &[ActivityTransferMember],
    ) -> Result<PreparedActivityAttackStates, ()> {
        if members.is_empty() {
            return Err(());
        }
        for (ordinal, member) in members.iter().enumerate() {
            runtime
                .player_control_facts(member.source_actor, member.source_session)
                .map_err(|_| ())?;
            if runtime
                .bound_attacker_lease(member.source_actor, member.source_session)
                .map_err(|_| ())?
                != member.source_lease
            {
                return Err(());
            }
            if members[..ordinal].iter().any(|old| {
                old.source_actor == member.source_actor
                    || old.source_session == member.source_session
                    || old.destination_session == member.destination_session
            }) || destination
                .entries
                .iter()
                .any(|entry| entry.session == member.destination_session)
                || self.step_clocks.iter().any(|(actor, clock)| {
                    *actor == member.source_actor
                        && matches!(clock, StepClock::InFlight(holder) if holder.strong_count() > 0)
                })
            {
                // A real in-flight movement owner must finish its existing
                // pacing claim before its actor can change namespace.
                return Err(());
            }
        }
        let mut players = Vec::new();
        players.try_reserve_exact(members.len()).map_err(|_| ())?;
        destination
            .entries
            .try_reserve(members.len())
            .map_err(|_| ())?;
        destination
            .step_clocks
            .try_reserve(members.len())
            .map_err(|_| ())?;
        for member in members {
            let attack = self
                .index(member.source_actor, member.source_session)
                .map(|index| self.entries.swap_remove(index));
            let step_clock = self
                .step_clocks
                .iter()
                .position(|(actor, _)| *actor == member.source_actor)
                .map(|index| self.step_clocks.swap_remove(index).1);
            players.push(HeldAttackState {
                source_actor: member.source_actor,
                source_session: member.source_session,
                destination_session: member.destination_session,
                source_lease: member.source_lease,
                destination_lease: member.destination_lease,
                attack,
                step_clock,
            });
        }
        Ok(PreparedActivityAttackStates { players })
    }

    pub(in crate::gameplay_transport) fn abort_activity_attacks(
        &mut self,
        held: &mut PreparedActivityAttackStates,
    ) {
        for player in held.players.drain(..) {
            if let Some(attack) = player.attack {
                self.entries.push(attack);
            }
            if let Some(clock) = player.step_clock {
                self.step_clocks.push((player.source_actor, clock));
            }
        }
    }

    pub(in crate::gameplay_transport) fn prepare_activity_attack_commit<'a>(
        &'a mut self,
        held: &'a mut PreparedActivityAttackStates,
        moved: &'a [ActivityTransferredActor],
    ) -> Result<ActivityAttackCommit<'a>, ()> {
        if held.players.len() != moved.len()
            || held.players.iter().zip(moved).any(|(player, next)| {
                player.source_actor != next.source_actor
                    || player.source_session != next.source_session
                    || player.destination_session != next.destination_session
                    || player.destination_lease != next.destination_lease
                    || player.source_lease.character_id() != next.character_id
                    || self.entries.iter().any(|entry| {
                        entry.actor == next.destination_actor
                            || entry.session == next.destination_session
                    })
                    || self
                        .step_clocks
                        .iter()
                        .any(|(actor, _)| *actor == next.destination_actor)
            })
            || moved.iter().enumerate().any(|(index, next)| {
                moved[..index].iter().any(|old| {
                    old.destination_actor == next.destination_actor
                        || old.destination_session == next.destination_session
                })
            })
        {
            return Err(());
        }
        Ok(ActivityAttackCommit {
            destination: self,
            held,
            moved,
        })
    }
}

#[cfg(test)]
#[path = "activity_transfer_tests.rs"]
mod tests;
