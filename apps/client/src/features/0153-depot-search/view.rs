use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[must_use]
    pub fn depot_search_panel(&self) -> oteryn_client_ui::depot_search::DepotSearchPanel {
        lock(&self.mailbox).depot_search.clone()
    }
}
#[cfg(any(windows, test))]
#[path = "native.rs"]
pub(in crate::play) mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[cfg(feature = "bevy-shell")]
use crate::play::feature_0037_depot::view::DepotBinding;
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn depot_search_binding(
        &self,
        generation: u64,
    ) -> DepotBinding<oteryn_client_ui::depot_search::DepotSearchPanel> {
        DepotBinding::new(self, generation, lock(&self.mailbox).depot_search.clone())
    }
    pub(crate) fn depot_search_dispatch_bound(
        &self,
        expected: &DepotBinding<oteryn_client_ui::depot_search::DepotSearchPanel>,
        generation: u64,
        query: Option<u32>,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        // Closing presentation preserves every in-flight operation and receipt.
        if query.is_none() {
            if !expected.matches(self, generation)
                || self.commands.is_closed()
                || mailbox.depot_shutdown
                || mailbox.quest_shutdown
                || mailbox.custom_hotkeys_ended
            {
                return false;
            }
            mailbox.depot_search.open = false;
            return true;
        }

        if !expected.matches(self, generation)
            || self.commands.is_closed()
            || mailbox.depot_shutdown
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
            || mailbox.activity_reset_commands
            || mailbox.depot_search != expected.panel
            || !mailbox.depot_search.open
            || !mailbox.depot_search.selected
            || mailbox.depot_search.pending
        {
            return false;
        }
        let Some(query) = query else {
            mailbox.depot_search.open = false;
            return true;
        };
        let mut prepared = mailbox.depot_search.clone();
        prepared.item_id = query;
        let Some(command) = oteryn_client_ui::depot_search::search(&mut prepared) else {
            return false;
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::ServerMechanic(command))
            .is_err()
        {
            return false;
        }
        mailbox.depot_search = prepared;
        true
    }
}
#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_depot_search_reply(&self) -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_client_runtime::cui3::depot_search::{
            self, Outcome, Reply, SearchResult, TierCount,
        };
        self.depot_search_result(
            &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
                command_id: 1,
                server_sequence: 1,
                status: oteryn_session::CommandStatus::Accepted,
                payload: depot_search::encode_reply(&Reply {
                    outcome: Outcome::Found,
                    retrieved: 0,
                    results: vec![SearchResult {
                        client_id: 42,
                        depot: 5,
                        stash: 3,
                        inbox: 0,
                        tiers: vec![TierCount { tier: 0, count: 8 }],
                    }],
                })
                .map_err(|error| format!("{error:?}"))?,
            },
        );
        Ok(())
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
mod bound_tests {
    #[test]
    fn depot_search_bound_rejects_modal_pending_closed_and_replaced_session() {
        let (link, mut commands, outbox) = crate::play::play_channel();
        outbox.bevy_depot_fixture();
        let bound = link.depot_search_binding(7);
        let modal = link.bevy_server_modal_fixture();
        assert!(!link.depot_search_dispatch_bound(&bound, 7, Some(42)));
        assert_eq!(link.depot_search_panel(), bound.panel);
        drop(modal);
        let (other, _, other_outbox) = crate::play::play_channel();
        other_outbox.bevy_depot_fixture();
        assert!(!other.depot_search_dispatch_bound(&bound, 7, Some(42)));
        assert!(link.depot_search_dispatch_bound(&bound, 7, Some(42)));
        assert!(!link.depot_search_dispatch_bound(&bound, 7, Some(42)));
        assert!(commands.try_recv().is_ok());
        outbox.bevy_depot_fixture();
        commands.close();
        let current = link.depot_search_binding(7);
        assert!(!link.depot_search_dispatch_bound(&current, 7, Some(42)));
        assert_eq!(link.depot_search_panel(), current.panel);
    }
}

#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn bevy_depot_search_input(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> bool {
        if !oteryn_client_ui::depot_search::open_key(events) {
            return false;
        }
        let mut mailbox = lock(&self.mailbox);
        if !self.commands.is_closed()
            && !mailbox.depot_shutdown
            && !mailbox.server_modal.is_active()
            && !mailbox.world_input_blocked()
            && !mailbox.activity_reset_commands
        {
            mailbox.depot_search.open = true;
        }
        true
    }
}
