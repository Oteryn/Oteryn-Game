#[path = "input/tracker.rs"]
pub(in crate::play) mod tracker;
use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::imbue::{Intent, ItemHandle, open_key, request};
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

impl PlayLink {
    /// Select only a handle in the session's current carried inventory projection.
    pub fn imbue_carried_item(&self, item: ItemHandle) -> bool {
        select_carried(&mut lock(&self.mailbox), &self.commands, item)
    }

    pub fn imbue_item(&self, item: ItemHandle) {
        lock(&self.mailbox).imbue.select_item(item);
    }

    pub fn imbue_slot(&self, slot: u8) -> bool {
        lock(&self.mailbox).imbue.select_slot(slot)
    }

    pub fn imbue_offer(&self, offer: usize) -> bool {
        lock(&self.mailbox).imbue.select_offer(offer)
    }

    pub fn imbue_details_request(&self) {
        enqueue_details(&mut lock(&self.mailbox), &self.commands);
    }

    pub fn imbue_request(&self, intent: Intent) {
        enqueue(&mut lock(&self.mailbox), &self.commands, intent);
    }

    pub fn imbue_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if oteryn_client_ui::imbue::tracker_key(events) {
            self.open_imbue_tracker();
            return true;
        }
        if !open_key(events) {
            return false;
        }
        self.open_imbue();
        true
    }
}

pub(in crate::play) fn enqueue(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    if let Some(command) = request(&mut mailbox.imbue, intent)
        && commands.send(PlayCommand::Imbue(command)).is_err()
    {
        mailbox.imbue.pending = false;
        mailbox.imbue.feedback = Some(tr!("The session has ended"));
    }
}

pub(in crate::play) fn enqueue_details(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
) {
    if let Some(command) = oteryn_client_ui::imbue::details::request(&mut mailbox.imbue)
        && commands.send(PlayCommand::Feature(command)).is_err()
    {
        mailbox.imbue.details_pending = None;
        mailbox.imbue.details_feedback = Some(tr!("The session has ended"));
    }
}

/// Includes equipment and only container views proven connected to carried inventory.
pub(in crate::play) fn carried_items(
    mailbox: &Mailbox,
) -> Vec<oteryn_client_runtime::depot::ItemEntry> {
    let items = &mailbox.depot.items;
    let mut carried: Vec<_> = items
        .inventory
        .main_backpack
        .iter()
        .chain(&items.inventory.entries)
        .chain(items.inventory.equipment.iter().map(|entry| &entry.item))
        .chain(&items.views)
        .cloned()
        .collect();
    carried.sort_by_key(|entry| entry.handle);
    carried.dedup_by_key(|entry| entry.handle);
    carried
}

pub(in crate::play) fn select_carried(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    item: ItemHandle,
) -> bool {
    if !mailbox.imbue.selected
        || mailbox.imbue.pending
        || !carried_items(mailbox)
            .iter()
            .any(|entry| entry.handle == item)
    {
        return false;
    }
    mailbox.imbue.select_item(item);
    enqueue(mailbox, commands, Intent::Refresh);
    true
}

/// Shared frontend actions are bound atomically to the exact displayed shrine snapshot.
#[derive(Clone, Copy)]
pub enum ImbueViewAction {
    Close,
    Item(ItemHandle),
    Slot(u8),
    Offer(usize),
    Request(Intent),
}
impl PlayLink {
    pub fn imbue_carried_items(&self) -> Vec<oteryn_client_runtime::depot::ItemEntry> {
        carried_items(&lock(&self.mailbox))
    }

    pub fn imbue_dispatch_bound(&self, expected: &oteryn_client_ui::imbue::ImbuePanel, action: ImbueViewAction) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !mailbox.imbue.open || &mailbox.imbue != expected {
            return false;
        }
        match action {
            ImbueViewAction::Close => mailbox.imbue.open = false,
            ImbueViewAction::Item(item) => return select_carried(&mut mailbox, &self.commands, item),
            ImbueViewAction::Slot(slot) => return mailbox.imbue.select_slot(slot),
            ImbueViewAction::Offer(offer) => return mailbox.imbue.select_offer(offer),
            ImbueViewAction::Request(intent) => enqueue(&mut mailbox, &self.commands, intent),
        }
        true
    }
}

#[cfg(any(windows, test))]
pub(in crate::play) fn enqueue_confirmed(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    expected: &oteryn_client_ui::imbue::ImbuePanel,
    intent: Intent,
    confirmed: bool,
) -> bool {
    if !confirmed
        || !mailbox.imbue.open
        || mailbox.imbue != *expected
        || mailbox.world_input_blocked()
        || commands.is_closed()
    {
        return false;
    }
    enqueue(mailbox, commands, intent);
    mailbox.imbue.pending
}
