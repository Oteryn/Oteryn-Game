//! Item spell augments, separate from Wheel allocations. Values use hundredths of a
//! percent, except Cooldown (milliseconds). Source import records the unit conversion.
use serde::{Deserialize, Serialize};

pub const MAX_ITEM_SPELL_AUGMENTS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemSpellAugmentKind {
    BaseDamage,
    BaseHealing,
    CriticalHitChance,
    CriticalExtraDamage,
    ManaLeech,
    LifeLeech,
    Cooldown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemSpellAugment {
    /// Case-folded source spell name; never a substring, words or a Wheel perk.
    pub spell: String,
    pub kind: ItemSpellAugmentKind,
    pub value: u32,
}
