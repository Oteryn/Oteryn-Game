//! D1213: one read of the existing actor/session combat owner, on its semantic axis.
//! This snapshot is data for the serialized caller, not a mutation or admission permit.

use super::{ComposedFreshAdmission, actor_spell::ChannelSpellStates, attack::ChannelAttackStates};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_simulation_determinism::SemanticTimeMicros;

/// Exhaustive on purpose: a future runtime PvP binding must update every consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PvpLockSource {
    NotWiredInRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CombatLockFact {
    actor: ExactActorRef,
    session: GameSessionId,
    sampled_at: SemanticTimeMicros,
    pub(crate) pve_in_fight_until: Option<SemanticTimeMicros>,
    pub(crate) pvp: PvpLockSource,
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
            pvp: PvpLockSource::NotWiredInRuntime,
        }
    }

    pub(crate) fn is_for(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        self.actor == actor && self.session == session
    }

    pub(crate) fn sampled_at(&self) -> SemanticTimeMicros {
        self.sampled_at
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Sample after both owning guards are acquired; never retain this snapshot across a wait.
    #[allow(
        dead_code,
        reason = "D1213 caller hunk awaits the exact shared-root allocation"
    )]
    pub(in crate::gameplay_transport) async fn combat_lock_fact(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> CombatLockFact {
        let states = self.spell_states.lock().await;
        let attack = self.attack.lock().await;
        CombatLockFact::read(&states, &attack, actor, session, self.owner_now())
    }
}
