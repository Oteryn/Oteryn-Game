use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::chat_private_channel::ChatPanel;
impl PlayLink {
    #[must_use]
    pub fn chat_private_panel(&self) -> ChatPanel {
        lock(&self.mailbox).chat_private.clone()
    }
}

impl PlayLink {
    /// Opens the private-channel window of the shared frontend and refreshes its list.
    pub fn chat_private_open(&self) {
        let mut mailbox = lock(&self.mailbox);
        mailbox.chat_private_open = true;
        super::input::enqueue(
            &mut mailbox,
            &self.commands,
            oteryn_client_ui::chat_private_channel::Intent::List,
        );
    }
}

/// The native chat window's private-channel controls as one shared-frontend form.
pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use oteryn_client_ui::chat_private_channel::{Intent, leave_label, panel_text};
    crate::play::feature_0115_mail::view::form::Form {
        title: tr!("Private channels"),
        labels: &["Channel number", "Player name", "Message"],
        buttons: &[
            "Create/Open own",
            "Refresh",
            "Join",
            "Leave",
            "Invite player",
            "Exclude player",
            "Send",
        ],
        snapshot: |mailbox| {
            let panel = &mailbox.chat_private;
            let mut text = String::new();
            if let Some(list) = &panel.list {
                if let Some(id) = list.own_channel {
                    text.push_str(&format!("Own channel: {id}\n"));
                }
                for invite in &list.invited {
                    text.push_str(&format!("{}: {}\n", invite.channel_id, invite.owner_name));
                }
            }
            text.push_str(&format!("Leave button: {}\n", leave_label(panel)));
            text.push_str(&panel_text(panel));
            (
                mailbox.chat_private_open,
                panel.pending,
                vec![
                    panel.channel.map(|id| id.to_string()).unwrap_or_default(),
                    panel.name.clone(),
                    panel.text.clone(),
                ],
                text,
            )
        },
        set_open: |mailbox, value| mailbox.chat_private_open = value,
        submit: |mailbox, commands, action, values| {
            use crate::play::feature_0115_mail::view::form::field;
            let intent = match action {
                0 => Intent::Create,
                1 => Intent::List,
                2 => Intent::Join,
                3 => Intent::Leave,
                4 => Intent::Invite,
                5 => Intent::Exclude,
                6 => Intent::Send,
                _ => return,
            };
            let channel = field(&values, 0).parse::<u16>().ok().filter(|channel| {
                mailbox.chat_private.list.as_ref().is_some_and(|list| {
                    list.own_channel == Some(*channel)
                        || list.invited.iter().any(|invite| invite.channel_id == *channel)
                })
            });
            match channel {
                Some(channel) => mailbox.chat_private.channel = Some(channel),
                // A typed channel that is neither the own channel nor an invitation is
                // refused; the earlier pick is never used in its place.
                None if matches!(intent, Intent::Join | Intent::Leave | Intent::Send) => {
                    mailbox.chat_private.feedback =
                        Some(tr!("Select your channel or an invitation").into());
                    return;
                }
                None => {}
            }
            mailbox.chat_private.name = field(&values, 1).to_owned();
            mailbox.chat_private.text = values.get(2).cloned().unwrap_or_default();
            super::input::enqueue(mailbox, commands, intent);
        },
    }
}
