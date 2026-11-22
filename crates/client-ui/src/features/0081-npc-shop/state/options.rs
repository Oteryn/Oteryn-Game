use super::LiveShop;
use crate::npc_shop::{NpcTradeSide, Quote};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShopOptions {
    pub ignore_capacity: bool,
    pub ignore_equipped: bool,
    pub only_owned: bool,
}
impl Default for ShopOptions {
    fn default() -> Self {
        Self {
            ignore_capacity: false,
            ignore_equipped: true,
            only_owned: false,
        }
    }
}
impl LiveShop {
    pub fn backpack_owned(&self, item: u32) -> u64 {
        self.inventory.as_ref().map_or(0, |inventory| {
            inventory
                .entries
                .iter()
                .filter(|entry| entry.item_definition_ref.get() == item)
                .map(|entry| u64::from(entry.count.get()))
                .sum()
        })
    }
    pub fn total_weight(&self, quote: Quote, amount: u32) -> Option<u128> {
        let weight = self.weights.get(&quote.item).copied().flatten()?;
        Some(u128::from(weight) * u128::from(quote.count) * u128::from(amount))
    }
    /// Capacity here is a preview. The server still checks custody and free space.
    pub fn can_trade(&self, quote: Quote, amount: u32) -> bool {
        let pays_with_coins = quote.price != 0
            && self.inventory.as_ref().is_some_and(|inventory| {
                inventory.entries.iter().any(|entry| {
                    // The item view projects source appearances for the three coins.
                    // Zero means no appearance, so it is never a coin stack.
                    matches!(entry.appearance_id, 3031 | 3035 | 3043)
                })
            });
        self.valid_amount(quote, amount)
            && (quote.side != NpcTradeSide::Buy
                || self.options.ignore_capacity
                || (!self.inspection_selected
                    || self.free_capacity.is_none()
                    || self.weights.contains_key(&quote.item))
                    && self
                        .free_capacity
                        .zip(self.total_weight(quote, amount))
                        .is_none_or(|(free, weight)| weight <= u128::from(free) || pays_with_coins))
            && (quote.side != NpcTradeSide::Buy
                || self.options.ignore_capacity
                || self.free_slots() != Some(0)
                // Paying can consume an entire direct stack. Let the server plan change
                // and check the final placement whenever the current view could allow it.
                || pays_with_coins)
    }
}
