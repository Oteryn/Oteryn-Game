//! EQMOD2: source monster hits reflect before HP and mana shield, never recursively.
use super::actor_spell::ChannelSpellStates;
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};

impl ChannelSpellStates {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn reflect_equipment(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        source: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        element: Option<&str>,
        magnitude: u32,
        stream: &[u8],
        now: SemanticTimeMicros,
    ) -> bool {
        if element.is_none() {
            return true;
        }
        let Some(state) = self.get(runtime, target, session) else {
            return false;
        };
        let offence = state.equipped_offence();
        if offence.reflect_flat == 0 && offence.reflect_percent == 0 {
            return true;
        }
        if self.is_dead(target) || state.vitals().health == 0 {
            return false;
        }
        let Ok(snapshot) = runtime.companion_snapshot(source) else {
            return false;
        };
        let Ok(from) = runtime.read_actor_position(source) else {
            return false;
        };
        let Ok(to) = runtime.read_actor_position(target) else {
            return false;
        };
        let (from, to) = (from.position(), to.position());
        let distance = from.x.abs_diff(to.x).max(from.y.abs_diff(to.y));
        let reflected = offence.reflected(
            magnitude,
            element == Some("physical"),
            distance,
            u64::try_from(snapshot.maximum_health).unwrap_or(0),
        );
        if reflected == 0 {
            return true;
        }
        let Ok(lease) = runtime.bound_attacker_lease(target, session) else {
            return false;
        };
        if self.equipment_reflection_kills.len() >= runtime.actor_slot_capacity()
            || self.equipment_reflection_kills.try_reserve(1).is_err()
        {
            return false;
        }
        let Ok(result) = runtime.commit_equipment_reflection(
            source,
            target,
            session,
            stream,
            now.get(),
            reflected,
        ) else {
            return false;
        };
        if result.applied && result.health_after == 0 {
            self.equipment_reflection_kills.push((
                source,
                crate::combat::CapturedRewardPrincipal {
                    character: lease.character_id(),
                    lease_generation: lease.generation(),
                    actor: target,
                    session,
                },
                now.get(),
            ));
        }
        true
    }
}
