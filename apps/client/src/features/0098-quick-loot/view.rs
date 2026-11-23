use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn discovery_panel(&self) -> oteryn_client_ui::quick_loot::Discovery {
        let mailbox = lock(&self.mailbox);
        let mut projection = mailbox.discovery.clone();
        if !mailbox.local_preferences.game_window.loot_messages {
            projection.loot.result = None;
        }
        projection
    }
    pub fn discovery_open(&self, panel: u32) {
        let mut mailbox = lock(&self.mailbox);
        match panel {
            98 => mailbox.discovery.loot.open = true,
            132 => mailbox.discovery.inspection.open = true,
            232 => mailbox.discovery.teams.open = true,
            285 => mailbox.discovery.history.open = true,
            _ => {}
        }
    }
}

/// The existing panel projection, including the player's loot-message visibility choice.
fn loot_panel(mailbox: &crate::play::mailbox::Mailbox) -> oteryn_client_ui::quick_loot::Panel {
    let mut panel = mailbox.discovery.loot.clone();
    if !mailbox.local_preferences.game_window.loot_messages {
        panel.result = None;
    }
    panel
}
#[cfg(bevy_shell)]
#[derive(Clone, PartialEq, Eq)]
pub enum QuickLootControl {
    Request(oteryn_client_ui::quick_loot::Action),
    OpenDestination(Option<oteryn_client_ui::quick_loot::QuickLootCategory>),
    Catalogue,
    Store,
    Close,
}
impl PlayLink {
    #[must_use]
    pub fn quick_loot_panel(&self) -> oteryn_client_ui::quick_loot::Panel {
        loot_panel(&lock(&self.mailbox))
    }
    #[cfg(bevy_shell)]
    pub fn quick_loot_dispatch_bound(
        &self,
        expected: &oteryn_client_ui::quick_loot::Panel,
        control: QuickLootControl,
    ) -> bool {
        use oteryn_client_ui::quick_loot::{Action, QuickLootCategory, QuickLootRecipient, request};
        let mut mailbox = lock(&self.mailbox);
        if loot_panel(&mailbox) != *expected || !mailbox.discovery.loot.open {
            return false;
        }
        let panel = &mut mailbox.discovery.loot;
        let action = match control {
            QuickLootControl::OpenDestination(category) => {
                if !panel.selected || panel.pending.is_some() || !panel.destinations.iter().any(|entry|
                    matches!(entry, Action::Destination {category: assigned, recipient: QuickLootRecipient::MainContainer, ..} if *assigned == category)) {
                    return false;
                }
                return self.quick_loot_open_main(&mut mailbox);
            }
            QuickLootControl::Close => {
                panel.open = false;
                return true;
            }
            QuickLootControl::Catalogue | QuickLootControl::Store => {
                if !panel.selected || panel.pending.is_some() {
                    return false;
                }
                panel.open = false;
                drop(mailbox);
                if control == QuickLootControl::Store {
                    self.store_open_products();
                } else {
                    self.item_catalogue_open();
                }
                return true;
            }
            QuickLootControl::Request(action) => action,
        };
        if !panel.selected || panel.pending.is_some() {
            return false;
        }
        let allowed = match &action {
            Action::LootAll | Action::Select(_) | Action::Clear(_) => true,
            Action::Destination { category, recipient, id, .. } =>
                (id.is_none() && (*recipient == QuickLootRecipient::MainContainer
                    || (*recipient == QuickLootRecipient::GoldPouch
                        && matches!(category, None | Some(QuickLootCategory::Gold)))))
                || panel.destinations.iter().any(|current| matches!(current,
                    Action::Destination {category: c,recipient: r,id: i,..} if c==category && r==recipient && i==id)),
            _ => false,
        };
        if !allowed {
            return false;
        }
        let Some(command) = request(panel, action) else {
            return false;
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::ServerMechanic(command))
            .is_err()
        {
            panel.pending = None;
            panel.feedback = tr!("Session ended").into();
            return false;
        }
        true
    }
}

#[cfg(all(test, bevy_shell))]
impl crate::play::Outbox {
    pub(crate) fn bevy_quick_loot_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
    pub(crate) fn bevy_quick_loot_selected(&self, selected: bool) {
        lock(&self.0).discovery.loot.selected = selected;
    }
    pub(crate) fn bevy_quick_loot_response(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        self.discovery_result(98, response);
    }
}
