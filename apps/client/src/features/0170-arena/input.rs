use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::arena::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn arena_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.arena, intent)
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.arena.pending = None;
            mailbox.arena.feedback = Some("The session has ended");
        }
    }
    pub fn arena_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::arena::open_key(events) {
            return false;
        }
        self.arena_request(Intent::Catalogue);
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).arena.open = true;
        }
        true
    }
}

impl PlayLink {
    pub(in crate::play) fn arena_refresh(&self, now: std::time::Instant) {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.arena;
        if !panel.selected
            || panel.pending.is_some()
            || !panel
                .drome
                .is_some_and(|reply| matches!(reply.phase, 1 | 2))
            || panel.refreshed_at.is_some_and(|last| {
                now.saturating_duration_since(last) < std::time::Duration::from_secs(1)
            })
        {
            return;
        }
        panel.refreshed_at = Some(now);
        if let Some(command) = request(
            panel,
            Intent::Action(oteryn_client_runtime::arena::ArenaAction::Drome(
                oteryn_client_runtime::arena::DromeAction::Status,
            )),
        ) && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.arena.pending = None;
        }
    }
}
