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

    pub fn store_open_xp_boost(&self) {
        self.store_open_products();
    }

    pub fn store_open_products(&self) {
        let mut mailbox = lock(&self.mailbox);
        mailbox.store.open = true;
        if let Err(message) = self.open_product_offers(&mut mailbox.store, "") {
            mailbox.store.feedback = Some(message.into());
        }
    }

    pub(in crate::play) fn open_product_offers(
        &self,
        store: &mut oteryn_client_ui::store::StorePanel,
        search: &str,
    ) -> Result<(), &'static str> {
        if !store.selected {
            return Err(tr!("Store unavailable in this session"));
        }
        if store.pending || store.confirmed_purchase.is_some() {
            return Err(tr!("Waiting for the server"));
        }
        let command = oteryn_client_ui::store::browse(store, "featured", search, 0, 0)
            .ok_or(tr!("Store unavailable in this session"))?;
        if self
            .commands
            .send(PlayCommand::ServerMechanic(command))
            .is_err()
        {
            store.pending = false;
            return Err(tr!("The session has ended"));
        }
        store.open = true;
        store.page = oteryn_client_ui::store::StorePage::Catalogue;
        store.product_navigation = store.product_navigation.wrapping_add(1);
        Ok(())
    }

    #[cfg(bevy_shell)]
    pub(crate) fn store_open_premium_time(&self) {
        {
            let mut mailbox = lock(&self.mailbox);
            if !mailbox.store.selected
                || mailbox.store.pending
                || mailbox.store.confirmed_purchase.is_some()
                || mailbox.store.gift.confirmed.is_some()
                || mailbox.store.gift.pending.is_some()
            {
                return;
            }
            mailbox.store.open = true;
            mailbox.store.page = oteryn_client_ui::store::StorePage::Catalogue;
        }
        self.store_browse("premium-time", "", 0, 0);
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
        if matches!(command_type, 6187 | 6188) && !mailbox.store.token_supported {
            return;
        }
        let Some(command) = oteryn_client_ui::store::command(command_type, payload) else {
            return;
        };
        let requested = if matches!(command_type, 6187 | 6188) {
            oteryn_client_ui::store_cosmetic_tokens::request(
                &mut mailbox.store,
                command_type,
                command.payload,
            )
        } else {
            oteryn_client_ui::store::request(&mut mailbox.store, command)
        };
        if requested.is_some() && command_type == 6187 {
            mailbox.store.token_balance_refresh = false;
        }
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

    #[cfg(bevy_shell)]
    pub(crate) fn store_inbox_refresh(&self) {
        self.store_inbox_refresh_locked(&mut lock(&self.mailbox));
    }

    pub(in crate::play) fn store_inbox_refresh_locked(
        &self,
        mailbox: &mut crate::play::mailbox::Mailbox,
    ) {
        if !mailbox.store.selected {
            return;
        }
        if mailbox.store.pending {
            mailbox.store_inbox_query_queued = true;
            return;
        }
        mailbox.store_inbox_query_queued = false;
        self.store_inbox_query_locked(mailbox, None);
    }

    #[cfg(bevy_shell)]
    pub(crate) fn store_inbox_next_claim_page(&self) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(after) = mailbox.store.inbox.next {
            self.store_inbox_query_locked(&mut mailbox, Some(after));
        }
    }

    fn store_inbox_query_locked(
        &self,
        mailbox: &mut crate::play::mailbox::Mailbox,
        after: Option<[u8; 16]>,
    ) {
        use oteryn_client_runtime::cui3::store::{COMMAND_TYPE_STORE_INBOX_CLAIM, encode_inbox_query};
        let Some(command) = oteryn_client_ui::store::command(
            COMMAND_TYPE_STORE_INBOX_CLAIM,
            encode_inbox_query(after),
        ) else {
            return;
        };
        self.send_inbox_request(mailbox, command);
    }

    #[cfg(bevy_shell)]
    pub(crate) fn store_inbox_place_lamp(&self) {
        use oteryn_client_runtime::world_interaction::world_object as wire;
        let mut mailbox = lock(&self.mailbox);
        let Some(definition) = mailbox.store.inbox.lamp_definition else {
            return;
        };
        let Ok(payload) = wire::encode_item_use_intent(&wire::UseIntent {
            target: wire::UseTarget::ItemByDefinition(definition),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        }) else {
            return;
        };
        self.send_inbox_request(&mut mailbox, oteryn_client_runtime::server_mechanic::ServerMechanicCommand {
            capability: wire::CAPABILITY_ITEM_USE_V1,
            command_type: wire::COMMAND_TYPE_USE_INTENT,
            payload,
        });
    }

    fn send_inbox_request(
        &self,
        mailbox: &mut crate::play::mailbox::Mailbox,
        command: oteryn_client_runtime::server_mechanic::ServerMechanicCommand,
    ) {
        if let Some(command) = oteryn_client_ui::store::request(&mut mailbox.store, command)
            && self.commands.send(PlayCommand::ServerMechanic(command)).is_err()
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
