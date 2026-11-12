use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::chat_public_channels::{
    ChatDisposition, ChatIntent, ChatLine, ChatOutcome, ChatRoomSet,
};
use oteryn_client_ui::chat::{Mode, panel_text};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;

struct Peer {
    commands: Arc<Mutex<Vec<ChatIntent>>>,
    rooms: ChatRoomSet,
    lines: Vec<ChatLine>,
}

impl Stepper for Peer {
    fn feature_selected(&self, capability: u32) -> bool {
        matches!(capability, 7 | 36)
    }
    fn guild_chat_state(&self) -> oteryn_session::GuildRoomState {
        oteryn_session::GuildRoomState::Member {
            guild_id: [1; 16],
            rank: 1,
            motd: "Welcome".into(),
        }
    }
    fn chat_state(&self) -> Option<(ChatRoomSet, Vec<ChatLine>)> {
        Some((self.rooms, self.lines.clone()))
    }
    async fn chat_public(&mut self, intent: &ChatIntent) -> Result<ChatOutcome, SessionError> {
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(intent.clone());
        match intent {
            ChatIntent::Guild { text } => {
                self.lines
                    .push(ChatLine::Guild(oteryn_session::GuildChatLine {
                        guild_id: [1; 16],
                        rank: 1,
                        speaker_name: "Player One".into(),
                        text: text.clone(),
                    }))
            }
            ChatIntent::Say { .. } => {}
            _ => return Err(SessionError::SessionUnusable),
        }
        Ok(ChatOutcome {
            command_id: 1,
            status: CommandStatus::Accepted,
            disposition: ChatDisposition::Ok,
            wait_seconds: 0,
            result_server_sequence: 1,
        })
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        vec![]
    }
    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(None)
    }
}

async fn wait_for(
    link: &PlayLink,
    predicate: impl Fn(&oteryn_client_ui::chat::ChatPanel) -> bool,
) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if predicate(&link.chat_panel()) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}

#[test]
fn guild_panel_opens_chat_and_send_reaches_session_and_transcript() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let peer = Peer {
                    commands: Arc::clone(&commands),
                    rooms: ChatRoomSet::default(),
                    lines: vec![],
                };
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(peer, incoming, outbox));
                wait_for(&link, oteryn_client_ui::chat_guild_room::available).await?;
                assert!(link.open_guild_chat());
                assert!(link.chat_panel().open);
                link.chat_text("hello guild".into());
                link.chat_request(Mode::Guild, "");
                wait_for(&link, |panel| !panel.lines.is_empty()).await?;
                assert!(panel_text(&link.chat_panel()).contains("[Guild] Player One: hello guild"));
                assert!(panel_text(&link.chat_panel()).contains("Welcome"));
                let sent = commands
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone();
                assert!(
                    matches!(sent.as_slice(), [ChatIntent::Guild { text }] if text == "hello guild")
                );
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "guild chat UI test thread panicked")?
}

#[test]
fn non_member_cannot_open_guild_composer() {
    for guild in [
        oteryn_session::GuildRoomState::Absent,
        oteryn_session::GuildRoomState::Unavailable,
    ] {
        let (link, _incoming, outbox) = play_channel();
        outbox.chat_initialize(true);
        oteryn_client_ui::chat_guild_room::replace_state(
            &mut crate::play::mailbox::lock(&link.mailbox).chat,
            true,
            guild,
        );
        assert!(link.chat_panel().selected);
        assert!(link.chat_panel().guild_selected);
        assert!(!link.open_guild_chat());
        assert!(!link.chat_panel().open);
    }
}

#[test]
fn session_end_and_logout_clear_guild_chat_membership() {
    for logout in [false, true] {
        let (link, _incoming, outbox) = play_channel();
        outbox.chat_initialize(true);
        {
            let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
            oteryn_client_ui::chat_guild_room::replace_state(
                &mut mailbox.chat,
                true,
                oteryn_session::GuildRoomState::Member {
                    guild_id: [1; 16],
                    rank: 1,
                    motd: "Welcome".into(),
                },
            );
        }
        assert!(oteryn_client_ui::chat_guild_room::available(
            &link.chat_panel()
        ));
        if logout {
            outbox.logged_out();
        } else {
            outbox.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
        }
        assert_eq!(
            link.chat_panel().guild,
            oteryn_session::GuildRoomState::Absent
        );
        assert!(!link.chat_panel().guild_selected);
        assert!(!link.open_guild_chat());
    }
}
