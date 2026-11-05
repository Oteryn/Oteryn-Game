use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::chat_public_channels::{
    ChatDisposition, ChatIntent, ChatLine, ChatOutcome, ChatRoom, ChatRoomSet,
};
use oteryn_client_ui::chat_public_channels::{Intent, panel_text};
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
    fn chat_public_selected(&self) -> bool {
        true
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
            ChatIntent::OpenRoom(room) => self.rooms.insert(*room),
            ChatIntent::CloseRoom(room) => self.rooms.remove(*room),
            ChatIntent::Room { room, text } => self.lines.push(ChatLine::Room {
                room: *room,
                speaker_name: "Player One".into(),
                text: text.clone(),
            }),
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
    predicate: impl Fn(&oteryn_client_ui::chat_public_channels::ChatPanel) -> bool,
) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if predicate(&link.chat_public_panel()) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}

#[test]
fn public_channel_commands_leave_the_outbox_and_server_state_updates_the_panel() -> Result<(), Error>
{
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let peer = Peer {
                    commands: Arc::clone(&commands),
                    rooms: ChatRoomSet::default(),
                    lines: vec![ChatLine::Room {
                        room: ChatRoom::World,
                        speaker_name: "Other Player".into(),
                        text: "welcome".into(),
                    }],
                };
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(peer, incoming, outbox));
                wait_for(&link, |panel| panel.selected).await?;
                let event = oteryn_input_actions::NormalizedInputEvent::Key {
                    code: oteryn_input_actions::KeyCode::new(6)?,
                    state: oteryn_input_actions::ButtonState::Pressed,
                    modifiers: oteryn_input_actions::Modifiers::one(
                        oteryn_input_actions::Modifier::Control,
                    ),
                    repeat: false,
                };
                assert!(link.chat_public_input(&[event]));
                assert!(link.chat_public_panel().open);
                assert!(
                    panel_text(&link.chat_public_panel()).contains("[World] Other Player: welcome")
                );
                link.chat_public_request(Intent::Open(ChatRoom::Tutor));
                wait_for(&link, |panel| panel.rooms.contains(ChatRoom::Tutor)).await?;
                link.chat_public_text("hello tutors".into());
                link.chat_public_request(Intent::Send(ChatRoom::Tutor));
                wait_for(&link, |panel| {
                    panel.lines.iter().any(|line| {
                        matches!(
                            line,
                            ChatLine::Room {
                                room: ChatRoom::Tutor,
                                ..
                            }
                        )
                    })
                })
                .await?;
                assert!(
                    panel_text(&link.chat_public_panel())
                        .contains("[Tutor] Player One: hello tutors")
                );
                link.chat_public_request(Intent::Close(ChatRoom::Tutor));
                wait_for(&link, |panel| {
                    !panel.pending && !panel.rooms.contains(ChatRoom::Tutor)
                })
                .await?;
                assert!(matches!(
                    commands
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .first(),
                    Some(ChatIntent::OpenRoom(ChatRoom::Tutor))
                ));
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "public chat UI test thread panicked")?
}
