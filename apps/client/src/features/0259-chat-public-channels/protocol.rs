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
        oteryn_client_ui::chat_public_channels::replace_state(
            &mut mailbox.chat_public,
            rooms,
            lines,
        );
    }

    pub(in crate::play) fn chat_public_result(&self, outcome: &ChatOutcome) {
        oteryn_client_ui::chat_public_channels::apply_result(
            &mut lock(&self.0).chat_public,
            outcome,
        );
    }
}

fn receipt_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
