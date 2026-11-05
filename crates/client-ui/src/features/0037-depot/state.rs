use super::{DepotReply, ItemHandle, Items};
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DepotPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub reply: Option<DepotReply>,
    pub items: Items,
    pub feedback: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Browse { box_number: u8, page: u32 },
    Deposit(ItemHandle),
    Withdraw(usize),
}
