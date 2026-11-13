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
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            native::open_window(self, panel);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
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
}

/// The existing panel projection, including the player's loot-message visibility choice.
fn loot_panel(mailbox: &crate::play::mailbox::Mailbox) -> oteryn_client_ui::quick_loot::Panel {
    let mut panel = mailbox.discovery.loot.clone();
    if !mailbox.local_preferences.game_window.loot_messages {
        panel.result = None;
    }
    panel
}
#[cfg(feature = "bevy-shell")]
#[derive(Clone, PartialEq, Eq)]
pub enum QuickLootControl {
    Request(oteryn_client_ui::quick_loot::Action),
    Close,
}
impl PlayLink {
    #[must_use]
    pub fn quick_loot_panel(&self) -> oteryn_client_ui::quick_loot::Panel {
        loot_panel(&lock(&self.mailbox))
    }
    #[cfg(feature = "bevy-shell")]
    pub fn quick_loot_dispatch_bound(
        &self,
        expected: &oteryn_client_ui::quick_loot::Panel,
        control: QuickLootControl,
    ) -> bool {
        use oteryn_client_ui::quick_loot::{Action, QuickLootRecipient, request};
        let mut mailbox = lock(&self.mailbox);
        if loot_panel(&mailbox) != *expected || !mailbox.discovery.loot.open {
            return false;
        }
        let panel = &mut mailbox.discovery.loot;
        let QuickLootControl::Request(action) = control else {
            panel.open = false;
            return true;
        };
        if !panel.selected || panel.pending.is_some() {
            return false;
        }
        let allowed = match &action {
            Action::LootAll | Action::Select(_) | Action::Clear(_) => true,
            Action::Destination { category, recipient, id, .. } =>
                (*recipient == QuickLootRecipient::MainContainer && id.is_none())
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
#[cfg(windows)]
#[path = "native.rs"]
mod native;

#[cfg(windows)]
pub(in crate::play) use native_context::{available, choose, decorate};
#[cfg(windows)]
#[path = "native_context.rs"]
mod native_context;

#[cfg(all(test, feature = "bevy-shell"))]
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
