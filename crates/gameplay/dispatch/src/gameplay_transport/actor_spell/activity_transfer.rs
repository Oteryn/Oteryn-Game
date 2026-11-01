//! Move live player-owned spell state while the native carrier freezes its actors.
//! The held value survives an unknown durable outcome. No health, mana, cooldown,
//! food, training, stamina or Prey state is recreated from admission defaults.
use super::*;
use crate::foundation::{ActivityTransferMember, ActivityTransferredActor};

#[derive(Debug)]
struct HeldPlayerState {
    actor: ExactActorRef,
    session: GameSessionId,
    spell: PlayerSpellState,
    death: Option<PlayerDeath>,
    charm: Option<crate::durability::charm_state::CombatCharmState>,
    pvp: Option<oteryn_gameplay_combat_rules::pvp::CombatLock>,
    prey: Option<super::super::prey::LivePrey>,
    stamina: Option<super::super::hunting_stamina::LiveStamina>,
    item_uses: super::super::item_use::ItemUseGates,
}

#[derive(Debug)]
#[must_use]
pub(in crate::gameplay_transport) struct PreparedActivitySpellStates {
    players: Vec<HeldPlayerState>,
}

impl ChannelSpellStates {
    pub(in crate::gameplay_transport) fn prepare_activity_states(
        &mut self,
        destination: &mut Self,
        members: &[ActivityTransferMember],
    ) -> Result<PreparedActivitySpellStates, ()> {
        self.prepare_activity_states_inner(destination, members, false)
    }

    /// Used only after the existing death and item owners prove a retained
    /// pending respawn. Preserve the same occurrence until actual placement
    /// consumes it; ordinary activity entry continues to reject dead players.
    pub(in crate::gameplay_transport) fn prepare_activity_death_states(
        &mut self,
        destination: &mut Self,
        members: &[ActivityTransferMember],
    ) -> Result<PreparedActivitySpellStates, ()> {
        if members.len() != 1 || !self.is_dead(members[0].source_actor) {
            return Err(());
        }
        self.prepare_activity_states_inner(destination, members, true)
    }

    fn prepare_activity_states_inner(
        &mut self,
        destination: &mut Self,
        members: &[ActivityTransferMember],
        allow_death: bool,
    ) -> Result<PreparedActivitySpellStates, ()> {
        if members.is_empty() || members.len() > 15 {
            return Err(());
        }
        for (ordinal, member) in members.iter().enumerate() {
            let actor = member.source_actor;
            let session = member.source_session;
            if (self.is_dead(actor) && !allow_death)
                || self.has_pending_spell_commit(actor, session)
                || (allow_death
                    && self
                        .deaths
                        .iter()
                        .filter(|(a, s, _)| *a == actor && *s == session)
                        .count()
                        != 1)
                || destination
                    .deaths
                    .iter()
                    .any(|(_, s, _)| *s == member.destination_session)
                || self
                    .actors
                    .iter()
                    .filter(|(a, s, _)| *a == actor && *s == session)
                    .count()
                    != 1
                || members[..ordinal].iter().any(|m| {
                    m.source_actor == actor || m.destination_session == member.destination_session
                })
                || destination
                    .actors
                    .iter()
                    .any(|(_, s, _)| *s == member.destination_session)
                || self.prey.iter().any(|p| p.activity_pending(actor, session))
                || self
                    .hunting_stamina
                    .iter()
                    .any(|p| p.activity_pending(actor, session))
                || !self.item_uses.activity_ready(actor, session)
            {
                return Err(());
            }
        }
        // Every allocation needed by the physical successor happens before SQL.
        let count = members.len();
        destination.actors.try_reserve(count).map_err(|_| ())?;
        destination.deaths.try_reserve(count).map_err(|_| ())?;
        destination
            .charm_states
            .try_reserve(count)
            .map_err(|_| ())?;
        destination.pvp_locks.try_reserve(count).map_err(|_| ())?;
        destination.prey.try_reserve(count).map_err(|_| ())?;
        destination
            .hunting_stamina
            .try_reserve(count)
            .map_err(|_| ())?;
        destination.item_uses.reserve_activity(count)?;
        let mut held = Vec::new();
        held.try_reserve_exact(count).map_err(|_| ())?;
        // Item-use extraction reserves its token before removing anything, so a
        // failure cannot leave a partially extracted set of live player state.
        let mut uses = Vec::new();
        uses.try_reserve_exact(count).map_err(|_| ())?;
        for member in members {
            uses.push(
                self.item_uses
                    .reserve_activity_token(member.source_actor, member.source_session)?,
            );
        }
        for (member, mut item_uses) in members.iter().zip(uses) {
            let actor = member.source_actor;
            let session = member.source_session;
            let Some(index) = self
                .actors
                .iter()
                .position(|(a, s, _)| *a == actor && *s == session)
            else {
                unreachable!("activity preflight under exclusive source state lock");
            };
            let (_, _, spell) = self.actors.swap_remove(index);
            let death = self
                .deaths
                .iter()
                .position(|(a, s, _)| *a == actor && *s == session)
                .map(|i| self.deaths.swap_remove(i).2);
            let charm = self
                .charm_states
                .iter()
                .position(|(a, s, _)| *a == actor && *s == session)
                .map(|i| self.charm_states.swap_remove(i).2);
            let pvp = self
                .pvp_locks
                .iter()
                .position(|(a, s, _)| *a == actor && *s == session)
                .map(|i| self.pvp_locks.swap_remove(i).2);
            let prey = self
                .prey
                .iter()
                .position(|p| p.activity_matches(actor, session))
                .map(|i| self.prey.swap_remove(i));
            let stamina = self
                .hunting_stamina
                .iter()
                .position(|p| p.activity_matches(actor, session))
                .map(|i| self.hunting_stamina.swap_remove(i));
            self.item_uses
                .extract_activity(actor, session, &mut item_uses);
            held.push(HeldPlayerState {
                actor,
                session,
                spell,
                death,
                charm,
                pvp,
                prey,
                stamina,
                item_uses,
            });
        }
        Ok(PreparedActivitySpellStates { players: held })
    }

    pub(in crate::gameplay_transport) fn abort_activity_states(
        &mut self,
        held: &mut PreparedActivitySpellStates,
    ) {
        for mut player in held.players.drain(..) {
            self.actors
                .push((player.actor, player.session, player.spell));
            if let Some(death) = player.death {
                self.deaths.push((player.actor, player.session, death));
            }
            if let Some(charm) = player.charm {
                self.charm_states
                    .push((player.actor, player.session, charm));
            }
            if let Some(pvp) = player.pvp {
                self.pvp_locks.push((player.actor, player.session, pvp));
            }
            if let Some(prey) = player.prey {
                self.prey.push(prey);
            }
            if let Some(stamina) = player.stamina {
                self.hunting_stamina.push(stamina);
            }
            self.item_uses.restore_activity(&mut player.item_uses);
        }
    }

    pub(in crate::gameplay_transport) fn commit_activity_states(
        &mut self,
        held: &mut PreparedActivitySpellStates,
        moved: &[ActivityTransferredActor],
    ) -> Result<(), ()> {
        if held.players.len() != moved.len()
            || held.players.iter().any(|p| {
                moved
                    .iter()
                    .filter(|m| m.source_actor == p.actor && m.source_session == p.session)
                    .count()
                    != 1
            })
            || moved.iter().any(|m| {
                self.actors
                    .iter()
                    .any(|(a, s, _)| *a == m.destination_actor || *s == m.destination_session)
            })
        {
            return Err(());
        }
        // All checks precede the first move. The destination reservations already
        // fixed each exact actor/session, and prepare reserved these Vec capacities.
        for mut player in held.players.drain(..) {
            let Some(next) = moved
                .iter()
                .find(|m| m.source_actor == player.actor && m.source_session == player.session)
            else {
                unreachable!("complete activity state mapping preflight");
            };
            self.actors.push((
                next.destination_actor,
                next.destination_session,
                player.spell,
            ));
            if let Some(death) = player.death {
                self.deaths
                    .push((next.destination_actor, next.destination_session, death));
            }
            if let Some(charm) = player.charm {
                self.charm_states
                    .push((next.destination_actor, next.destination_session, charm));
            }
            if let Some(pvp) = player.pvp {
                self.pvp_locks
                    .push((next.destination_actor, next.destination_session, pvp));
            }
            if let Some(mut prey) = player.prey {
                prey.rebind_activity(next.destination_actor, next.destination_session);
                self.prey.push(prey);
            }
            if let Some(mut stamina) = player.stamina {
                stamina.rebind_activity(next.destination_actor, next.destination_session);
                self.hunting_stamina.push(stamina);
            }
            self.item_uses.commit_activity(
                &mut player.item_uses,
                next.destination_actor,
                next.destination_session,
            );
        }
        self.owner_wake.notify_waiters();
        Ok(())
    }
}
