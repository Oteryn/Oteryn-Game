use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};

#[cfg(any(windows, test))]
pub(in crate::play) fn gift_submit(
    panel: &mut oteryn_client_ui::store::StorePanel,
    action: usize,
    fields: &[String],
) -> bool {
    if action == 2 {
        panel.gift_open = false;
        panel.gift_reviewed = None;
        return true;
    }
    let [recipient, amount] = fields else {
        return false;
    };
    if recipient.len() > 64 || amount.len() > 20 {
        panel.gift_reviewed = None;
        panel.feedback =
            Some(tr!("Enter a recipient of at most 64 bytes and a valid coin amount").into());
        return false;
    }
    panel.gift_fields = [recipient.clone(), amount.clone()];
    match action {
        0 => oteryn_client_ui::store::review_gift(panel, recipient, amount),
        1 => oteryn_client_ui::store::confirm_gift(panel, recipient, amount),
        _ => false,
    }
}

impl PlayLink {
    pub fn store_request(&self, command_type: u32, payload: Vec<u8>) {
        let mut mailbox = lock(&self.mailbox);
        let Some(command) = oteryn_client_ui::store::command(command_type, payload) else {
            return;
        };
        if let Some(command) = oteryn_client_ui::store::request(&mut mailbox.store, command)
            && self
                .commands
                .send(PlayCommand::ServerMechanic(command))
                .is_err()
        {
            mailbox.store.pending = false;
            mailbox.store.feedback = Some(tr!("The session has ended").into());
        }
    }

    pub fn store_input(&self, events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::store::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).store.open = true;
        }
        true
    }
}
