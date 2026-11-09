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
        capability == 57
    }
    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
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
        assert_eq!(command.capability, 57);
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
        while !link.cyclopedia_map_panel().selected || link.cyclopedia_map_panel().pending {
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
    link.cyclopedia_map_request(intent);
    assert!(link.cyclopedia_map_panel().pending);
    let draft = link.cyclopedia_map_panel().fields;
    link.cyclopedia_map_edit(0, "changed".into());
    assert_eq!(link.cyclopedia_map_panel().fields, draft);
    // A second click before a response must not produce another command.
    link.cyclopedia_map_request(intent);
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
use oteryn_client_runtime::cyclopedia_map as wire;
use oteryn_client_ui::cyclopedia_map::Intent;

#[test]
fn pc06_cyclopedia_map_scenario() -> Result<(), Error> {
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
                    link.cyclopedia_map_request(Intent::Query);
                    assert!(!link.cyclopedia_map_panel().pending);
                    assert!(incoming.try_recv().is_err());
                    let task = tokio::spawn(run_session(
                        Peer {
                            exchanges: Arc::clone(&exchanges),
                        },
                        incoming,
                        outbox.clone(),
                    ));
                    ready(&link).await.checked()?;
                    let event = oteryn_input_actions::NormalizedInputEvent::Key {
                        code: oteryn_input_actions::KeyCode::new(24).checked()?,
                        state: oteryn_input_actions::ButtonState::Pressed,
                        modifiers: oteryn_input_actions::Modifiers::one(
                            oteryn_input_actions::Modifier::Control,
                        )
                        .with(oteryn_input_actions::Modifier::Alt),
                        repeat: false,
                    };
                    assert!(link.cyclopedia_map_input(&[event]));
                    assert!(link.cyclopedia_map_panel().open);
                    let fields: [&str; 1] = ["12"];
                    for (i, value) in fields.iter().enumerate() {
                        link.cyclopedia_map_edit(i, (*value).into());
                    }
                    let intent = Intent::Query;
                    let payload = vec![];
                    let reply = wire::encode_result(&wire::MapResult {
                        code: 0,
                        revision: 1,
                        discovered: false,
                        found: 2,
                        area_percent: 20,
                        pool: 200,
                        epoch: 3,
                        improved: false,
                        visible: vec![(100, 200, -7)],
                    })
                    .checked()?;
                    send(
                        &link,
                        &exchanges,
                        intent,
                        72,
                        payload.clone(),
                        CommandStatus::Accepted,
                        reply.clone(),
                    )
                    .await
                    .checked()?;
                    assert!(!link.cyclopedia_map_panel().summary.is_empty());

                    for (action, operation) in [
                        (Intent::Start, 1),
                        (Intent::Shuffle, 2),
                        (Intent::Find, 3),
                        (Intent::Donate, 4),
                    ] {
                        let amount = if operation == 4 { 12u32 } else { 0 };
                        let mut bytes = vec![operation];
                        bytes.extend_from_slice(&1u32.to_le_bytes());
                        bytes.extend_from_slice(&amount.to_le_bytes());
                        send(
                            &link,
                            &exchanges,
                            action,
                            73,
                            bytes,
                            CommandStatus::Accepted,
                            reply.clone(),
                        )
                        .await
                        .checked()?;
                    }
                    let confirmed = link.cyclopedia_map_panel().summary;
                    for code in 1..=7 {
                        let mut refused = reply.clone();
                        refused[0] = code;
                        send(
                            &link,
                            &exchanges,
                            intent,
                            72,
                            vec![],
                            CommandStatus::Accepted,
                            refused,
                        )
                        .await
                        .checked()?;
                        assert_eq!(link.cyclopedia_map_panel().summary, confirmed);
                        assert!(!link.cyclopedia_map_panel().feedback.is_empty());
                    }
                    link.cyclopedia_map_edit(0, "0".into());
                    link.cyclopedia_map_request(Intent::Donate);
                    assert!(!link.cyclopedia_map_panel().pending);
                    link.cyclopedia_map_edit(0, "12".into());
                    let reconnect_payload = payload.clone();
                    let reconnect_reply = reply.clone();
                    let summary = link.cyclopedia_map_panel().summary;
                    assert!(!summary.is_empty());
                    send(
                        &link,
                        &exchanges,
                        intent,
                        72,
                        payload.clone(),
                        CommandStatus::Rejected,
                        vec![],
                    )
                    .await
                    .checked()?;
                    assert!(link.cyclopedia_map_panel().feedback.contains("refused"));
                    assert_eq!(link.cyclopedia_map_panel().summary, summary);
                    send(
                        &link,
                        &exchanges,
                        intent,
                        72,
                        payload.clone(),
                        CommandStatus::Accepted,
                        vec![255],
                    )
                    .await
                    .checked()?;
                    assert!(link.cyclopedia_map_panel().feedback.contains("Invalid"));
                    assert_eq!(link.cyclopedia_map_panel().summary, summary);
                    send(
                        &link,
                        &exchanges,
                        intent,
                        72,
                        payload,
                        CommandStatus::DuplicateReplay,
                        reply,
                    )
                    .await
                    .checked()?;
                    assert_eq!(link.cyclopedia_map_panel().summary, summary);
                    // An unexpected transport end while a request is in flight resets every session field.
                    link.cyclopedia_map_request(intent);
                    assert!(link.cyclopedia_map_panel().pending);
                    exchanges.release.add_permits(1);
                    task.await.checked()?;
                    let ended = link.cyclopedia_map_panel();
                    assert!(!ended.pending && !ended.selected && !ended.open);
                    assert!(ended.summary.is_empty());
                    assert!(ended.fields.iter().all(String::is_empty));
                    assert!(ended.feedback.contains("ended"));
                    link.cyclopedia_map_request(intent);
                    assert!(!link.cyclopedia_map_panel().pending);
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
                    let fresh = reconnected.cyclopedia_map_panel();
                    assert!(!fresh.open && !fresh.pending);
                    assert!(fresh.summary.is_empty() && fresh.fields.iter().all(String::is_empty));
                    for (index, value) in fields.into_iter().enumerate() {
                        reconnected.cyclopedia_map_edit(index, value.into());
                    }
                    send(
                        &reconnected,
                        &exchanges,
                        intent,
                        72,
                        reconnect_payload,
                        CommandStatus::Accepted,
                        reconnect_reply,
                    )
                    .await
                    .checked()?;
                    assert!(!reconnected.cyclopedia_map_panel().summary.is_empty());
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
