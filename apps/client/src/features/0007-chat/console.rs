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
    mailbox.chat.feedback = Some(lfmt!(
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
        Ok(()) => tr!("Channel log written").into(),
        Err(error) => lfmt!("Could not write channel log: {error}", error = error),
    });
    result
}
use crate::combat_input::PlayCommand;
use crate::play::Outbox;
use oteryn_client_ui::chat::{ConsoleDisplay, console_request};
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn console_display(&self, display: ConsoleDisplay) -> Result<(), &'static str> {
        if !display.valid() {
            return Err("Choose a supported console font size");
        }
        let mut mailbox = lock(&self.mailbox);
        mailbox.chat.console.display = display;
        refresh(&mut mailbox, &self.commands);
        Ok(())
    }
}
pub(super) fn refresh(mailbox: &mut Mailbox, commands: &UnboundedSender<PlayCommand>) {
    if let Some(payload) = console_request(&mut mailbox.chat)
        && commands
            .send(PlayCommand::Feature(
                oteryn_client_runtime::raw_feature::FeatureCommand {
                    capability: 7,
                    kind: 6117,
                    payload,
                },
            ))
            .is_err()
    {
        mailbox.chat.console.metadata.pending = None;
    }
}
impl Outbox {
    pub(in crate::play) fn console_events(&self, events: &[oteryn_session::SessionEvent]) {
        let received = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut mailbox = lock(&self.0);
        for event in events {
            match event {
                oteryn_session::SessionEvent::ChatLine(delta) => mailbox
                    .chat
                    .console
                    .metadata
                    .received(&delta.value, received),
                oteryn_session::SessionEvent::ServerMessage(delta) => {
                    use oteryn_client_ui::chat::NoticeClass;
                    let (class, text) = match &delta.value {
                        oteryn_session::ServerMessage::Advance(text) => {
                            (NoticeClass::Event, text.clone())
                        }
                        oteryn_session::ServerMessage::StatusText(text) => {
                            (NoticeClass::Status, text.clone())
                        }
                        oteryn_session::ServerMessage::Warning(text) => {
                            (NoticeClass::Status, format!("Warning: {text}"))
                        }
                    };
                    mailbox.chat.console.notices.push(class, &text, received);
                }
                oteryn_session::SessionEvent::ChatSnapshot(_) => {
                    mailbox.chat.console.metadata.reset_history()
                }
                _ => {}
            }
        }
    }

    pub(in crate::play) fn console_result(&self, result: &oteryn_session::GatedCommandResult) {
        let mut mailbox = lock(&self.0);
        let Some(names) = mailbox.chat.console.metadata.pending.take() else {
            return;
        };
        for name in &names {
            mailbox.chat.console.metadata.levels.remove(name);
        }
        if !matches!(
            result.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        ) {
            return;
        }
        let Some(levels) = oteryn_client_runtime::chat::console::decode_levels(&result.payload)
            .filter(|levels| levels.len() == names.len())
        else {
            return;
        };
        for (name, level) in names.into_iter().zip(levels) {
            if let Some(level) = level {
                mailbox.chat.console.metadata.levels.insert(name, level);
            } else {
                mailbox.chat.console.metadata.levels.remove(&name);
            }
        }
    }
}
