use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, SharedMailbox, lock},
};
use oteryn_client_runtime::guild::GuildCommand;
use oteryn_client_ui::chat::ChannelTab;
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn open_guild_chat(&self) -> bool {
        open(&self.mailbox, &self.commands)
    }
}

pub(in crate::play) fn open(
    mailbox: &SharedMailbox,
    commands: &UnboundedSender<PlayCommand>,
) -> bool {
    open_locked(&mut lock(mailbox), commands)
}

/// Opens the guild room in the chat window; the guild window's Guild chat button uses it.
pub(in crate::play) fn open_locked(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
) -> bool {
    if !oteryn_client_ui::chat_guild_room::available(&mailbox.chat) {
        return false;
    }
    let opening = !mailbox.chat.console.channels.contains(&ChannelTab::Guild);
    mailbox.chat.guild_open = true;
    mailbox.chat.open = true;
    mailbox.chat.console.open_channel(ChannelTab::Guild);
    if opening {
        mailbox.chat.guild_motd_notice = None;
        mailbox.chat.feedback = None;
        if mailbox.guild.selected {
            if commands
                .send(PlayCommand::Guild(GuildCommand::ChatOwn))
                .is_err()
            {
                mailbox.chat.feedback = Some(tr!("The session has ended").into());
            }
        } else {
            oteryn_client_ui::chat_guild_room::show_motd_notice(&mut mailbox.chat);
        }
    }
    true
}

fn parse_setmotd(input: &str) -> Option<&str> {
    if input.eq_ignore_ascii_case("!setmotd") {
        return Some("");
    }
    let boundary = input.find(|ch: char| ch.is_whitespace() || ch.is_control())?;
    let (command, suffix) = input.split_at(boundary);
    let text = suffix
        .strip_prefix(' ')
        .filter(|text| !text.is_empty())
        .unwrap_or(suffix);
    command.eq_ignore_ascii_case("!setmotd").then_some(text)
}

/// Consume the Guild composer command before building a speech intent.
pub(in crate::play) fn setmotd(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
) -> bool {
    if mailbox.chat.console.current != Some(ChannelTab::Guild) {
        return false;
    }
    let Some(text) = parse_setmotd(&mailbox.chat.text) else {
        return false;
    };
    let own = mailbox.guild.own.as_ref().filter(|own| {
        mailbox.guild.selected && !mailbox.chat.pending
            && oteryn_client_ui::chat_guild_room::available(&mailbox.chat)
            && matches!(mailbox.chat.guild, oteryn_session::GuildRoomState::Member { guild_id, .. } if guild_id == own.guild)
    });
    let command = own
        .ok_or_else(|| tr!("Guild chat unavailable or waiting for the server"))
        .and_then(|own| oteryn_client_ui::guild::request_message(own, text));
    let command = match command {
        Ok(command) => command,
        Err(message) => {
            mailbox.chat.feedback = Some(message.into());
            return true;
        }
    };
    if commands.send(PlayCommand::Guild(command)).is_err() {
        mailbox.chat.feedback = Some(tr!("The session has ended").into());
    } else {
        mailbox.chat.pending = true;
        mailbox.chat.feedback = None;
        mailbox.chat.console.sent();
        crate::play::feature_0233_player_typing::input::composer(mailbox, commands, false);
    }
    true
}

#[cfg(test)]
mod gmotd_tests {
    use super::*;
    use oteryn_client_ui::chat::{ChannelEntry, ChannelTab, Mode};

    fn login_panel() -> PlayLink {
        let (link, _, outbox) = crate::play::play_channel();
        outbox.chat_initialize(true);
        oteryn_client_ui::chat_guild_room::replace_state(
            &mut lock(&link.mailbox).chat,
            true,
            oteryn_session::GuildRoomState::Member {
                guild_id: [1; 16],
                rank: 1,
                motd: "Welcome".into(),
            },
        );
        link
    }

    #[test]
    fn gmotd_notice_appears_once_per_guild_tab_open() {
        let link = login_panel();
        assert!(
            !link
                .chat_panel()
                .console
                .channels
                .contains(&ChannelTab::Guild)
        );
        assert!(
            !oteryn_client_ui::chat::panel_text(&link.chat_panel()).contains("Message of the Day:")
        );
        let entry = ChannelEntry::Local(ChannelTab::Guild);
        for _ in 0..2 {
            link.chat_open_entry(&entry);
            assert!(link.open_guild_chat());
            assert_eq!(
                oteryn_client_ui::chat::panel_text(&link.chat_panel())
                    .matches("Message of the Day: Welcome")
                    .count(),
                1
            );
            link.chat_close_current();
            assert!(
                !oteryn_client_ui::chat::panel_text(&link.chat_panel())
                    .contains("Message of the Day:")
            );
        }
    }

    #[test]
    fn gmotd_composer_parses_only_the_complete_command() {
        for (input, expected) in [
            ("!setmotd", Some("")),
            ("!SETMOTD", Some("")),
            ("!setmotd ", Some(" ")),
            ("!setmotd  ", Some(" ")),
            ("!setmotd a Guild hunt ", Some("a Guild hunt ")),
            ("!setmotd\tbad", Some("\tbad")),
            ("!setmotd-more", None),
            ("hello !setmotd", None),
        ] {
            assert_eq!(parse_setmotd(input), expected);
        }
    }

    #[test]
    fn gmotd_composer_consumes_command_before_speech() {
        let (link, mut incoming, outbox) = crate::play::play_channel();
        outbox.chat_initialize(true);
        {
            let mut mailbox = lock(&link.mailbox);
            mailbox.guild.selected = true;
            oteryn_client_ui::chat_guild_room::replace_state(
                &mut mailbox.chat,
                true,
                oteryn_session::GuildRoomState::Member {
                    guild_id: [1; 16],
                    rank: 1,
                    motd: "Welcome".into(),
                },
            );
            mailbox.guild.own = Some(oteryn_client_runtime::guild::guild::GuildSummary {
                guild: [1; 16],
                name: "Guild Name".into(),
                state: 1,
                revision: 3,
                own_rank: 1,
                title: String::new(),
                message: "Welcome".into(),
                ranks: vec!["Leader".into(), "Vice Leader".into(), "Member".into()],
            });
        }
        assert!(link.open_guild_chat());
        assert!(matches!(
            incoming.try_recv(),
            Ok(PlayCommand::Guild(GuildCommand::ChatOwn))
        ));
        link.chat_text("!setmotd Open hunt".into());
        link.chat_request(Mode::Say, "");
        assert!(matches!(
            incoming.try_recv(),
            Ok(PlayCommand::Guild(GuildCommand::SetMessage(edit)))
                if edit.guild == [1; 16] && edit.expected_revision == 3 && edit.text == "Open hunt"
        ));
    }
}
