#[cfg(all(windows, not(test)))]
#[path = "view/notices.rs"]
mod notices;
use crate::play::{PlayView, ServerMessageInbox};
use oteryn_session::{AppliedDelta, MAX_QUEUED_EVENTS, ServerMessage};

impl PlayView {
    pub(in crate::play) fn apply_server_messages(&mut self, messages: &ServerMessageInbox) {
        for message in messages.iter() {
            if self.shared_frontend {
                self.screen_messages.show(&message.value, std::time::Instant::now());
            } else {
                self.game_notifications.observe(message);
            }
            if self.server_messages.len() == MAX_QUEUED_EVENTS {
                self.server_messages.pop_front();
            }
            self.server_messages.push_back(message.clone());
        }
        #[cfg(all(windows, not(test)))]
        if !self.shared_frontend {
            for notice in messages.interactive_notices() {
                notices::enqueue(notice);
            }
        }
    }

    /// Recent notices in delivery order, reset with the admitted play view on reconnect.
    pub fn server_messages(&self) -> impl Iterator<Item = &AppliedDelta<ServerMessage>> {
        self.server_messages.iter()
    }

    /// The text of one on-screen place of the shared frontend, while it is visible and banners are on.
    #[must_use]
    pub fn screen_message(&self, place: crate::play::feature_local_game_notifications::state::ScreenPlace) -> Option<String> {
        if !self.preferences().game_window.banner {
            return None;
        }
        self.screen_messages
            .text(place, std::time::Instant::now())
            .map(str::to_owned)
    }

    /// The existing native status surface displays the latest notice without erasing combat.
    #[must_use]
    pub fn server_message_status(&self) -> Option<String> {
        if !self.preferences().game_window.banner {
            return None;
        }
        self.server_messages
            .back()
            .map(|message| match &message.value {
                ServerMessage::StatusText(text) => text.clone(),
                ServerMessage::Warning(text) => lfmt!("Warning: {text}", text = text),
                ServerMessage::Advance(text) => lfmt!("Advance: {text}", text = text),
            })
    }
}
