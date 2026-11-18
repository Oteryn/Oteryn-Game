//! Vulnerability projection on the existing carrier encounter damage hook.
use oteryn_gameplay_entities::carrier::encounter_access::{
    EncounterHealProjection, EncounterHooks,
};
use oteryn_gameplay_entities::carrier::{ChannelActorCarrier, NamespaceContinuityGuard};
use oteryn_gameplay_entities::*;

const MAX_PROTECTED_ACTORS: usize = 1024;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct DamageGates {
    actors: Vec<(ExactActorRef, u8)>,
}

/// A source stop form cannot be killed while the shared phase owner is preparing
/// its replacement. Recycled slots are different actors and inherit no protection.
pub fn protect(
    runtime: &mut ChannelRuntimeV1,
    actors: &[ExactActorRef],
) -> Result<(), CarrierError> {
    set_damage_divisor(runtime, actors, 0)
}

pub(super) fn halve(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
) -> Result<(), CarrierError> {
    set_damage_divisor(runtime, &[actor], 2)
}

fn set_damage_divisor(
    runtime: &mut ChannelRuntimeV1,
    actors: &[ExactActorRef],
    divisor: u8,
) -> Result<(), CarrierError> {
    runtime.owner_fence()?;
    if actors.len() > MAX_PROTECTED_ACTORS
        || actors
            .iter()
            .any(|actor| !runtime.contains_live_creature(*actor))
    {
        return Err(CarrierError::CreatureTargetMismatch);
    }
    runtime.with_encounter_hooks::<DamageGates, _, CarrierError>(|runtime, gates| {
        gates
            .actors
            .retain(|(actor, _)| runtime.contains_live_creature(*actor));
        if gates.actors.len().saturating_add(actors.len()) > MAX_PROTECTED_ACTORS {
            return Err(CarrierError::CapacityExceeded);
        }
        gates
            .actors
            .try_reserve(actors.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        for actor in actors {
            if let Some((_, previous)) = gates.actors.iter_mut().find(|(saved, _)| saved == actor) {
                *previous = divisor;
            } else {
                gates.actors.push((*actor, divisor));
            }
        }
        Ok(())
    })
}

impl EncounterHooks for DamageGates {
    fn clone_hooks(&self) -> Box<dyn EncounterHooks> {
        Box::new(self.clone())
    }
    fn eq_hooks(&self, other: &dyn EncounterHooks) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn contains(&self, actor: ExactActorRef) -> bool {
        self.actors.iter().any(|(saved, _)| *saved == actor)
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "existing native carrier damage hook ABI"
    )]
    fn damage(
        &self,
        carrier: &mut ChannelActorCarrier,
        continuity: &NamespaceContinuityGuard,
        actor: ActorRef,
        mut command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail: bool,
        _: Option<&ChannelContentPin>,
        max_sub_ordinal: Option<u16>,
        now_ms: Option<u64>,
    ) -> Option<Result<OwnerDamageResult, CarrierError>> {
        let (_, divisor) = self
            .actors
            .iter()
            .find(|(saved, _)| *saved == ExactActorRef(actor))?;
        if *divisor == 0 {
            return Some(Err(CarrierError::PlanConflict));
        }
        command.damage /= i64::from(*divisor);
        Some(carrier.commit_encounter_single_damage(
            continuity,
            actor,
            command,
            attacker,
            fail,
            max_sub_ordinal,
            now_ms,
        ))
    }
    fn projected_death(&mut self, _: ExactActorRef, _: CreatureDeathOccurrenceKey) {}
    fn expand_heals(
        &self,
        _: &ChannelRuntimeV1,
        requests: &[(ExactActorRef, String, u64, u64)],
    ) -> Result<EncounterHealProjection, CarrierError> {
        if requests.len() > 64 {
            return Err(CarrierError::AllocationFailed);
        }
        if requests
            .iter()
            .enumerate()
            .any(|(i, (actor, _, maximum, draw))| {
                *maximum == 0
                    || *maximum > i64::MAX as u64
                    || *draw > i64::MAX as u64
                    || requests[..i].iter().any(|other| other.0 == *actor)
            })
        {
            return Err(CarrierError::PlanConflict);
        }
        Ok((requests.to_vec(), (0..requests.len()).collect()))
    }
}
