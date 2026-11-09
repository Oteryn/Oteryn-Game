use crate::play::{
    Outbox,
    mailbox::{Mailbox, lock},
};
use oteryn_session::SessionEvent;
impl Outbox {
    pub(in crate::play) fn cast_chat_initialize(&self, selected: bool) {
        lock(&self.0).cast_chat = oteryn_client_ui::cast_chat::Panel {
            selected,
            ..Default::default()
        };
    }
    pub(in crate::play) fn cast_chat_result(
        &self,
        result: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        let mut shared = lock(&self.0);
        shared.cast_chat.pending = false;
        let message = oteryn_client_runtime::cast::Message::decode(&result.payload);
        if result.status == oteryn_session::CommandStatus::Accepted
            && matches!(
                message,
                Ok(oteryn_client_runtime::cast::Message::CasterState { .. })
            )
        {
            if let Ok(message) = message {
                let _ = shared.cast_control.apply(message);
            }
            shared.cast_chat.feedback = tr!("Cast action applied").into();
        } else {
            shared.cast_chat.feedback =
                tr!("Cast action refused; the viewer or cast may have changed").into();
        }
    }
}
pub(in crate::play) fn reduce(mailbox: &mut Mailbox, events: &[SessionEvent]) {
    for event in events {
        match event {
            SessionEvent::CastRoster(roster)
                if mailbox.cast_control.active
                    && roster.value.revision == mailbox.cast_control.revision =>
            {
                mailbox.cast_control.viewers =
                    u8::try_from(roster.value.subjects.len()).unwrap_or(50);
                oteryn_client_ui::cast_chat::protocol::roster(
                    &mut mailbox.cast_chat,
                    roster.value.clone(),
                );
            }
            SessionEvent::CastCasterLine(line) if mailbox.cast_control.active => {
                oteryn_client_ui::cast_chat::protocol::append(
                    &mut mailbox.cast_chat,
                    line.value.clone(),
                )
            }
            SessionEvent::GatedSnapshotReplaced(snapshot) if snapshot.value.domain_id == 103 => {
                mailbox.cast_chat.lines.clear();
                mailbox.cast_chat.subjects.clear();
                mailbox.cast_chat.roster_revision = 0;
            }
            _ => {}
        }
    }
}
