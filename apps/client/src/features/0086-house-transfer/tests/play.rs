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
        capability == 86
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
        assert_eq!(command.capability, 86);
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
        while !link.house_transfer_panel().selected || link.house_transfer_panel().pending {
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
    link.house_transfer_request(intent);
    assert!(link.house_transfer_panel().pending);
    let draft = link.house_transfer_panel().fields;
    link.house_transfer_edit(0, "changed".into());
    assert_eq!(link.house_transfer_panel().fields, draft);
    // A second click before a response must not produce another command.
    link.house_transfer_request(intent);
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
use oteryn_client_runtime::house_transfer as wire;
use oteryn_client_ui::house_transfer::Intent;
fn id() -> [u8; 16] {
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = 1;
    id
}
#[test]
fn pc04_house_transfer_scenario() -> Result<(), Error> {
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
                    link.house_transfer_request(Intent::Enter);
                    assert!(!link.house_transfer_panel().pending);
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
                    assert!(link.house_transfer_input(&[event]));
                    assert!(link.house_transfer_panel().open);
                    let fields = ["oteryn:content.house.test"];
                    for (index, value) in fields.iter().enumerate() {
                        link.house_transfer_edit(index, (*value).into());
                    }

                    let ack = wire::HouseTransferAck {
                        transfer: id(),
                        session: id(),
                        world: id(),
                        channel: id(),
                        house: Some(fields[0].into()),
                        scope_generation: 2,
                        connection_generation: 1,
                        lease_generation: 1,
                        selection: vec![86],
                        position: [100, 200, 7],
                    };
                    let reply = wire::encode_house_transfer_ack(&ack).checked()?;
                    assert_eq!(wire::decode_house_transfer_ack(&reply).checked()?, ack);
                    for (intent, exit) in [(Intent::Enter, false), (Intent::Exit, true)] {
                        let request = wire::HouseTransferIntent {
                            house: fields[0].into(),
                            exit,
                        };
                        let payload = wire::encode_house_transfer_intent(&request).checked()?;
                        assert_eq!(
                            wire::decode_house_transfer_intent(&payload).checked()?,
                            request
                        );
                        let mut response = ack.clone();
                        if exit {
                            response.house = None;
                        }
                        send(
                            &link,
                            &exchanges,
                            intent,
                            102,
                            payload,
                            CommandStatus::Accepted,
                            wire::encode_house_transfer_ack(&response).checked()?,
                        )
                        .await
                        .checked()?;
                        assert!(!link.house_transfer_panel().summary.is_empty());
                    }
                    let intent = Intent::Enter;
                    let kind = 102;
                    let payload = wire::encode_house_transfer_intent(&wire::HouseTransferIntent {
                        house: fields[0].into(),
                        exit: false,
                    })
                    .checked()?;
                    send(
                        &link,
                        &exchanges,
                        intent,
                        kind,
                        payload.clone(),
                        CommandStatus::Accepted,
                        reply.clone(),
                    )
                    .await
                    .checked()?;
                    let summary = link.house_transfer_panel().summary;
                    for (status, response, message) in [
                        (CommandStatus::Rejected, vec![], "refused"),
                        (CommandStatus::Accepted, vec![255], "Invalid"),
                        (CommandStatus::DuplicateReplay, reply.clone(), ""),
                    ] {
                        send(
                            &link,
                            &exchanges,
                            intent,
                            kind,
                            payload.clone(),
                            status,
                            response,
                        )
                        .await
                        .checked()?;
                        assert!(link.house_transfer_panel().feedback.contains(message));
                        assert_eq!(link.house_transfer_panel().summary, summary);
                    }
                    link.house_transfer_edit(0, "invalid".into());
                    link.house_transfer_request(intent);
                    assert!(!link.house_transfer_panel().pending);
                    link.house_transfer_edit(0, fields[0].into());
                    link.house_transfer_request(intent);
                    assert!(link.house_transfer_panel().pending);
                    exchanges.release.add_permits(1);
                    task.await.checked()?;
                    let ended = link.house_transfer_panel();
                    assert!(!ended.pending && !ended.selected && !ended.open);
                    assert!(ended.summary.is_empty() && ended.fields.iter().all(String::is_empty));
                    assert!(ended.feedback.contains("ended"));
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
                    let fresh = reconnected.house_transfer_panel();
                    assert!(!fresh.pending && !fresh.open && fresh.summary.is_empty());
                    for (index, value) in fields.into_iter().enumerate() {
                        reconnected.house_transfer_edit(index, value.into());
                    }
                    send(
                        &reconnected,
                        &exchanges,
                        intent,
                        kind,
                        payload,
                        CommandStatus::Accepted,
                        reply,
                    )
                    .await
                    .checked()?;
                    assert!(!reconnected.house_transfer_panel().summary.is_empty());
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
