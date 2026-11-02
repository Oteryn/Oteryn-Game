use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::daily_reward::{
    DailyRewardCommand, DailyRewardReply, DailyRewardResponse, daily_reward,
};
use oteryn_client_ui::daily_reward::{Intent, panel_text};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
type Error = Box<dyn std::error::Error + Send + Sync>;
struct Peer {
    commands: Arc<Mutex<Vec<DailyRewardCommand>>>,
}
impl Stepper for Peer {
    fn daily_reward_selected(&self) -> bool {
        true
    }
    async fn daily_reward(
        &mut self,
        command: &DailyRewardCommand,
    ) -> Result<DailyRewardResponse, SessionError> {
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(*command);
        let reply = match command {
            DailyRewardCommand::Query => DailyRewardReply::Wall(daily_reward::DailyRewardWall {
                lane_day: 12,
                streak: 19,
                jokers: 2,
                claimed: false,
                claim_enabled: true,
            }),
            DailyRewardCommand::Claim => {
                DailyRewardReply::Result(daily_reward::DailyRewardResult {
                    disposition: 1,
                    transaction_id: Some([0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1]),
                })
            }
        };
        Ok(DailyRewardResponse {
            status: CommandStatus::Accepted,
            reply: Some(reply),
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
            let p = link.daily_reward_panel();
            if p.selected && !p.pending && (!loaded || p.wall.is_some()) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn daily_reward_query_and_claim_drive_the_panel() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let (peer_commands, peer) = (Arc::clone(&commands), Peer { commands });
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(peer, incoming, outbox));
                ready(&link, false).await?;
                link.daily_reward_request(Intent::Refresh);
                ready(&link, true).await?;
                assert!(panel_text(&link.daily_reward_panel()).contains("Day 12 of 30"));
                link.daily_reward_request(Intent::Claim);
                tokio::time::timeout(Duration::from_secs(3), async {
                    while peer_commands
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
                    *peer_commands.lock().unwrap_or_else(PoisonError::into_inner),
                    vec![DailyRewardCommand::Query, DailyRewardCommand::Claim]
                );
                assert_eq!(
                    link.daily_reward_panel().feedback,
                    Some("Daily reward claimed")
                );
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Daily reward UI test thread panicked")?
}
