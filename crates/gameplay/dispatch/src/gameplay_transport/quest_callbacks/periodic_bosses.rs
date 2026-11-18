//! The node's Channel source cycle owns the interval and exact live actor.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::periodic_bosses::PeriodicBoss;
use std::time::Duration;
use tokio::time::Instant;

#[derive(Default)]
pub(super) struct PeriodicBosses {
    scope: Option<(RuntimeScopeRefV1, ScopeOwnershipGeneration)>,
    entries: Vec<Occurrence>,
}

struct Occurrence {
    next: Instant,
    cooldown_until: Instant,
    live: Option<ExactActorRef>,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn tick_periodic_bosses(&self, runtime: &mut ChannelRuntimeV1, map: &BundleMap) {
        #[cfg(test)]
        let fixture = self.states.quest_callbacks.periodic_boss_fixture.get();
        #[cfg(not(test))]
        let fixture: Option<&Vec<PeriodicBoss>> = None;
        let plans = match fixture {
            Some(plans) => plans.as_slice(),
            None => match world_callbacks::periodic_bosses::load() {
                Ok(plans) => plans,
                Err(_) => return,
            },
        };
        if plans.is_empty()
            || !matches!(
                runtime.binding().runtime_scope(),
                RuntimeScopeRefV1::Channel { .. }
            )
        {
            return;
        }
        let Ok(mut schedule) = self.states.quest_callbacks.periodic_bosses.lock() else {
            return;
        };
        let scope = runtime.encounter_owner_scope();
        let now = Instant::now();
        if schedule.scope != Some(scope) {
            schedule.scope = Some(scope);
            schedule.entries = plans
                .iter()
                .map(|plan| Occurrence {
                    next: now + Duration::from_millis(plan.interval_ms.into()),
                    cooldown_until: now,
                    live: None,
                })
                .collect();
            return;
        }
        for (plan, occurrence) in plans.iter().zip(&mut schedule.entries) {
            if now < occurrence.next {
                continue;
            }
            occurrence.next = now + Duration::from_millis(plan.interval_ms.into());
            if now < occurrence.cooldown_until
                || occurrence
                    .live
                    .is_some_and(|actor| runtime.contains_live_creature(actor))
            {
                continue;
            }
            let Some(spawn) = pick(plan) else {
                continue;
            };
            if let Some(native) = self.active_generation.and_then(|a| a.native_gameplay())
                && native.install_companion_policies(runtime).is_ok()
                && let Some(actor) = spawn_one(self, runtime, map, &spawn.creature, spawn.position)
            {
                occurrence.live = Some(actor);
                occurrence.cooldown_until = now + Duration::from_millis(plan.cooldown_ms.into());
            }
        }
    }
}

fn pick(plan: &PeriodicBoss) -> Option<&world_callbacks::EncounterMonster> {
    let total: u32 = plan
        .choices
        .iter()
        .map(|choice| u32::from(choice.weight))
        .sum();
    // Multiply and reject the uneven tail, so equal weights get equal probability.
    let mut roll = random_below(total)?;
    for choice in &plan.choices {
        let weight = u32::from(choice.weight);
        if roll < weight {
            return Some(&choice.spawn);
        }
        roll -= weight;
    }
    None
}

pub(super) fn random_below(bound: u32) -> Option<u32> {
    if bound == 0 {
        return None;
    }
    let limit = u32::MAX - (u32::MAX % bound);
    loop {
        let draw = u32::from_be_bytes(super::super::SecureIdentifiers::random::<4>()?);
        if draw < limit {
            return Some(draw % bound);
        }
    }
}

pub(super) fn spawn_one(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    map: &BundleMap,
    creature: &str,
    position: Position,
) -> Option<ExactActorRef> {
    let at = local(position);
    if runtime
        .positioned_melee_census()
        .ok()?
        .iter()
        .any(|(_, pos, _)| pos.position() == at)
    {
        return None;
    }
    let health = runtime.companion_policy(creature).ok()?.maximum_health;
    if !spawn_cell(owner, map, position) {
        return None;
    }
    runtime
        .commit_quest_creature_spawn(&[(creature, health, at)])
        .ok()?
        .pop()
}

pub(super) fn spawn_cell(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    map: &BundleMap,
    position: Position,
) -> bool {
    let at = local(position);
    let cells = owner.movement_cells();
    cells.index().lookup(
        cells.scope(),
        LogicalCell {
            x: at.x,
            y: at.y,
            z: i32::from(at.floor),
        },
    ) == Ok(CollisionClass::Walkable)
        && !map
            .facts
            .quest_callback_blocked(map.overlay.base(), tile(position), false)
}
