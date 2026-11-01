//! Crystal 201189f `potions.lua:1-31,52-54,63,125-129`: four self skill conditions.
//! Q4/Q6 in PR #2782 explicitly corrects the unreachable achievement branch at :115-122.
use oteryn_gameplay_domain::domain::condition::ConditionValues;

use crate::item_use_achievement::Counted;

pub const DURATION_MS: u32 = 10 * 60 * 1_000;
/// The donor passes undefined bare JeanPierre* globals to lua_tonumber: nil becomes zero.
/// lua_enums.cpp:479-483 exports only AttrSubId_*; lua_functions_loader.hpp:74-88 is unchecked.
/// All four therefore replace the same slot. Using the intended 4/5/6 would change the donor.
pub const SUB_ID: u8 = 0;
pub const DEFINITION_REVISION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionPotion {
    Berserk,
    Mastermind,
    Bullseye,
    Transcendence,
}

impl ConditionPotion {
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "oteryn:item.tibia.i7439" => Some(Self::Berserk),
            "oteryn:item.tibia.i7440" => Some(Self::Mastermind),
            "oteryn:item.tibia.i7443" => Some(Self::Bullseye),
            "oteryn:item.tibia.i49271" => Some(Self::Transcendence),
            _ => None,
        }
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::Berserk => "oteryn:item.tibia.i7439",
            Self::Mastermind => "oteryn:item.tibia.i7440",
            Self::Bullseye => "oteryn:item.tibia.i7443",
            Self::Transcendence => "oteryn:item.tibia.i49271",
        }
    }

    pub fn allows_base_vocation(self, vocation: &str) -> bool {
        match self {
            Self::Berserk => vocation == "knight",
            Self::Mastermind => matches!(vocation, "sorcerer" | "druid"),
            Self::Bullseye => vocation == "paladin",
            // The donor has no vocation restriction for Transcendence.
            Self::Transcendence => matches!(
                vocation,
                "none" | "sorcerer" | "druid" | "knight" | "paladin" | "monk"
            ),
        }
    }

    pub const fn values(self) -> ConditionValues {
        let (magic_level, fist, melee, distance, shielding) = match self {
            Self::Berserk => (0, 0, 5, 0, -10),
            Self::Mastermind => (3, 0, 0, 0, 0),
            Self::Bullseye => (0, 0, 0, 5, -10),
            Self::Transcendence => (0, 4, 0, 0, 0),
        };
        ConditionValues::SpellSkills {
            duration_ms: DURATION_MS,
            sub_id: SUB_ID,
            magic_level,
            fist,
            melee,
            distance,
            shielding,
        }
    }

    pub const fn achievement(self) -> Counted {
        Counted {
            key: match self {
                Self::Berserk => "oteryn:achievement/berserker",
                Self::Mastermind => "oteryn:achievement/mastermind",
                Self::Bullseye => "oteryn:achievement/sharpshooter",
                Self::Transcendence => "oteryn:achievement/inner_peace",
            },
            total: 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condition_potions_source_profiles_and_earnable_keys()
    -> Result<(), crate::achievement_catalogue::AchievementCatalogueError> {
        let catalogue =
            crate::achievement_catalogue::parse_shards(&crate::achievement_catalogue::SHARDS)?;
        for (potion, expected) in [
            (ConditionPotion::Berserk, (0, 0, 5, 0, -10)),
            (ConditionPotion::Mastermind, (3, 0, 0, 0, 0)),
            (ConditionPotion::Bullseye, (0, 0, 0, 5, -10)),
            (ConditionPotion::Transcendence, (0, 4, 0, 0, 0)),
        ] {
            assert_eq!(ConditionPotion::from_key(potion.key()), Some(potion));
            assert_eq!(
                potion.values(),
                ConditionValues::SpellSkills {
                    duration_ms: 600_000,
                    sub_id: 0,
                    magic_level: expected.0,
                    fist: expected.1,
                    melee: expected.2,
                    distance: expected.3,
                    shielding: expected.4,
                }
            );
            let counted = potion.achievement();
            assert_eq!(counted.total, 100);
            assert!(
                catalogue
                    .get(counted.key)
                    .is_some_and(|entry| !entry.retired)
            );
        }
        assert_eq!(ConditionPotion::from_key("oteryn:item.tibia.i266"), None);
        assert!(ConditionPotion::Berserk.allows_base_vocation("knight"));
        assert!(!ConditionPotion::Berserk.allows_base_vocation("monk"));
        assert!(ConditionPotion::Mastermind.allows_base_vocation("druid"));
        assert!(!ConditionPotion::Mastermind.allows_base_vocation("knight"));
        assert!(ConditionPotion::Bullseye.allows_base_vocation("paladin"));
        assert!(ConditionPotion::Transcendence.allows_base_vocation("none"));
        assert!(ConditionPotion::Transcendence.allows_base_vocation("knight"));
        Ok(())
    }
}
