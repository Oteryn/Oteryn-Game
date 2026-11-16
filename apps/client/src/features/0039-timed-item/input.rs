use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::timed_item::Intent;
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    let panel = &mut mailbox.properties.timed_item;
    if !panel.selected {
        panel.feedback = tr!("Unavailable in this session").into();
        return;
    }
    if panel.pending {
        panel.feedback = tr!("Waiting for the server").into();
        return;
    }
    match super::protocol::command(panel, intent) {
        Ok(command) => {
            if oteryn_client_ui::timed_item::request(panel, command.kind)
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                panel.failed(tr!("The session has ended"));
            }
        }
        Err(message) => panel.feedback = message,
    }
}
impl PlayLink {
    pub fn timed_item_edit(&self, field: usize, value: String) {
        edit(&mut lock(&self.mailbox).properties.timed_item, field, value);
    }
    pub fn timed_item_request(&self, intent: Intent) {
        submit(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn timed_item_open(&self) {
        lock(&self.mailbox).properties.timed_item.open = true;
    }
    /// Opens the timed item window for one item and asks the server for its current revision;
    /// the reply fills the revision that Use and Repair then send.
    pub fn timed_item_inspect(&self, handle: oteryn_session::ItemHandle) {
        let mut mailbox = lock(&self.mailbox);
        let properties = &mut mailbox.properties;
        properties.timed_item.open = true;
        if properties.timed_item.pending {
            // The in-flight request keeps its item; its reply retargets the form and the next
            // form tick inspects this one.
            properties.timed_item_deferred = Some(handle);
            return;
        }
        properties.timed_item_deferred = None;
        retarget(&mut properties.timed_item, handle);
        submit(&mut mailbox, &self.commands, Intent::Inspect);
    }
    pub fn timed_item_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::timed_item::open_key(events) {
            return false;
        }
        self.timed_item_open();
        true
    }
}

/// Points the form at `handle`; a new target starts from revision 0 until it is inspected.
pub(in crate::play) fn retarget(
    panel: &mut oteryn_client_ui::timed_item::Panel,
    handle: oteryn_session::ItemHandle,
) {
    let value = handle.get().to_string();
    if panel.fields[0] != value {
        edit(panel, 0, value);
        edit(panel, 1, "0".into());
    }
}

/// Inspects the item chosen while an earlier request was pending, once its reply has arrived.
pub(in crate::play) fn tick(mailbox: &mut Mailbox, commands: &UnboundedSender<PlayCommand>) {
    if mailbox.properties.timed_item.pending {
        return;
    }
    let Some(handle) = mailbox.properties.timed_item_deferred.take() else {
        return;
    };
    retarget(&mut mailbox.properties.timed_item, handle);
    submit(mailbox, commands, Intent::Inspect);
}

pub(in crate::play) fn edit(
    panel: &mut oteryn_client_ui::timed_item::Panel,
    field: usize,
    value: String,
) {
    if panel.pending || value.len() > 64 || value.contains('\0') {
        return;
    }
    if let Some(slot) = panel.fields.get_mut(field) {
        if *slot == value {
            return;
        }
        *slot = value;
        // The previous item's details never describe a new target, even if its inspection fails.
        if field == 0 {
            panel.summary.clear();
        }
    }
}
