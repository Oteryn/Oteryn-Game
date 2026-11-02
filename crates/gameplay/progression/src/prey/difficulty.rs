//! PREY-DIFF (owner decision 2026-11-01). Tuning comes from generated content.
use super::{HUNTING_SECONDS, ListReroll, PreyRefusal, RerollQuote};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Safety bound shared with the bounded wire and storage projection, not a list size.
pub const MAX_LIST_SIZE: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreyDefenseMode {
    ReceivedDamage,
    DealtOrAttacked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreyActivityKind {
    DealtDamage,
    ReceivedDamage,
    Attacked,
}

impl PreyDefenseMode {
    pub fn consumes(self, bonus: u8, activity: PreyActivityKind) -> bool {
        match bonus {
            1 | 3 | 4 => activity == PreyActivityKind::DealtDamage,
            2 => match self {
                Self::ReceivedDamage => activity == PreyActivityKind::ReceivedDamage,
                Self::DealtOrAttacked => true,
            },
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreyCreatureValue {
    pub tier: u8,
    pub loot_per_kill_micros: u64,
    pub kills_per_hour_micros: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreyRules {
    pub schema_version: u8,
    pub list_size: u8,
    pub expanded_list_size: u8,
    pub minimum_levels: [u32; 5],
    pub pick_again_list_multiplier_ppm: u32,
    pub pick_again_value_factor_ppm: u32,
    pub defense_mode: PreyDefenseMode,
    pub creatures: BTreeMap<String, PreyCreatureValue>,
    pub npc_sell_prices: BTreeMap<String, u64>,
}

impl PreyRules {
    pub fn validate(&self) -> Result<(), PreyRefusal> {
        if self.schema_version != 1
            || self.list_size == 0
            || self.expanded_list_size < self.list_size
            || usize::from(self.expanded_list_size) > MAX_LIST_SIZE
            || self.minimum_levels[0] != 1
            || !self.minimum_levels.windows(2).all(|v| v[0] <= v[1])
            || self.creatures.is_empty()
            || self
                .creatures
                .iter()
                .any(|(key, value)| !key.starts_with("oteryn:") || !(1..=5).contains(&value.tier))
            || self.npc_sell_prices.values().any(|v| *v > i64::MAX as u64)
        {
            return Err(PreyRefusal::InvalidConfiguration);
        }
        Ok(())
    }

    pub fn highest_allowed_tier(&self, level: u32) -> Result<u8, PreyRefusal> {
        if level == 0 {
            return Err(PreyRefusal::InvalidLevel);
        }
        self.minimum_levels
            .iter()
            .rposition(|minimum| level >= *minimum)
            .map(|index| (index + 1) as u8)
            .ok_or(PreyRefusal::InvalidLevel)
    }

    pub fn check_choice(&self, tier: u8, level: u32) -> Result<(), PreyRefusal> {
        if !(1..=5).contains(&tier) {
            return Err(PreyRefusal::InvalidTier);
        }
        if tier > self.highest_allowed_tier(level)? {
            return Err(PreyRefusal::TierLocked);
        }
        Ok(())
    }

    /// Zero is only the pre-migration/unselected state. A level loss keeps the
    /// stored choice and caps this draw, like Task Board difficulty.
    pub fn effective_tier(&self, chosen: u8, level: u32) -> Result<u8, PreyRefusal> {
        let allowed = self.highest_allowed_tier(level)?;
        match chosen {
            0 => Ok(allowed),
            1..=5 => Ok(chosen.min(allowed)),
            _ => Err(PreyRefusal::InvalidTier),
        }
    }

    pub fn list_size(&self, expansion: bool) -> usize {
        usize::from(if expansion {
            self.expanded_list_size
        } else {
            self.list_size
        })
    }

    /// Uses the existing paid-list quote and exact fixed-point arithmetic.
    /// Integer truncation matches the nonnegative whole-gold list price.
    pub fn pick_again_price(&self, level: u32, creature: &str) -> Result<u64, PreyRefusal> {
        let list = RerollQuote::try_from(ListReroll {
            now: 0,
            free_reroll_at: 1,
            level,
        })?;
        let value = self
            .creatures
            .get(creature)
            .ok_or(PreyRefusal::InvalidCreature)?;
        let list_price = u128::from(list.gold)
            .checked_mul(u128::from(self.pick_again_list_multiplier_ppm))
            .ok_or(PreyRefusal::InvalidConfiguration)?
            / 1_000_000;
        let slot_price = u128::from(value.loot_per_kill_micros)
            .checked_mul(u128::from(value.kills_per_hour_micros))
            .and_then(|v| v.checked_mul(u128::from(HUNTING_SECONDS)))
            .and_then(|v| v.checked_mul(u128::from(self.pick_again_value_factor_ppm)))
            .ok_or(PreyRefusal::InvalidConfiguration)?
            / (1_000_000_u128 * 1_000_000 * 3_600 * 1_000_000);
        u64::try_from(list_price.max(slot_price))
            .ok()
            .filter(|price| *price <= i64::MAX as u64)
            .ok_or(PreyRefusal::InvalidConfiguration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> PreyRules {
        PreyRules {
            schema_version: 1,
            list_size: 9,
            expanded_list_size: 12,
            minimum_levels: [1, 50, 100, 300, 600],
            pick_again_list_multiplier_ppm: 2_000_000,
            pick_again_value_factor_ppm: 300_000,
            defense_mode: PreyDefenseMode::DealtOrAttacked,
            creatures: BTreeMap::from([
                (
                    "oteryn:creature.cheap".into(),
                    PreyCreatureValue {
                        tier: 1,
                        loot_per_kill_micros: 1_000_000,
                        kills_per_hour_micros: 10_000_000,
                    },
                ),
                (
                    "oteryn:creature.expensive".into(),
                    PreyCreatureValue {
                        tier: 5,
                        loot_per_kill_micros: 100_000_000_000,
                        kills_per_hour_micros: 600_000_000,
                    },
                ),
            ]),
            npc_sell_prices: BTreeMap::new(),
        }
    }

    #[test]
    fn prey_difficulty_choice_refuses_invalid_and_locked_tiers_and_preserves_level_loss()
    -> Result<(), PreyRefusal> {
        let rules = rules();
        rules.validate()?;
        assert_eq!(rules.check_choice(0, 100), Err(PreyRefusal::InvalidTier));
        assert_eq!(rules.check_choice(6, 100), Err(PreyRefusal::InvalidTier));
        assert_eq!(rules.check_choice(4, 100), Err(PreyRefusal::TierLocked));
        assert_eq!(rules.check_choice(3, 100), Ok(()));
        assert_eq!(rules.effective_tier(0, 100)?, 3);
        assert_eq!(rules.effective_tier(5, 100)?, 3);
        assert_eq!(rules.effective_tier(5, 600)?, 5);
        assert_eq!(rules.list_size(false), 9);
        assert_eq!(rules.list_size(true), 12);
        Ok(())
    }

    #[test]
    fn prey_pick_again_price_uses_existing_paid_price_and_configured_slot_value()
    -> Result<(), PreyRefusal> {
        let mut rules = rules();
        for level in [1, 50, 300, 1000] {
            let list = RerollQuote::try_from(ListReroll {
                now: 0,
                free_reroll_at: 1,
                level,
            })?;
            assert_eq!(
                rules.pick_again_price(level, "oteryn:creature.cheap")?,
                list.gold * 2
            );
            assert_eq!(
                rules.pick_again_price(level, "oteryn:creature.expensive")?,
                36_000_000
            );
        }
        rules.pick_again_list_multiplier_ppm = 3_000_000;
        rules.pick_again_value_factor_ppm = 500_000;
        let list = RerollQuote::try_from(ListReroll {
            now: 0,
            free_reroll_at: 1,
            level: 200,
        })?;
        assert_eq!(
            rules.pick_again_price(200, "oteryn:creature.cheap")?,
            list.gold * 3
        );
        assert_eq!(
            rules.pick_again_price(200, "oteryn:creature.expensive")?,
            60_000_000
        );
        rules
            .creatures
            .get_mut("oteryn:creature.expensive")
            .ok_or(PreyRefusal::InvalidCreature)?
            .loot_per_kill_micros = u64::MAX;
        rules
            .creatures
            .get_mut("oteryn:creature.expensive")
            .ok_or(PreyRefusal::InvalidCreature)?
            .kills_per_hour_micros = u64::MAX;
        assert_eq!(
            rules.pick_again_price(200, "oteryn:creature.expensive"),
            Err(PreyRefusal::InvalidConfiguration)
        );
        Ok(())
    }

    #[test]
    fn prey_activity_uses_offensive_and_two_defense_rules() {
        for mode in [
            PreyDefenseMode::ReceivedDamage,
            PreyDefenseMode::DealtOrAttacked,
        ] {
            for bonus in [1, 3, 4] {
                assert!(mode.consumes(bonus, PreyActivityKind::DealtDamage));
                assert!(!mode.consumes(bonus, PreyActivityKind::ReceivedDamage));
                assert!(!mode.consumes(bonus, PreyActivityKind::Attacked));
            }
            assert!(mode.consumes(2, PreyActivityKind::ReceivedDamage));
        }
        assert!(!PreyDefenseMode::ReceivedDamage.consumes(2, PreyActivityKind::Attacked));
        assert!(!PreyDefenseMode::ReceivedDamage.consumes(2, PreyActivityKind::DealtDamage));
        assert!(PreyDefenseMode::DealtOrAttacked.consumes(2, PreyActivityKind::Attacked));
        assert!(PreyDefenseMode::DealtOrAttacked.consumes(2, PreyActivityKind::DealtDamage));
    }
}
