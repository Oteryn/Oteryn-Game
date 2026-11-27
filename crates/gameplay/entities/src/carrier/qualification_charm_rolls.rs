use std::collections::VecDeque;

use super::*;

#[derive(Debug)]
pub(super) struct ForcedRolls {
    actor: ExactActorRef,
    session: GameSessionId,
    lease: CharacterLease,
    target: ExactActorRef,
    rolls: VecDeque<u16>,
}

impl ChannelRuntimeV1 {
    /// Disposable qualification only. Bind the roll queue to the current lease
    /// and to a single live Troll incarnation in this runtime.
    pub fn qualification_set_enflame_rolls(
        &self,
        session: GameSessionId,
        target_identity: [u8; 16],
        target_generation: u64,
        rolls: VecDeque<u16>,
    ) -> Option<()> {
        if rolls.is_empty() || rolls.len() > 64 || rolls.iter().any(|&roll| roll >= 10_000) {
            return None;
        }
        let (actor, _) = self.positioned_player_for_session(session).ok()??;
        let lease = self.bound_attacker_lease(actor, session).ok()?;
        let target =
            self.positioned_actor_census()
                .ok()?
                .into_iter()
                .find_map(|(actor, _, session)| {
                    (session.is_none()
                        && actor.placement_identity() == target_identity
                        && actor.actor_local_generation() == target_generation)
                        .then_some(actor)
                })?;
        let creature = self.companion_snapshot(target).ok()?;
        if creature.state.policy.definition_key != "oteryn:creature.troll"
            || creature.state.master.is_some()
        {
            return None;
        }
        let mut armed = self.qualification_charm_rolls.lock().ok()?;
        armed.retain(|old| {
            old.actor != actor
                && self.player_control_facts(old.actor, old.session).is_ok()
                && self.companion_snapshot(old.target).is_ok()
        });
        armed.push(ForcedRolls {
            actor,
            session,
            lease,
            target,
            rolls,
        });
        Some(())
    }

    /// The caller has already resolved the current attacker lease and reached an Enflame roll.
    pub fn qualification_take_enflame_roll(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        lease: CharacterLease,
        target: ExactActorRef,
    ) -> Option<u32> {
        let mut armed = self.qualification_charm_rolls.lock().ok()?;
        let index = armed.iter().position(|entry| {
            entry.actor == actor
                && entry.session == session
                && entry.lease == lease
                && entry.target == target
        })?;
        let roll = armed[index].rolls.pop_front()?;
        if armed[index].rolls.is_empty() {
            armed.swap_remove(index);
        }
        Some(u32::from(roll))
    }
}
