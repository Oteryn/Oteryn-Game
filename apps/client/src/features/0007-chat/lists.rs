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
        buttons: &[
            "Ignore",
            "Whitelist",
            "Blacklist",
            "Remove from lists",
            "Block private messages",
            "Block yells",
            "Suspend ignore list",
            "Suspend whitelist",
            "Allow VIP speakers",
        ],
        snapshot: |m| {
            (
                m.chat_lists.0,
                false,
                vec![m.chat_lists.1.clone()],
                format!(
                    "{}\nBlock private messages: {}. Block yells: {}. Suspend ignore list: {}. Suspend whitelist: {}. Allow VIP speakers: {}.",
                    m.chat.feedback.as_deref().unwrap_or_default(),
                    m.chat.console.communication.block_private,
                    m.chat.console.communication.block_yells,
                    m.chat.console.communication.ignore_suspended,
                    m.chat.console.communication.whitelist_suspended,
                    m.chat.console.communication.allow_vips
                ),
            )
        },
        set_open: |m, open| m.chat_lists.0 = open,
        submit: |m, commands, action, values| {
            let prefs = &mut m.chat.console.communication;
            let toggle = match action {
                4 => Some(&mut prefs.block_private),
                5 => Some(&mut prefs.block_yells),
                6 => Some(&mut prefs.ignore_suspended),
                7 => Some(&mut prefs.whitelist_suspended),
                8 => Some(&mut prefs.allow_vips),
                _ => None,
            };
            if let Some(toggle) = toggle {
                *toggle = !*toggle;
                m.chat_private.communication = prefs.clone();
                m.chat_party_room.communication = prefs.clone();
                if action == 8
                    && prefs.allow_vips
                    && let Some(command) = oteryn_client_ui::vip::request(
                        &mut m.vip,
                        oteryn_client_ui::vip::Intent::Query(0),
                    )
                    && commands
                        .send(crate::combat_input::PlayCommand::Vip(command))
                        .is_err()
                {
                    m.vip.pending = false;
                }
                super::console::show_lists(m);
                return;
            }
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
