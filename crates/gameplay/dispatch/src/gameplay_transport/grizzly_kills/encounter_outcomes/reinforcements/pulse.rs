//! Crystal 201189f8 overcharged_demon.lua:113-169 and
//! creature_functions.cpp:591-609. An exact living Demon is consumed once.
//! The retained player cursor resumes after a refused health publication.
use crate::{
    foundation::{
        ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
        owner_timer::SemanticTimeMicros,
    },
    gameplay_transport::actor_spell::ChannelSpellStates,
    map::{boot::BundleMap, overlay::TilePos},
};
use oteryn_gameplay_quests::quest::world_callbacks::EncounterReinforcementPulse;

#[derive(Clone, Debug)]
pub(super) struct Pending {
    targets: Vec<(ExactActorRef, GameSessionId)>,
    next: usize,
    damage: u32,
}

impl Pending {
    pub(super) fn begin(
        runtime: &mut ChannelRuntimeV1,
        boss: ExactActorRef,
        demon: ExactActorRef,
        rule: &EncounterReinforcementPulse,
        map: &BundleMap,
    ) -> Option<Self> {
        let center = runtime.read_actor_position(boss).ok()?.position();
        if center.floor != -i16::from(rule.boss_min.floor)
            || !(i32::from(rule.boss_min.x)..=i32::from(rule.boss_max.x)).contains(&center.x)
            || !(i32::from(rule.boss_min.y)..=i32::from(rule.boss_max.y)).contains(&center.y)
        {
            return None;
        }
        let current = runtime.companion_snapshot(boss).ok()?;
        if current.health <= 0 {
            return None;
        }
        runtime.assert_actor_spell_unreserved(boss).ok()?;
        runtime.assert_actor_spell_unreserved(demon).ok()?;
        let targets = runtime
            .positioned_melee_census()
            .ok()?
            .into_iter()
            .filter_map(|(actor, position, session)| {
                let at = position.position();
                (within_radius(center, at, rule.radius) && map_tile(map, at).is_some())
                    .then_some((actor, session?))
            })
            .collect();
        let health = current
            .health
            .saturating_add(i64::from(rule.boss_heal))
            .min(current.state.policy.maximum_health);
        runtime
            .commit_encounter_consumption(demon, boss, health)
            .ok()?;
        let radius = i32::from(rule.radius);
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                let at = MovementLocalPosition {
                    x: center.x + dx,
                    y: center.y + dy,
                    ..center
                };
                if let Some(tile) = map_tile(map, at) {
                    map.facts.emit_movement_effect(tile, rule.effect);
                }
            }
        }
        Some(Self {
            targets,
            next: 0,
            damage: rule.player_damage,
        })
    }

    pub(super) fn finish(
        &mut self,
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        now_ms: u64,
    ) -> bool {
        while let Some(&(actor, session)) = self.targets.get(self.next) {
            if runtime.player_control_facts(actor, session).is_ok()
                && !states.is_dead(actor)
                && states
                    .apply_encounter_pulse_health_loss(
                        runtime,
                        actor,
                        session,
                        self.damage,
                        SemanticTimeMicros::from_micros(now_ms.saturating_mul(1_000)),
                    )
                    .is_none()
            {
                return false;
            }
            self.next += 1;
        }
        true
    }
}

fn within_radius(center: MovementLocalPosition, at: MovementLocalPosition, radius: u8) -> bool {
    center.floor == at.floor
        && center.x.abs_diff(at.x) <= u32::from(radius)
        && center.y.abs_diff(at.y) <= u32::from(radius)
}

fn map_tile(map: &BundleMap, at: MovementLocalPosition) -> Option<TilePos> {
    let tile = TilePos {
        x: u16::try_from(at.x).ok()?,
        y: u16::try_from(at.y).ok()?,
        floor: i8::try_from(-at.floor).ok()?,
    };
    map.overlay.base().tile(tile.x, tile.y, tile.floor)?;
    Some(tile)
}
