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
fn player_typing_command_and_push_reach_the_panel() -> Result<(), Error> {
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
                link.player_typing_request(true);
                tokio::time::timeout(Duration::from_secs(3), async {
                    while commands
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .is_empty()
                    {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert_eq!(
                    *commands.lock().unwrap_or_else(PoisonError::into_inner),
                    vec![vec![8, 1]]
                );
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "player typing test thread panicked")?
}
