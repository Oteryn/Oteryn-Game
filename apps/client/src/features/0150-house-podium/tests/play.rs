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
        capability == 150
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
        assert_eq!(command.capability, 150);
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
        while !link.house_podium_panel().selected || link.house_podium_panel().pending {
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
    link.house_podium_request(intent);
    assert!(link.house_podium_panel().pending);
    // A second click before a response must not produce another command.
    link.house_podium_request(intent);
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
fn cp01_house_podium_scenario() -> Result<(), Error> {
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
                    link.house_podium_request(Intent::Refresh);
                    assert!(!link.house_podium_panel().pending);
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
                        code: oteryn_input_actions::KeyCode::new(19).checked()?,
                        state: oteryn_input_actions::ButtonState::Pressed,
                        modifiers: oteryn_input_actions::Modifiers::one(
                            oteryn_input_actions::Modifier::Control,
                        )
                        .with(oteryn_input_actions::Modifier::Alt),
                        repeat: false,
                    };
                    assert!(link.house_podium_input(&[event]));
                    assert!(link.house_podium_panel().open);

                    link.house_podium_edit(0, ID.into());
                    link.house_podium_request(Intent::Set);
                    assert!(!link.house_podium_panel().pending);
                    send(
                        &link,
                        &exchanges,
                        Intent::Refresh,
                        174,
                        ITEM.to_vec(),
                        CommandStatus::Accepted,
                        vec![0],
                    )
                    .await
                    .checked()?;
                    assert_eq!(link.house_podium_panel().revision, Some(0));
                    for (i, value) in [(1, "1"), (2, "2"), (3, "citizen")] {
                        link.house_podium_edit(i, value.into());
                    }
                    let set = wire::SetRequest {
                        item: ITEM,
                        expected_revision: 0,
                        display_kind: 1,
                        direction: 2,
                        key: "citizen".into(),
                    }
                    .encode()
                    .ok_or("set encode")
                    .checked()?;
                    send(
                        &link,
                        &exchanges,
                        Intent::Set,
                        175,
                        set,
                        CommandStatus::Accepted,
                        wire::SetResult::Accepted { revision: 7 }.encode(),
                    )
                    .await
                    .checked()?;
                    assert_eq!(link.house_podium_panel().revision, Some(7));
                    let projection = wire::Projection {
                        podium_kind: 1,
                        display_kind: 1,
                        direction: 2,
                        revision: 7,
                        key: "citizen".into(),
                    };
                    send(
                        &link,
                        &exchanges,
                        Intent::Projection,
                        176,
                        ITEM.to_vec(),
                        CommandStatus::DuplicateReplay,
                        wire::Projection::encode(Some(&projection))
                            .ok_or("projection encode")
                            .checked()?,
                    )
                    .await
                    .checked()?;
                    assert!(link.house_podium_panel().projection.contains("citizen"));
                    assert!(link.house_podium_panel().projection.contains("facing 2"));
                    link.house_podium_edit(0, "00000000-0000-7000-8000-000000000002".into());
                    link.house_podium_request(Intent::Set);
                    assert!(!link.house_podium_panel().pending);
                    assert!(link.house_podium_panel().feedback.contains("Refresh"));
                    link.house_podium_edit(0, ID.into());
                    send(
                        &link,
                        &exchanges,
                        Intent::Refresh,
                        174,
                        ITEM.to_vec(),
                        CommandStatus::Accepted,
                        wire::Projection::encode(Some(&projection))
                            .ok_or("projection encode")
                            .checked()?,
                    )
                    .await
                    .checked()?;
                    let set = wire::SetRequest {
                        item: ITEM,
                        expected_revision: 7,
                        display_kind: 1,
                        direction: 2,
                        key: "citizen".into(),
                    }
                    .encode()
                    .ok_or("set encode")
                    .checked()?;
                    send(
                        &link,
                        &exchanges,
                        Intent::Set,
                        175,
                        set,
                        CommandStatus::Accepted,
                        wire::SetResult::Refused(wire::SetRefusal::Changed).encode(),
                    )
                    .await
                    .checked()?;
                    assert_eq!(link.house_podium_panel().revision, None);
                    send(
                        &link,
                        &exchanges,
                        Intent::Refresh,
                        174,
                        ITEM.to_vec(),
                        CommandStatus::Accepted,
                        vec![1],
                    )
                    .await
                    .checked()?;
                    assert!(link.house_podium_panel().feedback.contains("Invalid"));
                    send(
                        &link,
                        &exchanges,
                        Intent::Projection,
                        176,
                        ITEM.to_vec(),
                        CommandStatus::Rejected,
                        vec![0],
                    )
                    .await
                    .checked()?;
                    assert!(link.house_podium_panel().feedback.contains("refused"));

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

use oteryn_client_ui::house_podium::Intent;
use oteryn_session::property_services::house_podium as wire;
const ID: &str = "00000000-0000-7000-8000-000000000001";
const ITEM: [u8; 16] = [0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1];

trait Checked<T> {
    fn checked(self) -> Result<T, Error>;
}
impl<T, E: std::fmt::Debug> Checked<T> for Result<T, E> {
    fn checked(self) -> Result<T, Error> {
        self.map_err(|error| format!("{error:?}").into())
    }
}

#[test]
fn podium_display_preview_controls_use_negotiated_wire_and_actual_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::feature_0150_house_podium::{input::display_action, view::preview::layers};
    use oteryn_client_runtime::house_podium_display as display;
    let (link, mut receiver, outbox) = play_channel();
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        let panel = &mut mailbox.properties.house_podium;
        panel.selected = true;
        panel.display_selected = true;
        panel.fields = [
            "01010101-0101-7101-8101-010101010101".into(),
            "3".into(),
            "2".into(),
            "oteryn:creature.rat".into(),
        ];
        panel.loaded_item = panel.fields[0].clone();
        panel.revision = Some(3);
        display_action(panel, 4);
        assert!(!panel.visibility.display);
    }
    link.house_podium_request(Intent::Preview);
    let crate::combat_input::PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("missing preview command".into());
    };
    assert_eq!((command.capability, command.kind), (449, 733));
    let Some(display::Request::Preview { value, visibility }) =
        display::Request::decode(&command.payload)
    else {
        return Err("preview request".into());
    };
    assert_eq!(
        (value.display_kind, value.direction, value.expected_revision),
        (3, 2, 3)
    );
    assert!(!visibility.display && visibility.platform);
    link.house_podium_request(Intent::Preview);
    assert!(receiver.try_recv().is_err());
    let state = display::State {
        committed: false,
        podium_kind: 3,
        item_definition_key: "oteryn:item.tibia.i42367".into(),
        item_definition_revision: "definition-r1".into(),
        platform_appearance: 42367,
        appearance_kind: 1,
        appearance: 21,
        visibility,
        projection: None,
    };
    let state = display::State {
        projection: Some(
            oteryn_session::property_services::house_podium::Projection {
                podium_kind: 3,
                display_kind: 3,
                direction: 2,
                revision: 3,
                key: "oteryn:creature.rat".into(),
            },
        ),
        ..state
    };
    let payload = display::Reply::State(state.clone())
        .encode()
        .ok_or("reply")?;
    outbox.properties_result(
        &command,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload,
        },
    );
    let panel = link.house_podium_panel();
    assert_eq!(panel.preview, Some(state.clone()));
    assert_eq!(
        layers(&state)?
            .iter()
            .map(|l| (l.kind, l.appearance))
            .collect::<Vec<_>>(),
        vec![(2, 42367)]
    );
    assert_eq!(panel.revision, Some(3));
    let mut state = state;
    state.visibility.display = true;
    state.visibility.platform = false;
    assert_eq!(
        layers(&state)?
            .iter()
            .map(|l| l.appearance)
            .collect::<Vec<_>>(),
        vec![21]
    );
    state.appearance = 0;
    assert!(
        layers(&state).is_err(),
        "missing metadata must not fabricate a placeholder skin"
    );
    link.house_podium_request(Intent::Set);
    let crate::combat_input::PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("missing save command".into());
    };
    assert!(
        matches!(display::Request::decode(&command.payload), Some(display::Request::Set { visibility, .. }) if !visibility.display)
    );
    Ok(())
}

struct DisplayPeer(Peer);
impl Stepper for DisplayPeer {
    fn feature_selected(&self, capability: u32) -> bool {
        matches!(capability, 150 | 449)
    }
    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        assert_eq!(command.capability, 449);
        let (kind, payload, status, reply) = self
            .0
            .exchanges
            .entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front()
            .ok_or(SessionError::SessionUnusable)?;
        assert_eq!(command.kind, kind);
        assert_eq!(command.payload, payload);
        self.0
            .exchanges
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
        self.0.serve(duration).await
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        self.0.drain()
    }
    async fn step(&mut self, direction: StepDirection) -> Result<StepOutcome, SessionError> {
        self.0.step(direction).await
    }
    async fn combat(&mut self, command: CombatCommand) -> Result<CombatReport, SessionError> {
        self.0.combat(command).await
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        self.0.logout().await
    }
}
#[test]
fn podium_display_live_selection_results_and_scope_cleanup_use_session_loop() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            let runtime = oteryn_client_runtime::ClientRuntime::new().checked()?;
            runtime
                .block_on(async {
                    use oteryn_client_runtime::house_podium_display as display;
                    let state = display::State {
                        committed: true,
                        podium_kind: 3,
                        item_definition_key: "oteryn:item.tibia.i42367".into(),
                        item_definition_revision: "definition-r1".into(),
                        platform_appearance: 0,
                        appearance_kind: 0,
                        appearance: 0,
                        visibility: display::Visibility::default(),
                        projection: None,
                    };
                    let script = Arc::new(Script {
                        entries: Mutex::new(VecDeque::from([(
                            733,
                            display::Request::QueryHandle(7).encode().ok_or("query")?,
                            CommandStatus::Accepted,
                            display::Reply::State(state.clone())
                                .encode()
                                .ok_or("reply")?,
                        )])),
                        release: tokio::sync::Semaphore::new(0),
                    });
                    let (link, incoming, outbox) = play_channel();
                    let task = tokio::spawn(run_session(
                        DisplayPeer(Peer {
                            exchanges: Arc::clone(&script),
                        }),
                        incoming,
                        outbox,
                    ));
                    ready(&link).await?;
                    assert!(
                        link.house_podium_panel().display_selected,
                        "449 must be selected during actual admission initialize"
                    );
                    link.house_podium_select(7);
                    assert!(link.house_podium_panel().pending);
                    script.release.add_permits(1);
                    ready(&link).await?;
                    assert_eq!(link.house_podium_panel().preview, Some(state));
                    assert_eq!(link.house_podium_panel().loaded_item, "7");
                    {
                        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
                        mailbox.cancel_activity_requests();
                        let panel = &mailbox.properties.house_podium;
                        assert!(panel.selected && panel.display_selected);
                        assert!(!panel.open && !panel.preview_open && !panel.pending);
                        assert!(panel.preview.is_none() && panel.revision.is_none());
                        assert!(panel.fields.iter().all(String::is_empty));
                    }
                    drop(link);
                    task.await.checked()?;
                    Ok::<(), Error>(())
                })
                .checked()?
        })
        .checked()?
        .join()
        .map_err(|_| "podium display session scenario thread failed")
        .checked()?
}

#[test]
fn podium_scope_receipts_clear_images_targets_and_apply_destination_selection()
-> Result<(), Box<dyn std::error::Error>> {
    let id = |byte| {
        let mut id = [byte; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id
    };
    let (link, mut receiver, outbox) = play_channel();
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        let panel = &mut mailbox.properties.house_podium;
        panel.selected = true;
        panel.display_selected = true;
        panel.pending = true;
        panel.open = true;
        panel.preview_open = true;
        panel.revision = Some(9);
        panel.fields[0] = "7".into();
        panel.loaded_item = "7".into();
    }
    let receipt = oteryn_session::activity_scope::ActivityScopeSnapshot {
        handoff: id(21),
        source_session: id(22),
        session: id(23),
        scope: oteryn_session::activity_scope::ActivityScope::Channel {
            world: id(24),
            channel: id(25),
        },
        scope_generation: 2,
        connection_generation: 1,
        lease_generation: 1,
        selection: vec![49, 117, 150],
        requires_ack: false,
    };
    let payload = oteryn_session::activity_scope::encode_activity_scope(&receipt)
        .map_err(|e| format!("{e:?}"))?;
    outbox.push(vec![SessionEvent::GatedSnapshotReplaced(
        oteryn_session::AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::GatedSnapshot {
                domain_id: 99,
                snapshot_type: 1,
                revision: 1,
                payload,
            },
        },
    )]);
    let panel = link.house_podium_panel();
    assert!(panel.selected && !panel.display_selected);
    assert!(!panel.pending && !panel.open && !panel.preview_open);
    assert!(panel.revision.is_none() && panel.preview.is_none() && panel.loaded_item.is_empty());
    assert!(panel.fields.iter().all(String::is_empty));
    link.house_podium_request(Intent::Preview);
    assert!(receiver.try_recv().is_err());
    use oteryn_client_runtime::house_transfer as transfer;
    let request = transfer::HouseTransferIntent {
        house: "oteryn:content.house.test".into(),
        exit: true,
    };
    let command = FeatureCommand {
        capability: 86,
        kind: transfer::COMMAND_TYPE_HOUSE_TRANSFER,
        payload: transfer::encode_house_transfer_intent(&request).map_err(|e| format!("{e:?}"))?,
    };
    let ack = transfer::HouseTransferAck {
        transfer: id(21),
        session: id(23),
        world: id(24),
        channel: id(25),
        house: None,
        scope_generation: 3,
        connection_generation: 1,
        lease_generation: 1,
        selection: vec![49, 86, 117],
        position: [100, 200, 7],
    };
    outbox.properties_result(
        &command,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 2,
            status: CommandStatus::Accepted,
            payload: transfer::encode_house_transfer_ack(&ack).map_err(|e| format!("{e:?}"))?,
        },
    );
    let panel = link.house_podium_panel();
    assert!(!panel.selected && !panel.display_selected);
    Ok(())
}
