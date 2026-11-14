//! Atomic presentation actions reuse the canonical command builder and request state.
use crate::{
    combat_input::PlayCommand,
    play::{PlayLink, mailbox::lock},
};
use oteryn_client_ui::character_info::{self, Intent, Panel};
impl PlayLink {
    pub(crate) fn character_titles_open_bound(&self, expected: &Panel) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.highscores_shutdown
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || self.commands.is_closed()
            || mailbox.properties.character_info != *expected
            || !expected.selected
            || expected.pending
        {
            return false;
        }
        let mut prepared = expected.clone();
        prepared.fields[0] = "0".into();
        prepared.fields[1] = "1".into();
        prepared.fields[2].clear();
        let Ok(command) = super::super::protocol::command(&prepared, Intent::Query) else {
            return false;
        };
        if !character_info::request(&mut prepared, command.kind)
            || self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            return false;
        }
        prepared.titles_open = true;
        mailbox.properties.character_info = prepared;
        true
    }

    pub(crate) fn character_info_open_bound(&self, expected: &Panel) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.highscores_shutdown
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || self.commands.is_closed()
            || mailbox.properties.character_info != *expected
            || !expected.selected
        {
            return false;
        }
        mailbox.properties.character_info.open = true;
        true
    }
    pub(crate) fn character_info_close_bound(&self, expected: &Panel) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.highscores_shutdown
            || mailbox.server_modal.is_active()
            || self.commands.is_closed()
            || mailbox.properties.character_info != *expected
            || !(expected.open || expected.titles_open)
        {
            return false;
        }
        mailbox.properties.character_info.open = false;
        mailbox.properties.character_info.titles_open = false;
        true
    }
    pub(crate) fn character_info_request_bound(
        &self,
        expected: &Panel,
        fields: [String; 4],
        intent: Intent,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.highscores_shutdown
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || self.commands.is_closed()
            || mailbox.properties.character_info != *expected
            || !(expected.open || expected.titles_open)
            || !expected.selected
            || expected.pending
            || fields
                .iter()
                .any(|field| field.len() > 7996 || field.contains('\0'))
        {
            return false;
        }
        let mut prepared = expected.clone();
        prepared.fields = fields;
        let Ok(command) = super::super::protocol::command(&prepared, intent) else {
            return false;
        };
        if !character_info::request(&mut prepared, command.kind)
            || self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            return false;
        }
        mailbox.properties.character_info = prepared;
        true
    }
}
