//! Echo Warden rules from Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
//!
//! These preserve that donor's encounter variant: every activated portal creates a Warden
//! and seven to twelve minions. Retail has additional raid outcomes; their unsourced selection
//! odds are not inferred here. The owner supplies random draws, live creature facts and time.

/// `data-global/lib/core/echo_warden.lua:22-41`.
pub const PORTAL_ITEM_ID: u16 = 54_133;
pub const PORTAL_DELAY_MS: u64 = 30_000;
pub const PORTAL_TTL_MS: u64 = 120_000;
pub const SPAWN_CHANCE_NUM: u32 = 100;
pub const SPAWN_CHANCE_DEN: u32 = 200_000;
pub const WARDEN_ADDS_MIN: u8 = 7;
pub const WARDEN_ADDS_MAX: u8 = 12;
pub const AURA_RANGE: u16 = 5;
pub const AURA_INTERVAL_MS: u64 = 2_000;
pub const SPAWN_STEP_MS: u64 = 400;
/// `data-global/scripts/custom/echo_warden/ondroploot_echo_warden.lua:10,22-24`.
pub const BASIC_SCROLL_MIN: u16 = 53_751;
pub const BASIC_SCROLL_MAX: u16 = 53_774;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EchoWardenError {
    InvalidDraw,
}

impl std::fmt::Display for EchoWardenError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Echo Warden draw is outside its donor range")
    }
}

impl std::error::Error for EchoWardenError {}

/// Facts captured from a dying monster, not supplied by a client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EchoDeathFacts {
    pub is_warden: bool,
    pub is_raid_spawned: bool,
    pub is_summon: bool,
    pub is_reward_boss: bool,
    pub has_boss_race: bool,
    pub forge_stacks: u16,
    pub bestiary_stars: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EchoDeathEffect {
    None,
    /// Grant through the Charm owner once per character/species, to every distinct live
    /// player in the damage map, including the player master of a summon (trigger:12-39).
    FirstSpeciesReward,
    SchedulePortal,
}

/// `echo_warden_trigger.lua:42-90`: tagged Wardens reward before the exclusion checks.
/// The source uses difficulty stars 1-2, despite calling them common/uncommon in its comments.
/// Soulpit is not excluded here; the C++ Warden modifier has a separate Soulpit guard.
pub fn death_effect(
    facts: EchoDeathFacts,
    portal_draw: u32,
) -> Result<EchoDeathEffect, EchoWardenError> {
    if facts.is_warden {
        return Ok(EchoDeathEffect::FirstSpeciesReward);
    }
    if facts.is_raid_spawned
        || facts.is_summon
        || facts.is_reward_boss
        || facts.has_boss_race
        || facts.forge_stacks > 0
        || !(1..=2).contains(&facts.bestiary_stars)
    {
        return Ok(EchoDeathEffect::None);
    }
    if !(1..=SPAWN_CHANCE_DEN).contains(&portal_draw) {
        return Err(EchoWardenError::InvalidDraw);
    }
    Ok(if portal_draw <= SPAWN_CHANCE_NUM {
        EchoDeathEffect::SchedulePortal
    } else {
        EchoDeathEffect::None
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EchoPortalTiming {
    pub appears_at_ms: u64,
    pub expires_at_ms: u64,
}

impl EchoPortalTiming {
    #[must_use]
    pub const fn is_visible_at(self, now_ms: u64) -> bool {
        now_ms >= self.appears_at_ms && now_ms < self.expires_at_ms
    }
}

/// The trigger waits 30 seconds; the portal then lives for two minutes
/// (`echo_warden_trigger.lua:90`, `echo_warden.lua:203-224`).
#[must_use]
pub fn portal_timing(death_at_ms: u64) -> Option<EchoPortalTiming> {
    let appears_at_ms = death_at_ms.checked_add(PORTAL_DELAY_MS)?;
    Some(EchoPortalTiming {
        appears_at_ms,
        expires_at_ms: appears_at_ms.checked_add(PORTAL_TTL_MS)?,
    })
}

/// A checked draw for `echo_warden.lua:227-241`. All spawns use the portal's tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EchoRaidPlan {
    minion_count: u8,
}

impl EchoRaidPlan {
    pub fn new(minion_draw: u8) -> Result<Self, EchoWardenError> {
        if !(WARDEN_ADDS_MIN..=WARDEN_ADDS_MAX).contains(&minion_draw) {
            return Err(EchoWardenError::InvalidDraw);
        }
        Ok(Self {
            minion_count: minion_draw,
        })
    }

    #[must_use]
    pub const fn minion_count(self) -> u8 {
        self.minion_count
    }

    #[must_use]
    pub const fn spawn_count(self) -> u8 {
        self.minion_count + 1
    }

    #[must_use]
    pub const fn variant_at(self, index: u8) -> Option<EchoVariant> {
        if index == 0 {
            Some(EchoVariant::Warden)
        } else if index <= self.minion_count {
            Some(EchoVariant::Minion)
        } else {
            None
        }
    }

    /// The Warden and first minion are immediate; subsequent minions are 400 ms apart
    /// (`echo_warden.lua:192-200,234-241`). A failed spawn still consumes that slot.
    #[must_use]
    pub const fn spawn_delay_ms(self, index: u8) -> Option<u64> {
        if index > self.minion_count {
            return None;
        }
        Some(index.saturating_sub(1) as u64 * SPAWN_STEP_MS)
    }
}

/// C++ protects variant application independently of the Lua death trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EchoWardenFacts {
    pub already_warden: bool,
    pub is_summon: bool,
    pub is_boss: bool,
    pub forge_stacks: u16,
    pub is_soulpit: bool,
}

/// `src/creatures/monsters/monster.cpp:2683-2694`.
#[must_use]
pub const fn can_apply_warden(facts: EchoWardenFacts) -> bool {
    !facts.already_warden
        && !facts.is_summon
        && !facts.is_boss
        && facts.forge_stacks == 0
        && !facts.is_soulpit
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EchoVariant {
    Warden,
    Minion,
}

impl EchoVariant {
    /// Apply once when assigning the variant, and fill current health to the result.
    /// Warden: `monster.cpp:2699-2701`; minion: `echo_warden.lua:80-89`.
    /// The Warden also scales its run-away threshold by the same factor.
    #[must_use]
    pub fn health(self, base: u64) -> Option<u64> {
        match self {
            Self::Warden => base.checked_mul(4),
            Self::Minion => u64::try_from(u128::from(base) * 3 / 2).ok(),
        }
    }

    /// `monster.cpp:2672-2680`, `echo_warden.lua:32-33,80-89`. Minions receive no attack boost.
    #[must_use]
    pub const fn attack_percent(self) -> u16 {
        match self {
            Self::Warden => 150,
            Self::Minion => 100,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EchoAuraFacts {
    pub same_species: bool,
    pub is_warden: bool,
    pub same_floor: bool,
    pub delta_x: u16,
    pub delta_y: u16,
}

/// `echo_warden.lua:175-189`: the aura can enlist naturally spawned creatures as minions.
/// Already-enlisted minions keep their tag and health; refreshing the aura never stacks it.
#[must_use]
pub const fn aura_can_enlist(facts: EchoAuraFacts) -> bool {
    facts.same_species
        && !facts.is_warden
        && facts.same_floor
        && facts.delta_x <= AURA_RANGE
        && facts.delta_y <= AURA_RANGE
}

/// First-species major Charm Points (`src/io/iobestiary.cpp:422-445`). The existing Charm
/// owner records first-kill custody and supplies these points to its derived currency balance.
#[must_use]
pub const fn first_species_charm_points(bestiary_stars: u8) -> u32 {
    match bestiary_stars {
        0 => 1,
        1 => 2,
        2 => 5,
        3 => 10,
        4 => 15,
        5 => 30,
        _ => 0,
    }
}

/// Validate the uniformly drawn scroll ID before minting exactly one into the Warden corpse
/// (`ondroploot_echo_warden.lua:14-24`). There is no catalyst or dust rule in this donor callback.
#[must_use]
pub fn basic_scroll_item_id(draw: u16) -> Option<u16> {
    (BASIC_SCROLL_MIN..=BASIC_SCROLL_MAX)
        .contains(&draw)
        .then_some(draw)
}

#[cfg(test)]
#[path = "echo_warden/tests.rs"]
mod tests;
