//! The shared boss Instance owns the exact Astral actors and their source timer.
//! Crystal 201189f actions_the_last_lore_keeper.lua and the four Astral creature events.
use super::{ComposedFreshAdmission, kill_reward::PendingKillSettlement};
use crate::content::{
    CollisionClass, LogicalCell, bosses::activity_template::QualifiedActivityRoomTemplate,
};
use crate::durability::activity_handoff::ActivityRefusal;
use crate::foundation::{CarrierError, ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};

pub(super) const LORE_KEEPER: &str = "oteryn:creature.the_last_lore_keeper";
const SHIELDED: &str = "oteryn:creature.a_shielded_astral_glyph";
const GLYPH: &str = "oteryn:creature.an_astral_glyph";
const POWER: &str = "oteryn:creature.bound_astral_power";
const SOURCE: &str = "oteryn:creature.the_astral_source";
const DISTORTED: &str = "oteryn:creature.the_distorted_astral_source";
const POWER_CELLS: [MovementLocalPosition; 4] = [
    cell(31975, 32856, 15),
    cell(31975, 32839, 15),
    cell(31995, 32839, 15),
    cell(31995, 32856, 15),
];
const START: [(&str, MovementLocalPosition); 5] = [
    (SHIELDED, cell(31986, 32840, 14)),
    (POWER, POWER_CELLS[0]),
    (SOURCE, cell(31987, 32839, 14)),
    (DISTORTED, cell(31986, 32823, 15)),
    (GLYPH, cell(31989, 32823, 15)),
];

const fn cell(x: i32, y: i32, floor: i16) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor }
}

pub(super) struct AstralEncounter {
    shielded: ExactActorRef,
    glyph: ExactActorRef,
    source: Option<ExactActorRef>,
    distorted: ExactActorRef,
    power: Option<ExactActorRef>,
    exposed: bool,
    power_until: u64,
    next_think: u64,
    boss_spawned: bool,
}

impl AstralEncounter {
    pub(super) fn prepare(
        runtime: &mut ChannelRuntimeV1,
        template: &QualifiedActivityRoomTemplate,
    ) -> Result<Self, ActivityRefusal> {
        let movement = template.room().movement_cells();
        if START
            .iter()
            .map(|(_, at)| at)
            .chain(&POWER_CELLS)
            .any(|at| {
                movement.index().lookup(
                    movement.scope(),
                    LogicalCell {
                        x: at.x,
                        y: at.y,
                        z: i32::from(at.floor),
                    },
                ) != Ok(CollisionClass::Walkable)
            })
        {
            return Err(ActivityRefusal::ContentChanged);
        }
        let monsters = START
            .into_iter()
            .map(|(key, at)| {
                runtime
                    .companion_policy(key)
                    .map(|policy| (key, policy.maximum_health, at))
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ActivityRefusal::ContentUnavailable)?;
        let [shielded, power, source, distorted, glyph] = runtime
            .commit_quest_creature_spawn(&monsters)
            .map_err(|_| ActivityRefusal::ContentUnavailable)?
            .try_into()
            .map_err(|_| ActivityRefusal::ContentUnavailable)?;
        Ok(Self {
            shielded,
            glyph,
            source: Some(source),
            distorted,
            power: Some(power),
            exposed: false,
            power_until: 0,
            next_think: 0,
            boss_spawned: false,
        })
    }

    pub(super) fn native_ready(runtime: &ChannelRuntimeV1) -> bool {
        let Ok(census) = runtime.current_live_creature_census() else {
            return false;
        };
        census.len() == START.len()
            && START.into_iter().all(|(key, at)| {
                census.iter().any(|&actor| {
                    runtime
                        .read_actor_position(actor)
                        .is_ok_and(|position| position.position() == at)
                        && runtime
                            .creature_spell_target_atom(actor)
                            .is_ok_and(|identity| identity == key)
                })
            })
    }

    fn think(&mut self, runtime: &mut ChannelRuntimeV1, now: u64) -> Result<(), CarrierError> {
        if now < self.next_think || self.boss_spawned {
            return Ok(());
        }
        let powered = now < self.power_until;
        if let Some(source) = self.source {
            if powered {
                let health = runtime
                    .live_creature_health(source)
                    .ok_or(CarrierError::PlanConflict)?;
                let maximum = u64::try_from(runtime.companion_policy(SOURCE)?.maximum_health)
                    .map_err(|_| CarrierError::InvalidCreatureHealth)?;
                if u128::from(health) * 1000 < u128::from(maximum) * 55 {
                    let at = runtime.read_actor_position(source)?.position();
                    self.swap_glyphs(runtime, true, Some((source, at)))?;
                    self.source = None;
                }
            } else {
                Self::heal_source(runtime, source, SOURCE)?;
            }
        } else {
            let health = runtime
                .live_creature_health(self.distorted)
                .ok_or(CarrierError::PlanConflict)?;
            if health <= 40_000 {
                Self::heal_source(runtime, self.distorted, DISTORTED)?;
            } else if (health >= 55_000 || !powered) && self.exposed {
                self.swap_glyphs(runtime, false, None)?;
                self.next_think = now.saturating_add(1_000_000);
                return Ok(());
            }
            if powered && !self.exposed {
                self.swap_glyphs(runtime, true, None)?;
            }
        }
        self.next_think = now.saturating_add(1_000_000);
        Ok(())
    }

    fn heal_source(
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        key: &str,
    ) -> Result<(), CarrierError> {
        let maximum = u64::try_from(runtime.companion_policy(key)?.maximum_health)
            .map_err(|_| CarrierError::InvalidCreatureHealth)?;
        runtime.commit_source_creature_heal_batch(
            runtime.content_pin().server_artifact_digest(),
            &[(actor, key.to_owned(), maximum, 10_000)],
        )?;
        Ok(())
    }

    fn swap_glyphs(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        expose: bool,
        source: Option<(ExactActorRef, MovementLocalPosition)>,
    ) -> Result<(), CarrierError> {
        let shield_position = runtime.read_actor_position(self.shielded)?.position();
        let clear_position = runtime.read_actor_position(self.glyph)?.position();
        let mut moves = vec![
            (self.shielded, clear_position),
            (self.glyph, shield_position),
        ];
        if let Some((_, at)) = source {
            moves.push((self.distorted, at));
        }
        let health = if expose {
            (self.shielded, self.glyph)
        } else {
            (self.glyph, self.shielded)
        };
        runtime.relocate_quest_creatures(&moves, source.map(|(actor, _)| actor), Some(health))?;
        self.exposed = expose;
        Ok(())
    }

    fn death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        death: &PendingKillSettlement,
    ) -> Result<(), CarrierError> {
        if self.boss_spawned {
            return Ok(());
        }
        if death.target == self.glyph && death.creature == GLYPH {
            let health = runtime.companion_policy(LORE_KEEPER)?.maximum_health;
            runtime.commit_quest_creature_spawn(&[(LORE_KEEPER, health, death.facts.corpse)])?;
            self.boss_spawned = true;
        } else if self.power == Some(death.target) && death.creature == POWER {
            self.power = if let Some(at) = death.lore_power_landing {
                let health = runtime.companion_policy(POWER)?.maximum_health;
                let actors = runtime.commit_quest_creature_spawn(&[(POWER, health, at)])?;
                self.power_until = death
                    .facts
                    .death_at_ms
                    .saturating_mul(1_000)
                    .saturating_add(60_000_000);
                Some(actors[0])
            } else {
                None
            };
        }
        Ok(())
    }
}

pub(super) fn is_astral_death(runtime: &ChannelRuntimeV1, creature: &str) -> bool {
    matches!(
        runtime.encounter_owner_scope().0,
        crate::foundation::RuntimeScopeRefV1::Instance { .. }
    ) && matches!(creature, GLYPH | POWER)
}

pub(super) fn capture_power_landing(
    runtime: &ChannelRuntimeV1,
    creature: &str,
    death: &crate::combat::ProjectedCreatureDeathFacts,
    shares: &[crate::foundation::RewardDamageShare],
) -> Option<MovementLocalPosition> {
    if creature != POWER || !is_astral_death(runtime, creature) {
        return None;
    }
    let winner = shares
        .iter()
        .find(|share| share.character == death.top_damage_character)
        .and_then(|share| share.session)
        .or_else(|| {
            (death.principal.character == death.top_damage_character)
                .then_some(death.principal.session)
        })?;
    let pos = runtime
        .positioned_actor_census()
        .ok()?
        .into_iter()
        .find_map(|(_, pos, session)| (session == Some(winner)).then_some(pos.position()))?;
    let index = POWER_CELLS.iter().position(|anchor| {
        pos.floor == anchor.floor && pos.x.abs_diff(anchor.x).max(pos.y.abs_diff(anchor.y)) < 7
    })?;
    Some(POWER_CELLS[(index + 1) % POWER_CELLS.len()])
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn tick_lore_keeper(&self) {
        if !self
            .states
            .quest_callbacks
            .lore_keeper
            .lock()
            .is_ok_and(|state| state.is_some())
        {
            return;
        }
        let Some(_permit) = self.spell_lane_permit().await else {
            return;
        };
        let mut runtime = self.runtime.lock().await;
        let Ok(mut state) = self.states.quest_callbacks.lore_keeper.lock() else {
            return;
        };
        if runtime.owner_fence().is_ok()
            && let Some(state) = state.as_mut()
        {
            let _ = state.think(&mut runtime, self.owner_now().get());
        }
    }

    pub(super) async fn settle_lore_keeper_death(&self, death: &PendingKillSettlement) -> bool {
        if !matches!(death.creature.as_str(), GLYPH | POWER) {
            return true;
        }
        let mut runtime = self.runtime.lock().await;
        let Ok(mut state) = self.states.quest_callbacks.lore_keeper.lock() else {
            return false;
        };
        match state.as_mut() {
            None => true,
            Some(state) => {
                runtime.owner_fence().is_ok() && state.death(&mut runtime, death).is_ok()
            }
        }
    }
}
