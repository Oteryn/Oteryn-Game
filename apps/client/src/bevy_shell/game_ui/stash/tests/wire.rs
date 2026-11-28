use super::{App, HostOwner, TestResult};
use crate::play::{PlayLink, play_channel, run_session};
use oteryn_session::test_support::wire::{
    self, DomainSnapshot, container_tree, item_view, stash, world_spatial_entities as spatial,
};
use oteryn_session::{Admission, CommandStatus, Session, read_frame, write_frame};
use std::time::{Duration, Instant};
use tokio::{
    io::DuplexStream,
    runtime::{Builder, Runtime},
};

type Error = Box<dyn std::error::Error>;
const DEADLINE: Duration = Duration::from_secs(5);

pub(in crate::bevy_shell::game_ui) fn invalid(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

fn id(byte: u8) -> [u8; 16] {
    let mut id = [byte; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}

pub(in crate::bevy_shell::game_ui) struct Peer {
    runtime: Runtime,
    stream: DuplexStream,
    command: u64,
    sequence: u64,
}

impl Peer {
    pub(in crate::bevy_shell::game_ui) fn new(
        extra: Option<u32>,
    ) -> Result<(Self, PlayLink), Error> {
        let runtime = Builder::new_multi_thread()
            .worker_threads(1)
            .thread_stack_size(64 << 20)
            .enable_all()
            .build()?;
        let (client, mut stream) = tokio::io::duplex(64 * 1024);
        let (link, commands, outbox) = play_channel();
        let mut caps = vec![4, 6, 12, 14, 37, 62, 98];
        caps.extend(extra);
        runtime.spawn(async move {
            let session = Session::admit(
                client,
                Admission {
                    appearance_items: None,
                    schema_revision: 1,
                    character_id: wire::CharacterId::decode(&id(4)).map_err(invalid)?,
                    admission_material: b"grant",
                    client_build_id: "stash-batch-test",
                    supported_capabilities: oteryn_session::CLIENT_SUPPORTED_CAPABILITIES,
                    deadline: DEADLINE,
                },
            )
            .await
            .map_err(invalid)?;
            run_session(session, commands, outbox).await;
            Ok::<(), String>(())
        });
        runtime.block_on(async {
            let bootstrap = tokio::time::timeout(DEADLINE, read_frame(&mut stream)).await??;
            let envelope = wire::decode_wire_envelope(&bootstrap)?;
            let advertised = envelope.client_bootstrap()?.supported_capabilities;
            assert!(
                caps.iter().all(|cap| advertised.contains(cap)),
                "missing advertised capability: {extra:?}"
            );
            write_frame(
                &mut stream,
                &wire::encode_server_accepted(&wire::ServerAcceptedValue {
                    game_session_id: wire::GameSessionId::decode(&id(1))?,
                    world_id: wire::WorldId::decode(&id(2))?,
                    channel_id: wire::ChannelId::decode(&id(3))?,
                    connection_generation: 1,
                    current_server_sequence: 0,
                    next_command_id: 7,
                    schema_revision: 1,
                    selected_capabilities: &caps,
                })?,
            )
            .await?;
            let identity = id(4);
            let position = wire::world_spatial::ActorPosition {
                x: 100,
                y: 100,
                floor: 7,
            };
            let spatial = spatial::encode_world_spatial_entities_snapshot_with_item_handles(
                &spatial::WorldSpatialEntitiesSnapshot {
                    content_generation: [17; 32],
                    actor_position: position,
                    own_identity: identity,
                    entities: vec![spatial::WorldSpatialEntity {
                        motion: None,
                        display_name: None,
                        entity: spatial::EntityRef {
                            identity,
                            generation: 1,
                        },
                        kind: spatial::EntityKind::Player,
                        position,
                        detail: spatial::EntityDetail::Actor {
                            direction: wire::world_spatial::StepDirection::South,
                            appearance_ref: 1,
                            health_percent: 100,
                        },
                    }],
                },
            )
            .map_err(invalid)?;
            let backpack = item_view::ItemEntry {
                handle: 10.try_into()?,
                item_definition_ref: 10.try_into()?,
                appearance_id: 0,
                count: 1.try_into()?,
                sub_type: 0,
            };
            let stack = item_view::ItemEntry {
                handle: 11.try_into()?,
                item_definition_ref: 11.try_into()?,
                count: 4.try_into()?,
                ..backpack
            };
            let inventory = item_view::encode_character_inventory_with_equip_drop(
                &item_view::CharacterInventory {
                    main_backpack: Some(backpack),
                    entries: vec![stack],
                    equipment: vec![],
                },
                true,
            )
            .map_err(invalid)?;
            let open = item_view::encode_open_container(&Default::default()).map_err(invalid)?;
            let overlay =
                wire::world_object::encode_world_object_overlay_snapshot(&[]).map_err(invalid)?;
            let views = encode_views(vec![stack])?;
            let domains = [
                (1, 2, &spatial),
                (2, 1, &overlay),
                (9, 1, &inventory),
                (11, 1, &open),
                (14, 1, &views),
            ]
            .map(|(domain_id, snapshot_type, payload)| DomainSnapshot {
                domain_id,
                snapshot_type,
                revision: 1,
                payload,
            });
            for frame in wire::encode_single_chunk_snapshot(1, 1, 0, &domains)? {
                write_frame(&mut stream, &frame).await?;
            }
            Ok::<(), Error>(())
        })?;
        Ok((
            Self {
                runtime,
                stream,
                command: 6,
                sequence: 0,
            },
            link,
        ))
    }

    pub(in crate::bevy_shell::game_ui) fn command(&mut self, kind: u32) -> Result<Vec<u8>, Error> {
        let frame = self.runtime.block_on(async {
            tokio::time::timeout(DEADLINE, read_frame(&mut self.stream)).await
        })??;
        let envelope = wire::decode_wire_envelope(&frame)?;
        let command = envelope.client_command(1)?;
        assert_eq!(command.command_type, kind);
        self.command += 1;
        assert_eq!(command.command_id, self.command);
        Ok(command.payload.to_vec())
    }

    pub(in crate::bevy_shell::game_ui) fn result(
        &mut self,
        status: CommandStatus,
        payload: &[u8],
    ) -> Result<u64, Error> {
        self.sequence += 1;
        let frame = wire::encode_command_result(1, self.sequence, self.command, status, payload)?;
        self.runtime
            .block_on(write_frame(&mut self.stream, &frame))?;
        Ok(self.command)
    }

    pub(super) fn stash(&mut self, reply: &stash::StashReply) -> Result<(), Error> {
        self.result(
            CommandStatus::Accepted,
            &stash::encode_stash_reply(reply).map_err(invalid)?,
        )?;
        Ok(())
    }

    pub(in crate::bevy_shell::game_ui) fn clear_container(&mut self) -> Result<(), Error> {
        self.sequence += 1;
        let payload = encode_views(vec![])?;
        let frame = wire::encode_state_delta(1, self.sequence, 14, 1, 2, 1, &payload)?;
        self.runtime
            .block_on(write_frame(&mut self.stream, &frame))?;
        Ok(())
    }
}

fn encode_views(entries: Vec<item_view::ItemEntry>) -> Result<Vec<u8>, Error> {
    container_tree::encode_container_views(&container_tree::ContainerViews {
        views: vec![container_tree::ContainerView {
            view_id: 0,
            container_handle: 10.try_into()?,
            parent_handle: None,
            capacity: 20,
            entries,
            browse: None,
        }],
    })
    .map_err(|error| invalid(error).into())
}

pub(in crate::bevy_shell::game_ui) fn wait(
    app: &mut App,
    ready: impl Fn(&PlayLink) -> bool,
) -> TestResult {
    let deadline = Instant::now() + DEADLINE;
    while Instant::now() < deadline {
        let owner = app.world().non_send::<HostOwner>();
        if owner.play.as_ref().is_some_and(|play| ready(&play.link)) {
            app.update();
            return Ok(());
        }
        std::thread::yield_now();
    }
    Err("session projection did not arrive".into())
}

pub(in crate::bevy_shell::game_ui) fn wait_text(app: &mut App, expected: &str) -> TestResult {
    let deadline = Instant::now() + DEADLINE;
    while Instant::now() < deadline {
        app.update();
        if app
            .world_mut()
            .query::<&bevy::prelude::Text>()
            .iter(app.world())
            .any(|text| text.0.contains(expected))
        {
            return Ok(());
        }
        std::thread::yield_now();
    }
    Err(format!("session projection missing text: {expected}").into())
}
