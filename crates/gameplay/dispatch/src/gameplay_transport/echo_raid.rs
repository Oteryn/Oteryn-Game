//! Channel-owned production integration for Global-style echo raids.

use super::*;
use oteryn_gameplay_encounter_rules::echo_raid::{
    Coordinator, EligibleSpawn, Outcome, Raid, Tile, Trigger, TriggerContext,
};
use std::collections::BTreeMap;

/// A materialised raid actor. The combat owner consumes these facts when it admits the creature;
/// keeping them here makes lifetime, Warden aura and guaranteed loot explicit and bounded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EchoActor {
    pub creature: String,
    pub tile: Tile,
    pub influenced: bool,
    pub warden: bool,
    pub damage_percent: u8,
    pub despawn_at_ms: u64,
}

#[derive(Default)]
pub(super) struct EchoRaidRuntime {
    coordinator: Coordinator,
    candidates: BTreeMap<u16, Vec<EligibleSpawn>>,
    actors: Vec<EchoActor>,
    sequence: u64,
}

impl EchoRaidRuntime {
    #[allow(dead_code, reason = "candidate installation is owned by bundle boot")]
    pub fn install_candidates(&mut self, candidates: BTreeMap<u16, Vec<EligibleSpawn>>) {
        self.candidates = candidates;
        self.drive(0);
    }

    pub fn drive(&mut self, now_ms: u64) {
        self.sequence = self.sequence.wrapping_add(1);
        self.coordinator
            .drive(now_ms, &self.candidates, self.sequence);
        self.actors.retain(|actor| actor.despawn_at_ms > now_ms);
    }

    #[allow(
        dead_code,
        reason = "kill-created echo hook is owned by death settlement"
    )]
    pub fn kill_echo(&mut self, creature: &str, tile: Tile, reachable: bool, roll: u16) -> bool {
        // Crystal's documented 0.05% donor chance is retained until Global publishes a rate.
        roll.is_multiple_of(2_000) && self.coordinator.arm_wild(creature, tile, reachable)
    }

    #[allow(
        dead_code,
        reason = "ground placement admission consumes this predicate"
    )]
    pub fn blocks_item(&self, tile: Tile) -> bool {
        self.coordinator.blocks_item_placement(tile)
    }

    pub fn trigger(
        &mut self,
        tile: Tile,
        trigger: Trigger,
        now_ms: u64,
        bestiary_complete: bool,
    ) -> Option<&[EchoActor]> {
        self.sequence = self.sequence.wrapping_add(1);
        let raid = self.coordinator.trigger_tile(
            tile,
            TriggerContext {
                trigger,
                now_ms,
                interval_roll: self.sequence,
                outcome_roll: (self.sequence % 100) as u8,
                bestiary_complete,
            },
        )?;
        self.materialise(raid);
        Some(&self.actors)
    }

    fn materialise(&mut self, raid: Raid) {
        let (regular, influenced, warden, damage) = match raid.outcome {
            Outcome::Regular { count } => (count, 0, false, 0),
            Outcome::Influenced { count } => (0, count, false, 0),
            Outcome::Warden {
                regular_count,
                aura_damage_percent,
                ..
            } => (regular_count, 0, true, aura_damage_percent),
        };
        if warden {
            self.actors.push(EchoActor {
                creature: raid.creature.clone(),
                tile: raid.tile,
                influenced: false,
                warden: true,
                damage_percent: 0,
                despawn_at_ms: raid.despawn_at_ms,
            });
        }
        for index in 0..regular.saturating_add(influenced) {
            self.actors.push(EchoActor {
                creature: raid.creature.clone(),
                tile: raid.tile,
                influenced: index >= regular,
                warden: false,
                damage_percent: damage,
                despawn_at_ms: raid.despawn_at_ms,
            });
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn trigger_echo_step(
        &self,
        _session: GameSessionId,
        position: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    ) {
        if let Ok(mut echoes) = self.states.echo_raids.lock() {
            let now = self.owner_now().get();
            echoes.drive(now);
            echoes.trigger(
                Tile {
                    x: position.x as u16,
                    y: position.y as u16,
                    floor: position.floor as i8,
                },
                Trigger::StepIn,
                now,
                false,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn() -> EligibleSpawn {
        EligibleSpawn {
            creature: "rat".into(),
            tile: Tile {
                x: 10,
                y: 11,
                floor: 7,
            },
            reachable: true,
            bestiary_stars: 1,
            common_or_uncommon: true,
        }
    }

    #[test]
    fn owner_seeds_triggers_materialises_and_despawns() -> Result<(), &'static str> {
        let mut owner = EchoRaidRuntime::default();
        owner.install_candidates(BTreeMap::from([(4, vec![spawn()])]));
        assert!(owner.blocks_item(spawn().tile));
        let actors = owner
            .trigger(spawn().tile, Trigger::Use, 1, false)
            .ok_or("seeded echo must trigger")?;
        assert!(!actors.is_empty());
        owner.drive(oteryn_gameplay_encounter_rules::echo_raid::RAID_LIFETIME_MS + 1);
        assert!(owner.actors.is_empty());
        Ok(())
    }
}
