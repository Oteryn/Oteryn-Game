//! Crystal 201189f8 arbaziloth_fight.lua:298-373. The encounter owns the
//! exact reinforcement actors and one delayed refill at a time.
mod pulse;

use super::{CarrierError, ChannelRuntimeV1, Encounter, ExactActorRef, MovementLocalPosition};
use oteryn_gameplay_quests::quest::world_callbacks::EncounterReinforcements;

#[derive(Clone, Debug)]
pub(super) struct Roster {
    rule: EncounterReinforcements,
    slots: Vec<Slot>,
    check_at: u64,
    refill: Option<(u64, Vec<usize>)>,
    pulse: Option<pulse::Pending>,
}

#[derive(Clone, Debug)]
struct Slot {
    position: MovementLocalPosition,
    actor: ExactActorRef,
}

impl Roster {
    pub(super) fn bind(
        runtime: &ChannelRuntimeV1,
        room: &Encounter,
        actors: &[ExactActorRef],
        rule: &EncounterReinforcements,
        now: u64,
    ) -> Result<Self, CarrierError> {
        let mut slots = Vec::new();
        for spawn in room
            .monsters
            .iter()
            .filter(|spawn| spawn.creature == rule.creature)
        {
            let position = MovementLocalPosition {
                x: i32::from(spawn.position.x),
                y: i32::from(spawn.position.y),
                floor: -i16::from(spawn.position.floor),
            };
            let actor = actors
                .iter()
                .copied()
                .find(|actor| {
                    runtime.matches_live_creature_identity(*actor, rule.creature.as_bytes())
                        && runtime
                            .read_actor_position(*actor)
                            .is_ok_and(|snapshot| snapshot.position() == position)
                })
                .ok_or(CarrierError::CreatureTargetMismatch)?;
            slots.push(Slot { position, actor });
        }
        if slots.is_empty() {
            return Err(CarrierError::PlanConflict);
        }
        Ok(Self {
            rule: rule.clone(),
            slots,
            check_at: now.saturating_add(u64::from(rule.check_interval_ms)),
            refill: None,
            pulse: None,
        })
    }

    pub(super) fn living(&self, runtime: &ChannelRuntimeV1) -> Vec<ExactActorRef> {
        self.slots
            .iter()
            .map(|slot| slot.actor)
            .filter(|actor| runtime.contains_live_creature(*actor))
            .collect()
    }

    pub(super) fn pulse_pending(&self) -> bool {
        self.pulse.is_some()
    }

    pub(super) fn tick_pulse(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut crate::gameplay_transport::actor_spell::ChannelSpellStates,
        map: &crate::map::boot::BundleMap,
        boss: ExactActorRef,
        now: u64,
    ) {
        if self.pulse.is_none()
            && let Some(rule) = &self.rule.low_health_pulse
            && let Some(demon) = self.slots.iter().map(|slot| slot.actor).find(|actor| {
                runtime
                    .live_creature_health(*actor)
                    .is_some_and(|health| health < u64::from(rule.below_health))
            })
        {
            self.pulse = pulse::Pending::begin(runtime, boss, demon, rule, map);
        }
        if let Some(pulse) = &mut self.pulse
            && pulse.finish(runtime, states, now)
        {
            self.pulse = None;
        }
    }

    pub(super) fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        terminal: ExactActorRef,
        now: u64,
        placeable: impl Fn(MovementLocalPosition, &ChannelRuntimeV1) -> bool,
    ) {
        if !runtime.contains_live_creature(terminal) {
            self.refill = None;
            return;
        }
        if let Some((due, slots)) = &self.refill {
            if now < *due {
                return;
            }
            let Ok(health) = runtime
                .companion_policy(&self.rule.creature)
                .map(|policy| policy.maximum_health)
            else {
                return;
            };
            let Ok(census) = runtime.positioned_melee_census() else {
                return;
            };
            let mut spawns = Vec::with_capacity(slots.len());
            for &index in slots {
                let position = self.slots[index].position;
                if !placeable(position, runtime)
                    || census
                        .iter()
                        .any(|(_, current, _)| current.position() == position)
                {
                    return;
                }
                spawns.push((self.rule.creature.as_str(), health, position));
            }
            let Ok(replacements) = runtime.commit_quest_creature_spawn(&spawns) else {
                return;
            };
            for (&index, actor) in slots.iter().zip(replacements) {
                self.slots[index].actor = actor;
            }
            self.refill = None;
            self.check_at = now.saturating_add(u64::from(self.rule.check_interval_ms));
        } else if now >= self.check_at {
            self.check_at = now.saturating_add(u64::from(self.rule.check_interval_ms));
            let missing: Vec<_> = self
                .slots
                .iter()
                .enumerate()
                .filter_map(|(index, slot)| {
                    (!runtime.contains_live_creature(slot.actor)).then_some(index)
                })
                .collect();
            if !missing.is_empty() {
                self.refill = Some((
                    now.saturating_add(u64::from(self.rule.respawn_delay_ms)),
                    missing,
                ));
            }
        }
    }
}
