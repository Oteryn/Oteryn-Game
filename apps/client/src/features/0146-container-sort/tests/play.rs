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
        capability == 146
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
        assert_eq!(command.capability, 146);
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
        while !link.container_sort_panel().selected || link.container_sort_panel().pending {
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
    link.container_sort_request(intent);
    assert!(link.container_sort_panel().pending);
    let draft = link.container_sort_panel().fields;
    link.container_sort_edit(0, "changed".into());
    assert_eq!(link.container_sort_panel().fields, draft);
    // A second click before a response must not produce another command.
    link.container_sort_request(intent);
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
use oteryn_client_runtime::container_sort as wire;
use oteryn_client_ui::container_sort::Intent;

#[test]
fn pc02_container_sort_scenario() -> Result<(), Error> {
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
                    link.container_sort_request(Intent::Sort);
                    assert!(!link.container_sort_panel().pending);
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
                        code: oteryn_input_actions::KeyCode::new(22).checked()?,
                        state: oteryn_input_actions::ButtonState::Pressed,
                        modifiers: oteryn_input_actions::Modifiers::one(
                            oteryn_input_actions::Modifier::Control,
                        )
                        .with(oteryn_input_actions::Modifier::Alt),
                        repeat: false,
                    };
                    assert!(link.container_sort_input(&[event]));
                    assert!(link.container_sort_panel().open);
                    let fields = ["3", "4", "1", "1"];
                    for (i, value) in fields.iter().enumerate() {
                        link.container_sort_edit(i, (*value).into());
                    }
                    let intent = Intent::Sort;
                    let payload = vec![3, 4, 1, 1];
                    let decoded = wire::Request::decode(&payload).ok_or("sort codec")?;
                    assert_eq!(decoded.key, wire::SortKey::StackSize);
                    assert!(decoded.descending && decoded.recursive);
                    let reply = vec![1];
                    send(
                        &link,
                        &exchanges,
                        intent,
                        168,
                        payload.clone(),
                        CommandStatus::Accepted,
                        reply.clone(),
                    )
                    .await
                    .checked()?;
                    assert!(link.container_sort_panel().summary.contains("sorted"));
                    send(
                        &link,
                        &exchanges,
                        intent,
                        168,
                        payload.clone(),
                        CommandStatus::Accepted,
                        vec![2],
                    )
                    .await
                    .checked()?;
                    assert!(link.container_sort_panel().feedback.contains("refused"));
                    // Every native sort option reaches the existing codec, including false flags.
                    for key in 1..=4 {
                        link.container_sort_edit(1, key.to_string());
                        link.container_sort_edit(2, "0".into());
                        link.container_sort_edit(3, "0".into());
                        send(
                            &link,
                            &exchanges,
                            intent,
                            168,
                            vec![3, key, 0, 0],
                            CommandStatus::Accepted,
                            vec![1],
                        )
                        .await
                        .checked()?;
                    }
                    for (i, value) in fields.iter().enumerate() {
                        link.container_sort_edit(i, (*value).into());
                    }
                    let reconnect_payload = payload.clone();
                    let reconnect_reply = reply.clone();
                    let valid_first = "3";
                    let summary = link.container_sort_panel().summary;
                    assert!(!summary.is_empty());
                    send(
                        &link,
                        &exchanges,
                        intent,
                        168,
                        payload.clone(),
                        CommandStatus::Rejected,
                        vec![],
                    )
                    .await
                    .checked()?;
                    assert!(link.container_sort_panel().feedback.contains("refused"));
                    assert_eq!(link.container_sort_panel().summary, summary);
                    send(
                        &link,
                        &exchanges,
                        intent,
                        168,
                        payload.clone(),
                        CommandStatus::Accepted,
                        vec![255],
                    )
                    .await
                    .checked()?;
                    assert!(link.container_sort_panel().feedback.contains("Invalid"));
                    assert_eq!(link.container_sort_panel().summary, summary);
                    send(
                        &link,
                        &exchanges,
                        intent,
                        168,
                        payload,
                        CommandStatus::DuplicateReplay,
                        reply,
                    )
                    .await
                    .checked()?;
                    assert_eq!(link.container_sort_panel().summary, summary);
                    link.container_sort_edit(0, "invalid".into());
                    link.container_sort_request(intent);
                    assert!(!link.container_sort_panel().pending);
                    assert!(
                        link.container_sort_panel().feedback.contains("valid")
                            || link.container_sort_panel().feedback.contains("numeric")
                    );
                    link.container_sort_edit(0, valid_first.into());
                    // An unexpected transport end while a request is in flight resets every session field.
                    link.container_sort_request(intent);
                    assert!(link.container_sort_panel().pending);
                    task.await.checked()?;
                    let ended = link.container_sort_panel();
                    assert!(!ended.pending && !ended.selected && !ended.open);
                    assert!(ended.summary.is_empty());
                    assert!(ended.fields.iter().all(String::is_empty));
                    assert!(ended.feedback.contains("ended"));
                    link.container_sort_request(intent);
                    assert!(!link.container_sort_panel().pending);
                    let (commands, receiver) = tokio::sync::mpsc::unbounded_channel();
                    let reconnected = PlayLink {
                        owns_session: true,
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
                    let fresh = reconnected.container_sort_panel();
                    assert!(!fresh.open && !fresh.pending);
                    assert!(fresh.summary.is_empty() && fresh.fields.iter().all(String::is_empty));
                    for (index, value) in fields.into_iter().enumerate() {
                        reconnected.container_sort_edit(index, value.into());
                    }
                    send(
                        &reconnected,
                        &exchanges,
                        intent,
                        168,
                        reconnect_payload,
                        CommandStatus::Accepted,
                        reconnect_reply,
                    )
                    .await
                    .checked()?;
                    assert!(!reconnected.container_sort_panel().summary.is_empty());
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
