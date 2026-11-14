//! Atomic presentation bindings over the existing native presenters and command owner.
use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use oteryn_client_ui::browse_field::BrowseFieldPanel;
use std::sync::{Arc, Mutex, Weak};
#[derive(Clone)]
pub(crate) struct Binding {
    pub panel: BrowseFieldPanel,
    mailbox: Weak<Mutex<Mailbox>>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Close,
    Open,
    Seek,
    Item(usize),
}
fn projection(mailbox: &Mailbox) -> BrowseFieldPanel {
    let mut panel = mailbox.browse_field.clone();
    panel.pending |= mailbox.world_context.pending;
    if mailbox.inventory_shutdown {
        panel.open = false;
        panel.selected = false;
    }
    panel
}
impl Binding {
    pub(crate) fn is_for(&self, link: &PlayLink) -> bool {
        self.mailbox.ptr_eq(&Arc::downgrade(&link.mailbox))
    }
    pub(crate) fn text(&self) -> String {
        super::native::result_text(&self.panel)
    }
}
impl PlayLink {
    pub(crate) fn browse_field_binding(&self) -> Binding {
        Binding {
            panel: projection(&lock(&self.mailbox)),
            mailbox: Arc::downgrade(&self.mailbox),
        }
    }
    pub(crate) fn browse_field_set_open_bound(&self, open: bool) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.inventory_shutdown
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || self.commands.is_closed()
        {
            return false;
        }
        mailbox.browse_field.open = open;
        true
    }
    pub(crate) fn browse_field_dispatch_bound(
        &self,
        expected: &Binding,
        action: Action,
        fields: &[String; 7],
        confirmed: bool,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !expected.is_for(self)
            || projection(&mailbox) != expected.panel
            || !mailbox.browse_field.open
            || mailbox.inventory_shutdown
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || self.commands.is_closed()
        {
            return false;
        }
        if action == Action::Close {
            mailbox.browse_field.open = false;
            return true;
        }
        if !expected.panel.selected
            || expected.panel.pending
            || action == Action::Item(106) && !confirmed
        {
            return false;
        }
        let mut next = mailbox.browse_field.clone();
        let command = match action {
            Action::Open | Action::Seek => super::native::submit(
                &mut next,
                action == Action::Seek,
                std::array::from_fn(|i| fields[i].as_str()),
            )
            .map(PlayCommand::ServerMechanic),
            Action::Item(id) => {
                let Ok(view_id) = fields[3].trim().parse::<u8>() else {
                    return false;
                };
                if view_id != expected.panel.view_id {
                    return false;
                }
                super::native::item_command(&mut next, id, &fields[5], &fields[6])
            }
            Action::Close => return false,
        };
        let Some(command) = command else {
            return false;
        };
        let using = matches!(command, PlayCommand::WorldContext(_));
        if self.commands.send(command).is_err() {
            return false;
        }
        mailbox.browse_field = next;
        if using {
            mailbox.world_context.pending = true;
        }
        true
    }
}
