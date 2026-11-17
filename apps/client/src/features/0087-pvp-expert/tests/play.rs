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
type Exchange = (u32, Vec<u8>, CommandStatus, Vec<u8>);
struct Script {
    entries: Mutex<VecDeque<Exchange>>,
    release: tokio::sync::Semaphore,
}
struct Peer {
    exchanges: Arc<Script>,
    snapshot: Vec<u8>,
}
impl Stepper for Peer {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == 87
    }
    fn feature_snapshot(&self, domain: u32) -> Option<Vec<u8>> {
        (domain == 52).then(|| self.snapshot.clone())
    }
    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        // The test releases the transport result only after observing pending.
        self.exchanges
            .release
            .acquire()
            .await
            .map_err(|_| SessionError::SessionUnusable)?
            .forget();
        let (kind, payload, status, reply) = self
            .exchanges
            .entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front()
            .ok_or(SessionError::SessionUnusable)?;
        assert_eq!(command.capability, 87);
        assert_eq!(command.kind, kind);
        assert_eq!(command.payload, payload);
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
        while !link.pvp_expert_panel().selected || link.pvp_expert_panel().pending {
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
        .push_back((kind, payload, status, reply));
    link.pvp_expert_request(intent);
    assert!(link.pvp_expert_panel().pending);
    // A second click before a response must not produce another command.
    link.pvp_expert_request(intent);
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
use oteryn_client_runtime::pvp_expert as wire;
use oteryn_client_ui::pvp_expert::Intent;
#[test]
fn pc01_pvp_expert_scenario() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            let runtime = oteryn_client_runtime::ClientRuntime::new().checked()?;
            runtime
                .block_on(async {
                    let initial = wire::ExpertFightModes::default();
                    let bytes = wire::encode_expert_fight_modes(&initial);
                    let exchanges = Arc::new(Script {
                        entries: Mutex::new(VecDeque::new()),
                        release: tokio::sync::Semaphore::new(0),
                    });
                    let (link, mut incoming, outbox) = play_channel();
                    link.pvp_expert_request(Intent::Apply);
                    assert!(!link.pvp_expert_panel().pending);
                    assert!(incoming.try_recv().is_err());
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                            snapshot: bytes.clone(),
                        },
                        incoming,
                        outbox.clone(),
                    ));
                    ready(&link).await.checked()?;
                    let event = oteryn_input_actions::NormalizedInputEvent::Key {
                        code: oteryn_input_actions::KeyCode::new(8).checked()?,
                        state: oteryn_input_actions::ButtonState::Pressed,
                        modifiers: oteryn_input_actions::Modifiers::one(
                            oteryn_input_actions::Modifier::Control,
                        )
                        .with(oteryn_input_actions::Modifier::Alt),
                        repeat: false,
                    };
                    assert!(link.pvp_expert_input(&[event]));
                    assert!(link.pvp_expert_panel().open);
                    assert_eq!(link.pvp_expert_panel().confirmed, Some(initial));
                    for (i, value) in [(0, "1"), (1, "2"), (2, "0"), (3, "3")] {
                        link.pvp_expert_edit(i, value.into());
                    }
                    let requested = wire::ExpertFightModes {
                        modes: wire::attack::FightModes {
                            fight_mode: wire::attack::FightMode::Offensive,
                            chase: wire::attack::ChaseMode::Chase,
                            secure: false,
                        },
                        expert: wire::ExpertMode::RedFist,
                    };
                    let payload = wire::encode_expert_fight_modes(&requested);
                    send(
                        &link,
                        &exchanges,
                        Intent::Apply,
                        103,
                        payload.clone(),
                        CommandStatus::Accepted,
                        wire::attack::encode_attack_intent_result(
                            wire::attack::AttackIntentDisposition::Ok,
                        ),
                    )
                    .await
                    .checked()?;
                    assert_eq!(link.pvp_expert_panel().confirmed, Some(initial));
                    // Optional PvP can override Red Fist to Dove. Confirm only the pushed tuple.
                    let confirmed = wire::ExpertFightModes {
                        expert: wire::ExpertMode::Dove,
                        ..requested
                    };
                    outbox.push(vec![SessionEvent::ExpertFightModes(
                        oteryn_session::AppliedDelta {
                            server_sequence: 2,
                            base_revision: 1,
                            new_revision: 2,
                            value: confirmed,
                        },
                    )]);
                    assert_eq!(link.pvp_expert_panel().confirmed, Some(confirmed));
                    assert!(link.pvp_expert_panel().summary.contains("Dove"));
                    let payload = wire::encode_expert_fight_modes(&confirmed);
                    let intent = Intent::Apply;
                    let refused = wire::attack::encode_attack_intent_result(
                        wire::attack::AttackIntentDisposition::Rejected,
                    );

                    send(
                        &link,
                        &exchanges,
                        intent,
                        103,
                        payload.clone(),
                        CommandStatus::Accepted,
                        refused,
                    )
                    .await
                    .checked()?;
                    assert!(
                        link.pvp_expert_panel()
                            .feedback
                            .to_lowercase()
                            .contains("refused")
                    );
                    send(
                        &link,
                        &exchanges,
                        intent,
                        103,
                        payload.clone(),
                        CommandStatus::Rejected,
                        vec![],
                    )
                    .await
                    .checked()?;
                    assert!(
                        link.pvp_expert_panel()
                            .feedback
                            .to_lowercase()
                            .contains("refused")
                    );
                    send(
                        &link,
                        &exchanges,
                        intent,
                        103,
                        payload,
                        CommandStatus::Accepted,
                        vec![255],
                    )
                    .await
                    .checked()?;
                    assert!(link.pvp_expert_panel().feedback.contains("Invalid"));
                    link.pvp_expert_edit(0, "invalid".into());
                    link.pvp_expert_request(Intent::Apply);
                    assert!(!link.pvp_expert_panel().pending);
                    // Ending with a request in flight clears pending, selection, drafts and confirmed state.
                    link.pvp_expert_edit(0, "2".into());
                    link.pvp_expert_request(Intent::Apply);
                    assert!(link.pvp_expert_panel().pending);
                    exchanges.release.add_permits(1);
                    task.await.checked()?;
                    let ended = link.pvp_expert_panel();
                    assert!(!ended.pending);
                    assert!(!ended.selected);
                    assert!(!ended.open);
                    assert!(ended.summary.is_empty());
                    assert!(ended.fields.iter().all(String::is_empty));
                    assert!(ended.feedback.contains("ended"));
                    // Re-admit on this mailbox with a fresh peer. No old selection/confirmation survives.
                    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
                    let reconnect = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                            snapshot: bytes,
                        },
                        receiver,
                        outbox.clone(),
                    ));
                    ready(&link).await.checked()?;
                    assert!(!link.pvp_expert_panel().open);
                    assert!(!link.pvp_expert_panel().pending);
                    assert_eq!(link.pvp_expert_panel().confirmed, Some(initial));
                    drop(sender);
                    reconnect.await.checked()?;
                    Ok::<(), Error>(())
                })
                .checked()?
                .checked()?;
            Ok(())
        })
        .checked()?
        .join()
        .map_err(|_| "scenario thread failed")
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
