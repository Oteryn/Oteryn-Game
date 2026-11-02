//! Cook selection and gourmet effects from the pinned Crystal source.

/// Crystal `data-global/npc/hireling.lua:2102-2103,2231-2273`.
pub const SURPRISE_PRICE: u64 = 15_000;
pub const SPECIFIC_PRICE: u64 = 90_000;
/// Crystal `data/scripts/actions/items/hireling_foods.lua:1-23,60`.
pub const GOURMET_COOLDOWN_SECONDS: u32 = 600;
pub const GOURMET_BUFF_SECONDS: u32 = 3_600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoodPlan {
    pub item_id: u32,
    pub gold: u64,
}

/// `roll` is a uniform zero-based server roll. The caller retains its selected plan on retry.
pub const fn surprise(roll: u8) -> Option<Option<FoodPlan>> {
    match roll {
        0..=4 => Some(Some(FoodPlan {
            item_id: 29_412 + roll as u32,
            gold: SURPRISE_PRICE,
        })),
        5 => Some(None),
        _ => None,
    }
}

/// The donor's plain `yes` path rolls only among the five recovery dishes.
pub const fn ordinary(roll: u8) -> Option<FoodPlan> {
    match surprise(roll) {
        Some(Some(plan)) => Some(plan),
        _ => None,
    }
}

pub fn skill_dish(keyword: &str) -> Option<FoodPlan> {
    // PR2783 ANSWER6548035915 (Q10=B): correct the donor distance typo.
    // hireling.lua:48 says toolbox35173; the registered distance food is29409
    // (hireling_foods.lua:26-27; items.xml:65742 identifies35173 as a toolbox).
    let item_id = match keyword {
        "magic" => 29_410,
        "melee" => 29_411,
        "shielding" => 29_408,
        "distance" => 29_409,
        _ => return None,
    };
    Some(FoodPlan {
        item_id,
        gold: SURPRISE_PRICE,
    })
}

pub fn specific(keyword: &str) -> Option<FoodPlan> {
    let item_id = match keyword {
        "chilli con carniphila" => 29_412,
        "svargrond salmon filet" => 29_413,
        "carrion casserole" => 29_414,
        "consecrated beef" => 29_415,
        "roasted wyvern wings" => 29_408,
        "carrot pie" => 29_409,
        "tropical marinated tiger" => 29_410,
        "delicatessen salad" => 29_411,
        _ => return None,
    };
    Some(FoodPlan {
        item_id,
        gold: SPECIFIC_PRICE,
    })
}

/// Runtime effects, not additional item definitions. Source: hireling_foods.lua:1-35,51-55.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GourmetEffect {
    Shielding(u16),
    Distance(u16),
    Magic(u16),
    Melee(u16),
    Speed(u16),
    Fishing(u16),
    RestoreHealthPercent(u8),
    RestoreManaPercent(u8),
    None,
}

pub const fn gourmet_effect(item_id: u32) -> Option<GourmetEffect> {
    Some(match item_id {
        29_408 => GourmetEffect::Shielding(7),
        29_409 => GourmetEffect::Distance(7),
        29_410 => GourmetEffect::Magic(3),
        29_411 => GourmetEffect::Melee(3),
        29_412 => GourmetEffect::Speed(80),
        29_413 => GourmetEffect::Fishing(30),
        29_414 => GourmetEffect::RestoreHealthPercent(30),
        29_415 => GourmetEffect::RestoreManaPercent(30),
        29_416 => GourmetEffect::None,
        _ => return None,
    })
}

/// A persistent default condition snapshot: remaining online ticks, not an
/// absolute wall-clock expiry. Crystal condition.cpp:175,356-369.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GourmetBuff {
    pub item_id: u32,
    pub remaining_ms: u32,
}
impl GourmetBuff {
    pub const fn valid(self) -> bool {
        self.remaining_ms > 0
            && self.remaining_ms <= GOURMET_BUFF_SECONDS * 1000
            && matches!(self.item_id, 29408..=29413)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GourmetConditions {
    pub attributes: Option<GourmetBuff>,
    pub haste: Option<GourmetBuff>,
}
impl GourmetConditions {
    pub fn valid(self) -> bool {
        self.attributes
            .is_none_or(|b| b.valid() && b.item_id != 29412)
            && self.haste.is_none_or(|b| b.valid() && b.item_id == 29412)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surprise_has_five_dishes_and_one_skill_choice_without_invalid_roll_aliases() {
        let meals: Vec<_> = (0..6).filter_map(surprise).collect();
        assert_eq!(
            meals
                .iter()
                .flatten()
                .map(|p| p.item_id)
                .collect::<Vec<_>>(),
            vec![29412, 29413, 29414, 29415, 29416]
        );
        assert_eq!(meals.iter().filter(|p| p.is_none()).count(), 1);
        assert!(meals.iter().flatten().all(|p| p.gold == 15000));
        assert_eq!(surprise(6), None);
        assert_eq!(ordinary(5), None);
    }

    #[test]
    fn specific_prices_and_approved_distance_source_correction_are_preserved() {
        for name in [
            "chilli con carniphila",
            "svargrond salmon filet",
            "carrion casserole",
            "consecrated beef",
            "roasted wyvern wings",
            "carrot pie",
            "tropical marinated tiger",
            "delicatessen salad",
        ] {
            assert_eq!(specific(name).map(|p| p.gold), Some(90000));
        }
        assert_eq!(
            skill_dish("distance"),
            Some(FoodPlan {
                item_id: 29409,
                gold: 15000
            })
        );
        assert_eq!(specific("carrot pie").map(|p| p.item_id), Some(29409));
        assert_eq!(specific("burnt dish"), None);
        assert_eq!(skill_dish("fishing"), None);
    }

    #[test]
    fn gourmet_rules_cover_only_the_registered_donor_items() {
        assert!((29408..=29416).all(|id| gourmet_effect(id).is_some()));
        assert_eq!(gourmet_effect(35173), None);
        assert_eq!(
            gourmet_effect(29414),
            Some(GourmetEffect::RestoreHealthPercent(30))
        );
        assert_eq!(gourmet_effect(29416), Some(GourmetEffect::None));
        assert_eq!(GOURMET_COOLDOWN_SECONDS, 600);
        assert_eq!(GOURMET_BUFF_SECONDS, 3600);
    }
}
