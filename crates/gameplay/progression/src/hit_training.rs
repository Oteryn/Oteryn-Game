//! W2b / SKILLS-0 §2, §3.4: the Canary rules that turn a resolved hit into skill tries. Pure and
//! runtime-only: the tries go into the live build training (`mana_training`), which commits them
//! with mana spent in one `training` build receipt (advance, 60 s checkpoint, logout).
//!
//! Reference evidence (`OTS_HYPOTHESIS_ONLY`, retained in
//! `imports/ots-source-evidence/item-fx-audio295/retained-source-files.json`):
//! - `Player::onAttackedCreatureBlockHit`: an unblocked hit grants the attack try and refills both
//!   streaks to 30; a defence- or armor-blocked hit grants it only while the blood streak lasts;
//!   any other block (immunity) grants nothing.
//! - `WeaponMelee::getSkillType`: 1 try when granted and the last block is not immunity; the
//!   fist attack (`Weapon::useFist`) adds 1 when granted.
//! - `WeaponDistance::getSkillType`: 2 tries after an unblocked hit, 1 after a blocked one.
//! - `Player::onBlockHit`: 1 shielding try per block while the shield streak lasts, and only
//!   with a shield.
//! - `fishing.lua`: 1 fishing try per use.
//!
//! Both streaks start at 0, as Canary's `bloodHitCount` and `shieldBlockCount` do.

/// The seven skills in the order of `character_build::SKILLS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillFamily {
    Fist,
    Club,
    Sword,
    Axe,
    Distance,
    Shielding,
    Fishing,
}

impl SkillFamily {
    pub const ALL: [Self; 7] = [
        Self::Fist,
        Self::Club,
        Self::Sword,
        Self::Axe,
        Self::Distance,
        Self::Shielding,
        Self::Fishing,
    ];
    /// The index into `DurableBuildState::skills`.
    pub const fn index(self) -> usize {
        match self {
            Self::Fist => 0,
            Self::Club => 1,
            Self::Sword => 2,
            Self::Axe => 3,
            Self::Distance => 4,
            Self::Shielding => 5,
            Self::Fishing => 6,
        }
    }
}

/// Canary `BlockType_t` of a resolved hit on the attacked creature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HitBlock {
    #[default]
    Unblocked,
    Defence,
    Armor,
    Immunity,
}

/// The number of hits a streak lasts (`bloodHitCount`, `shieldBlockCount`).
const STREAK: u8 = 30;
/// `fishing.lua`: one try per use.
pub const FISHING_TRIES_PER_USE: u64 = 1;

/// One player's runtime-only training streaks. Not durable: a login starts from the default, as
/// a Canary login does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HitTrainingCounters {
    blood_hits: u8,
    shield_blocks: u8,
    add_attack_skill: bool,
    last_block: HitBlock,
}

impl HitTrainingCounters {
    /// `Player::onAttackedCreatureBlockHit`.
    fn attacked_creature_block_hit(&mut self, block: HitBlock) {
        self.last_block = block;
        match block {
            HitBlock::Unblocked => {
                self.add_attack_skill = true;
                self.blood_hits = STREAK;
                self.shield_blocks = STREAK;
            }
            HitBlock::Defence | HitBlock::Armor => {
                self.add_attack_skill = self.blood_hits > 0;
                self.blood_hits = self.blood_hits.saturating_sub(1);
            }
            HitBlock::Immunity => self.add_attack_skill = false,
        }
    }

    /// A fist or melee weapon hit resolved with `block`: its tries for the swung family.
    pub fn melee_hit(&mut self, block: HitBlock) -> u64 {
        self.attacked_creature_block_hit(block);
        u64::from(self.add_attack_skill && self.last_block != HitBlock::Immunity)
    }

    /// A distance shot: `Some(block)` when it hit, `None` when it missed (a miss keeps the
    /// previous block state). Its distance tries.
    pub fn distance_shot(&mut self, hit: Option<HitBlock>) -> u64 {
        if let Some(block) = hit {
            self.attacked_creature_block_hit(block);
        }
        if !self.add_attack_skill {
            return 0;
        }
        match self.last_block {
            HitBlock::Unblocked => 2,
            HitBlock::Defence | HitBlock::Armor => 1,
            HitBlock::Immunity => 0,
        }
    }

    /// `Player::onBlockHit`: the player blocked a hit with its defence budget. Its shielding
    /// tries.
    pub fn block_hit(&mut self, has_shield: bool) -> u64 {
        if self.shield_blocks == 0 {
            return 0;
        }
        self.shield_blocks -= 1;
        u64::from(has_shield)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_hits_train_only_within_the_blood_streak() {
        let mut c = HitTrainingCounters::default();
        // Both streaks start empty: a blocked hit before any blood trains nothing.
        assert_eq!(c.melee_hit(HitBlock::Armor), 0);
        assert_eq!(c.melee_hit(HitBlock::Unblocked), 1);
        for _ in 0..30 {
            assert_eq!(c.melee_hit(HitBlock::Defence), 1);
        }
        assert_eq!(c.melee_hit(HitBlock::Armor), 0);
        assert_eq!(c.melee_hit(HitBlock::Unblocked), 1);
        assert_eq!(c.melee_hit(HitBlock::Armor), 1);
    }

    #[test]
    fn immunity_never_trains_and_clears_the_grant() {
        let mut c = HitTrainingCounters::default();
        assert_eq!(c.melee_hit(HitBlock::Unblocked), 1);
        assert_eq!(c.melee_hit(HitBlock::Immunity), 0);
        assert_eq!(c.distance_shot(None), 0);
    }

    #[test]
    fn distance_gives_two_unblocked_one_blocked_and_a_miss_repeats() {
        let mut c = HitTrainingCounters::default();
        assert_eq!(c.distance_shot(None), 0);
        assert_eq!(c.distance_shot(Some(HitBlock::Unblocked)), 2);
        assert_eq!(c.distance_shot(None), 2);
        assert_eq!(c.distance_shot(Some(HitBlock::Defence)), 1);
        assert_eq!(c.distance_shot(None), 1);
    }

    #[test]
    fn shielding_needs_a_shield_and_the_streak_refilled_by_blood() {
        let mut c = HitTrainingCounters::default();
        assert_eq!(c.block_hit(true), 0);
        c.melee_hit(HitBlock::Unblocked);
        // Without a shield the streak is still used up.
        assert_eq!(c.block_hit(false), 0);
        for _ in 0..29 {
            assert_eq!(c.block_hit(true), 1);
        }
        assert_eq!(c.block_hit(true), 0);
        c.melee_hit(HitBlock::Unblocked);
        assert_eq!(c.block_hit(true), 1);
    }

    #[test]
    fn family_indices_follow_the_build_column_order() {
        for (index, (family, name)) in SkillFamily::ALL
            .iter()
            .zip(oteryn_gameplay_domain::domain::character_build::SKILLS)
            .enumerate()
        {
            assert_eq!(family.index(), index, "{name}");
        }
    }
}
