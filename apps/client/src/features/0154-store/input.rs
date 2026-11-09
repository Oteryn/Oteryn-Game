use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};

impl PlayLink {
    pub fn store_browse(&self, category: &str, search: &str, sort: u8, step: i8) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) =
            oteryn_client_ui::store::browse(&mut mailbox.store, category, search, sort, step)
            && self
                .commands
                .send(PlayCommand::ServerMechanic(command))
                .is_err()
        {
            mailbox.store.pending = false;
            mailbox.store.feedback = Some("The session has ended".into());
        }
    }

    pub fn store_category(&self, key: &str) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let Some(command) = oteryn_client_ui::store::select_category(&mut mailbox.store, key)
        else {
            return false;
        };
        if self
            .commands
            .send(PlayCommand::ServerMechanic(command))
            .is_err()
        {
            mailbox.store.pending = false;
            mailbox.store.feedback = Some("The session has ended".into());
            return false;
        }
        true
    }

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

impl crate::play::PlayLink {
    pub fn coin_history(&self, next: bool) -> bool {
        let command = oteryn_client_ui::store::coin_history(&mut lock(&self.mailbox).store, next);
        command.is_some_and(|command| self.discovery_send(command))
    }
    pub fn coin_history_filter(&self, kind: &str, from: &str, until: &str) -> bool {
        oteryn_client_ui::store::coin_history_filter(
            &mut lock(&self.mailbox).store,
            kind,
            from,
            until,
        )
    }
}

#[cfg(any(windows, test))]
impl PlayLink {
    /// Called only after the native confirmation, with the exact displayed quote.
    pub fn store_confirm_purchase(
        &self,
        expected: &oteryn_client_runtime::cui3::store::StoreCataloguePage,
        name: &str,
        sex: &str,
        identity: &str,
    ) -> bool {
        let mut m = lock(&self.mailbox);
        if m.store.catalogue.as_ref() != Some(expected) || m.store.pending {
            return false;
        }
        super::view::native::customization(&mut m.store, name, sex, identity);
        let Some(command) = super::view::native::purchase(&mut m.store, 0, true) else {
            return false;
        };
        if self
            .commands
            .send(PlayCommand::ServerMechanic(command))
            .is_err()
        {
            m.store.pending = false;
            m.store.feedback =
                Some("The session has ended; retain the original request for retry".into());
            return false;
        }
        true
    }
}

#[path = "input/gift.rs"]
pub(in crate::play) mod gift;
