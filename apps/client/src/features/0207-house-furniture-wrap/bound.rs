//! Atomic presentation bindings; the existing panel and protocol remain the owners.
use crate::play::{PlayLink, mailbox::{Mailbox, lock}};
use oteryn_client_ui::house_furniture_wrap::{Intent, Panel};
use std::sync::{Arc, Mutex, Weak};

#[derive(Clone)]
pub(crate) struct FurnitureWrappingSnapshot {
    pub panel: Panel,
    session: Weak<Mutex<Mailbox>>,
}
impl PartialEq for FurnitureWrappingSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session) && self.panel == other.panel
    }
}
#[derive(Clone, Copy)]
pub(crate) enum FurnitureWrappingAction {
    Toggle,
    Close,
    Select(usize),
    Request(Intent),
}
impl PlayLink {
    pub(crate) fn house_furniture_snapshot(&self) -> FurnitureWrappingSnapshot {
        FurnitureWrappingSnapshot {
            panel: lock(&self.mailbox).properties.house_furniture_wrap.clone(),
            session: Arc::downgrade(&self.mailbox),
        }
    }
    /// Re-prove session, modal fence, target and confirmation before invoking the owner.
    pub(crate) fn house_furniture_dispatch_bound(
        &self,
        expected: &FurnitureWrappingSnapshot,
        action: FurnitureWrappingAction,
        blocked: bool,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if blocked
            || !expected.session.ptr_eq(&Arc::downgrade(&self.mailbox))
            || mailbox.ended.is_some()
            || self.commands.is_closed()
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
            || mailbox.lens.active
            || mailbox.properties.house_furniture_wrap != expected.panel
        {
            return false;
        }
        let panel = &mut mailbox.properties.house_furniture_wrap;
        match action {
            FurnitureWrappingAction::Toggle => {
                panel.open = !panel.open;
                if panel.open && panel.selected && !panel.pending {
                    super::super::input::submit(&mut mailbox, &self.commands, Intent::Refresh);
                }
            }
            FurnitureWrappingAction::Close => {
                if !panel.open { return false; }
                panel.open = false;
            }
            FurnitureWrappingAction::Select(index) => {
                if !panel.open || !panel.selected || panel.pending
                    || !panel.catalogue.as_ref().is_some_and(|catalogue| index < catalogue.entries.len())
                { return false; }
                panel.choice = (index + 1).to_string();
                // Selecting another target always starts a fresh owner review.
                panel.reviewed_payload = None;
                panel.feedback.clear();
            }
            FurnitureWrappingAction::Request(intent) => {
                if !panel.open || !panel.selected || panel.pending { return false; }
                super::super::input::submit(&mut mailbox, &self.commands, intent);
            }
        }
        true
    }
}
