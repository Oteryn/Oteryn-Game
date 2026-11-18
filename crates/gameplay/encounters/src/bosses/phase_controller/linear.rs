//! Consecutive native deaths on the shared phase owner.
use super::*;
use oteryn_gameplay_entities::MovementLocalPosition;
use oteryn_gameplay_quests::quest::world_callbacks::LinearPhases;

#[derive(Debug)]
pub struct LinearEncounter {
    phase: NativePhaseOwner,
    next_at: Option<MovementLocalPosition>,
    speeches: Vec<String>,
    deadline_us: u64,
}

pub struct Arrival<'a> {
    pub actor: ExactActorRef,
    pub position: MovementLocalPosition,
    pub speech: &'a str,
}

impl LinearEncounter {
    pub fn bind(
        runtime: &ChannelRuntimeV1,
        definition: &LinearPhases,
        players: &[(ExactActorRef, GameSessionId)],
        roster: &[ExactActorRef],
        now_us: u64,
    ) -> Result<Self, CarrierError> {
        let Some(first) = definition.forms.first() else {
            return Err(CarrierError::PlanConflict);
        };
        if !(2..=8).contains(&definition.forms.len())
            || definition.speeches.len() + 1 != definition.forms.len()
        {
            return Err(CarrierError::PlanConflict);
        }
        for key in &definition.forms {
            runtime.companion_policy(key)?;
        }
        let mut candidates = roster
            .iter()
            .filter(|actor| runtime.matches_live_creature_identity(**actor, first.as_bytes()));
        let opening = *candidates
            .next()
            .ok_or(CarrierError::CreatureTargetMismatch)?;
        if candidates.next().is_some() {
            return Err(CarrierError::PlanConflict);
        }
        let roles = definition
            .forms
            .iter()
            .enumerate()
            .map(|(i, key)| (key.clone(), if i == 0 { 0 } else { 1 << (i - 1) }))
            .collect();
        let progress = Progress::from_roles(roles, 1 << (definition.forms.len() - 1))?;
        let mut phase = NativePhaseOwner::with_progress(runtime, progress, players)?;
        phase.attach(runtime, 0, opening)?;
        let deadline_us = now_us
            .checked_add(u64::from(definition.reservation_ms) * 1000)
            .ok_or(CarrierError::PlanConflict)?;
        Ok(Self {
            phase,
            next_at: None,
            speeches: definition.speeches.clone(),
            deadline_us,
        })
    }

    pub fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.phase.current(runtime).is_ok()
    }

    pub fn contains(&self, actor: ExactActorRef) -> bool {
        self.phase.actors.contains(&Some(actor))
    }

    pub fn projected_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Result<bool, CarrierError> {
        let at = runtime.read_actor_position(actor)?.position();
        let new = self.phase.projected_death(runtime, actor)?;
        if new {
            self.next_at = Some(at);
        }
        Ok(new)
    }

    pub fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now_us: u64,
        map_admits: impl Fn(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
    ) -> Result<Option<Arrival<'_>>, CarrierError> {
        self.phase.current(runtime)?;
        if self.phase.progress.completed() || self.expired(now_us) {
            // Crystal 201189f8 actions_mission11_payback_time_lever.lua:23–32
            // releases the room before the next admission retires its old roster.
            return Ok(None);
        }
        let Some((role, _)) = self.phase.pending().next() else {
            return Ok(None);
        };
        let at = self.next_at.ok_or(CarrierError::PlanConflict)?;
        let actor = self
            .phase
            .spawn_role(runtime, role, at, None, None, map_admits)?;
        self.next_at = None;
        let speech = self
            .speeches
            .get(role - 1)
            .ok_or(CarrierError::PlanConflict)?;
        Ok(Some(Arrival {
            actor,
            position: at,
            speech,
        }))
    }

    pub fn expired(&self, now_us: u64) -> bool {
        now_us >= self.deadline_us
    }

    pub fn victors(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<&[(ExactActorRef, GameSessionId)]>, CarrierError> {
        self.phase.victors(runtime)
    }
}
