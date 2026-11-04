//! Crystal grand_master_oberon.lua:125-136 and grand_master_oberon_functions.lua:43-60.

pub const OBERON_KEY: &str = "oteryn:creature.grand_master_oberon";
pub const OBERON_LIVES: u8 = 3;
pub const OBERON_HEAL_THRESHOLD_PERCENT: u64 = 20;
pub const OBERON_ASKING: [&str; 9] = [
    "You appear like a worm among men!",
    "The world will suffer for its iddle laziness!",
    "People fall at my feet when they see me coming!",
    "This will be the end of mortal man!",
    "I will remove you from this plane of existence!",
    "Dragons will soon rule this world, I am their herald!",
    "The true virtue of chivalry are my belief!",
    "I lead the most honourable and formidable following of knights!",
    "ULTAH SALID'AR, ESDO LO!",
];
pub const OBERON_SUMMONS: [&str; 2] = [
    "oteryn:creature.falcon_knight",
    "oteryn:creature.falcon_paladin",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OberonEffect {
    /// Register the OberonImmunity health-change visual (HOLYAREA) until a correct reply.
    RegisterImmunityEvent,
    /// Monster speech (TALKTYPE_MONSTER_SAY).
    Say {
        text: &'static str,
    },
    HealToFull,
    /// Created on Oberon's tile with extended, forced placement.
    Summon {
        creature_key: &'static str,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OberonRefusal {
    InvalidDraw,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OberonThinkOwner {
    lives_used: u8,
    asking: Option<u8>,
}

impl OberonThinkOwner {
    /// One think. `draw(n)` returns a value in `0..n` (refused otherwise); it is called first
    /// with 9 (taunt) and then with 2 (summon), only when the asking fires.
    pub fn think(
        &mut self,
        health: u64,
        max_health: u64,
        mut draw: impl FnMut(u32) -> u32,
    ) -> Result<Vec<OberonEffect>, OberonRefusal> {
        if self.lives_used >= OBERON_LIVES
            || max_health == 0
            || u128::from(health) * 100
                > u128::from(OBERON_HEAL_THRESHOLD_PERCENT) * u128::from(max_health)
        {
            return Ok(vec![]);
        }

        let taunt = draw(OBERON_ASKING.len() as u32);
        if taunt >= OBERON_ASKING.len() as u32 {
            return Err(OberonRefusal::InvalidDraw);
        }
        let summon = draw(OBERON_SUMMONS.len() as u32);
        if summon >= OBERON_SUMMONS.len() as u32 {
            return Err(OberonRefusal::InvalidDraw);
        }

        self.asking = Some(taunt as u8 + 1);
        self.lives_used += 1;

        Ok(vec![
            OberonEffect::RegisterImmunityEvent,
            OberonEffect::Say {
                text: OBERON_ASKING[taunt as usize],
            },
            OberonEffect::HealToFull,
            OberonEffect::Summon {
                creature_key: OBERON_SUMMONS[summon as usize],
            },
        ])
    }

    #[must_use]
    pub const fn lives_used(&self) -> u8 {
        self.lives_used
    }

    /// The donor `Asking` storage: 1-based index into OBERON_ASKING of the last taunt.
    #[must_use]
    pub const fn asking(&self) -> Option<u8> {
        self.asking
    }
}

#[cfg(test)]
#[path = "oberon_think_tests.rs"]
mod tests;
