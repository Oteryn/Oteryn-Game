use crate::coin_shop::launcher::Launcher;
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use std::sync::Arc;

impl PlayLink {
    #[cfg(bevy_shell)]
    pub(crate) fn coin_checkout_available(&self) -> bool {
        lock(&self.mailbox)
            .connection
            .coin_shop
            .as_ref()
            .is_some_and(|shop| shop.can_start())
    }

    #[cfg(bevy_shell)]
    pub(crate) fn open_coin_checkout(&self) -> bool {
        let mailbox = lock(&self.mailbox);
        !mailbox.scope_pending
            && mailbox
                .connection
                .coin_shop
                .as_ref()
                .is_some_and(|shop| shop.start())
    }

    pub(crate) fn configure_coin_shop(
        &self,
        config: Option<crate::NativeLoginConfig>,
        character_id: &str,
    ) {
        let Some(config) = config else {
            return;
        };
        let weak = Arc::downgrade(&self.mailbox);
        let commands = self.commands.downgrade();
        let after = Arc::new(move || {
            let Some(commands) = commands.upgrade() else {
                return;
            };
            let command = weak.upgrade().and_then(|shared| {
                let mut mailbox = lock(&shared);
                if commands.is_closed() {
                    return None;
                }
                let panel = &mut mailbox.store;
                if !panel.selected || panel.pending {
                    return None;
                }
                panel.catalogue = None;
                let b = panel.browser.clone();
                oteryn_client_ui::store::browse(panel, &b.category, &b.search, b.sort, 0)
            });
            if let Some(command) = command {
                let _ = commands.send(PlayCommand::ServerMechanic(command));
            }
        });
        lock(&self.mailbox).connection.coin_shop =
            Some(Launcher::new(config, character_id.to_owned(), after));
    }
}
