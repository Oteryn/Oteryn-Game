use super::{DepotReply, ItemHandle, Items};
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DepotPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub reply: Option<DepotReply>,
    pub items: Items,
    pub feedback: &'static str,
    /// Manual Sort Mode (capability 510): the carried item picked for a deposit, placed when
    /// the player chooses its box slot.
    pub held: Option<ItemHandle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Browse { box_number: u8, page: u32 },
    Deposit(ItemHandle),
    Withdraw(usize),
}
