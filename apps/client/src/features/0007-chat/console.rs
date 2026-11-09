use crate::combat_input::PlayCommand;
use crate::play::{
    Outbox, PlayLink,
    mailbox::{Mailbox, lock},
};
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
                    kind: 736,
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
