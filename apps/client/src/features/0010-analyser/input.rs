use crate::combat_input::PlayCommand;
use crate::play::PlayLink;
use crate::play::mailbox::{Mailbox, lock};
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn analyser_xp_goal(&self, value: &str) -> bool {
        oteryn_client_ui::analyser::set_xp_goal(
            &mut crate::play::mailbox::lock(&self.mailbox).analyser,
            value,
        )
    }
    pub fn analyser_item_price(&self, item: &str, price: &str) -> bool {
        oteryn_client_ui::analyser::set_item_price(
            &mut crate::play::mailbox::lock(&self.mailbox).analyser,
            item,
            price,
        )
    }
    pub fn analyser_resolve_items(&self) {
        resolve_items(&mut lock(&self.mailbox), &self.commands);
    }
    pub fn analyser_select(&self, key: &str) -> bool {
        oteryn_client_ui::analyser::select_tool(
            &mut crate::play::mailbox::lock(&self.mailbox).analyser,
            key,
        )
    }
    pub fn analyser_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::analyser::open_key(events) {
            return false;
        }
        self.analyser_open();
        true
    }
    pub fn analyser_open(&self) {
        self.analyser_resolve_items();
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            crate::play::mailbox::lock(&self.mailbox).analyser.open = true;
        }
    }
}

pub(in crate::play) fn resolve_items(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
) {
    if let Some(command) =
        oteryn_client_ui::analyser::protocol::catalogue_request(&mut mailbox.analyser)
        && commands.send(PlayCommand::Feature(command)).is_err()
    {
        mailbox.analyser.catalogue_pending.clear();
        mailbox.analyser.feedback = Some("Session ended");
    }
}
