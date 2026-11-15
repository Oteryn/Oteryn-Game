use crate::play::{Outbox, Pushed, mailbox::lock};
use oteryn_session::{AppliedDelta, ChatLine, ChatRoomSet};
impl Outbox {
    pub(in crate::play) fn chat_party_room_initialize(&self, selected: bool) {
        lock(&self.0).chat_party_room.selected = selected;
    }
    pub(in crate::play) fn chat_party_room_result(&self, outcome: &oteryn_session::ChatOutcome) {
        let mut mailbox = lock(&self.0);
        oteryn_client_ui::chat_party_room::apply_result(&mut mailbox.chat_party_room);
        let mut result = oteryn_client_ui::chat_public_channels::ChatPanel::default();
        oteryn_client_ui::chat_public_channels::apply_result(&mut result, outcome);
        mailbox.chat_party_room.feedback = result.feedback;
    }
    pub(in crate::play) fn chat_result(
        &self,
        intent: &oteryn_session::ChatIntent,
        outcome: &oteryn_session::ChatOutcome,
    ) {
        if self.chat_private_result(intent, outcome) {
            return;
        }
        // Clear the accepted draft before releasing pending to the native composer.
        if matches!(
            outcome.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        ) && outcome.disposition == oteryn_session::ChatDisposition::Ok
        {
            let mut mailbox = lock(&self.0);
            let submitted = match intent {
                oteryn_session::ChatIntent::Party { text } => {
                    Some((&mut mailbox.chat_party_room.text, text))
                }
                oteryn_session::ChatIntent::Room { text, .. } => {
                    Some((&mut mailbox.chat_public.text, text))
                }
                _ => None,
            };
            if let Some((draft, submitted)) = submitted
                && draft == submitted
            {
                draft.clear();
            }
        }
        if matches!(intent, oteryn_session::ChatIntent::Party { .. }) {
            self.chat_party_room_result(outcome);
        } else if matches!(
            intent,
            oteryn_session::ChatIntent::Say { .. }
                | oteryn_session::ChatIntent::Private { .. }
                | oteryn_session::ChatIntent::Guild { .. }
        ) {
            self.local_chat_result(outcome);
        } else {
            self.chat_public_result(intent, outcome);
        }
    }
}
pub(in crate::play) fn chat_line(pushed: &mut Pushed, delta: AppliedDelta<ChatLine>) {
    pushed.chat_lines.push(delta.value);
}
pub(in crate::play) fn chat_rooms(pushed: &mut Pushed, delta: AppliedDelta<ChatRoomSet>) {
    pushed.chat_rooms = Some(delta.value);
}

pub(in crate::play) fn chat_snapshot(
    pushed: &mut Pushed,
    snapshot: AppliedDelta<oteryn_session::ChatLog>,
) {
    pushed.chat_reset = true;
    pushed.chat_rooms = Some(snapshot.value.rooms());
    pushed.chat_lines = snapshot.value.lines().cloned().collect();
}

pub(in crate::play) fn reset_if_replaced(mailbox: &mut crate::play::mailbox::Mailbox) {
    if std::mem::take(&mut mailbox.pushed.chat_reset) {
        mailbox.chat_party_room.lines.clear();
    }
}
