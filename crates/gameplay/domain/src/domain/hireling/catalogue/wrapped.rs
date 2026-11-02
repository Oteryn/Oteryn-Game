//! Generated from the complete Crystal201189f native Item capture.
//! `items.hpp:168-170` / `game.cpp:2761-2767`: every buyable isWrappable target
//! is delivered as decoration kit23398 with its original unWrapId.
pub const DECORATION_KIT_ITEM_ID: u32 = 23398;
// BEGIN GENERATED HIRELING FURNITURE TARGETS
#[rustfmt::skip]
pub const WRAPPED_TRADE_ITEM_IDS: &[u32] = &[
    2352, 2426, 32473, 32474, 32475, 32476, 32477, 32478, 32479, 32480,
    32482, 32483, 32484, 32485, 32486, 32487, 32488, 32489,
];
// END GENERATED HIRELING FURNITURE TARGETS

pub fn wrapped_trade_item(item: u32) -> bool {
    WRAPPED_TRADE_ITEM_IDS.binary_search(&item).is_ok()
}

pub fn wrapped_buy(row: usize) -> Option<&'static super::TradeRow> {
    let row = super::ROWS.get(row)?;
    (row.buy > 0 && wrapped_trade_item(row.item_id)).then_some(row)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_complete_capture_has_eighteen_source_backed_buy_targets() {
        assert_eq!(WRAPPED_TRADE_ITEM_IDS.len(), 18);
        assert!(
            WRAPPED_TRADE_ITEM_IDS
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
        assert_eq!(
            super::super::ROWS
                .iter()
                .enumerate()
                .filter(|(index, _)| wrapped_buy(*index).is_some())
                .count(),
            18
        );
        assert!(WRAPPED_TRADE_ITEM_IDS.iter().all(|id| {
            super::super::ROWS
                .iter()
                .any(|row| row.item_id == *id && row.buy > 0)
        }));
        // The three malformed source SELL rows are not furniture and cannot opt in.
        assert!(
            [3657, 5526, 25686, 23398]
                .into_iter()
                .all(|id| !wrapped_trade_item(id))
        );
    }
}
