use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[must_use]
    pub fn chat_party_room_panel(&self) -> oteryn_client_ui::chat_party_room::ChatPartyRoomPanel {
        lock(&self.mailbox).chat_party_room.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

/// The party chat room for the shared frontend.
pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use oteryn_client_ui::chat_party_room::{panel_text, request};
    crate::play::feature_0115_mail::view::form::Form {
        title: tr!("Party Chat"),
        labels: &["Message"],
        buttons: &["Send"],
        snapshot: |mailbox| {
            let panel = &mailbox.chat_party_room;
            (
                panel.open && !mailbox.chat_public_shutdown,
                panel.pending,
                vec![panel.text.clone()],
                panel_text(panel),
            )
        },
        set_open: |mailbox, value| mailbox.chat_party_room.open = value,
        submit: |mailbox, commands, _, values| {
            mailbox.chat_party_room.text = values.into_iter().next().unwrap_or_default();
            if let Some(intent) = request(&mut mailbox.chat_party_room)
                && commands
                    .send(crate::combat_input::PlayCommand::ChatPublic(intent))
                    .is_err()
            {
                mailbox.chat_party_room.pending = false;
                mailbox.chat_party_room.feedback = Some(tr!("The session has ended").into());
            }
        },
    }
}
