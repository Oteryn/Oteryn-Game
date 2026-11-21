//! Ignore, whitelist and blacklist editing for the chat communication lists. The generic
//! form window presents it; a chat message's menu opens it with the speaker's name.
use crate::play::{PlayLink, feature_0115_mail::view::form, mailbox::lock};
use oteryn_client_ui::chat::CommunicationList;

impl PlayLink {
    /// Opens the communication-lists form, the name field set to `name` when one is given.
    pub fn chat_lists_open(&self, name: Option<&str>) {
        let mut mailbox = lock(&self.mailbox);
        mailbox.chat_lists.0 = true;
        if let Some(name) = name {
            name.clone_into(&mut mailbox.chat_lists.1);
        }
        super::console::show_lists(&mut mailbox);
    }
}

pub(in crate::play) fn form(_link: &PlayLink) -> form::Form {
    form::Form {
        title: "Communication lists",
        labels: &["Player name"],
        buttons: &["Ignore", "Whitelist", "Blacklist", "Remove from lists"],
        snapshot: |m| {
            (
                m.chat_lists.0,
                false,
                vec![m.chat_lists.1.clone()],
                m.chat.feedback.clone().unwrap_or_default(),
            )
        },
        set_open: |m, open| m.chat_lists.0 = open,
        submit: |m, _commands, action, values| {
            let list = match action {
                0 => Some(CommunicationList::Ignore),
                1 => Some(CommunicationList::Whitelist),
                2 => Some(CommunicationList::Blacklist),
                _ => None,
            };
            form::field(&values, 0).clone_into(&mut m.chat_lists.1);
            let name = m.chat_lists.1.clone();
            super::console::edit_lists(m, list, &name, list.is_some());
        },
    }
}
