use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::daily_reward::{
    DailyRewardCommand, DailyRewardReply, DailyRewardResponse, daily_reward,
};
use oteryn_client_ui::daily_reward::{Intent, panel_text};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
type Error = Box<dyn std::error::Error + Send + Sync>;
struct Peer {
    commands: Arc<Mutex<Vec<DailyRewardCommand>>>,
    claimed: bool,
    returner_offer: Option<[u8; 16]>,
}
impl Stepper for Peer {
    fn daily_reward_selected(&self) -> bool {
        true
    }
    async fn feature(
        &mut self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
    ) -> Result<oteryn_session::GatedCommandResult, SessionError> {
        assert_eq!(
            (command.capability, command.kind),
            (182, daily_reward::returner::QUERY)
        );
        Ok(oteryn_session::GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: daily_reward::returner::Reply {
                state: if self.returner_offer.is_some() { 1 } else { 0 },
                id: self.returner_offer,
            }
            .encode()
            .map_err(|_| SessionError::SessionUnusable)?,
        })
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
                streak: if self.claimed { 14 } else { 13 },
                jokers: 2,
                claimed: self.claimed,
                claim_enabled: !self.claimed,
            }),
            DailyRewardCommand::Claim => {
                self.claimed = true;
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
                let (peer_commands, peer) = (
                    Arc::clone(&commands),
                    Peer {
                        commands,
                        claimed: false,
                        returner_offer: None,
                    },
                );
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(peer, incoming, outbox));
                ready(&link, false).await?;
                assert!(
                    include_str!("../../../windows_shell/input.rs")
                        .contains("play.link.daily_reward_input(events)")
                );
                assert!(link.daily_reward_input(&[NormalizedInputEvent::Key {
                    code: KeyCode::new(7)?,
                    state: ButtonState::Pressed,
                    modifiers: Modifiers::one(Modifier::Control),
                    repeat: false,
                }]));
                assert!(link.daily_reward_panel().open);
                ready(&link, true).await?;
                assert!(panel_text(&link.daily_reward_panel()).contains("Day 5 of 7"));
                let text = panel_text(&link.daily_reward_panel());
                assert!(text.contains(
                    "Your reward streak currently grants you a 7% chance to gain double weapon proficiency XP."
                ));
                assert!(text.contains("Claim your daily reward once between two regular server saves"));
                link.daily_reward_request(Intent::Claim);
                tokio::time::timeout(Duration::from_secs(3), async {
                    while link.daily_reward_panel().pending {
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
                assert!(panel_text(&link.daily_reward_panel()).contains("Status: claimed"));
                link.daily_reward_request(Intent::Claim);
                assert_eq!(
                    peer_commands
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .len(),
                    2
                );
                link.daily_reward_request(Intent::Refresh);
                ready(&link, true).await?;
                assert!(
                    link.daily_reward_panel()
                        .wall
                        .is_some_and(|wall| wall.claimed && !wall.claim_enabled)
                );
                assert!(panel_text(&link.daily_reward_panel())
                    .contains("grants you a 7% chance to gain double weapon proficiency XP"));
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Daily reward UI test thread panicked")?
}
#[path = "details.rs"]
mod details;

#[path = "returner.rs"]
mod returner;
