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

#[path = "npc_voice.rs"]
mod npc_voice;

struct Peer {
    commands: Arc<Mutex<Vec<ChatIntent>>>,
    rooms: ChatRoomSet,
    lines: Vec<ChatLine>,
}

impl Stepper for Peer {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == 7
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
            ChatIntent::Private { text, .. } => self.lines.push(ChatLine::Private {
                speaker_name: "Player One".into(),
                text: text.clone(),
            }),
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
fn local_chat_input_reaches_session_and_received_private_message_is_visible() -> Result<(), Error> {
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
                wait_for(&link, |panel| panel.selected).await?;
                link.chat_text("hello".into());
                link.chat_request(Mode::Say, "");
                wait_for(&link, |panel| !panel.pending).await?;
                link.chat_request(Mode::Private, "Player One");
                wait_for(&link, |panel| !panel.lines.is_empty()).await?;
                assert!(panel_text(&link.chat_panel()).contains("[Private] Player One: hello"));
                let sent = commands.lock().unwrap_or_else(PoisonError::into_inner).clone();
                assert!(matches!(sent.as_slice(), [ChatIntent::Say { .. }, ChatIntent::Private { recipient_name, .. }] if recipient_name == "Player One"));
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "local chat UI test thread panicked")?
}

#[test]
fn channel_logs_save_replace_and_append_confirmed_channel_contents() -> Result<(), Error> {
    let (link, _incoming, outbox) = play_channel();
    let path = std::env::temp_dir().join(format!(
        "oteryn-channel-log-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    outbox.chat_public_state(
        ChatRoomSet::default(),
        vec![ChatLine::Private {
            speaker_name: "Alice".into(),
            text: "confirmed".into(),
        }],
    );
    link.chat_console(
        oteryn_client_ui::chat::ConsoleTab::Private,
        oteryn_client_ui::chat::ComposerMode::On,
        Default::default(),
    );
    link.chat_save_log(&path, false)?;
    let saved = std::fs::read_to_string(&path)?;
    assert!(saved.contains("[Private] Alice: confirmed"));
    link.chat_save_log(&path, true)?;
    assert_eq!(std::fs::read_to_string(&path)?, saved.repeat(2));
    link.chat_save_log(&path, false)?;
    assert_eq!(std::fs::read_to_string(&path)?, saved);
    std::fs::remove_file(&path)?;
    assert!(
        link.chat_save_log(&path.join("missing/file"), false)
            .is_err()
    );
    Ok(())
}

#[test]
fn console_channel_reply_releases_only_the_matching_composer() {
    use oteryn_client_runtime::chat_public_channels::ChatRoom;
    let (link, _incoming, outbox) = play_channel();
    outbox.chat_initialize(true);
    link.chat_console(
        oteryn_client_ui::chat::ConsoleTab::Public(ChatRoom::Help),
        oteryn_client_ui::chat::ComposerMode::On,
        Default::default(),
    );
    link.chat_text("hello".into());
    link.chat_request(Mode::Say, "");
    let outcome = ChatOutcome {
        command_id: 1,
        status: CommandStatus::Accepted,
        disposition: ChatDisposition::Ok,
        wait_seconds: 0,
        result_server_sequence: 1,
    };
    outbox.chat_result(
        &ChatIntent::Room {
            room: ChatRoom::World,
            text: "other".into(),
        },
        &outcome,
    );
    assert!(link.chat_panel().pending);
    outbox.chat_result(
        &ChatIntent::Room {
            room: ChatRoom::Help,
            text: "hello".into(),
        },
        &outcome,
    );
    assert!(!link.chat_panel().pending);
}
