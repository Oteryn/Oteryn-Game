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

#[path = "console.rs"]
mod console;

#[test]
fn n021_alt_d_selects_the_default_tab_and_keypad_enter_sets_temporary_mode()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_ui::chat::{ComposerMode, ConsoleTab};
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (link, _, _) = crate::play::play_channel();
    let press =
        |code: u16, modifiers| -> Result<[NormalizedInputEvent; 1], Box<dyn std::error::Error>> {
            Ok([NormalizedInputEvent::Key {
                code: KeyCode::new(code)?,
                state: ButtonState::Pressed,
                modifiers,
                repeat: false,
            }])
        };
    let open = || crate::play::mailbox::lock(&link.mailbox).chat.open;
    assert!(!open());
    link.chat_console_tab(ConsoleTab::Guild);
    assert!(link.chat_input(&press(7, Modifiers::one(Modifier::Alt))?));
    assert!(open(), "Alt+D opens the console it selects the tab of");
    crate::play::mailbox::lock(&link.mailbox).chat.open = false;
    assert_eq!(
        crate::play::mailbox::lock(&link.mailbox).chat.console.tab,
        ConsoleTab::All
    );
    assert!(link.chat_input(&press(88, Modifiers::NONE)?));
    assert!(open(), "Keypad Enter opens the console");
    assert_eq!(
        crate::play::mailbox::lock(&link.mailbox).chat.console.mode,
        ComposerMode::Temporary
    );
    Ok(())
}
