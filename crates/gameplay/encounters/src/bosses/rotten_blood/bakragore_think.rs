//! Crystal bakragore.lua:312-611: Bakragore's summons, room tracking and the one-shot echo spawn.
//! Agony stacks and the centre teleport cycle are the shared Chagorz owners (chagorz_think.rs).

use super::{
    RoomPosition,
    chagorz_think::{CentreTeleport, TeleportCycle},
};

pub const BAKRAGORE_KEY: &str = "oteryn:creature.bakragore";
pub const MUSHROOM_KEY: &str = "oteryn:creature.mushroom";
pub const PILLAR_KEY: &str = "oteryn:creature.pillar_of_dark_energy";
pub const ELDER_BLOODJAW_KEY: &str = "oteryn:creature.elder_bloodjaw";
pub const BAKRAGORE_ROOM_CENTRE: RoomPosition = RoomPosition {
    x: 33_043,
    y: 32_397,
    floor: 15,
};
pub const AGONY_ROOM_CENTRE: RoomPosition = RoomPosition {
    x: 33_044,
    y: 32_398,
    floor: 15,
};
pub const AGONY_ROOM_RANGE_TILES: u16 = 20;
pub const AGONY_ROOM_DEFAULT_Y_TILES: u16 = 11;
pub const AGONY_CHECK_INTERVAL_MS: u64 = 1_000;
pub const SUMMON_COUNT_RANGE_TILES: u16 = 20;
pub const SUMMON_OFFSET_TILES: u8 = 1;
pub const SUMMON_FLOOR: u8 = 4;
pub const FINAL_TAINT_LEVEL: u8 = 5;
pub const ECHO_CHECK_RANGE_TILES: u16 = 15;
pub const TELEPORT_CENSUS_RANGE_TILES: u16 = 15;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Essence {
    pub storage: &'static str,
    pub echo_key: &'static str,
    pub echo_at: RoomPosition,
    pub flame_at: RoomPosition,
    pub flame_item: u16,
    pub flame_reset_item: u16,
    pub pillar_at: RoomPosition,
    pub pillar_item: u16,
    pub pillar_reset_item: u16,
}

/// bakragore.lua:369-414, in donor index order.
pub const ESSENCES: [Essence; 4] = [
    Essence {
        storage: "essenceOfMurcion",
        echo_key: "oteryn:creature.echo_of_murcion",
        echo_at: RoomPosition {
            x: 33_047,
            y: 32_398,
            floor: 15,
        },
        flame_at: RoomPosition {
            x: 33_085,
            y: 32_398,
            floor: 15,
        },
        flame_item: 43_984,
        flame_reset_item: 43_983,
        pillar_at: RoomPosition {
            x: 33_081,
            y: 32_398,
            floor: 15,
        },
        pillar_item: 43_508,
        pillar_reset_item: 43_507,
    },
    Essence {
        storage: "essenceOfIchgahal",
        echo_key: "oteryn:creature.echo_of_ichgahal",
        echo_at: RoomPosition {
            x: 33_044,
            y: 32_393,
            floor: 15,
        },
        flame_at: RoomPosition {
            x: 33_076,
            y: 32_389,
            floor: 15,
        },
        flame_item: 43_498,
        flame_reset_item: 43_497,
        pillar_at: RoomPosition {
            x: 33_076,
            y: 32_395,
            floor: 15,
        },
        pillar_item: 43_508,
        pillar_reset_item: 43_507,
    },
    Essence {
        storage: "essenceOfChagorz",
        echo_key: "oteryn:creature.echo_of_chagorz",
        echo_at: RoomPosition {
            x: 33_044,
            y: 32_399,
            floor: 15,
        },
        flame_at: RoomPosition {
            x: 33_067,
            y: 32_398,
            floor: 15,
        },
        flame_item: 43_988,
        flame_reset_item: 43_987,
        pillar_at: RoomPosition {
            x: 33_071,
            y: 32_398,
            floor: 15,
        },
        pillar_item: 43_508,
        pillar_reset_item: 43_507,
    },
    Essence {
        storage: "essenceOfVemiath",
        echo_key: "oteryn:creature.echo_of_vemiath",
        echo_at: RoomPosition {
            x: 33_039,
            y: 32_396,
            floor: 15,
        },
        flame_at: RoomPosition {
            x: 33_076,
            y: 32_407,
            floor: 15,
        },
        flame_item: 43_986,
        flame_reset_item: 43_985,
        pillar_at: RoomPosition {
            x: 33_076,
            y: 32_401,
            floor: 15,
        },
        pillar_item: 43_508,
        pillar_reset_item: 43_507,
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BakragoreEffect {
    StartTeleportCycle,
    /// Passed to the native Item owner; this request alone mints no Item.
    CreateCentreTeleport(CentreTeleport),
    Spawn {
        creature_key: &'static str,
        around: RoomPosition,
        offset_tiles: u8,
    },
    /// Forget agony tracking for every player not listed, then run the shared check over them.
    RunAgonyCheck {
        room_players: Vec<u64>,
    },
    /// Boss storage 88888.
    SetEssenceCount {
        count: u8,
    },
    SpawnEcho {
        creature_key: &'static str,
        at: RoomPosition,
    },
    ResetEssence {
        storage: &'static str,
    },
    TransformItem {
        at: RoomPosition,
        from: u16,
        to: u16,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EchoPlayer {
    pub actor: u64,
    pub position: RoomPosition,
    pub taint_level: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct BakragoreThinkFacts<'a> {
    pub now_ms: u64,
    /// Elapsed source onThink interval; independent of absolute timer time.
    pub interval_ms: u64,
    pub teleport_paused: bool,
    /// Elder Bloodjaw in the source centre spectator box (15 tiles, same floor).
    pub teleport_bloodjaw_present: bool,
    pub boss: RoomPosition,
    pub mechanics_disabled: bool,
    pub bloodjaw_spawn_paused: bool,
    pub bloodjaw_target: u8,
    pub nearby_bloodjaws: u8,
    pub nearby_mushrooms: u8,
    pub nearby_pillars: u8,
    /// Active flags of the four essence storages (> 0), in ESSENCES order.
    pub active_essences: [bool; 4],
    /// Players the caller read on floor 15 around the room (filtered here).
    pub players: &'a [EchoPlayer],
}

pub fn counts_summon(boss: RoomPosition, other: RoomPosition) -> bool {
    boss.in_range(other, SUMMON_COUNT_RANGE_TILES)
}

/// Crystal201189f bakragore.lua:443 passes20 in X and0 in Y.
/// src/map/spectators.cpp:181–184 normalizes0 to the default viewport;
/// src/map/map_const.hpp:20–23 defines its Y extent as6+5=11.
pub fn in_agony_room(position: RoomPosition) -> bool {
    position.floor == AGONY_ROOM_CENTRE.floor
        && position.y.abs_diff(AGONY_ROOM_CENTRE.y) <= u32::from(AGONY_ROOM_DEFAULT_Y_TILES)
        && position.x.abs_diff(AGONY_ROOM_CENTRE.x) <= u32::from(AGONY_ROOM_RANGE_TILES)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BakragoreThinkOwner {
    initialized: bool,
    echoes_checked: bool,
    agony_elapsed_ms: u64,
    teleport: TeleportCycle,
}

impl Default for BakragoreThinkOwner {
    fn default() -> Self {
        Self {
            initialized: false,
            echoes_checked: false,
            agony_elapsed_ms: 0,
            teleport: TeleportCycle::new(BAKRAGORE_ROOM_CENTRE),
        }
    }
}

impl BakragoreThinkOwner {
    pub fn think(&mut self, facts: &BakragoreThinkFacts<'_>) -> Vec<BakragoreEffect> {
        let mut effects = Vec::new();
        if !self.initialized {
            self.teleport.start(facts.now_ms);
            effects.push(BakragoreEffect::StartTeleportCycle);
            self.initialized = true;
        }
        // The source pause branch rearms before reading the centre census.
        let boss_in_centre = facts.teleport_paused
            || BAKRAGORE_ROOM_CENTRE.in_range(facts.boss, TELEPORT_CENSUS_RANGE_TILES);
        if let Some(teleport) = self.teleport.due(
            facts.now_ms,
            boss_in_centre,
            facts.teleport_paused,
            facts.teleport_bloodjaw_present,
        ) {
            effects.push(BakragoreEffect::CreateCentreTeleport(teleport));
        }

        if !facts.mechanics_disabled {
            if !facts.bloodjaw_spawn_paused {
                add_spawns(
                    &mut effects,
                    ELDER_BLOODJAW_KEY,
                    facts.bloodjaw_target.saturating_sub(facts.nearby_bloodjaws),
                    facts.boss,
                );
            }
            add_spawns(
                &mut effects,
                MUSHROOM_KEY,
                SUMMON_FLOOR.saturating_sub(facts.nearby_mushrooms),
                facts.boss,
            );
            add_spawns(
                &mut effects,
                PILLAR_KEY,
                SUMMON_FLOOR.saturating_sub(facts.nearby_pillars),
                facts.boss,
            );
            self.agony_elapsed_ms = self.agony_elapsed_ms.saturating_add(facts.interval_ms);
            if self.agony_elapsed_ms >= AGONY_CHECK_INTERVAL_MS {
                // Source checkTime resets to zero; it does not carry overshoot.
                self.agony_elapsed_ms = 0;
                effects.push(BakragoreEffect::RunAgonyCheck {
                    room_players: facts
                        .players
                        .iter()
                        .filter(|player| in_agony_room(player.position))
                        .map(|player| player.actor)
                        .collect(),
                });
            }
        }

        if !self.echoes_checked {
            let count = facts
                .active_essences
                .iter()
                .filter(|active| **active)
                .count() as u8;
            effects.push(BakragoreEffect::SetEssenceCount { count });
            if count > 0 {
                let mut nearby_players = facts.players.iter().filter(|player| {
                    BAKRAGORE_ROOM_CENTRE.in_range(player.position, ECHO_CHECK_RANGE_TILES)
                });
                let spawn = nearby_players
                    .next()
                    .is_some_and(|first| first.taint_level >= FINAL_TAINT_LEVEL)
                    && nearby_players.all(|player| player.taint_level >= FINAL_TAINT_LEVEL);
                for (essence, active) in ESSENCES.iter().zip(facts.active_essences) {
                    if !active {
                        continue;
                    }
                    if spawn {
                        effects.push(BakragoreEffect::SpawnEcho {
                            creature_key: essence.echo_key,
                            at: essence.echo_at,
                        });
                    }
                    effects.push(BakragoreEffect::ResetEssence {
                        storage: essence.storage,
                    });
                    effects.push(BakragoreEffect::TransformItem {
                        at: essence.flame_at,
                        from: essence.flame_item,
                        to: essence.flame_reset_item,
                    });
                    effects.push(BakragoreEffect::TransformItem {
                        at: essence.pillar_at,
                        from: essence.pillar_item,
                        to: essence.pillar_reset_item,
                    });
                }
            }
            self.echoes_checked = true;
        }
        effects
    }
}

fn add_spawns(
    effects: &mut Vec<BakragoreEffect>,
    creature_key: &'static str,
    count: u8,
    around: RoomPosition,
) {
    effects.extend((0..count).map(|_| BakragoreEffect::Spawn {
        creature_key,
        around,
        offset_tiles: SUMMON_OFFSET_TILES,
    }));
}

#[cfg(test)]
#[path = "bakragore_think_tests.rs"]
mod tests;
