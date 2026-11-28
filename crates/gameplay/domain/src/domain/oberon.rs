//! Grand Master Oberon's debate, from the Tibia Fandom archive.
//! Grand_Master_Oberon (2026-01-06) and The_Secret_Library_Quest/Spoiler (2026-07-07).

pub const OBERON_KEY: &str = "oteryn:creature.grand_master_oberon";
pub const OBERON_LIVES: u8 = 3;
pub const OBERON_HEAL_THRESHOLD_PERCENT: i64 = 10;
pub const OBERON_DEBATE: [(&str, &str); 9] = [
    (
        "You appear like a worm among men!",
        "How appropriate, you look like something worms already got the better of!",
    ),
    (
        "The world will suffer for its idle laziness!",
        "Are you ever going to fight or do you prefer talking!",
    ),
    (
        "People fall at my feet when they see me coming!",
        "Even before they smell your breath?",
    ),
    (
        "This will be the end of mortal men!",
        "Then let me show you the concept of mortality before it!",
    ),
    (
        "I will remove you from this plane of existence!",
        "Too bad you barely exist at all!",
    ),
    (
        "Dragons will soon rule this world, I am their herald!",
        "Excuse me but I still do not get the message!",
    ),
    (
        "The true virtues of chivalry are my belief!",
        "Dare strike up a Minnesang and you will receive your last accolade!",
    ),
    (
        "I lead the most honourable and formidable following of knights!",
        "Then why are we fighting alone right now?",
    ),
    ("ULTAH SALID'AR, ESDO LO!", "SEHWO ASIMO, TOLIDO ESD"),
];
pub const OBERON_SUMMONS: [&str; 2] = [
    "oteryn:creature.falcon_knight",
    "oteryn:creature.falcon_paladin",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OberonPhase {
    pub taunt: &'static str,
    pub summon: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OberonRefusal {
    InvalidDraw,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OberonThinkOwner {
    lives_used: u8,
    asking: Option<usize>,
    asked: u16,
    last_phase_sequence: u64,
}

impl OberonThinkOwner {
    fn threshold(maximum: i64) -> i64 {
        (maximum / (100 / OBERON_HEAL_THRESHOLD_PERCENT)).max(1)
    }

    pub fn phase_due(self, sequence: u64, health: i64, maximum: i64) -> bool {
        sequence > self.last_phase_sequence
            && self.asking.is_none()
            && self.lives_used < OBERON_LIVES
            && health > 0
            && maximum > 0
            && health <= Self::threshold(maximum)
    }

    /// Preserve red HP until the reserved think starts the next debate.
    /// Both direct hits and periodic damage use the same phase guard.
    pub fn limit_damage(self, health: i64, maximum: i64, damage: i64) -> i64 {
        if self.asking.is_some() {
            0
        } else if self.lives_used < OBERON_LIVES {
            damage.min(health.saturating_sub(Self::threshold(maximum)).max(0))
        } else {
            damage
        }
    }

    pub fn think(
        &mut self,
        sequence: u64,
        health: i64,
        maximum: i64,
        mut draw: impl FnMut(u32) -> u32,
    ) -> Result<Option<OberonPhase>, OberonRefusal> {
        if !self.phase_due(sequence, health, maximum) {
            return Ok(None);
        }
        let count = OBERON_DEBATE.len() as u32 - self.asked.count_ones();
        let taunt = (0..OBERON_DEBATE.len())
            .filter(|index| self.asked & (1 << index) == 0)
            .nth(draw(count) as usize)
            .ok_or(OberonRefusal::InvalidDraw)?;
        let summon = OBERON_SUMMONS
            .get(draw(OBERON_SUMMONS.len() as u32) as usize)
            .ok_or(OberonRefusal::InvalidDraw)?;
        self.asking = Some(taunt);
        self.asked |= 1 << taunt;
        self.lives_used += 1;
        self.last_phase_sequence = sequence;
        Ok(Some(OberonPhase {
            taunt: OBERON_DEBATE[taunt].0,
            summon,
        }))
    }

    pub fn reply(&mut self, text: &str) -> Option<&'static str> {
        let (_, reply) = OBERON_DEBATE[self.asking?];
        if text
            .to_ascii_lowercase()
            .contains(&reply.to_ascii_lowercase())
        {
            self.asking = None;
            Some("GRRRAAANNGH!")
        } else {
            Some("HAHAHAHA!")
        }
    }
}
