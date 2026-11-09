use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::party::{Intent, request};

impl PlayLink {
    pub fn party_own_character(&self, name: &str) {
        lock(&self.mailbox).party.own_character_name = name.to_owned();
    }

    pub fn party_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.party, intent)
            && self.commands.send(PlayCommand::Party(command)).is_err()
        {
            mailbox.party.pending = None;
            mailbox.party.feedback = Some(tr!("The session has ended"));
        }
    }

    pub fn open_party(&self) {
        self.party_request(Intent::Refresh);
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).party.open = true;
        }
    }
}

impl crate::play::PlayView {
    /// Resolve only a currently drawn player, preserving its exact server generation.
    #[must_use]
    pub fn party_target(
        &self,
        tile: oteryn_renderer::TileCoord,
    ) -> Option<oteryn_session::EntityRef> {
        self.entities
            .iter()
            .rev()
            .find(|entry| {
                entry.kind == oteryn_session::EntityKind::Player
                    && entry.position.floor == self.floor
                    && oteryn_renderer::TileCoord::new(entry.position.x, entry.position.y) == tile
                    && tile != self.own
            })
            .map(|entry| entry.entity)
    }
}

impl PlayLink {
    pub fn party_input(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
        view: &crate::play::PlayView,
        input: &mut oteryn_client_ui::party::PartyInput,
    ) -> bool {
        let actions = input.route(events);
        let mut consumed = false;
        for action in actions {
            match action {
                oteryn_client_ui::party::Action::Open => {
                    self.open_party();
                    consumed = true;
                }
                oteryn_client_ui::party::Action::Context(x, y) => {
                    let target =
                        crate::input::click_tile(view.scene().view(), f64::from(x), f64::from(y))
                            .and_then(|tile| view.party_target(tile));
                    let Some(target) = target else {
                        continue;
                    };
                    consumed = true;
                    #[cfg(windows)]
                    super::view::context_menu(self, target);
                    #[cfg(not(windows))]
                    let _ = target;
                }
            }
        }
        consumed
    }
}

impl PlayLink {
    pub fn party_select_member(&self, row: usize) {
        let mut mailbox = lock(&self.mailbox);
        mailbox.party.selected_member = mailbox
            .party
            .state
            .as_ref()
            .and_then(|state| state.members.get(row))
            .map(|entry| entry.handle);
    }

    pub fn party_select_invitation(&self, row: usize) {
        let mut mailbox = lock(&self.mailbox);
        mailbox.party.selected_invitation = mailbox
            .party
            .state
            .as_ref()
            .and_then(|state| state.invitations.get(row))
            .map(|entry| entry.handle);
    }
}
