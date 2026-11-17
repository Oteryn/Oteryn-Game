use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::player_typing::{PlayerTypingCommand, PlayerTypingResponse};
use oteryn_session::{
    AppliedDelta, ChatSpeaker, CommandStatus, LogoutResult, SessionError, SessionEvent,
    StepDirection, StepOutcome, player_typing,
};
use std::num::NonZeroU64;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
type Error = Box<dyn std::error::Error + Send + Sync>;
struct Peer {
    commands: Arc<Mutex<Vec<Vec<u8>>>>,
    pushed: bool,
}
impl Stepper for Peer {
    fn player_typing_selected(&self) -> bool {
        true
    }
    async fn player_typing(
        &mut self,
        command: &PlayerTypingCommand,
    ) -> Result<PlayerTypingResponse, SessionError> {
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(player_typing::encode_intent(command.typing));
        Ok(PlayerTypingResponse {
            status: CommandStatus::Accepted,
        })
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        if self.pushed {
            return Vec::new();
        }
        self.pushed = true;
        vec![SessionEvent::PlayerTyping(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: player_typing::TypingUpdate {
                speaker: ChatSpeaker {
                    identity: [7; 16],
                    generation: NonZeroU64::new(9).unwrap_or(NonZeroU64::MIN),
                },
                typing: true,
            },
        })]
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
async fn ready(link: &PlayLink) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let p = link.player_typing_panel();
            if p.selected && !p.indicators.is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn composer_typing_and_push_reach_the_panel() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let peer = Peer {
                    commands: Arc::clone(&commands),
                    pushed: false,
                };
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(peer, incoming, outbox));
                ready(&link).await?;
                assert_eq!(link.player_typing_panel().indicators[0].identity, [7; 16]);
                link.chat_composer(true, "hello".into());
                link.chat_composer(true, "hello again".into());
                link.chat_composer(false, "hello again".into());
                tokio::time::timeout(Duration::from_secs(3), async {
                    while commands
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .len()
                        < 2
                    {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert_eq!(
                    *commands.lock().unwrap_or_else(PoisonError::into_inner),
                    vec![vec![8, 1], vec![8, 0]]
                );
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "player typing test thread panicked")?
}

#[test]
fn sending_chat_queues_stop_behind_an_in_flight_start() -> Result<(), Error> {
    let (link, mut incoming, outbox) = play_channel();
    outbox.player_typing_initialize(true);
    outbox.chat_initialize(true);
    link.chat_text("hello".into());
    link.chat_request(oteryn_client_ui::chat::Mode::Say, "");
    assert!(matches!(
        incoming.try_recv()?,
        crate::combat_input::PlayCommand::PlayerTyping(PlayerTypingCommand { typing: true, .. })
    ));
    assert!(matches!(
        incoming.try_recv()?,
        crate::combat_input::PlayCommand::PlayerTyping(PlayerTypingCommand { typing: false, .. })
    ));
    assert!(matches!(
        incoming.try_recv()?,
        crate::combat_input::PlayCommand::ChatPublic(_)
    ));
    assert!(incoming.try_recv().is_err());
    Ok(())
}

#[test]
fn typing_initialization_resets_the_composer_start_gate() -> Result<(), Error> {
    let (link, mut incoming, outbox) = play_channel();
    outbox.player_typing_initialize(true);
    link.chat_text("hello".into());
    assert!(matches!(
        incoming.try_recv()?,
        crate::combat_input::PlayCommand::PlayerTyping(_)
    ));
    outbox.player_typing_initialize(false);
    link.chat_text("hello".into());
    assert!(incoming.try_recv().is_err());
    outbox.player_typing_initialize(true);
    link.chat_text("hello".into());
    assert!(matches!(
        incoming.try_recv()?,
        crate::combat_input::PlayCommand::PlayerTyping(_)
    ));
    Ok(())
}
