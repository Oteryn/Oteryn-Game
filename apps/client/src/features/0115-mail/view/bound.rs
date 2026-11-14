//! Atomic shared-frontend adapter over the native mail owner.
use crate::play::{PlayLink, mailbox::{lock, SharedMailbox}};
use std::sync::Arc;
use oteryn_client_ui::{inventory::InventoryPanel, mail::{Intent, Panel}};

#[derive(Clone)]
pub struct MailBinding {
    pub panel: Panel,
    inventory: InventoryPanel,
    session: SharedMailbox,
}
impl PartialEq for MailBinding {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.session, &other.session) && self.panel == other.panel && self.inventory == other.inventory
    }
}
impl Eq for MailBinding {}
impl PlayLink {
    pub fn mail_binding(&self) -> MailBinding {
        let mailbox = lock(&self.mailbox);
        MailBinding { panel: mailbox.properties.mail.clone(), inventory: mailbox.inventory.clone(), session: Arc::clone(&self.mailbox) }
    }
    pub fn mail_dispatch_bound(&self, expected: &MailBinding, values: [String; 3], intent: Option<Intent>) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !Arc::ptr_eq(&self.mailbox, &expected.session) || mailbox.properties.mail != expected.panel || mailbox.inventory != expected.inventory
            || !expected.panel.open || !expected.panel.selected || expected.panel.pending
            || mailbox.server_modal.is_active() || mailbox.inventory_shutdown || self.commands.is_closed()
        { return false; }
        let Some(intent) = intent else {
            mailbox.properties.mail.open = false;
            return true;
        };
        if values.iter().any(|value| !oteryn_client_ui::modal_dialogs::TextEditor::accepts(value, 7996)) { return false; }
        let mut panel = expected.panel.clone();
        panel.fields = values;
        if super::super::protocol::command(&panel, intent).is_err() { return false; }
        mailbox.properties.mail.fields = panel.fields;
        super::super::input::submit(&mut mailbox, &self.commands, intent);
        true
    }
}

#[cfg(test)]
#[path = "../tests/bound.rs"]
mod tests;
