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
}
impl Stepper for Peer {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == 54
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
        assert_eq!(command.capability, 54);
        assert_eq!(command.kind, kind);
        assert_eq!(command.payload, payload);
        self.exchanges
            .release
            .acquire()
            .await
            .map_err(|_| SessionError::SessionUnusable)?
            .forget();
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
        while !link.training_panel().selected || link.training_panel().pending {
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
    link.training_request(intent);
    assert!(link.training_panel().pending);
    // A second click before a response must not produce another command.
    link.training_request(intent);
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
#[test]
fn cp01_training_scenario() -> Result<(), Error> {
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
                    link.training_request(Intent::Status);
                    assert!(!link.training_panel().pending);
                    assert!(incoming.try_recv().is_err());
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                        },
                        incoming,
                        outbox,
                    ));
                    ready(&link).await.checked()?;
                    let event = oteryn_input_actions::NormalizedInputEvent::Key {
                        code: oteryn_input_actions::KeyCode::new(17).checked()?,
                        state: oteryn_input_actions::ButtonState::Pressed,
                        modifiers: oteryn_input_actions::Modifiers::one(
                            oteryn_input_actions::Modifier::Control,
                        )
                        .with(oteryn_input_actions::Modifier::Alt),
                        repeat: false,
                    };
                    assert!(link.training_input(&[event]));
                    assert!(link.training_panel().open);

                    for (i, value) in [(0, "7"), (1, "9"), (2, "11")] {
                        link.training_edit(i, value.into());
                    }
                    let cases = [
                        (Intent::Status, 67, vec![8, 2], 1, 100, 0),
                        (Intent::Start, 67, vec![8, 1, 16, 9, 24, 11], 1, 99, 0),
                        (Intent::Stop, 67, vec![8, 3], 2, 98, 0),
                        (Intent::Offline, 66, vec![8, 7], 3, 0, 43200),
                    ];
                    for (intent, kind, payload, disposition, charges, pool_seconds) in cases {
                        if kind == 66 {
                            assert_eq!(wire::decode_offline_training(&payload).checked()?.get(), 7);
                        } else {
                            wire::decode_exercise_training(&payload).checked()?;
                        }
                        send(
                            &link,
                            &exchanges,
                            intent,
                            kind,
                            payload,
                            CommandStatus::Accepted,
                            wire::encode_training_result(wire::TrainingResult {
                                disposition,
                                revision: 1,
                                charges,
                                pool_seconds,
                            })
                            .checked()?,
                        )
                        .await
                        .checked()?;
                        assert_eq!(link.training_panel().charges, Some(charges));
                        assert_eq!(link.training_panel().pool_seconds, Some(pool_seconds));
                    }
                    assert!(link.training_panel().feedback.contains("logging you out"));
                    send(
                        &link,
                        &exchanges,
                        Intent::Status,
                        67,
                        vec![8, 2],
                        CommandStatus::Accepted,
                        wire::encode_training_result(wire::TrainingResult::REFUSED).checked()?,
                    )
                    .await
                    .checked()?;
                    assert!(link.training_panel().feedback.contains("refused"));
                    send(
                        &link,
                        &exchanges,
                        Intent::Stop,
                        67,
                        vec![8, 3],
                        CommandStatus::Rejected,
                        vec![],
                    )
                    .await
                    .checked()?;
                    assert!(link.training_panel().feedback.contains("refused"));
                    send(
                        &link,
                        &exchanges,
                        Intent::Status,
                        67,
                        vec![8, 2],
                        CommandStatus::Accepted,
                        vec![8, 1],
                    )
                    .await
                    .checked()?;
                    assert!(link.training_panel().feedback.contains("Invalid"));
                    link.training_edit(1, "0".into());
                    link.training_request(Intent::Start);
                    assert!(!link.training_panel().pending);

                    drop(link);
                    task.await.checked()?;
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

use oteryn_client_ui::training::Intent;
use oteryn_session::property_services::loop_training as wire;

trait Checked<T> {
    fn checked(self) -> Result<T, Error>;
}
impl<T, E: std::fmt::Debug> Checked<T> for Result<T, E> {
    fn checked(self) -> Result<T, Error> {
        self.map_err(|error| format!("{error:?}").into())
    }
}

include!("batch.rs");
