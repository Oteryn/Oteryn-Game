use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::chat_public_channels::ChatPanel;

impl PlayLink {
    #[must_use]
    pub fn chat_public_panel(&self) -> ChatPanel {
        lock(&self.mailbox).chat_public.clone()
    }
}


/// The public channels window: room number, message, room and movement-spell buttons.
pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use crate::play::feature_0115_mail::view::form::{Form, field};
    use oteryn_client_runtime::chat_public_channels::ChatRoom;
    use oteryn_client_ui::chat_public_channels::{Intent, panel_text, room_name};
    use oteryn_client_ui::spell_action_bar::movement::MovementSpell;
    Form {
        title: tr!("Public Channels"),
        labels: &["Room number", "Message"],
        buttons: &[
            "Open",
            "Close",
            "Send",
            "Levitate Up",
            "Levitate Down",
            "Magic Rope",
        ],
        snapshot: |mailbox| {
            let panel = &mailbox.chat_public;
            let row = ChatRoom::ALL
                .iter()
                .position(|room| *room == panel.room)
                .map_or(1, |row| row + 1);
            let mut text = panel_text(panel);
            for (index, room) in ChatRoom::ALL.iter().enumerate() {
                text.push_str(&format!("\n{}. {}", index + 1, room_name(*room)));
            }
            (
                panel.open && !mailbox.chat_public_shutdown,
                panel.pending,
                vec![row.to_string(), panel.text.clone()],
                text,
            )
        },
        set_open: |mailbox, value| mailbox.chat_public.open = value,
        submit: |mailbox, commands, action, values| {
            let movement = match action {
                3 => Some(MovementSpell::LevitateUp),
                4 => Some(MovementSpell::LevitateDown),
                5 => Some(MovementSpell::MagicRope),
                _ => None,
            };
            if let Some(spell) = movement {
                crate::play::feature_local_movement_spells::input::enqueue(mailbox, commands, spell);
                return;
            }
            let Some(&room) = field(&values, 0)
                .parse::<usize>()
                .ok()
                .and_then(|row| row.checked_sub(1))
                .and_then(|row| ChatRoom::ALL.get(row))
            else {
                // An unknown room number is refused; the earlier room is never used instead.
                mailbox.chat_public.feedback = Some(tr!("Enter a room number from the list").into());
                return;
            };
            mailbox.chat_public.room = room;
            let intent = match action {
                0 => Intent::Open(room),
                1 => Intent::Close(room),
                2 => {
                    mailbox.chat_public.text = values.get(1).cloned().unwrap_or_default();
                    Intent::Send(room)
                }
                _ => return,
            };
            super::input::enqueue(mailbox, commands, intent);
        },
    }
}
