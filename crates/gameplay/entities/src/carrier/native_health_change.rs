//! Source HEALTHCHANGE observations. These values grant no HP or cast authority.
//! Canary47df creaturescripts_healthchange_forgotten.lua:5-22 scans the literal
//! same-floor +/-7 rectangle and doubles only signed primary. Game.cpp:8364-8376
//! and8464-8476 run that callback once before Wheel, clearing origin on reentry.
use super::*;
use oteryn_gameplay_content::content::native_monster_callbacks::{
    NativeHealthEventKind, QualifiedNativeHealthEvent,
};

/// Closed results of the two literal HealthForgotten name lookups. `Other`
/// means an actual owner read proved neither name; an unavailable read is None.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeHealthGuardName {
    ShadowTentacle,
    PossessedTree,
    Other,
}
/// Read-only DATA supplied by the current all-role name owner. Implementations
/// must prove the same current actor identity/session/lease covered by their
/// completed name read, then bind that name DATA to the actual current pose.
/// Known owner movement does not edit an older observation; it issues a fresh one.
pub trait NativeHealthNameRead {
    fn observed(
        &self,
        actor: ExactActorRef,
        at: MovementPositionSnapshot,
    ) -> Option<NativeHealthGuardName>;
}
impl<F> NativeHealthNameRead for F
where
    F: Fn(ExactActorRef, MovementPositionSnapshot) -> Option<NativeHealthGuardName>,
{
    fn observed(
        &self,
        actor: ExactActorRef,
        at: MovementPositionSnapshot,
    ) -> Option<NativeHealthGuardName> {
        self(actor, at)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeHealthChangeObservation {
    target: ExactActorRef,
    source: QualifiedNativeHealthEvent,
    pins: [[u8; 32]; 4],
    activation: u64,
    cohort: Vec<(
        ExactActorRef,
        MovementPositionSnapshot,
        Option<NativeHealthGuardName>,
    )>,
    external_names: Vec<(
        ExactActorRef,
        MovementPositionSnapshot,
        NativeHealthGuardName,
    )>,
    doubles: bool,
}
impl NativeHealthChangeObservation {
    pub fn target(&self) -> ExactActorRef {
        self.target
    }
    /// The caller applies this only to the original primary component before
    /// Wheel modifiers. Secondary components and ORIGIN_NONE bypass it.
    pub fn primary(&self, value: i64) -> Result<i64, CarrierError> {
        let value = if self.doubles {
            value.checked_mul(2).ok_or(CarrierError::DamageOverflow)?
        } else {
            value
        };
        i32::try_from(value).map_err(|_| CarrierError::DamageOverflow)?;
        Ok(value)
    }
}
fn guard_name(name: &str) -> NativeHealthGuardName {
    if name.eq_ignore_ascii_case("shadow tentacle") {
        NativeHealthGuardName::ShadowTentacle
    } else if name.eq_ignore_ascii_case("possessed tree") {
        NativeHealthGuardName::PossessedTree
    } else {
        NativeHealthGuardName::Other
    }
}
impl ChannelRuntimeV1 {
    /// Current attached source event and complete physical spectator observations.
    /// A Player/NPC outside the literal same-floor +/-7 rectangle needs no name.
    /// Unknown names inside it refuse rather than masquerading as no guard.
    pub fn observe_native_health_change(
        &self,
        target: ExactActorRef,
        names: &impl NativeHealthNameRead,
    ) -> Result<Option<NativeHealthChangeObservation>, CarrierError> {
        self.owner_fence()?;
        self.carrier.validate_current_continuity(&self.continuity)?;
        let index = self.carrier.validate_ref(&self.continuity, target.0)?;
        match &self.carrier.slots[index] {
            Slot::Occupied {
                committed: true, ..
            } => return Ok(None),
            Slot::CreatureOccupied {
                companion: None,
                target_identity,
                ..
            } => {
                let key = std::str::from_utf8(target_identity.as_ref())
                    .map_err(|_| CarrierError::InvalidCreatureTarget)?;
                if oteryn_gameplay_content::content::native_monster_callbacks::requires_native_health_event(key) {
                    return Err(CarrierError::InvalidCreatureTarget);
                }
                return Ok(None);
            }
            Slot::CreatureOccupied { .. } => {}
            _ => return Err(CarrierError::NotCreature),
        }
        let Some(source) = self.native_health_event(target)? else {
            return Ok(None);
        };
        if source.kind() != NativeHealthEventKind::HealthForgotten {
            return Ok(None);
        }
        let snapshot = self.companion_snapshot(target)?;
        let key = snapshot.state.policy.definition_key.as_str();
        let required = match key {
            "oteryn:creature.lady_tenebris" => NativeHealthGuardName::ShadowTentacle,
            "oteryn:creature.mounted_thorn_knight"
            | "oteryn:creature.the_enraged_thorn_knight"
            | "oteryn:creature.the_shielded_thorn_knight" => NativeHealthGuardName::PossessedTree,
            _ => return Err(CarrierError::InvalidCreatureTarget),
        };
        let origin = self.read_actor_position(target)?.position();
        let mut cohort = Vec::new();
        let mut external_names = Vec::new();
        cohort
            .try_reserve(self.carrier.occupied.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        external_names
            .try_reserve(self.carrier.occupied.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        let mut guarded = false;
        for &slot in &self.carrier.occupied {
            let index =
                usize::try_from(slot).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
            let (generation, native_name, external) = match &self.carrier.slots[index] {
                Slot::CreatureOccupied {
                    generation,
                    position: Some(_),
                    companion,
                    spell_combat,
                    ..
                } if !spell_combat.source_death_completed => (
                    *generation,
                    companion
                        .as_ref()
                        .map(|state| state.policy.display_name.as_str()),
                    false,
                ),
                Slot::Occupied {
                    generation,
                    committed: true,
                    position: Some(_),
                    ..
                }
                | Slot::NpcOccupied { generation, .. } => (*generation, None, true),
                _ => continue,
            };
            let actor = ExactActorRef(ActorRef::from_scoped_owner_parts(
                self.binding.runtime_scope,
                self.binding.scope_generation,
                ActorLocalId(
                    u32::try_from(index + 1)
                        .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                ),
                ActorLocalGeneration(generation),
            ));
            let at = self.read_movement_actor_position(actor)?;
            let position = at.position();
            let inside = position.floor == origin.floor
                && i64::from(position.x).abs_diff(i64::from(origin.x)) <= 7
                && i64::from(position.y).abs_diff(i64::from(origin.y)) <= 7;
            let named = match native_name {
                Some(name) => Some(guard_name(name)),
                None if external && inside => {
                    let value = names
                        .observed(actor, at)
                        .ok_or(CarrierError::InvalidCreatureTarget)?;
                    external_names.push((actor, at, value));
                    Some(value)
                }
                None if inside => return Err(CarrierError::InvalidCreatureTarget),
                None => None,
            };
            guarded |= inside && named == Some(required);
            cohort.push((actor, at, named));
        }
        cohort.sort_by_key(|(actor, _, _)| actor.actor_local_id());
        external_names.sort_by_key(|(actor, _, _)| actor.actor_local_id());
        let pin = self.content_pin();
        Ok(Some(NativeHealthChangeObservation {
            target,
            source: source.clone(),
            activation: pin.activation_sequence(),
            pins: [
                pin.server_artifact_digest(),
                pin.client_artifact_digest(),
                pin.frame_binding_digest(),
                self.map_revision_digest(),
            ],
            cohort,
            external_names,
            doubles: !guarded,
        }))
    }
    /// Reprove physical cohort, registration, type/pose and current source pins.
    /// The name read remains the immutable DATA of the same owner preparation.
    pub fn validate_native_health_change_observation(
        &self,
        observation: &NativeHealthChangeObservation,
    ) -> Result<(), CarrierError> {
        let names = |actor, at| {
            observation
                .external_names
                .iter()
                .find(|(held, position, _)| *held == actor && *position == at)
                .map(|(_, _, name)| *name)
        };
        if self
            .observe_native_health_change(observation.target, &names)?
            .as_ref()
            != Some(observation)
        {
            return Err(CarrierError::PlanConflict);
        }
        Ok(())
    }
}
