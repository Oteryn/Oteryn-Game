//! IMBUE-FORGE0 §3/§4/§6 rules. These are data, not access or mutation authority.
//!
//! D1163 allows every tier on every slot-bearing item. Content must still admit
//! the category, and shrine use independently requires quest and Premium checks.
//! The reference content tier enum (Two/Three/Ten) is not a runtime tier codec.

/// Twenty hours of eligible equipped time, never wall-clock/offline time.
/// Reference: `docs/reference/tibia-manual/characters.md` §5.1.8.
pub const IMBUEMENT_DURATION_MS: u64 = 72_000_000;
/// Reference: `docs/reference/tibia-manual/characters.md` §5.1.8.
pub const IMBUEMENT_SLOTS_MAX: u8 = 3;
/// Accepted accounting rule IMBUE-FORGE0 §4.3/R1.
pub const IMBUEMENT_CHECKPOINT_INTERVAL_MS: u64 = 60_000;

/// Speed and capacity consume while equipped and online (TibiaWiki, Imbuing).
/// Combat categories follow the fight state, including a fight hidden by a PZ.
/// Paralysis deflection keeps pinned Canary's non-aggressive classification
/// (47dfd51f, imbuements.xml category 19); not verified against Global.
pub fn consumes_elapsed(category: u8, in_combat: bool) -> bool {
    matches!(category, 8 | 9 | 12 | 25) || in_combat
}
// Single-slot shrine removal; the 30,000-gold cleansing item in news 8396 is separate.
// Reference: imports/canary/imbuements-xml/imbuements.xml (47dfd51f).
pub const IMBUEMENT_CLEAR_GOLD: u64 = 15_000;

/// Global item ids used by the item lifecycle.  These are deliberately kept beside the
/// recipe catalogue: callers must not infer scroll meaning from a contiguous id range (the
/// upstream order is not the recipe/family order).
pub const IMBUEMENT_ETCHER_ITEM_ID: u32 = 51_443;
pub const BLANK_IMBUEMENT_SCROLL_ITEM_ID: u32 = 51_442;
pub const BLANK_IMBUEMENT_SCROLL_GOLD: u64 = 25_000;
pub const BLANK_IMBUEMENT_SCROLL_TOMES: u32 = 5;

/// The recipe carried by a filled scroll. Basic imbuements have no scroll on Global.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImbuementScroll {
    pub item_id: u32,
    pub family: u8,
    pub tier: u8,
}

/// All 24 Powerful and 24 Intricate scrolls; ids from pinned Canary items.xml.
/// Ordered by (family, tier), as required by the shrine wire window.
pub const IMBUEMENT_SCROLLS: [ImbuementScroll; 48] = [
    ImbuementScroll {
        item_id: 51742,
        family: 1,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51462,
        family: 1,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51734,
        family: 3,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51454,
        family: 3,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51741,
        family: 4,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51461,
        family: 4,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51727,
        family: 5,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51447,
        family: 5,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51729,
        family: 6,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51449,
        family: 6,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51728,
        family: 7,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51448,
        family: 7,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51737,
        family: 8,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51457,
        family: 8,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51732,
        family: 9,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51452,
        family: 9,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51743,
        family: 10,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51463,
        family: 10,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51744,
        family: 11,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51464,
        family: 11,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51747,
        family: 12,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51467,
        family: 12,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51726,
        family: 14,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51446,
        family: 14,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51724,
        family: 15,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51444,
        family: 15,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51735,
        family: 16,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51455,
        family: 16,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51736,
        family: 17,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51456,
        family: 17,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51731,
        family: 18,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51451,
        family: 18,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51725,
        family: 19,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51445,
        family: 19,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51740,
        family: 20,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51460,
        family: 20,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51739,
        family: 21,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51459,
        family: 21,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51745,
        family: 22,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51465,
        family: 22,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51733,
        family: 23,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51453,
        family: 23,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51730,
        family: 24,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51450,
        family: 24,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51738,
        family: 25,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51458,
        family: 25,
        tier: 3,
    },
    ImbuementScroll {
        item_id: 51746,
        family: 26,
        tier: 2,
    },
    ImbuementScroll {
        item_id: 51466,
        family: 26,
        tier: 3,
    },
];

pub fn imbuement_scroll(item_id: u32) -> Option<ImbuementScroll> {
    IMBUEMENT_SCROLLS
        .iter()
        .copied()
        .find(|scroll| scroll.item_id == item_id)
}

pub fn scroll_for_imbuement(family: u8, tier: u8) -> Option<ImbuementScroll> {
    IMBUEMENT_SCROLLS
        .iter()
        .copied()
        .find(|scroll| scroll.family == family && scroll.tier == tier)
}

/// Qualify blank-scroll inscription before the atomic material/gold/blank-to-filled write.
/// Creator access applies here (unlike use by the eventual recipient), and only tiers which
/// actually have a filled-scroll definition are admitted.
pub fn qualify_scroll_inscription(
    family: u8,
    tier: u8,
    creator_eligible: bool,
    at_shrine: bool,
) -> Option<(ImbuementScroll, &'static ImbuementRecipe)> {
    if !creator_eligible || !at_shrine {
        return None;
    }
    let scroll = scroll_for_imbuement(family, tier)?;
    Some((scroll, imbuement_recipe(family, tier)?))
}

/// Albinius' purchase gate. The caller remains responsible for atomically exchanging the price
/// and five tomes for the blank scroll; this predicate cannot grant or mint an item.
pub const fn can_buy_blank_imbuement_scroll(
    world_quest_complete: bool,
    heavy_old_tomes: u32,
) -> bool {
    world_quest_complete && heavy_old_tomes >= BLANK_IMBUEMENT_SCROLL_TOMES
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ImbuementTier {
    Basic = 1,
    Intricate = 2,
    Powerful = 3,
}

/// Reference prices remain content defaults (PARITY_PENDING), not permission to
/// charge a fee. A writer must bind the admitted content revision and materials.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImbuementTierRules {
    pub tier: ImbuementTier,
    pub apply_gold: u64,
    pub requires_premium: bool,
    pub success_percent: u8,
}

// Prices: tibia.com news 8396 (current application fees).
// Success: IMBUE-FORGE0 §3, architect ruling R2: always succeeds, no protection.
// Canary's XML percent=90/70/50 is unused by Player::onApplyImbuement:
// https://github.com/opentibiabr/canary/blob/04b83b51/src/creatures/players/player.cpp
pub const IMBUEMENT_TIER_RULES: [ImbuementTierRules; 3] = [
    ImbuementTierRules {
        tier: ImbuementTier::Basic,
        apply_gold: 7_500,
        requires_premium: false,
        success_percent: 100,
    },
    ImbuementTierRules {
        tier: ImbuementTier::Intricate,
        apply_gold: 60_000,
        requires_premium: true,
        success_percent: 100,
    },
    ImbuementTierRules {
        tier: ImbuementTier::Powerful,
        apply_gold: 250_000,
        requires_premium: true,
        success_percent: 100,
    },
];

/// One applicable family and tier with its cumulative material list
/// (`IMBUEMENT_RECIPES`, generated from the pinned Canary catalogue).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImbuementRecipe {
    /// Wire family 1-20; the durable category is `family - 1`.
    pub family: u8,
    pub tier: u8,
    pub key: &'static str,
    pub requires_premium: bool,
    pub apply_gold: u64,
    /// `(material item key, count)`, each consumed from one main-backpack stack.
    pub materials: &'static [(&'static str, u32)],
}

#[path = "imbuement_catalogue.rs"]
mod catalogue;
pub use catalogue::{IMBUEMENT_CATALOGUE_REVISION, IMBUEMENT_RECIPES};

/// The catalogue recipe of one family and tier.
pub fn imbuement_recipe(family: u8, tier: u8) -> Option<&'static ImbuementRecipe> {
    // Family 13 is the pre-catalogue paralysis family; existing items remain readable.
    let family = if family == 13 { 26 } else { family };
    IMBUEMENT_RECIPES
        .binary_search_by_key(&(family, tier), |recipe| (recipe.family, recipe.tier))
        .ok()
        .map(|index| &IMBUEMENT_RECIPES[index])
}

#[cfg(test)]
#[path = "imbuement_tests.rs"]
mod tests;

/// Elemental conversions conflict with each other and legacy family 2.
/// Legacy paralysis and Vibrancy likewise share one exclusion group.
pub const fn imbuement_category_group(category: u8) -> u8 {
    match category {
        20..=24 => 1,
        25 => 12,
        other => other,
    }
}
