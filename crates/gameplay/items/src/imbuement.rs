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

/// Capacity (Featherweight) and speed (Swiftness) are utility imbuements. Global consumes
/// their budget whenever the item is worn and the character is online; every other category
/// consumes only while the wearer is in combat. Protection-zone presence is deliberately not
/// an input: Global's fight state, rather than a donor-specific PZ shortcut, is authoritative.
pub fn consumes_elapsed(category: u8, in_combat: bool) -> bool {
    matches!(category, 8 | 9) || in_combat
}
// Reference: https://github.com/opentibiabr/canary/blob/04b83b51/data/XML/imbuements.xml
pub const IMBUEMENT_CLEAR_GOLD: u64 = 15_000;

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

// Prices: https://github.com/opentibiabr/canary/blob/04b83b51/data/XML/imbuements.xml
// Success: IMBUE-FORGE0 §3, architect ruling R2: always succeeds, no protection.
// Canary's XML percent=90/70/50 is unused by Player::onApplyImbuement:
// https://github.com/opentibiabr/canary/blob/04b83b51/src/creatures/players/player.cpp
pub const IMBUEMENT_TIER_RULES: [ImbuementTierRules; 3] = [
    ImbuementTierRules {
        tier: ImbuementTier::Basic,
        apply_gold: 5_000,
        requires_premium: false,
        success_percent: 100,
    },
    ImbuementTierRules {
        tier: ImbuementTier::Intricate,
        apply_gold: 30_000,
        requires_premium: true,
        success_percent: 100,
    },
    ImbuementTierRules {
        tier: ImbuementTier::Powerful,
        apply_gold: 200_000,
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
    IMBUEMENT_RECIPES
        .binary_search_by_key(&(family, tier), |recipe| (recipe.family, recipe.tier))
        .ok()
        .map(|index| &IMBUEMENT_RECIPES[index])
}

#[cfg(test)]
#[path = "imbuement_tests.rs"]
mod tests;
