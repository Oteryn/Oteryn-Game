//! Capability 498 `HIGHSCORES_LOYALTY_V1`: highscores category 13, Loyalty, ranks the account
//! loyalty points (the primary value). Without the capability the category is not offered and
//! the server answers `NOT_SUPPORTED` for it.
use crate::highscores::{CATEGORIES, CATEGORY_IDS};

/// The wire id of the Loyalty category.
pub const LOYALTY_CATEGORY: u32 = 13;
const LOYALTY_LABEL: &str = "Loyalty points";

/// The categories a session offers, in id order: the registered ones, and Loyalty when the
/// session selected the capability.
#[must_use]
pub fn categories(loyalty: bool) -> Vec<(u32, &'static str)> {
    let mut list: Vec<(u32, &'static str)> = CATEGORY_IDS
        .iter()
        .copied()
        .zip(CATEGORIES.iter().copied())
        .collect();
    if loyalty {
        let at = list.partition_point(|(id, _)| *id < LOYALTY_CATEGORY);
        list.insert(at, (LOYALTY_CATEGORY, LOYALTY_LABEL));
    }
    list
}

/// The category of a native combo row.
#[must_use]
pub fn category_for_selection(index: usize, loyalty: bool) -> Option<u32> {
    categories(loyalty).get(index).map(|(id, _)| *id)
}

/// The native combo row of a category.
#[must_use]
pub fn category_selection(category: u32, loyalty: bool) -> Option<usize> {
    categories(loyalty)
        .iter()
        .position(|(id, _)| *id == category)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
