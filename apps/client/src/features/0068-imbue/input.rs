use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::imbue::{Intent, ItemHandle, open_key, request};
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

impl PlayLink {
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
        mailbox.imbue.feedback = Some("The session has ended");
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
        mailbox.imbue.details_feedback = Some("The session has ended");
    }
}
