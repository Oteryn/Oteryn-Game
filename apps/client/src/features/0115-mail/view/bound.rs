//! Atomic shared-frontend adapter over the native mail owner.
use crate::play::{
    PlayLink,
    mailbox::{SharedMailbox, lock},
};
use oteryn_client_ui::{
    inventory::InventoryPanel,
    mail::{Intent, Panel},
};
use std::sync::Arc;

#[derive(Clone)]
pub struct MailBinding {
    pub panel: Panel,
    inventory: InventoryPanel,
    session: SharedMailbox,
    containers: oteryn_client_ui::container_tree::ContainerPanel,
}
impl PartialEq for MailBinding {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.session, &other.session)
            && self.panel == other.panel
            && self.inventory == other.inventory
            && self.containers == other.containers
    }
}
impl Eq for MailBinding {}
impl PlayLink {
    pub fn mail_binding(&self) -> MailBinding {
        let mailbox = lock(&self.mailbox);
        MailBinding {
            panel: mailbox.properties.mail.clone(),
            inventory: mailbox.inventory.clone(),
            session: Arc::clone(&self.mailbox),
            containers: mailbox.containers.clone(),
        }
    }
    pub fn mail_dispatch_bound(
        &self,
        expected: &MailBinding,
        values: [String; 3],
        intent: Option<Intent>,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !Arc::ptr_eq(&self.mailbox, &expected.session)
            || mailbox.properties.mail != expected.panel
            || mailbox.inventory != expected.inventory
            || mailbox.containers != expected.containers
            || !expected.panel.open
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
            || mailbox.inventory_shutdown
            || self.commands.is_closed()
        {
            return false;
        }
        let Some(intent) = intent else {
            mailbox.properties.mail.open = false;
            return true;
        };
        if !expected.panel.selected || expected.panel.pending {
            return false;
        }
        if values
            .iter()
            .any(|value| !oteryn_client_ui::modal_dialogs::TextEditor::accepts(value, 7996))
        {
            return false;
        }
        mailbox.properties.mail.fields = values;
        super::super::input::submit(&mut mailbox, &self.commands, intent);
        mailbox.properties.mail.pending
    }
}

#[cfg(test)]
#[path = "../tests/bound.rs"]
mod tests;

#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn bevy_mail_panel(&self, panel: Panel) {
        lock(&self.0).properties.mail = panel;
    }
}
