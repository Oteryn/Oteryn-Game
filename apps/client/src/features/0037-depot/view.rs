use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::depot::{DepotPanel, Intent};
impl PlayLink {
    pub fn depot_panel(&self) -> DepotPanel {
        lock(&self.mailbox).depot.clone()
    }
    pub fn open_depot(&self) {
        self.depot_request(Intent::Browse {
            box_number: 1,
            page: 0,
        });
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            native::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).depot.open = true;
        }
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;

/// A displayed snapshot is tied to the connection that produced it, not just equal contents.
#[cfg(feature = "bevy-shell")]
#[derive(Clone)]
pub(crate) struct DepotBinding<T> {
    pub panel: T,
    generation: u64,
    session: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
}
#[cfg(feature = "bevy-shell")]
impl<T> DepotBinding<T> {
    pub(in crate::play) fn new(link: &PlayLink, generation: u64, panel: T) -> Self {
        Self {
            panel,
            generation,
            session: std::sync::Arc::downgrade(&link.mailbox),
        }
    }
    pub(crate) fn same(&self, other: &Self) -> bool
    where
        T: PartialEq,
    {
        self.generation == other.generation
            && self.session.ptr_eq(&other.session)
            && self.panel == other.panel
    }
    pub(in crate::play) fn matches(&self, link: &PlayLink, generation: u64) -> bool {
        self.generation == generation
            && self
                .session
                .ptr_eq(&std::sync::Arc::downgrade(&link.mailbox))
    }
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn depot_binding(&self, generation: u64) -> DepotBinding<DepotPanel> {
        DepotBinding::new(self, generation, lock(&self.mailbox).depot.clone())
    }
    pub(crate) fn depot_dispatch_bound(
        &self,
        expected: &DepotBinding<DepotPanel>,
        generation: u64,
        intent: Option<Intent>,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        // Closing presentation preserves every in-flight operation and receipt.
        if intent.is_none() {
            if !expected.matches(self, generation)
                || self.commands.is_closed()
                || mailbox.depot_shutdown
                || mailbox.quest_shutdown
                || mailbox.custom_hotkeys_ended
            {
                return false;
            }
            mailbox.depot.open = false;
            return true;
        }

        if !expected.matches(self, generation)
            || self.commands.is_closed()
            || mailbox.depot_shutdown
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
            || mailbox.activity_reset_commands
            || mailbox.depot != expected.panel
            || !mailbox.depot.open
            || !mailbox.depot.selected
            || mailbox.depot.pending
        {
            return false;
        }
        let Some(intent) = intent else {
            mailbox.depot.open = false;
            return true;
        };
        let mut prepared = mailbox.depot.clone();
        let Some(command) = oteryn_client_ui::depot::request(&mut prepared, intent) else {
            return false;
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::Depot(command))
            .is_err()
        {
            return false;
        }
        mailbox.depot = prepared;
        true
    }
}
#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_depot_fixture(&self) {
        self.depot_initialize(true);
        self.depot_result(Some(oteryn_client_ui::depot::DepotReply {
            outcome: oteryn_client_ui::depot::DepotOutcome::Opened,
            town: Some(2),
            box_number: 1,
            page: 0,
            total: 1,
            entries: vec![oteryn_client_runtime::depot::DepotEntry {
                ordinal: 91,
                definition: 42,
                count: 20,
            }],
        }));
        self.stash_initialize(true);
        self.stash_result(&oteryn_client_runtime::stash::StashResponse {
            status: oteryn_session::CommandStatus::Accepted,
            reply: oteryn_client_ui::stash::stash::StashReply {
                outcome: oteryn_client_ui::stash::stash::StashOutcome::Opened,
                stowed_quantity: 0,
                transaction: None,
                page: 0,
                has_more: true,
                rows: vec![oteryn_client_ui::stash::stash::StashRow {
                    handle: [3; 16],
                    definition_key: "Test supply".into(),
                    definition_revision: "1".into(),
                    quantity: 80,
                    migration_required: false,
                }],
            },
        });
        self.depot_search_initialize(true);
        let mut mailbox = lock(&self.0);
        mailbox.depot.items.views = vec![oteryn_session::ItemEntry {
            handle: std::num::NonZeroU64::MIN,
            item_definition_ref: std::num::NonZeroU32::MIN,
            count: std::num::NonZeroU32::MIN,
            sub_type: 0,
        }];
        mailbox.depot.open = true;
        mailbox.stash.open = true;
        mailbox.depot_search.open = true;
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
mod bound_tests {
    use super::*;
    #[test]
    fn depot_bound_rejects_modal_closed_pending_and_changed_snapshot() {
        let (link, mut commands, outbox) = crate::play::play_channel();
        outbox.bevy_depot_fixture();
        let bound = link.depot_binding(7);
        let modal = link.bevy_server_modal_fixture();
        assert!(!link.depot_dispatch_bound(&bound, 7, Some(Intent::Withdraw(0))));
        assert_eq!(link.depot_panel(), bound.panel);
        drop(modal);
        {
            lock(&outbox.0).depot.reply = None;
        }
        let changed = link.depot_panel();
        assert!(!link.depot_dispatch_bound(&bound, 7, Some(Intent::Withdraw(0))));
        assert_eq!(link.depot_panel(), changed);
        outbox.bevy_depot_fixture();
        assert!(link.depot_dispatch_bound(&bound, 7, Some(Intent::Withdraw(0))));
        assert!(!link.depot_dispatch_bound(&bound, 7, Some(Intent::Withdraw(0))));
        assert!(commands.try_recv().is_ok());
        assert!(commands.try_recv().is_err());
        outbox.bevy_depot_fixture();
        lock(&outbox.0).depot_shutdown = true;
        assert!(!link.depot_dispatch_bound(&bound, 7, Some(Intent::Withdraw(0))));
        assert_eq!(link.depot_panel(), bound.panel);
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_depot_window(&self, window: usize) {
        let mut mailbox = lock(&self.0);
        mailbox.depot.open = window == 0;
        mailbox.stash.open = window == 1;
        mailbox.depot_search.open = window == 2;
    }
    pub(crate) fn bevy_depot_reply(&self, reply: Option<DepotPanelReply>) {
        self.depot_result(reply);
    }
}
#[cfg(all(test, feature = "bevy-shell"))]
type DepotPanelReply = oteryn_client_ui::depot::DepotReply;

#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn bevy_depot_input(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> bool {
        if !oteryn_client_ui::depot::open_key(events) {
            return false;
        }
        let mut mailbox = lock(&self.mailbox);
        if self.commands.is_closed()
            || mailbox.depot_shutdown
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
            || mailbox.activity_reset_commands
        {
            return true;
        }
        mailbox.depot.open = true;
        if !mailbox.depot.selected || mailbox.depot.pending {
            return true;
        }
        let mut prepared = mailbox.depot.clone();
        if let Some(command) = oteryn_client_ui::depot::request(
            &mut prepared,
            Intent::Browse {
                box_number: 1,
                page: 0,
            },
        ) && self
            .commands
            .send(crate::combat_input::PlayCommand::Depot(command))
            .is_ok()
        {
            prepared.open = true;
            mailbox.depot = prepared;
        }
        true
    }
}
