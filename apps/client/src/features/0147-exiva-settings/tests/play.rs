use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_ui::exiva_settings::{PlayerList, panel_text};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_session::{
    GatedCommandResult, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
type Error = Box<dyn std::error::Error + Send + Sync>;
struct Peer {
    list: PlayerList,
    commands: Arc<Mutex<Vec<Vec<u8>>>>,
}
impl Stepper for Peer {
    fn exiva_selected(&self) -> bool {
        true
    }
    async fn exiva(&mut self, payload: &[u8]) -> Result<GatedCommandResult, SessionError> {
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(payload.to_vec());
        match payload {
            [6, rest @ ..] => {
                assert_eq!(rest, b"Al");
                self.list.players.push(([3; 16], "Al".into()));
            }
            [2, rest @ ..] => {
                assert_eq!(rest, &[3; 16]);
                self.list.players.clear();
            }
            [5] => {}
            _ => return Err(SessionError::SessionUnusable),
        }
        Ok(GatedCommandResult {
            command_id: 1,
            status: oteryn_session::CommandStatus::Accepted,
            server_sequence: 1,
            payload: if payload == [5] {
                self.list.encode().ok_or(SessionError::SessionUnusable)?
            } else {
                vec![1]
            },
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
async fn ready(link: &PlayLink, count: Option<usize>) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let panel = link.exiva_panel();
            if panel.selected
                && !panel.pending
                && count.is_none_or(|n| panel.list.as_ref().is_some_and(|l| l.players.len() == n))
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn exiva_native_input_runs_edits_and_server_refresh_through_the_session_queue() -> Result<(), Error>
{
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let peer = Peer {
                    list: PlayerList {
                        modes: 2,
                        players: vec![],
                    },
                    commands: Arc::clone(&commands),
                };
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(peer, incoming, outbox));
                ready(&link, None).await?;
                let open = NormalizedInputEvent::Key {
                    code: KeyCode::new(8)?,
                    state: ButtonState::Pressed,
                    modifiers: Modifiers::one(Modifier::Control),
                    repeat: false,
                };
                assert!(link.exiva_input(&[open]));
                ready(&link, Some(0)).await?;
                assert!(link.exiva_panel().open);
                link.exiva_request(vec![6, b'A', b'l']);
                ready(&link, Some(1)).await?;
                assert!(panel_text(&link.exiva_panel()).contains("Al"));
                let mut remove = vec![2];
                remove.extend([3; 16]);
                link.exiva_request(remove.clone());
                ready(&link, Some(0)).await?;
                assert_eq!(
                    *commands.lock().unwrap_or_else(PoisonError::into_inner),
                    vec![vec![5], vec![6, b'A', b'l'], vec![5], remove, vec![5]]
                );
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Exiva UI test thread panicked")?
}
