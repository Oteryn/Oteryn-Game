//! D1213: one read of the existing actor/session combat owner, on its semantic axis.
//! This snapshot is data for the serialized caller, not a mutation or admission permit.

use super::{ComposedFreshAdmission, actor_spell::ChannelSpellStates, attack::ChannelAttackStates};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_simulation_determinism::SemanticTimeMicros;

/// Exhaustive on purpose: a future runtime PvP binding must update every consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PvpLockSource {
    NotWiredInRuntime,
    LiveBlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombatLockFact {
    actor: ExactActorRef,
    session: GameSessionId,
    sampled_at: SemanticTimeMicros,
    pub pve_in_fight_until: Option<SemanticTimeMicros>,
    pub pvp: PvpLockSource,
}

impl CombatLockFact {
    /// The guards are borrowed from the current owning books. A dead actor is never held.
    pub(super) fn read(
        states: &ChannelSpellStates,
        attack: &ChannelAttackStates,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
    ) -> Self {
        Self {
            actor,
            session,
            sampled_at: now,
            pve_in_fight_until: (!states.is_dead(actor))
                .then(|| attack.in_fight_until(actor, session, now))
                .flatten(),
            pvp: if super::pvp_live::logout_blocked(states, actor, session, now) {
                PvpLockSource::LiveBlocked
            } else {
                PvpLockSource::NotWiredInRuntime
            },
        }
    }

    #[allow(
        dead_code,
        reason = "D1204 actor-bound witness consumer is owned by #2174"
    )]
    pub fn is_for(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        self.actor == actor && self.session == session
    }

    pub fn sampled_at(&self) -> SemanticTimeMicros {
        self.sampled_at
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Sample after both owning guards are acquired; never retain this snapshot across a wait.
    pub(in crate::gameplay_transport) async fn combat_lock_fact(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> CombatLockFact {
        let states = self.states.spell_states.lock().await;
        let attack = self.states.attack.lock().await;
        CombatLockFact::read(&states, &attack, actor, session, self.owner_now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use {
        crate::foundation::{ChannelId, ChannelRuntimeV1, NodeId, WorldId},
        oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
    };

    fn id(last: u8) -> [u8; 16] {
        let mut bytes = [0; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = last;
        bytes
    }

    #[test]
    fn combat_fact_reads_exact_expiry_and_logs_each_renewal_cause()
    -> Result<(), Box<dyn std::error::Error>> {
        fn error(e: impl std::fmt::Debug) -> String {
            format!("{e:?}")
        }
        let world = WorldId::decode(&id(1)).map_err(error)?;
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
            (2, 11, -7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).map_err(error)?,
            NodeId::decode(&id(3)).map_err(error)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(error)?;
        let session = GameSessionId::decode(&id(4)).map_err(error)?;
        let foreign = GameSessionId::decode(&id(5)).map_err(error)?;
        let reservation = runtime.reserve_fresh_session(session).map_err(error)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(error)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(error)?;
        let mut states = ChannelSpellStates::default();
        let mut attack = ChannelAttackStates::default();
        let now = SemanticTimeMicros::from_micros(10);
        let absent = CombatLockFact::read(&states, &attack, actor, session, now);
        assert_eq!(absent.pve_in_fight_until, None);
        use super::super::pvp_live::{CombatLockCause, record_pvp_hit};
        let earlier = SemanticTimeMicros::from_micros(5);
        let lines = oteryn_error_codes::capture(|| {
            attack.record_fight_activity(
                &runtime,
                actor,
                session,
                earlier,
                CombatLockCause::Attack,
            );
            attack.record_hit_taken(&runtime, actor, session, now);
            attack.record_hit_taken(&runtime, actor, session, now);
        });
        let actual = attack
            .in_fight_until(actor, session, now)
            .ok_or("missing actual hit")?;
        let fact = CombatLockFact::read(&states, &attack, actor, session, now);
        assert!(fact.is_for(actor, session));
        assert!(!fact.is_for(actor, foreign));
        assert_eq!(fact.sampled_at(), now);
        assert_eq!(fact.pve_in_fight_until, Some(actual));
        assert_eq!(fact.pvp, PvpLockSource::NotWiredInRuntime);
        assert_eq!(
            CombatLockFact::read(&states, &attack, actor, foreign, now).pve_in_fight_until,
            None
        );
        assert_eq!(
            CombatLockFact::read(&states, &attack, actor, session, actual).pve_in_fight_until,
            None
        );
        let pvp_lines = oteryn_error_codes::capture(|| {
            record_pvp_hit(
                &runtime,
                &mut states,
                actor,
                session,
                now,
                CombatLockCause::Attack,
                true,
            );
            record_pvp_hit(
                &runtime,
                &mut states,
                actor,
                session,
                now,
                CombatLockCause::Attack,
                true,
            );
        });
        let lock = super::super::pvp_live::lock_mut(&mut states, actor, session);
        let pz = lock.pz_until().ok_or("missing PZ expiry")?;
        let logout = lock.logout_until().ok_or("missing PvP logout expiry")?;
        let tile = runtime
            .read_actor_position(actor)
            .map_err(error)?
            .position();
        assert_eq!(lines.len(), 2);
        assert_eq!(pvp_lines.len(), 2);
        let attack_until = actual.get() - (now.get() - earlier.get());
        for (line, cause, kind, expiry) in [
            (&lines[0], "attack", "in_fight", attack_until),
            (&lines[1], "damage", "in_fight", actual.get()),
            (&pvp_lines[0], "attack", "pvp_logout", logout.get()),
            (&pvp_lines[1], "aggressor", "pz", pz.get()),
        ] {
            let parsed = oteryn_error_codes::parse_line(line).map_err(error)?;
            assert_eq!(parsed.level, oteryn_error_codes::Level::Info);
            assert_eq!(parsed.event, "transport_diagnostic");
            let expected = format!(
                "combat_lock_renewed actor={}/{} cause={cause} lock={kind} expires_at_us={expiry} tile=({},{},{})",
                actor.actor_local_id(),
                actor.actor_local_generation(),
                tile.x,
                tile.y,
                tile.floor.unsigned_abs()
            );
            assert_eq!(parsed.detail.as_deref(), Some(expected.as_str()));
        }
        Ok(())
    }
}
