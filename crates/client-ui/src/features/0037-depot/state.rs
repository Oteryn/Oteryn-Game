use super::{DepotReply, ItemHandle, Items};
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DepotPanel {
    pub selected: bool,
    pub cross_town_selected: bool,
    pub selected_town: Option<u16>,
    pub open: bool,
    pub pending: bool,
    pub reply: Option<DepotReply>,
    pub items: Items,
    pub feedback: &'static str,
    /// Manual Sort Mode (capability 510): the carried item picked for a deposit, placed when
    /// the player chooses its box slot.
    pub held: Option<ItemHandle>,
}

impl DepotPanel {
    pub fn replace_items(&mut self, items: Items) {
        self.items = items;
        if self.held.is_some_and(|held| {
            !super::deposit_items(self)
                .iter()
                .any(|item| item.handle == held)
        }) {
            self.held = None;
            self.feedback = tr!("The depot or item changed; refresh and try again");
        }
    }

    pub fn placement_slot(&self, row: usize) -> Option<u32> {
        let reply = self.reply.as_ref()?;
        reply.entries.get(row)?;
        let slot = reply
            .page
            .checked_mul(32)?
            .checked_add(u32::try_from(row).ok()?)?;
        (slot < 500).then_some(slot)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Browse { box_number: u8, page: u32 },
    SelectTown(Option<u16>),
    Deposit(ItemHandle),
    Withdraw(usize),
}
