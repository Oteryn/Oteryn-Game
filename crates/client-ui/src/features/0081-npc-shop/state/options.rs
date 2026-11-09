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
    /// Capacity here is a preview. The server still checks custody and free space.
    pub fn can_trade(&self, quote: Quote, amount: u32) -> bool {
        self.valid_amount(quote, amount)
            && (quote.side != NpcTradeSide::Buy
                || self.options.ignore_capacity
                || self.free_slots() != Some(0))
    }
}
