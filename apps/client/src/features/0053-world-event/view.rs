use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::world_event::WorldEventPanel;
impl PlayLink {
    pub fn world_event_panel(&self) -> WorldEventPanel {
        lock(&self.mailbox).world_event.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

/// A displayed query/status view belongs to this exact retained session mailbox.
#[cfg(feature = "bevy-shell")]
#[derive(Clone)]
pub(crate) struct WorldEventBinding {
    session: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    pub(crate) panel: WorldEventPanel,
}
#[cfg(feature = "bevy-shell")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorldEventControl {
    Toggle,
    Refresh,
    Close,
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn world_event_binding(&self) -> Option<WorldEventBinding> {
        let mailbox = lock(&self.mailbox);
        if mailbox.world_event_session_closed() || self.commands.is_closed() {
            return None;
        }
        Some(WorldEventBinding {
            session: std::sync::Arc::downgrade(&self.mailbox),
            panel: mailbox.world_event.clone(),
        })
    }
    /// Revalidate and enqueue under the same owner lock; rejected actions change nothing.
    pub(crate) fn world_event_dispatch_bound(
        &self,
        expected: &WorldEventBinding,
        action: WorldEventControl,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !expected
            .session
            .ptr_eq(&std::sync::Arc::downgrade(&self.mailbox))
            || mailbox.world_event_session_closed()
            || self.commands.is_closed()
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || mailbox.world_event != expected.panel
        {
            return false;
        }
        let panel = &mut mailbox.world_event;
        if action == WorldEventControl::Close || (action == WorldEventControl::Toggle && panel.open)
        {
            if !panel.open {
                return false;
            }
            panel.open = false;
            return true;
        }
        if action == WorldEventControl::Toggle && !panel.selected && !panel.pending {
            panel.open = true;
            return true;
        }
        if !panel.selected || panel.pending || (action == WorldEventControl::Refresh && !panel.open)
        {
            return false;
        }
        let mut next = panel.clone();
        let Some(command) = oteryn_client_ui::world_event::request(&mut next) else {
            return false;
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::Feature(command))
            .is_err()
        {
            return false;
        }
        next.open = true;
        *panel = next;
        true
    }
}
#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_world_event_selected(&self, selected: bool) {
        self.world_event_initialize(selected);
    }
    pub(crate) fn bevy_world_event_result(
        &self,
        result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        self.world_event_result(result);
    }
    pub(crate) fn bevy_world_event_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
#[path = "tests/bound.rs"]
mod bound_tests;

#[cfg(feature = "bevy-shell")]
impl crate::play::mailbox::Mailbox {
    pub(in crate::play) fn world_event_session_closed(&self) -> bool {
        // The session end reducer closes inventory before the owning PlayLink is dropped.
        self.inventory_shutdown || self.world_event_shutdown
    }
}
