use crate::play::{
    PlayLink,
    mailbox::{Mailbox, SharedMailbox, lock},
};
use oteryn_client_ui::chat::{CommunicationList, ComposerMode, ConsoleTab};
use std::path::Path;

impl PlayLink {
    pub fn chat_console_tab(&self, tab: ConsoleTab) {
        select(&mut lock(&self.mailbox), tab);
    }
    pub fn chat_console_filter(&self) {
        toggle_filter(&mut lock(&self.mailbox));
    }
    pub fn chat_console_mode(&self) {
        cycle_mode(&mut lock(&self.mailbox));
    }
    pub fn chat_communication(
        &self,
        list: Option<CommunicationList>,
        name: &str,
        add: bool,
    ) -> bool {
        edit_lists(&mut lock(&self.mailbox), list, name, add)
    }
    pub fn chat_save_log(&self, path: &Path, append: bool) -> std::io::Result<()> {
        save_log(&self.mailbox, path, append)
    }
}
pub(in crate::play) fn select(mailbox: &mut Mailbox, tab: ConsoleTab) {
    mailbox.chat.console.select(tab);
}
pub(in crate::play) fn toggle_filter(mailbox: &mut Mailbox) {
    mailbox.chat.console.show_messages = !mailbox.chat.console.show_messages;
}
pub(in crate::play) fn cycle_mode(mailbox: &mut Mailbox) {
    mailbox.chat.console.mode = match mailbox.chat.console.mode {
        ComposerMode::On => ComposerMode::Off,
        ComposerMode::Off => ComposerMode::Temporary,
        ComposerMode::Temporary => ComposerMode::On,
    };
}
pub(in crate::play) fn edit_lists(
    mailbox: &mut Mailbox,
    list: Option<CommunicationList>,
    name: &str,
    add: bool,
) -> bool {
    let changed = if let Some(list) = list {
        mailbox.chat.console.communication.edit(list, name, add)
    } else {
        [
            CommunicationList::Ignore,
            CommunicationList::Whitelist,
            CommunicationList::Blacklist,
        ]
        .into_iter()
        .all(|list| mailbox.chat.console.communication.edit(list, name, false))
    };
    if !changed {
        mailbox.chat.feedback =
            Some("Choose a valid player name; communication lists hold at most 200 names".into());
        return false;
    }
    mailbox.chat_private.communication = mailbox.chat.console.communication.clone();
    mailbox.chat_party_room.communication = mailbox.chat.console.communication.clone();
    mailbox.chat.feedback = Some(format!(
        "Ignore: {:?}; whitelist: {:?}; blacklist: {:?}",
        mailbox.chat.console.communication.ignored,
        mailbox.chat.console.communication.whitelist,
        mailbox.chat.console.communication.blacklist
    ));
    true
}
pub(in crate::play) fn save_log(
    shared: &SharedMailbox,
    path: &Path,
    append: bool,
) -> std::io::Result<()> {
    let snapshot = lock(shared).chat.clone();
    let result = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(path)
        .and_then(|mut file| oteryn_client_ui::chat::write_log(&snapshot, &mut file));
    lock(shared).chat.feedback = Some(match &result {
        Ok(()) => "Channel log written".into(),
        Err(error) => format!("Could not write channel log: {error}"),
    });
    result
}
