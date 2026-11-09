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

#[test]
fn pc05_boss_activity_refusal_and_reconnect_scenario() -> Result<(), Error> {
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
                    let (link, mut incoming, outbox) = play_channel();
                    link.boss_encounter_request(Intent::Enter);
                    assert!(!link.boss_encounter_panel().pending);
                    assert!(incoming.try_recv().is_err());
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                        },
                        incoming,
                        outbox.clone(),
                    ));
                    ready(&link).await.checked()?;
                    link.boss_encounter_edit(0, "boss:test".into());
                    let key = wire::encode_boss_key("boss:test").checked()?;
                    let mut handoff = [1; 16];
                    handoff[6] = 0x70;
                    handoff[8] = 0x80;
                    let accepted =
                        wire::encode_boss_activity_reply(&wire::BossActivityReply::Accepted {
                            handoff,
                            deadline_unix_ms: Some(123),
                        })
                        .checked()?;
                    for (intent, kind) in [
                        (Intent::Enter, 178),
                        (Intent::Defeated, 179),
                        (Intent::Timeout, 180),
                    ] {
                        send(
                            &link,
                            &exchanges,
                            intent,
                            kind,
                            key.clone(),
                            CommandStatus::Accepted,
                            accepted.clone(),
                        )
                        .await
                        .checked()?;
                        assert!(link.boss_encounter_panel().summary.contains("accepted"));
                        let summary = link.boss_encounter_panel().summary;
                        send(
                            &link,
                            &exchanges,
                            intent,
                            kind,
                            key.clone(),
                            CommandStatus::Rejected,
                            vec![],
                        )
                        .await
                        .checked()?;
                        assert!(link.boss_encounter_panel().feedback.is_some());
                        assert_eq!(link.boss_encounter_panel().summary, summary);
                        let refused = wire::encode_boss_activity_reply(
                            &wire::BossActivityReply::Refused(wire::BossActivityRefusal::Cooldown),
                        )
                        .checked()?;
                        send(
                            &link,
                            &exchanges,
                            intent,
                            kind,
                            key.clone(),
                            CommandStatus::Rejected,
                            refused,
                        )
                        .await
                        .checked()?;
                        assert!(link.boss_encounter_panel().summary.contains("Cooldown"));
                        send(
                            &link,
                            &exchanges,
                            intent,
                            kind,
                            key.clone(),
                            CommandStatus::DuplicateReplay,
                            accepted.clone(),
                        )
                        .await
                        .checked()?;
                        assert_eq!(link.boss_encounter_panel().summary, summary);
                        send(
                            &link,
                            &exchanges,
                            intent,
                            kind,
                            key.clone(),
                            CommandStatus::Accepted,
                            vec![255],
                        )
                        .await
                        .checked()?;
                        assert_eq!(link.boss_encounter_panel().summary, summary);
                    }
                    link.boss_encounter_edit(0, "oteryn:creature.test".into());
                    link.boss_encounter_edit(1, "2".into());
                    link.boss_encounter_edit(2, "3".into());
                    let chest = wire::encode_reward_chest(
                        100,
                        &[wire::BossRewardChestEntry {
                            death: [7; 16],
                            boss_key: "boss:test".into(),
                            minted_at_unix_ms: 1,
                            expires_at_unix_ms: 1 + wire::BOSS_REWARD_EXPIRY_MS,
                            item_count: 2,
                        }],
                    )
                    .checked()?;
                    for (intent, kind, payload, reply, text) in [
                        (Intent::HazardQuery, 169, vec![], vec![5, 2], "Hazard: 2"),
                        (Intent::HazardSelect, 169, vec![3], vec![5, 3], "Hazard: 3"),
                        (
                            Intent::SlotsQuery,
                            491,
                            vec![0],
                            vec![0; 26],
                            "Boss slots: Updated",
                        ),
                        (
                            Intent::SlotEquip,
                            491,
                            [vec![1, 2], b"oteryn:creature.test".to_vec()].concat(),
                            vec![0; 26],
                            "Boss slots: Updated",
                        ),
                        (
                            Intent::SlotRemove,
                            491,
                            vec![2, 2],
                            vec![0; 26],
                            "Boss slots: Updated",
                        ),
                        (Intent::Rewards, 340, vec![], chest, "boss:test"),
                    ] {
                        send(
                            &link,
                            &exchanges,
                            intent,
                            kind,
                            payload.clone(),
                            CommandStatus::Accepted,
                            reply,
                        )
                        .await
                        .checked()?;
                        assert!(link.boss_encounter_panel().summary.contains(text));
                        let summary = link.boss_encounter_panel().summary;
                        send(
                            &link,
                            &exchanges,
                            intent,
                            kind,
                            payload.clone(),
                            CommandStatus::Rejected,
                            vec![],
                        )
                        .await
                        .checked()?;
                        assert_eq!(link.boss_encounter_panel().summary, summary);
                        send(
                            &link,
                            &exchanges,
                            intent,
                            kind,
                            payload,
                            CommandStatus::Accepted,
                            vec![255],
                        )
                        .await
                        .checked()?;
                        assert_eq!(link.boss_encounter_panel().summary, summary);
                    }
                    // Hold the transport failure until the pending state has been observed.
                    exchanges
                        .entries
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .push_back((
                            178,
                            wire::encode_boss_key("oteryn:creature.test").checked()?,
                            None,
                            vec![],
                        ));
                    link.boss_encounter_request(Intent::Enter);
                    assert!(link.boss_encounter_panel().pending);
                    exchanges.release.add_permits(1);
                    task.await.checked()?;
                    let ended = link.boss_encounter_panel();
                    assert!(!ended.pending && !ended.open && !ended.selected);
                    assert!(ended.fields.iter().all(String::is_empty));
                    assert!(
                        ended.summary.is_empty()
                            && ended.snapshot.is_none()
                            && ended.rewards.is_none()
                    );
                    assert!(ended.feedback.is_some_and(|v| v.contains("ended")));
                    let (commands, receiver) = tokio::sync::mpsc::unbounded_channel();
                    let reconnected = PlayLink {
                        commands,
                        mailbox: Arc::clone(&link.mailbox),
                    };
                    let reconnect = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                        },
                        receiver,
                        outbox,
                    ));
                    ready(&reconnected).await.checked()?;
                    assert!(reconnected.boss_encounter_panel().summary.is_empty());
                    reconnected.boss_encounter_edit(0, "boss:test".into());
                    send(
                        &reconnected,
                        &exchanges,
                        Intent::Enter,
                        178,
                        key,
                        CommandStatus::Accepted,
                        accepted,
                    )
                    .await
                    .checked()?;
                    assert!(
                        reconnected
                            .boss_encounter_panel()
                            .summary
                            .contains("accepted")
                    );
                    drop(reconnected);
                    reconnect.await.checked()?;
                    Ok::<(), Error>(())
                })
                .checked()?
                .checked()?;
            Ok(())
        })
        .checked()?
        .join()
        .map_err(|_| "boss activity test thread failed")
        .checked()?
}
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
