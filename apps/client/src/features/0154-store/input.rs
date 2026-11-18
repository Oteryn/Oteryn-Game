use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};

impl PlayLink {
    /// Shared presentation uses the same server catalogue request as the native window.
    pub fn store_set_open(&self, open: bool) {
        {
            let mut mailbox = lock(&self.mailbox);
            mailbox.store.open = open;
            if !open {
                mailbox.store.gift_open = false;
            }
        }
        if open {
            let panel = self.store_panel();
            if panel.confirmed_purchase.is_none() {
                self.store_browse(
                    &panel.browser.category,
                    &panel.browser.search,
                    panel.browser.sort,
                    0,
                );
            }
        }
    }

    pub fn store_select_product(&self, index: usize) -> bool {
        oteryn_client_ui::store::select_product(&mut lock(&self.mailbox).store, index)
    }

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
        if matches!(command_type, 6187 | 6188) && !mailbox.store.token_supported { return; }
        let Some(command) = oteryn_client_ui::store::command(command_type, payload) else {
            return;
        };
        let requested = if matches!(command_type, 6187 | 6188) {
            oteryn_client_ui::store_cosmetic_tokens::request(&mut mailbox.store, command_type, command.payload)
        } else { oteryn_client_ui::store::request(&mut mailbox.store, command) };
        if requested.is_some() && command_type == 6187 { mailbox.store.token_balance_refresh = false; }
        if let Some(command) = requested
            && self
                .commands
                .send(PlayCommand::ServerMechanic(command))
                .is_err()
        {
            mailbox.store.pending = false;
            mailbox.store.feedback = Some(tr!("The session has ended").into());
        }
    }

    pub fn store_payment(&self, tokens: bool) -> bool {
        oteryn_client_ui::store::choose_payment(&mut lock(&self.mailbox).store, tokens)
    }

    pub fn store_input(&self, events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
        if !crate::input::store_key(events) {
            return false;
        }
        lock(&self.mailbox).store.open = true;
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

#[cfg(any(test, bevy_shell))]
impl PlayLink {
    /// Called only after confirmation, with the exact displayed quote.
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

#[cfg(bevy_shell)]
impl PlayLink {
    pub(crate) fn store_retry_purchase_bound(
        &self,
        expected: &oteryn_client_ui::store::StorePanel,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.activity_reset_commands
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || self.commands.is_closed()
            || mailbox.store != *expected
            || !expected.open
            || !expected.selected
            || expected.pending
            || expected.gift.pending.is_some()
            || expected.gift.confirmed.is_some()
            || expected.hireling_modal_id.is_some()
            || expected.confirmed_purchase.is_none()
        {
            return false;
        }
        let mut prepared = expected.clone();
        prepared.purchase_confirmed = true;
        let Some(command) = oteryn_client_ui::store::purchase(&mut prepared) else {
            return false;
        };
        if self
            .commands
            .send(PlayCommand::ServerMechanic(command))
            .is_err()
        {
            return false;
        }
        mailbox.store = prepared;
        true
    }
}

#[path = "input/gift.rs"]
pub(in crate::play) mod gift;

#[cfg(all(test, bevy_shell))]
impl crate::play::Outbox {
    pub(crate) fn bevy_store_initialize(&self) {
        self.store_initialize(true, false);
        lock(&self.0).discovery.history.selected = true;
    }
    pub(crate) fn bevy_store_reply(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        self.store_result(response);
    }
    pub(crate) fn bevy_coin_gift_reply(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        self.coin_gift_result(response);
    }
    pub(crate) fn bevy_purchase_history_reply(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        oteryn_client_ui::purchase_history::protocol::apply_result(
            &mut lock(&self.0).discovery.history,
            response,
        );
    }
}
