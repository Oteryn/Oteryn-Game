//! Channel-owned production integration for Global-style echo raids.

use super::*;
pub(super) use oteryn_gameplay_encounter_rules::echo_raid::Tile;
use oteryn_gameplay_encounter_rules::echo_raid::{
    Coordinator, EligibleSpawn, Outcome, Raid, Trigger, TriggerContext,
};
use std::collections::BTreeMap;

/// A materialised raid actor. The combat owner consumes these facts when it admits the creature;
/// keeping them here makes lifetime, Warden aura and guaranteed loot explicit and bounded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EchoActor {
    pub id: u64,
    pub creature: String,
    pub tile: Tile,
    pub influenced: bool,
    pub warden: bool,
    pub damage_percent: u8,
    pub health_percent: u8,
    /// One shared activity-instance identifier for every actor of this raid.
    pub encounter: u64,
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
        let expired: Vec<_> = self
            .actors
            .iter()
            .filter(|actor| actor.despawn_at_ms <= now_ms)
            .map(|actor| actor.id)
            .collect();
        for id in expired {
            self.despawn(id);
        }
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

    /// The visibility/targeting projection consumed by the Channel owner. Expired actors are
    /// removed by `drive` before this view is read; wardens and influenced creatures are ordinary
    /// targetable creatures, rather than a parallel item or effect projection.
    pub fn visible_targetable(&self) -> &[EchoActor] {
        &self.actors
    }

    /// Administrative removal uses the same bounded retained projection as lifetime expiry.
    pub fn despawn(&mut self, id: u64) -> bool {
        let before = self.actors.len();
        self.actors.retain(|actor| actor.id != id);
        self.actors.len() != before
    }

    /// Apply damage through the actor projection. A Warden's health is authoritative here and
    /// its aura percentage is exposed by `damage_percent`; no separate boss-room mechanic exists.
    pub fn damage(&mut self, id: u64, percent: u8) -> Option<bool> {
        let actor = self.actors.iter_mut().find(|actor| actor.id == id)?;
        actor.health_percent = actor.health_percent.saturating_sub(percent);
        Some(actor.health_percent == 0)
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
        Some(self.visible_targetable())
    }

    fn materialise(&mut self, raid: Raid) {
        let encounter = self.sequence;
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
            self.sequence = self.sequence.wrapping_add(1);
            self.actors.push(EchoActor {
                id: self.sequence,
                creature: raid.creature.clone(),
                tile: raid.tile,
                influenced: false,
                warden: true,
                damage_percent: 0,
                health_percent: 100,
                encounter,
                despawn_at_ms: raid.despawn_at_ms,
            });
        }
        for index in 0..regular.saturating_add(influenced) {
            self.sequence = self.sequence.wrapping_add(1);
            self.actors.push(EchoActor {
                id: self.sequence,
                creature: raid.creature.clone(),
                tile: raid.tile,
                influenced: index >= regular,
                warden: false,
                damage_percent: damage,
                health_percent: 100,
                encounter,
                despawn_at_ms: raid.despawn_at_ms,
            });
        }
        // Actor admission initializes health through the same mutation seam combat uses.
        for id in self
            .actors
            .iter()
            .filter(|actor| actor.encounter == encounter)
            .map(|actor| actor.id)
            .collect::<Vec<_>>()
        {
            let _ = self.damage(id, 0);
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
            let now = self.owner_now().get() / 1_000;
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

    pub(super) async fn trigger_echo_use(&self, actor: ExactActorRef) {
        let position = {
            let mut runtime = self.runtime.lock().await;
            runtime
                .borrow_movement_position()
                .read(actor)
                .ok()
                .map(|snapshot| snapshot.position())
        };
        let Some(position) = position else { return };
        if let Ok(mut echoes) = self.states.echo_raids.lock() {
            let now = self.owner_now().get() / 1_000;
            echoes.trigger(
                Tile {
                    x: position.x as u16,
                    y: position.y as u16,
                    floor: position.floor as i8,
                },
                Trigger::Use,
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
        let actor = actors[0].id;
        assert_eq!(owner.visible_targetable()[0].health_percent, 100);
        assert_eq!(owner.damage(actor, 25), Some(false));
        assert_eq!(owner.visible_targetable()[0].health_percent, 75);
        assert!(owner.despawn(actor));
        owner.drive(oteryn_gameplay_encounter_rules::echo_raid::RAID_LIFETIME_MS + 1);
        assert!(owner.actors.is_empty());
        Ok(())
    }
}
