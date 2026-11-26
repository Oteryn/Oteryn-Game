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

    pub(in crate::play) fn party_notice_refresh(&self, now: std::time::Instant) {
        use oteryn_client_ui::party::PartyCommand;
        use oteryn_client_ui::party_membership_notices::take_refresh;

        let mut mailbox = lock(&self.mailbox);
        let mailbox = &mut *mailbox;
        if !mailbox.party_initialized || mailbox.scope_pending || mailbox.activity_reset_commands {
            return;
        }
        if take_refresh(&mailbox.party, &mut mailbox.party_notice_refresh, now) {
            let command = PartyCommand::query();
            if self.commands.send(PlayCommand::Party(command.clone())).is_ok() {
                mailbox.party.pending = Some(command);
            } else {
                mailbox.party_notice_refresh.cancel_query();
            }
        }
    }

    pub fn open_party(&self) {
        self.party_request(Intent::Refresh);
        self.open_shared_party();
    }

    pub(in crate::play) fn open_shared_party(&self) {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.party.open { return; }
        mailbox.party.open = true;
        drop(mailbox);
        let mailbox = std::sync::Arc::downgrade(&self.mailbox);
        let commands = self.commands.downgrade();
        if std::thread::Builder::new().name("party-refresh".into()).spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                let (Some(mailbox), Some(commands)) = (mailbox.upgrade(), commands.upgrade()) else { break; };
                let mut mailbox = lock(&mailbox);
                if !mailbox.party.open || !mailbox.party.selected || commands.is_closed() { break; }
                if mailbox.party.pending.is_some() || mailbox.party.received_at.is_some_and(|at| at.elapsed() < std::time::Duration::from_secs(1)) { continue; }
                if let Some(command) = request(&mut mailbox.party, Intent::Refresh)
                    && commands.send(PlayCommand::Party(command)).is_err() {
                    mailbox.party.pending = None;
                    mailbox.party.feedback = Some(tr!("The session has ended"));
                    break;
                }
            }
        }).is_err() {
            let mut mailbox = lock(&self.mailbox);
            mailbox.party.open = false;
            mailbox.party.feedback = Some(tr!("The current party view is unavailable; refresh again"));
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
                    && Some(entry.entity.identity) != self.scene.own_identity()
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
                        view.click_tile(f64::from(x), f64::from(y))
                            .and_then(|tile| view.party_target(tile));
                    let Some(target) = target else {
                        continue;
                    };
                    consumed = true;
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
        let visible = mailbox.party.list_rows();
        let selected = if let Some((handle, entry)) = visible.get(row) {
            (!entry.summon).then_some(*handle)
        } else if mailbox.party.list_hidden[0] {
            None
        } else {
            mailbox.party.state.as_ref().and_then(|state| {
                state
                    .members
                    .iter()
                    .filter(|entry| entry.list.is_empty())
                    .nth(row - visible.len())
                    .map(|entry| entry.handle)
            })
        };
        mailbox.party.selected_member = selected;
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

impl PlayLink {
    pub fn party_list_action(&self, action: oteryn_client_ui::party::ListAction) {
        lock(&self.mailbox).party.list_action(action);
    }
}
