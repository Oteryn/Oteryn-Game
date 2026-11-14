use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::arena::ArenaPanel;
impl PlayLink {
    pub fn arena_panel(&self) -> ArenaPanel {
        lock(&self.mailbox).arena.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

/// A presentation snapshot tied to the actual session mailbox, never a second arena owner.
#[cfg(feature = "bevy-shell")]
#[derive(Clone)]
pub(crate) struct ArenaSnapshot {
    pub panel: ArenaPanel,
    session: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
}
#[cfg(feature = "bevy-shell")]
pub(crate) fn arena_needs_confirmation(intent: oteryn_client_ui::arena::Intent) -> bool {
    use oteryn_client_runtime::arena::{ArenaAction, DromeAction};
    use oteryn_client_ui::arena::Intent;
    !matches!(intent, Intent::Catalogue | Intent::Action(ArenaAction::Status | ArenaAction::Drome(DromeAction::Status)))
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn arena_snapshot(&self) -> ArenaSnapshot {
        ArenaSnapshot {
            panel: self.arena_panel(),
            session: std::sync::Arc::downgrade(&self.mailbox),
        }
    }
    pub(crate) fn arena_toggle_bound(&self, expected: &ArenaSnapshot) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !expected.session.ptr_eq(&std::sync::Arc::downgrade(&self.mailbox))
            || mailbox.arena != expected.panel
            || !mailbox.arena.selected
            || mailbox.server_modal.is_active()
            || self.commands.is_closed()
        {
            return false;
        }
        if mailbox.arena.open {
            mailbox.arena.open = false;
            return true;
        }
        mailbox.arena.open = true;
        // Reopening while a query is in flight must not send a second request.
        if mailbox.arena.pending.is_none()
            && let Some(command) = oteryn_client_ui::arena::request(
                &mut mailbox.arena, oteryn_client_ui::arena::Intent::Catalogue,
            )
            && self.commands.send(crate::combat_input::PlayCommand::Feature(command)).is_err()
        {
            mailbox.arena = expected.panel.clone();
            return false;
        }
        true
    }
    pub(crate) fn arena_dispatch_bound(
        &self,
        expected: &ArenaSnapshot,
        intent: oteryn_client_ui::arena::Intent,
        confirmed: bool,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !expected.session.ptr_eq(&std::sync::Arc::downgrade(&self.mailbox))
            || mailbox.arena != expected.panel
            || !mailbox.arena.open
            || !mailbox.arena.selected
            || mailbox.arena.pending.is_some()
            || mailbox.server_modal.is_active()
            || self.commands.is_closed()
            || arena_needs_confirmation(intent) && !confirmed
        {
            return false;
        }
        let Some(command) = oteryn_client_ui::arena::request(&mut mailbox.arena, intent) else {
            mailbox.arena = expected.panel.clone();
            return false;
        };
        if self.commands.send(crate::combat_input::PlayCommand::Feature(command)).is_err() {
            mailbox.arena = expected.panel.clone();
            return false;
        }
        true
    }
}
