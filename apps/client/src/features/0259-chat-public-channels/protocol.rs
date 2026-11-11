use crate::play::{Outbox, mailbox::lock};
use oteryn_session::{ChatLine, ChatOutcome, ChatRoomSet};

impl Outbox {
    pub(in crate::play) fn chat_public_initialize(
        &self,
        selected: bool,
        state: Option<(ChatRoomSet, Vec<ChatLine>)>,
    ) {
        let mut mailbox = lock(&self.0);
        mailbox.chat_public.selected = selected;
        if let Some((rooms, lines)) = state {
            mailbox
                .chat
                .console
                .metadata
                .observe(&lines, receipt_time());
            mailbox.chat.lines = lines.clone();
            mailbox.chat.console.sync_rooms(rooms);
            oteryn_client_ui::chat_public_channels::replace_state(
                &mut mailbox.chat_public,
                rooms,
                lines,
            );
        }
    }

    pub(in crate::play) fn chat_public_state(&self, rooms: ChatRoomSet, lines: Vec<ChatLine>) {
        let mut mailbox = lock(&self.0);
        mailbox
            .chat
            .console
            .metadata
            .observe(&lines, receipt_time());
        mailbox.chat.lines = lines.clone();
        mailbox.chat.console.sync_rooms(rooms);
        oteryn_client_ui::chat_public_channels::replace_state(
            &mut mailbox.chat_public,
            rooms,
            lines,
        );
    }

    pub(in crate::play) fn chat_public_result(
        &self,
        intent: &oteryn_session::ChatIntent,
        outcome: &ChatOutcome,
    ) {
        let mut mailbox = lock(&self.0);
        oteryn_client_ui::chat_public_channels::apply_result(&mut mailbox.chat_public, outcome);
        // A refused open leaves no tab to select; a room line sent from the console tab
        // ends its pending state here too.
        let refused = outcome.disposition != oteryn_session::ChatDisposition::Ok;
        let management = matches!(
            intent,
            oteryn_session::ChatIntent::OpenRoom(_) | oteryn_session::ChatIntent::CloseRoom(_)
        );
        if refused && management {
            mailbox.chat.console.focus = None;
        }
        if mailbox.chat.pending && matches!(intent, oteryn_session::ChatIntent::Room { .. }) {
            oteryn_client_ui::chat::apply_result(&mut mailbox.chat, outcome);
        }
    }
}

fn receipt_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
