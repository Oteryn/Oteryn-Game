use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::stash::StashPanel;
impl PlayLink {
    pub fn stash_panel(&self) -> StashPanel {
        lock(&self.mailbox).stash.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[cfg(feature = "bevy-shell")]
use crate::play::feature_0037_depot::view::DepotBinding;
#[cfg(feature = "bevy-shell")]
#[derive(Clone)]
pub(crate) enum StashControl {
    Request(oteryn_client_ui::stash::Intent),
    Filter(String),
    Close,
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn stash_binding(&self, generation: u64) -> DepotBinding<StashPanel> {
        DepotBinding::new(self, generation, lock(&self.mailbox).stash.clone())
    }
    pub(crate) fn stash_dispatch_bound(
        &self,
        expected: &DepotBinding<StashPanel>,
        generation: u64,
        control: StashControl,
    ) -> bool {
        use oteryn_client_ui::stash::{Intent, request};
        let mut mailbox = lock(&self.mailbox);
        if !expected.matches(self, generation)
            || self.commands.is_closed()
            || mailbox.depot_shutdown
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
            || mailbox.activity_reset_commands
            || mailbox.stash != expected.panel
            || !mailbox.stash.open
            || !mailbox.stash.selected
            || mailbox.stash.pending
        {
            return false;
        }
        let mut prepared = mailbox.stash.clone();
        let intent = match control {
            StashControl::Close => {
                mailbox.stash.open = false;
                return true;
            }
            StashControl::Filter(query) => {
                prepared.query = query;
                Intent::Filter
            }
            StashControl::Request(intent) => intent,
        };
        if let Intent::Withdraw { row, amount } = intent
            && (amount == 0
                || prepared
                    .rows
                    .get(row)
                    .is_none_or(|row| amount > row.quantity || row.migration_required))
        {
            return false;
        }
        let Some(command) = request(&mut prepared, intent) else {
            return false;
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::Stash(command))
            .is_err()
        {
            return false;
        }
        mailbox.stash = prepared;
        true
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
mod bound_tests {
    use super::*;
    use oteryn_client_ui::stash::Intent;
    #[test]
    fn stash_bound_rejects_invalid_quantity_filter_modal_and_old_rows() {
        let (link, mut commands, outbox) = crate::play::play_channel();
        outbox.bevy_depot_fixture();
        let bound = link.stash_binding(7);
        for amount in [0, 81] {
            assert!(!link.stash_dispatch_bound(
                &bound,
                7,
                StashControl::Request(Intent::Withdraw { row: 0, amount })
            ));
            assert_eq!(link.stash_panel(), bound.panel);
        }
        assert!(!link.stash_dispatch_bound(&bound, 7, StashControl::Filter("a\nb".into())));
        assert_eq!(link.stash_panel(), bound.panel);
        let modal = link.bevy_server_modal_fixture();
        assert!(!link.stash_dispatch_bound(&bound, 7, StashControl::Request(Intent::Next)));
        assert_eq!(link.stash_panel(), bound.panel);
        drop(modal);
        lock(&outbox.0).stash.rows[0].handle = [4; 16];
        let changed = link.stash_panel();
        assert!(!link.stash_dispatch_bound(
            &bound,
            7,
            StashControl::Request(Intent::Withdraw { row: 0, amount: 1 })
        ));
        assert_eq!(link.stash_panel(), changed);
        assert!(commands.try_recv().is_err());
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_stash_page(&self, page: u32) {
        let rows = lock(&self.0).stash.rows.clone();
        self.stash_result(&oteryn_client_runtime::stash::StashResponse {
            status: oteryn_session::CommandStatus::Accepted,
            reply: oteryn_client_ui::stash::stash::StashReply {
                outcome: oteryn_client_ui::stash::stash::StashOutcome::Opened,
                stowed_quantity: 0,
                transaction: None,
                page,
                has_more: false,
                rows,
            },
        });
    }
}

#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn bevy_stash_input(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> bool {
        if !oteryn_client_ui::stash::open_key(events) {
            return false;
        }
        let mut mailbox = lock(&self.mailbox);
        if self.commands.is_closed()
            || mailbox.depot_shutdown
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
            || mailbox.activity_reset_commands
            || !mailbox.stash.selected
            || mailbox.stash.pending
        {
            return true;
        }
        let mut prepared = mailbox.stash.clone();
        if let Some(command) = oteryn_client_ui::stash::request(
            &mut prepared,
            oteryn_client_ui::stash::Intent::Refresh,
        ) && self
            .commands
            .send(crate::combat_input::PlayCommand::Stash(command))
            .is_ok()
        {
            prepared.open = true;
            mailbox.stash = prepared;
        }
        true
    }
}
