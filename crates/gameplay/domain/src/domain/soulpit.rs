//! Soulpit rules from Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
//! Callers provide current actor, inventory and creature facts; this module grants no authority.

mod encounter;
mod items;
mod source_bestiary;

pub use encounter::{
    EncounterCheckpoint, EncounterEffect, EncounterPhase, EncounterSnapshot, SoulpitEncounter,
    SoulpitMonster,
};
pub use items::*;
pub use source_bestiary::{
    SOURCE_BESTIARY_ROSTER_SHA256, SourceMonsterFacts, source_bestiary_candidates, source_monster,
};

#[cfg(test)]
#[path = "soulpit/roster_tests.rs"]
mod roster_tests;

use super::forge_creature::ForgeCreature;
use super::map_position::TilePos;

// data-global/lib/others/soulpit.lua:31-84,132-140.
pub const REQUIRED_LEVEL: u32 = 8;
pub const MAX_PARTICIPANTS: usize = 5;
pub const WAVE_COUNT: u8 = 4;
pub const MONSTERS_PER_WAVE: usize = 7;
pub const TIME_LIMIT_MICROS: u64 = 600_000_000;
pub const SPAWN_DELAY_MICROS: u64 = 4_000_000;
pub const WAVE_CHECK_DELAY_MICROS: u64 = 4_500_000;
pub const OBELISK_INACTIVE_SOURCE_ID: u32 = 47367;
pub const OBELISK_ACTIVE_SOURCE_ID: u32 = 47379;
pub const OBELISK_POSITION: TilePos = TilePos {
    x: 32375,
    y: 31157,
    floor: -8,
};
pub const BOSS_POSITION: TilePos = TilePos {
    x: 32376,
    y: 31144,
    floor: -8,
};
pub const EXIT_POSITION: TilePos = TilePos {
    x: 32373,
    y: 31158,
    floor: -8,
};
pub const ARENA_EXIT_POSITION: TilePos = TilePos {
    x: 32375,
    y: 31153,
    floor: -8,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoulpitError {
    ArenaOccupied,
    NotOnEntryTile,
    InvalidParticipants,
    LevelTooLow,
    NotEnoughItems,
    InvalidWave,
    InvalidStack,
    InvalidClock,
    InvalidRoll,
    InvalidHealth,
    InvalidCore,
    InvalidMonster,
    NoCandidate,
    InsufficientCapacity,
    InvalidState,
    ConflictingReplay,
    Overflow,
}

impl std::fmt::Display for SoulpitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Soulpit rule refusal: {self:?}")
    }
}

impl std::error::Error for SoulpitError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntryParticipant {
    pub slot: u8,
    pub level: u32,
    pub game_master: bool,
}

/// Native floors use `-source_z` (ADR-0021); staging order is donor playerPositions order.
pub fn entry_slot(position: TilePos) -> Option<u8> {
    if position.x == 32375 && position.floor == -8 && (31158..=31162).contains(&position.y) {
        u8::try_from(position.y - 31158).ok()
    } else {
        None
    }
}

pub fn entry_destination(slot: u8) -> Result<TilePos, SoulpitError> {
    if usize::from(slot) >= MAX_PARTICIPANTS {
        return Err(SoulpitError::NotOnEntryTile);
    }
    Ok(TilePos {
        x: 32373 + u16::from(slot),
        y: 31151,
        floor: -8,
    })
}

pub fn is_arena_position(position: TilePos) -> bool {
    position.floor == -8
        && (32362..=32390).contains(&position.x)
        && (31132..=31153).contains(&position.y)
}

/// soulpit_fight.lua:37-77 checks all occupants before consuming one core. Generic use-with
/// adds adjacency (actions.hpp:137; actions.cpp:269-280,601,688-690), so only slot zero starts it.
pub fn validate_entry(
    actor_slot: u8,
    participants: &[EntryParticipant],
    arena_occupied: bool,
    soul_core_count: u32,
) -> Result<(), SoulpitError> {
    if arena_occupied {
        return Err(SoulpitError::ArenaOccupied);
    }
    if soul_core_count == 0 {
        return Err(SoulpitError::NotEnoughItems);
    }
    if participants.is_empty() || participants.len() > MAX_PARTICIPANTS {
        return Err(SoulpitError::InvalidParticipants);
    }
    let mut slots = 0_u8;
    for participant in participants {
        if usize::from(participant.slot) >= MAX_PARTICIPANTS || slots & (1 << participant.slot) != 0
        {
            return Err(SoulpitError::InvalidParticipants);
        }
        slots |= 1 << participant.slot;
        if !participant.game_master && participant.level < REQUIRED_LEVEL {
            return Err(SoulpitError::LevelTooLow);
        }
    }
    if actor_slot != 0 || slots & 1 == 0 {
        return Err(SoulpitError::NotOnEntryTile);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum SoulpitStack {
    Normal = 1,
    Strong = 5,
    Powerful = 15,
    Boss = 40,
}

impl SoulpitStack {
    pub fn from_count(count: u8) -> Result<Self, SoulpitError> {
        match count {
            1 => Ok(Self::Normal),
            5 => Ok(Self::Strong),
            15 => Ok(Self::Powerful),
            40 => Ok(Self::Boss),
            _ => Err(SoulpitError::InvalidStack),
        }
    }

    pub const fn get(self) -> u8 {
        self as u8
    }

    /// monster.cpp:2752-2757 shares the Forge stack health curve.
    pub const fn health_percent(self) -> u16 {
        match self {
            Self::Normal => ForgeCreature::Influenced(1).health_percent(),
            Self::Strong => ForgeCreature::Influenced(5).health_percent(),
            Self::Powerful => ForgeCreature::Fiendish.health_percent(),
            Self::Boss => 135 + 15 * 40,
        }
    }

    /// monster.cpp:2672-2676: `1.35 + (stack - 1) * 0.1`.
    pub const fn attack_percent(self) -> u16 {
        135 + (self.get() as u16 - 1) * 10
    }

    pub fn scaled_health(self, base: u64) -> Result<u64, SoulpitError> {
        let scaled = u128::from(base) * u128::from(self.health_percent());
        u64::try_from(scaled.div_ceil(100)).map_err(|_| SoulpitError::Overflow)
    }

    /// monster.cpp:747-750,2341-2348: only stack 40, without a master, gives experience.
    pub fn experience(self, base: u64, is_summon: bool) -> Result<u64, SoulpitError> {
        if self == Self::Boss && !is_summon {
            base.checked_mul(28).ok_or(SoulpitError::Overflow)
        } else {
            Ok(0)
        }
    }
}

pub fn wave_stacks(wave: u8) -> Result<[SoulpitStack; MONSTERS_PER_WAVE], SoulpitError> {
    use SoulpitStack::{Boss, Normal, Powerful, Strong};
    match wave {
        1 => Ok([Normal; MONSTERS_PER_WAVE]),
        2 => Ok([Normal, Normal, Normal, Normal, Strong, Strong, Strong]),
        3 => Ok([Normal, Normal, Normal, Normal, Normal, Powerful, Powerful]),
        4 => Ok([Normal, Normal, Normal, Strong, Strong, Strong, Boss]),
        _ => Err(SoulpitError::InvalidWave),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WaveProgress {
    pub wave: u8,
    pub encounter_started_at_micros: u64,
    pub wave_started_at_micros: u64,
    pub now_micros: u64,
    pub spawn_applied: bool,
    /// Includes monster summons, as donor Zone:countMonsters does.
    pub monsters_alive: usize,
    pub participants: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WaveTransition {
    Waiting,
    Spawn([SoulpitStack; MONSTERS_PER_WAVE]),
    NextWave(u8),
    Completed,
    TimedOut,
    Abandoned,
}

/// soulpit_fight.lua:78-84,114-154; encounters.lua:37-60,286-295,312-316.
pub fn plan_wave(facts: WaveProgress) -> Result<WaveTransition, SoulpitError> {
    let stacks = wave_stacks(facts.wave)?;
    if facts.wave_started_at_micros < facts.encounter_started_at_micros
        || facts.now_micros < facts.wave_started_at_micros
    {
        return Err(SoulpitError::InvalidClock);
    }
    if usize::from(facts.participants) > MAX_PARTICIPANTS {
        return Err(SoulpitError::InvalidParticipants);
    }
    if facts.now_micros - facts.encounter_started_at_micros >= TIME_LIMIT_MICROS {
        return Ok(WaveTransition::TimedOut);
    }
    if facts.participants == 0 {
        return Ok(WaveTransition::Abandoned);
    }
    let wave_elapsed = facts.now_micros - facts.wave_started_at_micros;
    if !facts.spawn_applied {
        return Ok(if wave_elapsed >= SPAWN_DELAY_MICROS {
            WaveTransition::Spawn(stacks)
        } else {
            WaveTransition::Waiting
        });
    }
    if wave_elapsed < WAVE_CHECK_DELAY_MICROS || facts.monsters_alive != 0 {
        return Ok(WaveTransition::Waiting);
    }
    Ok(if facts.wave == WAVE_COUNT {
        WaveTransition::Completed
    } else {
        WaveTransition::NextWave(facts.wave + 1)
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BossAbility {
    Overpower,
    Enrage,
    Oppressor,
}

impl BossAbility {
    /// soulpit_fight.lua:127-140: uniformly one modifier for the stack-40 creature.
    pub fn from_roll(roll: u8) -> Result<Self, SoulpitError> {
        match roll {
            1 => Ok(Self::Overpower),
            2 => Ok(Self::Enrage),
            3 => Ok(Self::Oppressor),
            _ => Err(SoulpitError::InvalidRoll),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DamageOutcome {
    pub amount: u64,
    pub critical: bool,
}

fn validate_percent_roll(roll: u8) -> Result<(), SoulpitError> {
    if (1..=100).contains(&roll) {
        Ok(())
    } else {
        Err(SoulpitError::InvalidRoll)
    }
}

/// soulpit.lua:97-103; combat.cpp:2849-2865. Critical truncation precedes stack attack scaling.
pub fn overpower_damage(amount: u64, roll: u8) -> Result<DamageOutcome, SoulpitError> {
    validate_percent_roll(roll)?;
    let critical = roll <= 50;
    let amount = if critical {
        u64::try_from(u128::from(amount) * 125 / 100).map_err(|_| SoulpitError::Overflow)?
    } else {
        amount
    };
    Ok(DamageOutcome { amount, critical })
}

/// Preserves the literal donor's reversed final bound `{0.0,0.2}`: <=20% gets no reduction.
/// soulpit.lua:105-114; soulpit_creatureevents.lua:7-15.
pub fn enrage_retained_percent(health: u64, maximum: u64) -> Result<u8, SoulpitError> {
    if maximum == 0 || health > maximum {
        return Err(SoulpitError::InvalidHealth);
    }
    let scaled = u128::from(health) * 100;
    let maximum = u128::from(maximum);
    Ok(if scaled > maximum * 60 && scaled <= maximum * 80 {
        90
    } else if scaled > maximum * 40 && scaled <= maximum * 60 {
        75
    } else if scaled > maximum * 20 && scaled <= maximum * 40 {
        60
    } else {
        100
    })
}

// soulpit.lua:118-120; soulpit_{opressor,powerless,intensehex}.lua.
pub const OPPRESSOR_INTERVAL_MICROS: u64 = 2_000_000;
pub const OPPRESSOR_CONDITION_MICROS: u64 = 3_000_000;
pub const INTENSE_HEX_DAMAGE_DEALT_PERCENT: u8 = 50;
pub const INTENSE_HEX_HEALING_RECEIVED_PERCENT: u8 = 50;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OppressorSpell {
    RootAndFear,
    Powerless,
    IntenseHex,
}

impl OppressorSpell {
    pub fn triggers(self, roll: u8) -> Result<bool, SoulpitError> {
        validate_percent_roll(roll)?;
        let chance = match self {
            Self::RootAndFear => 25,
            Self::Powerless => 30,
            Self::IntenseHex => 15,
        };
        Ok(roll <= chance)
    }
}

#[cfg(test)]
mod tests;
