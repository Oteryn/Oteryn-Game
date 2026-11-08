use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_session::{
    CommandStatus, GatedCommandResult, LogoutResult, SessionError, SessionEvent, StepDirection,
    StepOutcome,
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};
type Error = Box<dyn std::error::Error + Send + Sync>;
type Exchange = (u32, Vec<u8>, Option<CommandStatus>, Vec<u8>);
struct Script {
    entries: Mutex<VecDeque<Exchange>>,
    release: tokio::sync::Semaphore,
}
struct Peer {
    exchanges: Arc<Script>,
}
impl Stepper for Peer {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == 117
    }
    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        let (kind, payload, status, reply) = self
            .exchanges
            .entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front()
            .ok_or(SessionError::SessionUnusable)?;
        assert_eq!(command.capability, 117);
        assert_eq!(command.kind, kind);
        assert_eq!(command.payload, payload);
        self.exchanges
            .release
            .acquire()
            .await
            .map_err(|_| SessionError::SessionUnusable)?
            .forget();
        let status = status.ok_or(SessionError::SessionUnusable)?;
        Ok(GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status,
            payload: reply,
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
async fn ready(link: &PlayLink) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !link.boss_encounter_panel().selected || link.boss_encounter_panel().pending {
            tokio::task::yield_now().await;
        }
    })
    .await
    .checked()?;
    Ok(())
}
async fn send(
    link: &PlayLink,
    exchanges: &Arc<Script>,
    intent: Intent,
    kind: u32,
    payload: Vec<u8>,
    status: CommandStatus,
    reply: Vec<u8>,
) -> Result<(), Error> {
    exchanges
        .entries
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push_back((kind, payload, Some(status), reply));
    link.boss_encounter_request(intent);
    assert!(link.boss_encounter_panel().pending);
    let draft = link.boss_encounter_panel().fields;
    link.boss_encounter_edit(0, "changed".into());
    assert_eq!(link.boss_encounter_panel().fields, draft);
    // A second click before a response must not produce another command.
    link.boss_encounter_request(intent);
    exchanges.release.add_permits(1);
    ready(link).await.checked()?;
    assert!(
        exchanges
            .entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_empty()
    );
    Ok(())
}
use oteryn_client_runtime::boss_encounter as wire;
use oteryn_client_ui::boss_encounter::Intent;

include!("activity_pc05_boss_activity_refusal_and_reconnect_scenario_1.rs");
trait Checked<T> {
    fn checked(self) -> Result<T, Error>;
}
impl<T, E: std::fmt::Debug> Checked<T> for Result<T, E> {
    fn checked(self) -> Result<T, Error> {
        self.map_err(|error| format!("{error:?}").into())
    }
}

#[test]
fn endgame_native_selection_submits_each_source_boss_and_browses_rewards() -> Result<(), Error> {
    use oteryn_client_ui::boss_encounter::EndgameChain;
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            let runtime = oteryn_client_runtime::ClientRuntime::new().checked()?;
            runtime
                .block_on(async {
                    let exchanges = Arc::new(Script {
                        entries: Mutex::new(VecDeque::new()),
                        release: tokio::sync::Semaphore::new(0),
                    });
                    let (link, incoming, outbox) = play_channel();
                    assert!(!select_chain(&link, EndgameChain::SoulWar));
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                        },
                        incoming,
                        outbox,
                    ));
                    ready(&link).await.checked()?;
                    for (chain, bosses) in [
                        (
                            EndgameChain::SoulWar,
                            vec![
                                "goshnar_s_cruelty",
                                "goshnar_s_greed",
                                "goshnar_s_hatred",
                                "goshnar_s_malice",
                                "goshnar_s_spite",
                                "goshnar_s_megalomania_purple",
                            ],
                        ),
                        (
                            EndgameChain::RottenBlood,
                            vec!["ichgahal", "murcion", "vemiath", "chagorz", "bakragore"],
                        ),
                    ] {
                        for boss in bosses {
                            assert!(select_chain(&link, chain));
                            let key = format!("oteryn:creature.{boss}");
                            assert_eq!(link.boss_encounter_panel().fields[0], key);
                            let reply = wire::encode_boss_activity_reply(
                                &wire::BossActivityReply::Refused(
                                    wire::BossActivityRefusal::ContentUnavailable,
                                ),
                            )
                            .checked()?;
                            send(
                                &link,
                                &exchanges,
                                Intent::Enter,
                                178,
                                wire::encode_boss_key(&key).checked()?,
                                CommandStatus::Rejected,
                                reply,
                            )
                            .await
                            .checked()?;
                            assert!(
                                link.boss_encounter_panel()
                                    .summary
                                    .contains("ContentUnavailable")
                            );
                            // An unavailable source room must remain a visible refusal, never
                            // advance local quest state or silently choose another boss.
                            assert_eq!(link.boss_encounter_panel().fields[0], key);
                        }
                        let chest = wire::encode_reward_chest(100, &[]).checked()?;
                        send(
                            &link,
                            &exchanges,
                            Intent::Rewards,
                            340,
                            vec![],
                            CommandStatus::Accepted,
                            chest,
                        )
                        .await
                        .checked()?;
                        assert!(link.boss_encounter_panel().rewards.is_some());
                    }
                    drop(link);
                    task.await.checked()?;
                    Ok::<(), Error>(())
                })
                .checked()?
                .checked()?;
            Ok(())
        })?
        .join()
        .map_err(|_| "endgame client selection thread panicked")?
}

fn select_chain(link: &PlayLink, chain: oteryn_client_ui::boss_encounter::EndgameChain) -> bool {
    oteryn_client_ui::boss_encounter::select_next_boss(
        &mut crate::play::mailbox::lock(&link.mailbox).boss_encounter, chain,
    )
}
