//! Per-attempt adapter for Crystal Ichgahal/Murcion onThink summon waves.
//! The runtime supplies its current census and consumes the resulting requests;
//! these descriptions confer no creature placement or removal authority.
use super::{RoomPosition, elder_bloodjaw_think, ichgahal_think, murcion_think};

#[derive(Clone, Copy, Debug)]
pub struct SummonNeighbour<'a> {
    pub creature_key: &'a str,
    pub position: RoomPosition,
}

pub struct SummonWaveFacts<'a> {
    pub boss: RoomPosition,
    pub interval_ms: u64,
    pub neighbours: &'a [SummonNeighbour<'a>],
    pub max_room_taint: u8,
    pub mechanics_disabled: bool,
    pub bloodjaw_spawn_paused: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SummonPlacement {
    Close { offset_tiles: u8 },
    Random { offset_tiles: u8 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SummonWaveEffect {
    Spawn {
        creature_key: &'static str,
        around: RoomPosition,
        placement: SummonPlacement,
    },
    RemoveSummon {
        actor: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SummonWaveRefusal {
    InvalidTaint,
    InvalidIntervalDraw,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum WaveState {
    Ichgahal(ichgahal_think::IchgahalThinkOwner),
    Murcion(murcion_think::MurcionThinkOwner),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SummonWaveOwner {
    state: WaveState,
}

impl SummonWaveOwner {
    pub fn for_boss(creature_key: &str) -> Option<Self> {
        let state = match creature_key {
            ichgahal_think::Ichgahal_KEY => WaveState::Ichgahal(Default::default()),
            murcion_think::Murcion_KEY => WaveState::Murcion(Default::default()),
            _ => return None,
        };
        Some(Self { state })
    }

    pub fn think(
        &mut self,
        facts: &SummonWaveFacts<'_>,
        draw_interval_ms: impl FnMut() -> u64,
    ) -> Result<Vec<SummonWaveEffect>, SummonWaveRefusal> {
        if facts.max_room_taint > 9 {
            return Err(SummonWaveRefusal::InvalidTaint);
        }
        let target = elder_bloodjaw_think::bloodjaw_target_count(facts.max_room_taint);
        // Counts above the source population floor remain sufficient. Saturating
        // avoids truncating a large current census back below that floor.
        let count = |key: &str, in_range: fn(RoomPosition, RoomPosition) -> bool| {
            facts.neighbours.iter().fold(0u8, |count, row| {
                count.saturating_add(u8::from(
                    row.creature_key == key && in_range(facts.boss, row.position),
                ))
            })
        };
        match &mut self.state {
            WaveState::Ichgahal(owner) => owner
                .think(
                    &ichgahal_think::IchgahalThinkFacts {
                        boss: facts.boss,
                        interval_ms: facts.interval_ms,
                        nearby_mushrooms: count(
                            ichgahal_think::MUSHROOM_KEY,
                            ichgahal_think::counts_mushroom,
                        ),
                        nearby_bloodjaws: count(
                            ichgahal_think::ELDER_BLOODJAW_KEY,
                            ichgahal_think::counts_bloodjaw,
                        ),
                        bloodjaw_target: target,
                        mechanics_disabled: facts.mechanics_disabled,
                        bloodjaw_spawn_paused: facts.bloodjaw_spawn_paused,
                    },
                    draw_interval_ms,
                )
                .map(|effects| effects.into_iter().map(SummonWaveEffect::from).collect())
                .map_err(|_| SummonWaveRefusal::InvalidIntervalDraw),
            WaveState::Murcion(owner) => owner
                .think(
                    &murcion_think::MurcionThinkFacts {
                        boss: facts.boss,
                        interval_ms: facts.interval_ms,
                        nearby_mushrooms: count(
                            murcion_think::MUSHROOM_KEY,
                            murcion_think::counts_mushroom,
                        ),
                        nearby_bloodjaws: count(
                            murcion_think::ELDER_BLOODJAW_KEY,
                            murcion_think::counts_bloodjaw,
                        ),
                        bloodjaw_target: target,
                        mechanics_disabled: facts.mechanics_disabled,
                        bloodjaw_spawn_paused: facts.bloodjaw_spawn_paused,
                    },
                    draw_interval_ms,
                )
                .map(|effects| effects.into_iter().map(SummonWaveEffect::from).collect())
                .map_err(|_| SummonWaveRefusal::InvalidIntervalDraw),
        }
    }

    /// Called only after the creature owner successfully places a wave summon.
    pub fn record_wave_summon(&mut self, actor: u64) {
        match &mut self.state {
            WaveState::Ichgahal(owner) => owner.record_wave_summon(actor),
            WaveState::Murcion(owner) => owner.record_wave_summon(actor),
        }
    }
}

impl From<ichgahal_think::IchgahalEffect> for SummonWaveEffect {
    fn from(effect: ichgahal_think::IchgahalEffect) -> Self {
        match effect {
            ichgahal_think::IchgahalEffect::Spawn {
                creature_key,
                around,
                placement,
            } => Self::Spawn {
                creature_key,
                around,
                placement: match placement {
                    ichgahal_think::Placement::Close { offset_tiles } => {
                        SummonPlacement::Close { offset_tiles }
                    }
                    ichgahal_think::Placement::Random { offset_tiles } => {
                        SummonPlacement::Random { offset_tiles }
                    }
                },
            },
            ichgahal_think::IchgahalEffect::RemoveSummon { actor } => Self::RemoveSummon { actor },
        }
    }
}

impl From<murcion_think::MurcionEffect> for SummonWaveEffect {
    fn from(effect: murcion_think::MurcionEffect) -> Self {
        match effect {
            murcion_think::MurcionEffect::Spawn {
                creature_key,
                around,
                placement,
            } => Self::Spawn {
                creature_key,
                around,
                placement: match placement {
                    murcion_think::Placement::Close { offset_tiles } => {
                        SummonPlacement::Close { offset_tiles }
                    }
                    murcion_think::Placement::Random { offset_tiles } => {
                        SummonPlacement::Random { offset_tiles }
                    }
                },
            },
            murcion_think::MurcionEffect::RemoveSummon { actor } => Self::RemoveSummon { actor },
        }
    }
}

#[cfg(test)]
#[path = "summon_wave_tests.rs"]
mod tests;
