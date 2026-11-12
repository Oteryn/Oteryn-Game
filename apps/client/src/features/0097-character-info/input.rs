use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::character_info::Intent;
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    let panel = &mut mailbox.properties.character_info;
    if !panel.selected {
        panel.feedback = tr!("Unavailable in this session").into();
        return;
    }
    if panel.pending {
        panel.feedback = tr!("Waiting for the server").into();
        return;
    }
    match super::protocol::command(panel, intent) {
        Ok(command) => {
            if oteryn_client_ui::character_info::request(panel, command.kind)
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                panel.failed(tr!("The session has ended"));
            }
        }
        Err(message) => panel.feedback = message,
    }
}
impl PlayLink {
    pub fn character_info_edit(&self, field: usize, value: String) {
        edit(
            &mut lock(&self.mailbox).properties.character_info,
            field,
            value,
        );
    }
    pub fn character_info_request(&self, intent: Intent) {
        submit(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn character_info_open(&self) {
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).properties.character_info.open = true;
        }
    }
    pub fn character_titles_open(&self) {
        {
            let mut mailbox = lock(&self.mailbox);
            let panel = &mut mailbox.properties.character_info;
            if !panel.pending {
                panel.fields[0] = "0".into();
                panel.fields[1] = "1".into();
                panel.fields[2].clear();
                submit(&mut mailbox, &self.commands, Intent::Query);
            }
        }
        #[cfg(windows)]
        super::view::open_titles(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).properties.character_info.titles_open = true;
        }
    }
    pub fn character_info_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if oteryn_client_ui::character_info::titles_key(events) {
            self.character_titles_open();
            return true;
        }
        if !oteryn_client_ui::character_info::open_key(events) {
            return false;
        }
        self.character_info_open();
        true
    }
}

pub(in crate::play) fn edit(
    panel: &mut oteryn_client_ui::character_info::Panel,
    field: usize,
    value: String,
) {
    if panel.pending || value.len() > 7996 || value.contains('\0') {
        return;
    }
    if let Some(slot) = panel.fields.get_mut(field) {
        if *slot == value {
            return;
        }
        *slot = value;
    }
}
