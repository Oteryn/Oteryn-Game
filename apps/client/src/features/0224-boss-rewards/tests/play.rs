use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::boss_rewards::{
    BossRewardsCommand, BossRewardsReply, BossRewardsResponse, boss_rewards,
};
use oteryn_client_ui::boss_rewards::{Intent, panel_text};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
type Error = Box<dyn std::error::Error + Send + Sync>;
const DEATH: [u8; 16] = [9; 16];
type Commands = Arc<Mutex<Vec<(u32, Vec<u8>)>>>;
struct Peer {
    commands: Commands,
}
impl Stepper for Peer {
    fn boss_rewards_selected(&self) -> bool {
        true
    }
    async fn boss_rewards(
        &mut self,
        command: &BossRewardsCommand,
    ) -> Result<BossRewardsResponse, SessionError> {
        let (kind, bytes, reply) = match command {
            BossRewardsCommand::Browse(value) => (
                boss_rewards::COMMAND_TYPE_BOSS_REWARD_BROWSE,
                boss_rewards::encode_browse_request(value)
                    .map_err(|_| SessionError::SessionUnusable)?,
                BossRewardsReply::Browse(
                    boss_rewards::decode_browse_reply(
                        &boss_rewards::encode_browse_reply(&boss_rewards::BrowseReply {
                            status: boss_rewards::BossRewardStatus::Ok,
                            death: value.death,
                            expires_at_unix_ms: 99,
                            rows: vec![boss_rewards::RewardRow {
                                ordinal: 4,
                                definition_reference: 700,
                                quantity: 3,
                            }],
                            more: false,
                        })
                        .map_err(|_| SessionError::SessionUnusable)?,
                    )
                    .map_err(|_| SessionError::SessionUnusable)?,
                ),
            ),
            BossRewardsCommand::Take(value) => (
                boss_rewards::COMMAND_TYPE_BOSS_REWARD_TAKE,
                boss_rewards::encode_take_request(value)
                    .map_err(|_| SessionError::SessionUnusable)?,
                BossRewardsReply::Collect(boss_rewards::CollectReply {
                    status: boss_rewards::BossRewardStatus::Ok,
                    moved: 1,
                    remaining: 0,
                }),
            ),
            BossRewardsCommand::CollectAll(value) => (
                boss_rewards::COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL,
                boss_rewards::encode_collect_all_request(value)
                    .map_err(|_| SessionError::SessionUnusable)?,
                BossRewardsReply::Collect(boss_rewards::CollectReply {
                    status: boss_rewards::BossRewardStatus::Ok,
                    moved: 2,
                    remaining: 0,
                }),
            ),
        };
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((kind, bytes));
        Ok(BossRewardsResponse {
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
            let p = link.boss_rewards_panel();
            if p.selected && !p.pending && (!loaded || p.browse.is_some()) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn boss_reward_actions_emit_exact_payloads_and_apply_server_projection() -> Result<(), Error> {
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
                {
                    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
                    mailbox.boss_rewards.death = DEATH;
                    mailbox.boss_rewards.after_ordinal = 2;
                    mailbox.boss_rewards.selected_ordinal = 4;
                }
                link.boss_rewards_request(Intent::Browse);
                ready(&link, true).await?;
                assert!(panel_text(&link.boss_rewards_panel()).contains("item 700 x3"));
                link.boss_rewards_request(Intent::Take);
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
                {
                    let held = commands.lock().unwrap_or_else(PoisonError::into_inner);
                    assert_eq!(held[0].0, 428);
                    assert_eq!(
                        held[0].1,
                        [vec![1], DEATH.to_vec(), 2_u16.to_le_bytes().to_vec()].concat()
                    );
                    assert_eq!(held[1].0, 429);
                    assert_eq!(held[1].1.len(), 19);
                }
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "boss rewards UI test thread panicked")?
}
