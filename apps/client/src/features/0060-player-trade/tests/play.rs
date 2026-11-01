use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::player_trade::{
    EntityRef, PlayerTradeCommand, PlayerTradeResponse, player_trade,
};
use oteryn_client_ui::player_trade::{Intent, panel_text};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::{
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};
type Error = Box<dyn std::error::Error + Send + Sync>;
struct Peer {
    state: player_trade::State,
    commands: Arc<Mutex<Vec<player_trade::Intent>>>,
}
impl Stepper for Peer {
    fn player_trade_selected(&self) -> bool {
        true
    }
    fn player_trade_state(&self) -> Option<player_trade::State> {
        Some(self.state.clone())
    }
    async fn player_trade(
        &mut self,
        request: &PlayerTradeCommand,
    ) -> Result<PlayerTradeResponse, SessionError> {
        let intent = player_trade::decode_intent(&request.payload)
            .map_err(|_| SessionError::SessionUnusable)?;
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(intent);
        self.state.look = if matches!(
            intent,
            player_trade::Intent::Look {
                counter_offer: true,
                ..
            }
        ) {
            "a steel sword".into()
        } else {
            String::new()
        };
        let encoded =
            player_trade::encode_state(&self.state).map_err(|_| SessionError::SessionUnusable)?;
        Ok(PlayerTradeResponse {
            status: CommandStatus::Accepted,
            result: player_trade::decode_result(&player_trade::ResultCode::Accepted.payload())
                .map_err(|_| SessionError::SessionUnusable)?,
            state: player_trade::decode_state(&encoded)
                .map_err(|_| SessionError::SessionUnusable)?,
        })
    }
    async fn serve(&mut self, d: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(d).await;
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
async fn ready(
    link: &PlayLink,
    commands: &Arc<Mutex<Vec<player_trade::Intent>>>,
    count: usize,
) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let panel = link.player_trade_panel();
            let received = commands
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .len();
            if panel.selected && !panel.pending && received == count {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn player_trade_controls_encode_requests_and_apply_real_wire_state() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let state = player_trade::State {
                    trade_id: Some([3; 16]),
                    partner: Some(EntityRef {
                        identity: [4; 16],
                        generation: 2,
                    }),
                    own: Some(player_trade::Offer {
                        definition: 10,
                        count: 1,
                        sub_type: 0,
                    }),
                    counter: Some(player_trade::Offer {
                        definition: 11,
                        count: 2,
                        sub_type: 3,
                    }),
                    accepted: [false, false],
                    phase: 2,
                    reason: 0,
                    message: String::new(),
                    look: String::new(),
                };
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(
                    Peer {
                        state,
                        commands: Arc::clone(&commands),
                    },
                    incoming,
                    outbox,
                ));
                ready(&link, &commands, 0).await?;
                for (index, intent) in [
                    Intent::Accept,
                    Intent::Cancel,
                    Intent::LookOwn,
                    Intent::LookCounter,
                ]
                .into_iter()
                .enumerate()
                {
                    link.player_trade_request(intent);
                    ready(&link, &commands, index + 1).await?;
                }
                assert!(panel_text(&link.player_trade_panel()).contains("steel sword"));
                {
                    let got = commands.lock().unwrap_or_else(PoisonError::into_inner);
                    assert_eq!(got.len(), 4);
                }
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "player trade UI test thread panicked")?
}
