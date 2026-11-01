//! Exact Crystal source rows. Pages preserve source order and never truncate a category.
use super::Skill;

/// Commented Crystal placeholders are reported, never converted into invented item ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisabledRow {
    pub name: &'static str,
    pub source_line: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeRow {
    pub category: u8,
    pub item_id: u32,
    pub buy: u64,
    pub sell: u64,
    pub sub_type: u32,
    pub source_line: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Package {
    pub name: &'static str,
    pub gold: u64,
    pub items: [(u32, u32); 3],
    pub source_line: u16,
}

mod generated;
mod wrapped;
pub use generated::{CATEGORIES, DISABLED_ROWS, PACKAGES, ROWS, SOURCE_SHA256};
pub use wrapped::{
    DECORATION_KIT_ITEM_ID, WRAPPED_TRADE_ITEM_IDS, wrapped_buy, wrapped_trade_item,
};

pub const PRESENT_ITEM_ID: u32 = 2856;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Offer {
    /// Global source row, retained separately from the page-local wire index.
    pub row: usize,
    pub side: Side,
    pub price: u64,
}

pub fn category(name: &str) -> Option<u8> {
    CATEGORIES
        .iter()
        .position(|value| *value == name)
        .and_then(|n| u8::try_from(n).ok())
}

pub fn allowed(category: u8, skills: u8) -> bool {
    usize::from(category) < CATEGORIES.len() && (category == 0 || Skill::Trader.owned(skills))
}

pub fn offers(category: u8) -> impl Iterator<Item = Offer> {
    ROWS.iter()
        .enumerate()
        .filter(move |(_, row)| row.category == category)
        .flat_map(|(index, row)| {
            [
                (row.buy > 0).then_some(Offer {
                    row: index,
                    side: Side::Buy,
                    price: row.buy,
                }),
                (row.sell > 0).then_some(Offer {
                    row: index,
                    side: Side::Sell,
                    price: row.sell,
                }),
            ]
            .into_iter()
            .flatten()
        })
}

/// `maximum` is the existing protocol window limit, not an invented gameplay constant.
pub fn page(category: u8, page: u32, maximum: usize) -> Option<Vec<Offer>> {
    if maximum == 0 || usize::from(category) >= CATEGORIES.len() {
        return None;
    }
    let start = usize::try_from(page).ok()?.checked_mul(maximum)?;
    let result: Vec<_> = offers(category).skip(start).take(maximum).collect();
    (!result.is_empty()).then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_priced_crystal_buy_loot_row_is_reachable_through_bounded_pages() {
        // ANSWER6547428201 Q4: preserve the existing 1024-offer wire bound.
        let mut seen = BTreeSet::new();
        let mut page_number = 0;
        while let Some(entries) = page(11, page_number, 1024) {
            assert!(!entries.is_empty() && entries.len() <= 1024);
            for entry in entries {
                assert_eq!(ROWS[entry.row].category, 11);
                seen.insert(entry.row);
            }
            page_number += 1;
        }
        // Crystal hireling.lua:1691 preserves bronze goblet5807 without either
        // price. npc.cpp:597-606 refuses a zero sell price; never invent one.
        let unpriced: Vec<_> = ROWS
            .iter()
            .filter(|row| row.category == 11 && row.buy == 0 && row.sell == 0)
            .collect();
        assert_eq!(unpriced.len(), 1);
        assert_eq!((unpriced[0].item_id, unpriced[0].source_line), (5807, 1691));
        assert_eq!(seen.len(), 1375);
        assert_eq!(seen.len() + unpriced.len(), 1376);
        assert!(
            ROWS.iter()
                .enumerate()
                .filter(|(_, r)| r.category == 11 && (r.buy > 0 || r.sell > 0))
                .all(|(i, _)| seen.contains(&i))
        );
        assert_eq!(page_number, 2);
        assert_eq!(page(11, u32::MAX, 1024), None);
    }

    #[test]
    fn all_categories_and_packages_retain_the_pinned_source_counts() {
        let counts: Vec<_> = (0..CATEGORIES.len())
            .map(|i| {
                ROWS.iter()
                    .filter(|row| usize::from(row.category) == i)
                    .count()
            })
            .collect();
        assert_eq!(counts, vec![109, 21, 53, 24, 9, 9, 15, 34, 4, 18, 3, 1376]);
        assert_eq!(ROWS.len(), 1675);
        assert_eq!(DISABLED_ROWS.map(|row| row.source_line), [1468, 1630]);
        assert_eq!(
            ROWS.iter().filter(|row| row.category == 11).count() + DISABLED_ROWS.len(),
            1378
        );
        assert_eq!(PACKAGES.len(), 23);
        assert_eq!(ROWS.iter().filter(|row| row.sub_type != 0).count(), 24);
        assert!(
            PACKAGES
                .iter()
                .all(|p| p.gold > 0 && p.items.iter().all(|(_, n)| *n > 0))
        );
        assert_eq!(SOURCE_SHA256.len(), 64);
    }

    #[test]
    fn free_goods_do_not_unlock_other_categories_or_invalid_pages() {
        assert!(allowed(0, 0));
        for index in 1..12 {
            assert!(!allowed(index, 0));
            assert!(allowed(index, 8));
        }
        assert!(!allowed(12, 255));
        assert_eq!(page(12, 0, 1024), None);
        assert_eq!(page(0, 0, 0), None);
    }
}
