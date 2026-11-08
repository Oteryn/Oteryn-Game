//! Bounded phase dependencies for the existing boss encounter mechanic.
//!
//! Source adapters retain admission, map mutation and durable settlement authority. This
//! owner accepts only exact current native actor deaths, never creature-name kill counts.
use oteryn_gameplay_entities::carrier::Slot;
use oteryn_gameplay_entities::{CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId};
use oteryn_gameplay_foundation::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};

pub mod cults;
pub mod gates;
#[cfg(test)]
mod tests;

pub const MAX_ROLES: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Role {
    pub creature: &'static str,
    /// All these roles must have an encounter-owned native death before this role is active.
    pub after: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Plan {
    pub roles: &'static [Role],
    pub terminal: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Progress {
    plan: Plan,
    dead: u64,
}
impl Progress {
    pub fn new(plan: Plan) -> Result<Self, CarrierError> {
        if plan.roles.is_empty() || plan.roles.len() > MAX_ROLES || plan.terminal == 0 {
            return Err(CarrierError::PlanConflict);
        }
        let mask = u64::MAX >> (MAX_ROLES - plan.roles.len());
        if plan.terminal & !mask != 0 {
            return Err(CarrierError::PlanConflict);
        }
        // Dependency order rules out cycles, unknown nodes and self dependencies.
        for (i, role) in plan.roles.iter().enumerate() {
            let predecessors = if i == 0 {
                0
            } else {
                u64::MAX >> (MAX_ROLES - i)
            };
            if role.creature.is_empty() || role.after & !predecessors != 0 {
                return Err(CarrierError::PlanConflict);
            }
        }
        Ok(Self { plan, dead: 0 })
    }
    pub fn ready(&self, role: usize) -> bool {
        self.plan.roles.get(role).is_some_and(|node| {
            self.dead & (1_u64 << role) == 0 && self.dead & node.after == node.after
        })
    }
    pub fn record_death(&mut self, role: usize) -> Result<bool, CarrierError> {
        if role >= self.plan.roles.len() {
            return Err(CarrierError::PlanConflict);
        }
        let bit = 1_u64 << role;
        if self.dead & bit != 0 {
            return Ok(false);
        }
        if !self.ready(role) {
            return Err(CarrierError::PlanConflict);
        }
        self.dead |= bit;
        Ok(true)
    }
    pub fn dead(&self) -> u64 {
        self.dead
    }
    pub fn completed(&self) -> bool {
        self.dead & self.plan.terminal == self.plan.terminal
    }
}

/// One admitted encounter, tied to the existing runtime scope, content activation and
/// exact player sessions. It grants no new activity, loot or quest-write authority.
#[derive(Debug)]
pub struct NativePhaseOwner {
    progress: Progress,
    actors: Vec<Option<ExactActorRef>>,
    participants: Vec<(ExactActorRef, GameSessionId)>,
    scope: (RuntimeScopeRefV1, ScopeOwnershipGeneration),
    activation: u64,
    pins: [[u8; 32]; 4],
}
impl NativePhaseOwner {
    pub fn bind(
        runtime: &ChannelRuntimeV1,
        plan: Plan,
        participants: &[(ExactActorRef, GameSessionId)],
    ) -> Result<Self, CarrierError> {
        runtime.owner_fence()?;
        if participants.is_empty() || participants.len() > 16 {
            return Err(CarrierError::PlanConflict);
        }
        for (i, (actor, session)) in participants.iter().enumerate() {
            runtime.player_control_facts(*actor, *session)?;
            if participants[..i]
                .iter()
                .any(|(a, s)| a == actor || s == session)
            {
                return Err(CarrierError::PlanConflict);
            }
        }
        let pin = runtime.content_pin();
        Ok(Self {
            progress: Progress::new(plan)?,
            actors: vec![None; plan.roles.len()],
            participants: participants.to_vec(),
            scope: runtime.encounter_owner_scope(),
            activation: pin.activation_sequence(),
            pins: [
                pin.server_artifact_digest(),
                pin.client_artifact_digest(),
                pin.frame_binding_digest(),
                pin.map_revision_digest(),
            ],
        })
    }
    fn current(&self, runtime: &ChannelRuntimeV1) -> Result<(), CarrierError> {
        runtime.owner_fence()?;
        let pin = runtime.content_pin();
        if self.scope != runtime.encounter_owner_scope()
            || self.activation != pin.activation_sequence()
            || self.pins
                != [
                    pin.server_artifact_digest(),
                    pin.client_artifact_digest(),
                    pin.frame_binding_digest(),
                    pin.map_revision_digest(),
                ]
        {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        Ok(())
    }
    /// Called by the existing source spawn/admission owner, after it commits a ready role.
    pub fn attach(
        &mut self,
        runtime: &ChannelRuntimeV1,
        role: usize,
        actor: ExactActorRef,
    ) -> Result<(), CarrierError> {
        self.current(runtime)?;
        let node = self
            .progress
            .plan
            .roles
            .get(role)
            .ok_or(CarrierError::PlanConflict)?;
        if !self.progress.ready(role)
            || self.actors[role].is_some()
            || self.actors.contains(&Some(actor))
            || !runtime.matches_live_creature_identity(actor, node.creature.as_bytes())
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        self.actors[role] = Some(actor);
        Ok(())
    }
    /// A matching name or zero HP is insufficient: the native death owner must already
    /// have published the sealed corpse for this exact actor generation.
    pub fn projected_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Result<bool, CarrierError> {
        self.current(runtime)?;
        let role = self
            .actors
            .iter()
            .position(|a| *a == Some(actor))
            .ok_or(CarrierError::CreatureTargetMismatch)?;
        let index = runtime.encounter_actor_index(actor)?;
        match runtime.encounter_slot(index) {
            Slot::CreatureOccupied {
                generation,
                target_identity,
                ..
            } if *generation == actor.0.actor_local_generation.0
                && target_identity.as_ref()
                    == self.progress.plan.roles[role].creature.as_bytes() => {}
            _ => return Err(CarrierError::StaleActorGeneration),
        }
        runtime.borrow_combat_death().projected_death(actor)?;
        self.progress.record_death(role)
    }
    pub fn pending(&self) -> impl Iterator<Item = (usize, Role)> + '_ {
        self.progress
            .plan
            .roles
            .iter()
            .copied()
            .enumerate()
            .filter(|(i, _)| self.actors[*i].is_none() && self.progress.ready(*i))
    }
    /// The settlement adapter reuses the existing boss/quest receipt owner. Until the
    /// final native death there is no participant completion set to persist.
    pub fn victors(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<&[(ExactActorRef, GameSessionId)]>, CarrierError> {
        self.current(runtime)?;
        Ok(self
            .progress
            .completed()
            .then_some(self.participants.as_slice()))
    }
}

impl NativePhaseOwner {
    /// Source phase hooks commit removals and replacement actors under one existing
    /// carrier borrow. A refused map/policy/admission leaves both carrier and phase unchanged.
    pub fn spawn_role(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        role: usize,
        position: oteryn_gameplay_entities::MovementLocalPosition,
        remove: Option<ExactActorRef>,
        decoration: Option<(&str, oteryn_gameplay_entities::MovementLocalPosition)>,
        map_admits: impl Fn(&ChannelRuntimeV1, oteryn_gameplay_entities::MovementLocalPosition) -> bool,
    ) -> Result<ExactActorRef, CarrierError> {
        self.current(runtime)?;
        let node = self
            .progress
            .plan
            .roles
            .get(role)
            .ok_or(CarrierError::PlanConflict)?;
        if !self.progress.ready(role) || self.actors[role].is_some() {
            return Err(CarrierError::PlanConflict);
        }
        if !map_admits(runtime, position)
            || decoration.is_some_and(|(_, at)| !map_admits(runtime, at))
        {
            return Err(CarrierError::PlanConflict);
        }
        let policy = runtime.companion_policy(node.creature)?;
        let mut spawns = vec![(node.creature, policy.maximum_health, position)];
        if let Some((key, at)) = decoration {
            let policy = runtime.companion_policy(key)?;
            spawns.push((key, policy.maximum_health, at));
        }
        let before = runtime.encounter_parts().0.clone();
        let result = (|| {
            if let Some(actor) = remove {
                runtime.remove_encounter_actor(actor)?;
            }
            let actors = runtime.commit_quest_creature_spawn(&spawns)?;
            let actor = actors[0];
            self.attach(runtime, role, actor)?;
            Ok(actor)
        })();
        if result.is_err() {
            *runtime.encounter_parts_mut().0 = before;
        }
        result
    }
}
