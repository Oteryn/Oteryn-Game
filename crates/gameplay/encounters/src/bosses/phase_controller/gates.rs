//! Vulnerability projection on the existing carrier encounter damage hook.
use oteryn_gameplay_entities::carrier::encounter_access::{
    EncounterHealProjection, EncounterHooks,
};
use oteryn_gameplay_entities::carrier::{ChannelActorCarrier, NamespaceContinuityGuard};
use oteryn_gameplay_entities::*;

const MAX_PROTECTED_ACTORS: usize = 1024;
const MAX_LINKED_GROUPS: usize = 64;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct DamageGates {
    actors: Vec<(ExactActorRef, u8)>,
    linked: Vec<([ExactActorRef; 4], &'static str)>,
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

pub(super) fn unprotect(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
) -> Result<(), CarrierError> {
    runtime.owner_fence()?;
    runtime.with_encounter_hooks::<DamageGates, _, CarrierError>(|_, gates| {
        gates.actors.retain(|(saved, _)| *saved != actor);
        Ok(())
    })
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

pub(super) fn link_health(
    runtime: &mut ChannelRuntimeV1,
    actors: [ExactActorRef; 4],
    key: &'static str,
) -> Result<(), CarrierError> {
    runtime.owner_fence()?;
    for (i, &actor) in actors.iter().enumerate() {
        if !runtime.matches_live_creature_identity(actor, key.as_bytes())
            || actors[..i].contains(&actor)
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
    }
    runtime.with_encounter_hooks::<DamageGates, _, CarrierError>(|runtime, gates| {
        gates.linked.retain(|(group, _)| {
            group
                .iter()
                .any(|&actor| runtime.contains_live_creature(actor))
        });
        if gates.linked.len() >= MAX_LINKED_GROUPS
            || gates
                .linked
                .iter()
                .any(|(group, _)| group.iter().any(|actor| actors.contains(actor)))
        {
            return Err(CarrierError::PlanConflict);
        }
        gates
            .linked
            .try_reserve(1)
            .map_err(|_| CarrierError::AllocationFailed)?;
        gates.linked.push((actors, key));
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
            || self.linked.iter().any(|(group, _)| group.contains(&actor))
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
        if let Some((group, key)) = self
            .linked
            .iter()
            .find(|(group, _)| group.contains(&ExactActorRef(actor)))
        {
            return Some((|| {
                if command.target != key.as_bytes() {
                    return Err(CarrierError::CreatureTargetMismatch);
                }
                let index = group
                    .iter()
                    .position(|&member| member == ExactActorRef(actor))
                    .ok_or(CarrierError::CreatureTargetMismatch)?;
                let results = carrier.commit_encounter_damage_batch(
                    continuity,
                    *group,
                    [key.as_bytes(); 4],
                    command,
                    attacker,
                    fail,
                    max_sub_ordinal,
                    now_ms,
                    |_, _, results| {
                        if results.iter().any(|result| result != &results[0]) {
                            return Err(CarrierError::PlanConflict);
                        }
                        Ok(())
                    },
                )?;
                Ok(results[index])
            })());
        }
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
        runtime: &ChannelRuntimeV1,
        requests: &[(ExactActorRef, String, u64, u64)],
    ) -> Result<EncounterHealProjection, CarrierError> {
        if requests.len() > 64 {
            return Err(CarrierError::AllocationFailed);
        }
        let mut expanded = Vec::new();
        let mut map = Vec::with_capacity(requests.len());
        for (i, (actor, key, maximum, draw)) in requests.iter().enumerate() {
            if *maximum == 0
                || *maximum > i64::MAX as u64
                || *draw > i64::MAX as u64
                || requests[..i].iter().any(|other| other.0 == *actor)
            {
                return Err(CarrierError::PlanConflict);
            }
            if let Some((group, registered)) =
                self.linked.iter().find(|(group, _)| group.contains(actor))
            {
                if key.as_str() != *registered
                    || group.iter().any(|&member| {
                        !runtime.matches_live_creature_identity(member, key.as_bytes())
                    })
                {
                    return Err(CarrierError::CreatureTargetMismatch);
                }
                if !expanded
                    .iter()
                    .any(|(saved, _, _, _)| group.contains(saved))
                {
                    expanded.extend(
                        group
                            .iter()
                            .map(|&member| (member, key.clone(), *maximum, *draw)),
                    );
                }
            } else {
                expanded.push((*actor, key.clone(), *maximum, *draw));
            }
            if expanded.len() > 64 {
                return Err(CarrierError::AllocationFailed);
            }
            map.push(
                expanded
                    .iter()
                    .position(|(saved, _, _, _)| saved == actor)
                    .ok_or(CarrierError::PlanConflict)?,
            );
        }
        Ok((expanded, map))
    }
}
