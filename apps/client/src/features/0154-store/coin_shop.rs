use crate::coin_shop::launcher::Launcher;
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use std::sync::Arc;

impl PlayLink {
    pub(crate) fn configure_coin_shop(
        &self,
        config: Option<crate::NativeLoginConfig>,
        character_id: &str,
    ) {
        let Some(config) = config else {
            return;
        };
        let weak = Arc::downgrade(&self.mailbox);
        let commands = self.commands.clone();
        let after = Arc::new(move || {
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
        lock(&self.mailbox).coin_shop = Some(Launcher::new(config, character_id.to_owned(), after));
    }
}
