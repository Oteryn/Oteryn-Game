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
        capability == 39
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
        assert_eq!(command.capability, 39);
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
        while !link.timed_item_panel().selected || link.timed_item_panel().pending {
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
    link.timed_item_request(intent);
    assert!(link.timed_item_panel().pending);
    let draft = link.timed_item_panel().fields;
    link.timed_item_edit(0, "changed".into());
    assert_eq!(link.timed_item_panel().fields, draft);
    // A second click before a response must not produce another command.
    link.timed_item_request(intent);
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
use oteryn_client_runtime::timed_item as wire;
use oteryn_client_ui::timed_item::Intent;

#[test]
fn pc06_timed_item_scenario() -> Result<(), Error> {
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
                    link.timed_item_request(Intent::Inspect);
                    assert!(!link.timed_item_panel().pending);
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
                        code: oteryn_input_actions::KeyCode::new(18).checked()?,
                        state: oteryn_input_actions::ButtonState::Pressed,
                        modifiers: oteryn_input_actions::Modifiers::one(
                            oteryn_input_actions::Modifier::Control,
                        )
                        .with(oteryn_input_actions::Modifier::Alt),
                        repeat: false,
                    };
                    assert!(link.timed_item_input(&[event]));
                    assert!(link.timed_item_panel().open);
                    let fields: [&str; 4] = ["7", "0", "", ""];
                    for (i, value) in fields.iter().enumerate() {
                        link.timed_item_edit(i, (*value).into());
                    }
                    let intent = Intent::Inspect;
                    let payload = wire::encode_timed_item_intent(wire::TimedItemIntent {
                        action: wire::TimedAction::Inspect,
                        target: oteryn_session::ItemHandle::new(7).ok_or("handle")?,
                        expected_revision: 0,
                        npc: None,
                    })
                    .checked()?;
                    let reply = wire::encode_timed_item_result(wire::TimedItemResult {
                        disposition: wire::TimedDisposition::Ok,
                        revision: 2,
                        charges: Some(8),
                        remaining_ms: Some(1000),
                        retired: false,
                        fee_gold: 0,
                    })
                    .checked()?;
                    send(
                        &link,
                        &exchanges,
                        intent,
                        38,
                        payload.clone(),
                        CommandStatus::Accepted,
                        reply.clone(),
                    )
                    .await
                    .checked()?;
                    assert!(!link.timed_item_panel().summary.is_empty());

                    for action in [Intent::Use, Intent::Repair] {
                        link.timed_item_edit(2, "11111111-1111-1111-1111-111111111111".into());
                        link.timed_item_edit(3, "3".into());
                        let request = wire::TimedItemIntent {
                            action: if action == Intent::Use {
                                wire::TimedAction::Use
                            } else {
                                wire::TimedAction::Repair
                            },
                            target: oteryn_session::ItemHandle::new(7).ok_or("handle")?,
                            expected_revision: 0,
                            npc: (action == Intent::Repair).then_some(oteryn_session::EntityRef {
                                identity: [0x11; 16],
                                generation: 3,
                            }),
                        };
                        let bytes = wire::encode_timed_item_intent(request).checked()?;
                        send(
                            &link,
                            &exchanges,
                            action,
                            38,
                            bytes,
                            CommandStatus::Accepted,
                            reply.clone(),
                        )
                        .await
                        .checked()?;
                    }
                    let confirmed = link.timed_item_panel().summary;
                    for disposition in [
                        wire::TimedDisposition::Rejected,
                        wire::TimedDisposition::Stale,
                        wire::TimedDisposition::InsufficientFunds,
                    ] {
                        let refused = wire::encode_timed_item_result(
                            wire::TimedItemResult::refused(disposition),
                        )
                        .checked()?;
                        send(
                            &link,
                            &exchanges,
                            intent,
                            38,
                            payload.clone(),
                            CommandStatus::Rejected,
                            refused,
                        )
                        .await
                        .checked()?;
                        assert_eq!(link.timed_item_panel().summary, confirmed);
                        assert!(!link.timed_item_panel().feedback.is_empty());
                    }
                    link.timed_item_edit(0, "0".into());
                    link.timed_item_request(intent);
                    assert!(!link.timed_item_panel().pending);
                    link.timed_item_edit(0, "7".into());
                    let reconnect_payload = payload.clone();
                    let reconnect_reply = reply.clone();
                    let summary = link.timed_item_panel().summary;
                    assert!(!summary.is_empty());
                    send(
                        &link,
                        &exchanges,
                        intent,
                        38,
                        payload.clone(),
                        CommandStatus::Rejected,
                        vec![],
                    )
                    .await
                    .checked()?;
                    assert!(link.timed_item_panel().feedback.contains("refused"));
                    assert_eq!(link.timed_item_panel().summary, summary);
                    send(
                        &link,
                        &exchanges,
                        intent,
                        38,
                        payload.clone(),
                        CommandStatus::Accepted,
                        vec![255],
                    )
                    .await
                    .checked()?;
                    assert!(link.timed_item_panel().feedback.contains("Invalid"));
                    assert_eq!(link.timed_item_panel().summary, summary);
                    send(
                        &link,
                        &exchanges,
                        intent,
                        38,
                        payload,
                        CommandStatus::DuplicateReplay,
                        reply,
                    )
                    .await
                    .checked()?;
                    assert_eq!(link.timed_item_panel().summary, summary);
                    // An unexpected transport end while a request is in flight resets every session field.
                    link.timed_item_request(intent);
                    assert!(link.timed_item_panel().pending);
                    exchanges.release.add_permits(1);
                    task.await.checked()?;
                    let ended = link.timed_item_panel();
                    assert!(!ended.pending && !ended.selected && !ended.open);
                    assert!(ended.summary.is_empty());
                    assert!(ended.fields.iter().all(String::is_empty));
                    assert!(ended.feedback.contains("ended"));
                    link.timed_item_request(intent);
                    assert!(!link.timed_item_panel().pending);
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
                    let fresh = reconnected.timed_item_panel();
                    assert!(!fresh.open && !fresh.pending);
                    assert!(fresh.summary.is_empty() && fresh.fields.iter().all(String::is_empty));
                    for (index, value) in fields.into_iter().enumerate() {
                        reconnected.timed_item_edit(index, value.into());
                    }
                    send(
                        &reconnected,
                        &exchanges,
                        intent,
                        38,
                        reconnect_payload,
                        CommandStatus::Accepted,
                        reconnect_reply,
                    )
                    .await
                    .checked()?;
                    assert!(!reconnected.timed_item_panel().summary.is_empty());
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
