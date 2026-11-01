use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::prey::{PreyCommand, PreyResponse, prey};
use oteryn_client_ui::prey::{Intent, panel_text};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
type Error = Box<dyn std::error::Error + Send + Sync>;
struct Peer {
    commands: Arc<Mutex<Vec<prey::Request>>>,
}
impl Stepper for Peer {
    fn prey_selected(&self) -> bool {
        true
    }
    async fn prey(&mut self, command: &PreyCommand) -> Result<PreyResponse, SessionError> {
        let encoded =
            prey::encode_request(&command.request).map_err(|_| SessionError::SessionUnusable)?;
        let decoded = prey::decode_request(&encoded).map_err(|_| SessionError::SessionUnusable)?;
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(decoded);
        let reply = prey::Reply {
            status: 0,
            wildcard_balance: 7,
            slots: vec![prey::Slot {
                slot: 1,
                state: 1,
                creatures: vec!["oteryn:rat".into()],
                selected: String::new(),
                bonus: 0,
                grade: 0,
                percent: 0,
                hunting_seconds: 0,
                option: 0,
                free_reroll_at: 10,
            }],
        };
        let bytes = prey::encode_reply(&reply).map_err(|_| SessionError::SessionUnusable)?;
        Ok(PreyResponse {
            status: CommandStatus::Accepted,
            reply: Some(prey::decode_reply(&bytes).map_err(|_| SessionError::SessionUnusable)?),
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
async fn ready(link: &PlayLink, loaded: bool) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let panel = link.prey_panel();
            if panel.selected && !panel.pending && (!loaded || panel.reply.is_some()) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn prey_actions_use_real_codecs_and_apply_the_server_projection() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let peer = Peer {
                    commands: Arc::clone(&commands),
                };
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(peer, incoming, outbox));
                ready(&link, false).await?;
                link.prey_request(Intent::Refresh);
                ready(&link, true).await?;
                assert!(panel_text(&link.prey_panel()).contains("oteryn:rat"));
                link.prey_request(Intent::ListReroll);
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
                    vec![prey::Request::Query, prey::Request::ListReroll { slot: 1 }]
                );
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Prey UI test thread panicked")?
}
