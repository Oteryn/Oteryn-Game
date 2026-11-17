//! Crystal murcion.lua onThink (:191-215) for Murcion.
use super::RoomPosition;

#[allow(
    non_upper_case_globals,
    reason = "the follow-up hook API requires this exported name"
)]
pub const MURCION_KEY: &str = "oteryn:creature.murcion";
pub const MUSHROOM_KEY: &str = "oteryn:creature.mushroom";
pub const ELDER_BLOODJAW_KEY: &str = "oteryn:creature.elder_bloodjaw";
pub const MUSHROOM_FLOOR: u8 = 4;
pub const MUSHROOM_COUNT_RANGE_TILES: u16 = 16;
pub const BLOODJAW_COUNT_RANGE_TILES: u16 = 15;
pub const SUMMON_1_TILES: u8 = 1;
pub const BLOODJAW_1_TILES: u8 = 2;
pub const WAVE_MUSHROOMS: u8 = 3;
pub const WAVE_INTERVAL_MIN_MS: u64 = 8_000;
pub const WAVE_INTERVAL_MAX_MS: u64 = 9_000;

/// How the caller places a creature: `Close` tries ten walkable tiles, then the centre;
/// `Random` uses any tile within the offset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Placement {
    Close { offset_tiles: u8 },
    Random { offset_tiles: u8 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MurcionEffect {
    Spawn {
        creature_key: &'static str,
        around: RoomPosition,
        placement: Placement,
    },
    /// Remove a previous wave summon if it still exists (the caller skips a missing one).
    RemoveSummon { actor: u64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MurcionThinkFacts {
    pub boss: RoomPosition,
    pub interval_ms: u64,
    /// Mushrooms within `MUSHROOM_COUNT_RANGE_TILES`, counted with `counts_mushroom`.
    pub nearby_mushrooms: u8,
    /// Elder Bloodjaws within `BLOODJAW_COUNT_RANGE_TILES` of the boss.
    pub nearby_bloodjaws: u8,
    /// `bloodjaw_target_count(max room taint)` from the Elder Bloodjaw module.
    pub bloodjaw_target: u8,
    pub mechanics_disabled: bool,
    pub bloodjaw_spawn_paused: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MurcionRefusal {
    InvalidWaveIntervalDraw,
}

/// Whether a Mushroom at `other` counts toward the floor of four.
pub fn counts_mushroom(boss: RoomPosition, other: RoomPosition) -> bool {
    boss.in_range(other, MUSHROOM_COUNT_RANGE_TILES)
}

/// Whether an Elder Bloodjaw at `other` counts toward the taint-based target.
pub fn counts_bloodjaw(boss: RoomPosition, other: RoomPosition) -> bool {
    boss.in_range(other, BLOODJAW_COUNT_RANGE_TILES)
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MurcionThinkOwner {
    accumulated_ms: u64,
    wave_interval_ms: u64,
    active_wave: Vec<u64>,
}

impl MurcionThinkOwner {
    /// Runs one boss think. The interval draw occurs once per wave.
    pub fn think(
        &mut self,
        facts: &MurcionThinkFacts,
        mut draw_interval_ms: impl FnMut() -> u64,
    ) -> Result<Vec<MurcionEffect>, MurcionRefusal> {
        let mut next = self.clone();
        next.accumulated_ms = next.accumulated_ms.saturating_add(facts.interval_ms);

        let bloodjaws = if facts.mechanics_disabled || facts.bloodjaw_spawn_paused {
            0
        } else {
            facts.bloodjaw_target.saturating_sub(facts.nearby_bloodjaws)
        };
        let mushrooms = MUSHROOM_FLOOR.saturating_sub(facts.nearby_mushrooms);
        let mut effects = Vec::with_capacity(
            usize::from(bloodjaws) + usize::from(mushrooms) + usize::from(WAVE_MUSHROOMS),
        );

        for _ in 0..bloodjaws {
            effects.push(MurcionEffect::Spawn {
                creature_key: ELDER_BLOODJAW_KEY,
                around: facts.boss,
                placement: Placement::Close {
                    offset_tiles: BLOODJAW_1_TILES,
                },
            });
        }
        for _ in 0..mushrooms {
            effects.push(MurcionEffect::Spawn {
                creature_key: MUSHROOM_KEY,
                around: facts.boss,
                placement: Placement::Close {
                    offset_tiles: SUMMON_1_TILES,
                },
            });
        }

        if next.accumulated_ms >= next.wave_interval_ms {
            let interval = draw_interval_ms();
            if !(WAVE_INTERVAL_MIN_MS..=WAVE_INTERVAL_MAX_MS).contains(&interval) {
                return Err(MurcionRefusal::InvalidWaveIntervalDraw);
            }
            effects.extend(
                next.active_wave
                    .drain(..)
                    .map(|actor| MurcionEffect::RemoveSummon { actor }),
            );
            for _ in 0..WAVE_MUSHROOMS {
                effects.push(MurcionEffect::Spawn {
                    creature_key: MUSHROOM_KEY,
                    around: facts.boss,
                    placement: Placement::Random {
                        offset_tiles: SUMMON_1_TILES,
                    },
                });
            }
            next.accumulated_ms = 0;
            next.wave_interval_ms = interval;
        }

        *self = next;
        Ok(effects)
    }

    /// Records a wave Mushroom that the caller successfully created.
    pub fn record_wave_summon(&mut self, actor: u64) {
        self.active_wave.push(actor);
    }

    #[must_use]
    pub fn active_wave(&self) -> &[u64] {
        &self.active_wave
    }
}

#[cfg(test)]
#[path = "murcion_think_tests.rs"]
mod tests;
