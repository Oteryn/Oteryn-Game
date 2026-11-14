use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::prey::PreyPanel;

impl PlayLink {
    #[must_use]
    pub fn prey_panel(&self) -> PreyPanel {
        lock(&self.mailbox).prey.clone()
    }
}

#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub fn prey_dispatch_bound(
        &self,
        expected: &PreyPanel,
        intent: Option<oteryn_client_ui::prey::Intent>,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if self.commands.is_closed() || mailbox.prey_shutdown || !mailbox.prey.open {
            return false;
        }
        if intent.is_none() {
            mailbox.prey.open = false;
            return true;
        }
        if crate::play::feature_0450_server_modal::view::active()
            || mailbox.world_input_blocked()
            || mailbox.prey != *expected
            || !expected.selected
            || expected.pending.is_some()
        {
            return false;
        }
        if let Some(intent) = intent {
            if let oteryn_client_ui::prey::Intent::ConfirmPickAgain(quote) = &intent {
                if mailbox.prey.offered_slot != Some(quote.slot) {
                    return false;
                }
                let Some(row) = mailbox.prey.offered_today.iter().position(|row| {
                    row.creature == quote.creature
                        && row.price_gold == quote.price_gold
                        && (row.occupied_slot == 0 || row.occupied_slot == quote.slot)
                }) else {
                    return false;
                };
                if mailbox.prey.confirm_offer(row).as_ref() != Some(quote) {
                    return false;
                }
            }

            super::input::enqueue(&mut mailbox, &self.commands, intent);
        } else {
            mailbox.prey.open = false;
        }
        true
    }
}

#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub fn prey_choose_bound(&self, expected: &PreyPanel, slot: u8, index: u8) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if self.commands.is_closed()
            || crate::play::feature_0450_server_modal::view::active()
            || mailbox.prey_shutdown
            || mailbox.world_input_blocked()
            || mailbox.prey != *expected
            || !expected.open
        {
            return false;
        }
        mailbox.prey.choose_candidate(slot, index)
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_prey_initialize(&self, selected: bool) {
        self.prey_initialize(selected);
    }
    pub(crate) fn bevy_prey_response(&self, response: &oteryn_client_runtime::prey::PreyResponse) {
        self.prey_result(response);
    }
    pub(crate) fn bevy_prey_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}

#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub fn prey_open_bound(&self, expected: &PreyPanel) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if self.commands.is_closed()
            || mailbox.prey_shutdown
            || mailbox.prey != *expected
            || mailbox.world_input_blocked()
            || crate::play::feature_0450_server_modal::view::active()
        {
            return false;
        }
        super::input::enqueue(
            &mut mailbox,
            &self.commands,
            oteryn_client_ui::prey::Intent::Refresh,
        );
        mailbox.prey.open = true;
        true
    }
}
