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
        capability == 134
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
        assert_eq!(command.capability, 134);
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
        while !link.party_skull_panel().selected || link.party_skull_panel().pending {
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
    link.party_skull_request(intent);
    assert!(link.party_skull_panel().pending);
    let draft = link.party_skull_panel().fields;
    link.party_skull_edit(0, "changed".into());
    assert_eq!(link.party_skull_panel().fields, draft);
    // A second click before a response must not produce another command.
    link.party_skull_request(intent);
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
use oteryn_client_runtime::party_skull as wire;
use oteryn_client_ui::party_skull::Intent;

#[test]
fn pc06_party_skull_scenario() -> Result<(), Error> {
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
                    link.party_skull_request(Intent::Query);
                    assert!(!link.party_skull_panel().pending);
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
                        code: oteryn_input_actions::KeyCode::new(25).checked()?,
                        state: oteryn_input_actions::ButtonState::Pressed,
                        modifiers: oteryn_input_actions::Modifiers::one(
                            oteryn_input_actions::Modifier::Control,
                        )
                        .with(oteryn_input_actions::Modifier::Alt),
                        repeat: false,
                    };
                    assert!(link.party_skull_input(&[event]));
                    assert!(link.party_skull_panel().open);
                    let fields: [&str; 0] = [];
                    for (i, value) in fields.iter().enumerate() {
                        link.party_skull_edit(i, (*value).into());
                    }
                    let intent = Intent::Query;
                    let payload = vec![];
                    let reply = wire::encode_party_icons(&[wire::PartyIcon {
                        entity: oteryn_session::EntityRef {
                            identity: [1; 16],
                            generation: 2,
                        },
                        shield: 4,
                        skull: 3,
                    }])
                    .checked()?;
                    send(
                        &link,
                        &exchanges,
                        intent,
                        155,
                        payload.clone(),
                        CommandStatus::Accepted,
                        reply.clone(),
                    )
                    .await
                    .checked()?;
                    assert!(!link.party_skull_panel().summary.is_empty());

                    send(
                        &link,
                        &exchanges,
                        intent,
                        155,
                        vec![],
                        CommandStatus::Accepted,
                        vec![],
                    )
                    .await
                    .checked()?;
                    assert!(link.party_skull_panel().summary.contains("No visible"));
                    send(
                        &link,
                        &exchanges,
                        intent,
                        155,
                        vec![],
                        CommandStatus::Accepted,
                        reply.clone(),
                    )
                    .await
                    .checked()?;
                    let reconnect_payload = payload.clone();
                    let reconnect_reply = reply.clone();
                    let summary = link.party_skull_panel().summary;
                    assert!(!summary.is_empty());
                    send(
                        &link,
                        &exchanges,
                        intent,
                        155,
                        payload.clone(),
                        CommandStatus::Rejected,
                        vec![],
                    )
                    .await
                    .checked()?;
                    assert!(link.party_skull_panel().feedback.contains("refused"));
                    assert_eq!(link.party_skull_panel().summary, summary);
                    send(
                        &link,
                        &exchanges,
                        intent,
                        155,
                        payload.clone(),
                        CommandStatus::Accepted,
                        vec![255],
                    )
                    .await
                    .checked()?;
                    assert!(link.party_skull_panel().feedback.contains("Invalid"));
                    assert_eq!(link.party_skull_panel().summary, summary);
                    send(
                        &link,
                        &exchanges,
                        intent,
                        155,
                        payload,
                        CommandStatus::DuplicateReplay,
                        reply,
                    )
                    .await
                    .checked()?;
                    assert_eq!(link.party_skull_panel().summary, summary);
                    // An unexpected transport end while a request is in flight resets every session field.
                    link.party_skull_request(intent);
                    assert!(link.party_skull_panel().pending);
                    task.await.checked()?;
                    let ended = link.party_skull_panel();
                    assert!(!ended.pending && !ended.selected && !ended.open);
                    assert!(ended.summary.is_empty());
                    assert!(ended.fields.iter().all(String::is_empty));
                    assert!(ended.feedback.contains("ended"));
                    link.party_skull_request(intent);
                    assert!(!link.party_skull_panel().pending);
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
                    let fresh = reconnected.party_skull_panel();
                    assert!(!fresh.open && !fresh.pending);
                    assert!(fresh.summary.is_empty() && fresh.fields.iter().all(String::is_empty));
                    for (index, value) in fields.into_iter().enumerate() {
                        reconnected.party_skull_edit(index, value.into());
                    }
                    send(
                        &reconnected,
                        &exchanges,
                        intent,
                        155,
                        reconnect_payload,
                        CommandStatus::Accepted,
                        reconnect_reply,
                    )
                    .await
                    .checked()?;
                    assert!(!reconnected.party_skull_panel().summary.is_empty());
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

#[test]
fn pushed_party_icons_reach_the_panel_and_entity_leaves_remove_them() {
    use oteryn_session::{AppliedDelta, EntityRef, WorldSpatialEntitiesDelta};
    let (link, _, outbox) = play_channel();
    crate::play::mailbox::lock(&outbox.0)
        .properties
        .party_skull
        .selected = true;
    let entity = EntityRef {
        identity: [7; 16],
        generation: 3,
    };
    let icon = oteryn_client_runtime::party_skull::PartyIcon {
        entity,
        shield: 3,
        skull: 2,
    };
    outbox.push(vec![SessionEvent::PartyIcons(AppliedDelta {
        server_sequence: 2,
        base_revision: 1,
        new_revision: 2,
        value: vec![icon],
    })]);
    let panel = link.party_skull_panel();
    assert_eq!(panel.icons, vec![icon]);
    assert!(panel.summary.contains("Member"));
    assert!(panel.summary.contains("Green"));
    assert!(!panel.open);
    outbox.push(vec![SessionEvent::WorldSpatialEntities(AppliedDelta {
        server_sequence: 3,
        base_revision: 2,
        new_revision: 3,
        value: WorldSpatialEntitiesDelta {
            content_generation: [1; 32],
            actor_position: oteryn_session::ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
            enter: vec![],
            update: vec![],
            leave: vec![entity],
        },
    })]);
    assert!(link.party_skull_panel().icons.is_empty());
    assert_eq!(link.party_skull_panel().summary, "No visible party icons");
}

#[path = "pvp_status.rs"]
mod pvp_status;
