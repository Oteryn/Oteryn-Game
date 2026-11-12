//! Crystal vemiath.lua:304-363: Vemiath's summons and agony gate. Agony stacks and the centre
//! teleport cycle are the shared Chagorz owners (chagorz_think.rs), reused by the wiring.

use super::RoomPosition;

pub const VEMIATH_KEY: &str = "oteryn:creature.vemiath";
pub const PILLAR_KEY: &str = "oteryn:creature.pillar_of_dark_energy";
pub const ELDER_BLOODJAW_KEY: &str = "oteryn:creature.elder_bloodjaw";
pub const VEMIATH_ROOM_CENTRE: RoomPosition = RoomPosition {
    x: 33_043,
    y: 32_335,
    floor: 15,
};
pub const SUMMON_COUNT_RANGE_TILES: u16 = 17;
pub const SUMMON_OFFSET_TILES: u8 = 1;
pub const PILLAR_FLOOR: u8 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VemiathEffect {
    /// First think: the caller starts the shared TeleportCycle on VEMIATH_ROOM_CENTRE.
    StartTeleportCycle,
    /// Close placement: first walkable of 10 tries within ±offset, else the boss tile.
    Spawn {
        creature_key: &'static str,
        around: RoomPosition,
        offset_tiles: u8,
    },
    /// Run the shared AgonyStacks check (range 16) this think.
    RunAgonyCheck,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VemiathThinkFacts {
    pub boss: RoomPosition,
    pub mechanics_disabled: bool,
    pub bloodjaw_spawn_paused: bool,
    /// `bloodjaw_target_count(max taint within 15 of VEMIATH_ROOM_CENTRE)`.
    pub bloodjaw_target: u8,
    /// Elder Bloodjaws and pillars within 17 tiles (`counts_summon`).
    pub nearby_bloodjaws: u8,
    pub nearby_pillars: u8,
}

pub fn counts_summon(boss: RoomPosition, other: RoomPosition) -> bool {
    boss.in_range(other, SUMMON_COUNT_RANGE_TILES)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VemiathThinkOwner {
    initialized: bool,
}

impl VemiathThinkOwner {
    pub fn think(&mut self, facts: &VemiathThinkFacts) -> Vec<VemiathEffect> {
        let mut effects = Vec::new();

        if !self.initialized {
            self.initialized = true;
            effects.push(VemiathEffect::StartTeleportCycle);
        }

        if !facts.mechanics_disabled && !facts.bloodjaw_spawn_paused {
            for _ in 0..facts.bloodjaw_target.saturating_sub(facts.nearby_bloodjaws) {
                effects.push(VemiathEffect::Spawn {
                    creature_key: ELDER_BLOODJAW_KEY,
                    around: facts.boss,
                    offset_tiles: SUMMON_OFFSET_TILES,
                });
            }
        }

        for _ in 0..PILLAR_FLOOR.saturating_sub(facts.nearby_pillars) {
            effects.push(VemiathEffect::Spawn {
                creature_key: PILLAR_KEY,
                around: facts.boss,
                offset_tiles: SUMMON_OFFSET_TILES,
            });
        }

        if !facts.mechanics_disabled {
            effects.push(VemiathEffect::RunAgonyCheck);
        }

        effects
    }
}

#[cfg(test)]
#[path = "vemiath_think_tests.rs"]
mod tests;
