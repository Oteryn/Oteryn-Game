//! Crystal ichgahal.lua onThink (:201-225) for Ichgahal.
use super::RoomPosition;

#[allow(
    non_upper_case_globals,
    reason = "the packet API fixes this exported name"
)]
pub const ICHGAHAL_KEY: &str = "oteryn:creature.ichgahal";
pub const MUSHROOM_KEY: &str = "oteryn:creature.mushroom";
pub const ELDER_BLOODJAW_KEY: &str = "oteryn:creature.elder_bloodjaw";
pub const MUSHROOM_FLOOR: u8 = 4;
pub const MUSHROOM_COUNT_RANGE_TILES: u16 = 14;
pub const BLOODJAW_COUNT_RANGE_TILES: u16 = 15;
pub const SUMMON_2_TILES: u8 = 2;
pub const BLOODJAW_2_TILES: u8 = 2;
pub const WAVE_MUSHROOMS: u8 = 3;
pub const WAVE_INTERVAL_MIN_MS: u64 = 8_000;
pub const WAVE_INTERVAL_MAX_MS: u64 = 9_000;

/// How the caller places a creature.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Placement {
    /// First walkable tile from ten attempts within the offset, or the centre.
    Close { offset_tiles: u8 },
    /// Any tile within the offset, without a walkability check.
    Random { offset_tiles: u8 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IchgahalEffect {
    Spawn {
        creature_key: &'static str,
        around: RoomPosition,
        placement: Placement,
    },
    /// Remove a previous wave summon if it still exists (the caller skips a missing one).
    RemoveSummon { actor: u64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IchgahalThinkFacts {
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
pub enum IchgahalRefusal {
    InvalidWaveIntervalDraw,
}

/// Whether a Mushroom counts toward the same-floor, 14-tile floor of four.
pub fn counts_mushroom(boss: RoomPosition, other: RoomPosition) -> bool {
    boss.in_range(other, MUSHROOM_COUNT_RANGE_TILES)
}

/// Whether an Elder Bloodjaw is on the same floor within 15 tiles of the boss.
pub fn counts_bloodjaw(boss: RoomPosition, other: RoomPosition) -> bool {
    boss.in_range(other, BLOODJAW_COUNT_RANGE_TILES)
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IchgahalThinkOwner {
    accumulated_ms: u64,
    wave_interval_ms: u64,
    active_wave: Vec<u64>,
}

impl IchgahalThinkOwner {
    /// Runs one boss think. The interval draw occurs once per wave and must be 8000..=9000.
    pub fn think(
        &mut self,
        facts: &IchgahalThinkFacts,
        mut draw_interval_ms: impl FnMut() -> u64,
    ) -> Result<Vec<IchgahalEffect>, IchgahalRefusal> {
        let mut next = self.clone();
        next.accumulated_ms = next.accumulated_ms.saturating_add(facts.interval_ms);

        let mut effects = Vec::new();
        if !facts.mechanics_disabled && !facts.bloodjaw_spawn_paused {
            for _ in 0..facts.bloodjaw_target.saturating_sub(facts.nearby_bloodjaws) {
                effects.push(IchgahalEffect::Spawn {
                    creature_key: ELDER_BLOODJAW_KEY,
                    around: facts.boss,
                    placement: Placement::Close {
                        offset_tiles: BLOODJAW_2_TILES,
                    },
                });
            }
        }
        for _ in 0..MUSHROOM_FLOOR.saturating_sub(facts.nearby_mushrooms) {
            effects.push(IchgahalEffect::Spawn {
                creature_key: MUSHROOM_KEY,
                around: facts.boss,
                placement: Placement::Close {
                    offset_tiles: SUMMON_2_TILES,
                },
            });
        }

        if next.accumulated_ms >= next.wave_interval_ms {
            let wave_interval_ms = draw_interval_ms();
            if !(WAVE_INTERVAL_MIN_MS..=WAVE_INTERVAL_MAX_MS).contains(&wave_interval_ms) {
                return Err(IchgahalRefusal::InvalidWaveIntervalDraw);
            }
            effects.extend(
                next.active_wave
                    .drain(..)
                    .map(|actor| IchgahalEffect::RemoveSummon { actor }),
            );
            for _ in 0..WAVE_MUSHROOMS {
                effects.push(IchgahalEffect::Spawn {
                    creature_key: MUSHROOM_KEY,
                    around: facts.boss,
                    placement: Placement::Random {
                        offset_tiles: SUMMON_2_TILES,
                    },
                });
            }
            next.accumulated_ms = 0;
            next.wave_interval_ms = wave_interval_ms;
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
#[path = "ichgahal_think_tests.rs"]
mod tests;
