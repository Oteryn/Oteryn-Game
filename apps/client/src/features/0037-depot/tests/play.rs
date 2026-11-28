use crate::play::play_channel;
use oteryn_client_ui::depot::{DepotPanel, Intent, request};
#[test]
fn depot_commands_require_authoritative_town_and_one_pending_request() {
    let mut p = DepotPanel {
        selected: true,
        ..Default::default()
    };
    assert!(request(&mut p, Intent::Withdraw(0)).is_none());
    assert!(
        request(
            &mut p,
            Intent::Browse {
                box_number: 1,
                page: 0
            }
        )
        .is_some()
    );
    assert!(
        request(
            &mut p,
            Intent::Browse {
                box_number: 2,
                page: 0
            }
        )
        .is_none()
    );
}
#[test]
fn depot_visible_panel_receives_authoritative_refusal() {
    let (link, _commands, outbox) = play_channel();
    outbox.depot_initialize(true, false);
    link.open_depot();
    assert!(link.depot_panel().pending);
    outbox.depot_result(None);
    assert!(!link.depot_panel().pending);
    assert!(link.depot_panel().feedback.contains("refused"));
}

type Error = Box<dyn std::error::Error + Send + Sync>;
const DEPOT: u32 = oteryn_protocol_oteryn::depot::CAPABILITY_DEPOT_V1;
const CROSS_TOWN: u32 = oteryn_protocol_oteryn::depot::CAPABILITY_CROSS_TOWN_DEPOT_V1;
// Thais and Carlin use these IDs in the authored town catalogue.
const THAIS: u16 = 8;
const CARLIN: u16 = 6;
const LOCAL: Intent = Intent::Browse {
    box_number: 1,
    page: 0,
};
const LEGACY_OPEN: &[u8] = &[8, 1, 16, 1, 24, 0, 40, 0];
const CROSS_TOWN_OPEN: &[u8] = &[8, 1, 16, 1, 24, 0, 40, 0, 32, 6, 56, 1];

#[test]
fn depot_town_selector_sends_field_7_when_627_selected() -> Result<(), Error> {
    depot_session(
        true,
        &[(Intent::SelectTown(Some(CARLIN)), CROSS_TOWN_OPEN, CARLIN)],
    )
}

#[test]
fn depot_without_town_keeps_legacy_bytes() -> Result<(), Error> {
    depot_session(true, &[(LOCAL, LEGACY_OPEN, THAIS)])?;
    depot_session(
        false,
        &[(Intent::SelectTown(Some(CARLIN)), LEGACY_OPEN, THAIS)],
    )
}

#[test]
fn cross_town_depot_round_trip_through_session() -> Result<(), Error> {
    depot_session(
        true,
        &[
            (LOCAL, LEGACY_OPEN, THAIS),
            (Intent::SelectTown(Some(CARLIN)), CROSS_TOWN_OPEN, CARLIN),
            (
                Intent::Browse {
                    box_number: 2,
                    page: 0,
                },
                &[8, 1, 16, 2, 24, 0, 40, 0, 32, 6, 56, 1],
                CARLIN,
            ),
            (
                Intent::SelectTown(None),
                &[8, 1, 16, 2, 24, 0, 40, 0],
                THAIS,
            ),
        ],
    )
}

fn depot_session(
    cross_town: bool,
    exchanges: &'static [(Intent, &'static [u8], u16)],
) -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(async {
                tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    depot_exchanges(cross_town, exchanges),
                )
                .await?
            })
        })?
        .join()
        .map_err(|_| "depot session thread panicked")?
}

async fn depot_exchanges(
    cross_town: bool,
    exchanges: &[(Intent, &[u8], u16)],
) -> Result<(), Error> {
    use oteryn_protocol_oteryn as wire;
    use oteryn_session::{
        Admission, CLIENT_SUPPORTED_CAPABILITIES, Session, read_frame, write_frame,
    };
    use wire::depot::{COMMAND_TYPE_DEPOT_INTENT, DepotEntry, DepotOutcome, DepotReply};
    let caps = if cross_town {
        &[4, 6, DEPOT, CROSS_TOWN][..]
    } else {
        &[4, 6, DEPOT][..]
    };
    let id = |marker| {
        let mut id = [marker; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id
    };
    let (client, mut peer) = tokio::io::duplex(64 * 1024);
    let admission = Session::admit(
        client,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: wire::CharacterId::decode(&id(4))?,
            admission_material: b"grant",
            client_build_id: "depot-session-test",
            supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
            deadline: std::time::Duration::from_secs(3),
        },
    );
    let join = async {
        let bootstrap = read_frame(&mut peer).await?;
        let bootstrap = wire::decode_wire_envelope(&bootstrap)?;
        let hello = bootstrap.client_bootstrap()?;
        assert!(hello.supported_capabilities.contains(&CROSS_TOWN));
        write_frame(
            &mut peer,
            &wire::encode_server_accepted(&wire::ServerAcceptedValue {
                game_session_id: wire::GameSessionId::decode(&id(1))?,
                world_id: wire::WorldId::decode(&id(2))?,
                channel_id: wire::ChannelId::decode(&id(3))?,
                connection_generation: 1,
                current_server_sequence: 0,
                next_command_id: 1,
                schema_revision: 1,
                selected_capabilities: caps,
            })?,
        )
        .await?;
        use oteryn_session::{
            ActorPosition, EntityDetail, EntityKind, EntityRef, StepDirection, WorldSpatialEntity,
        };
        use wire::world_spatial_entities::{
            SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2, WorldSpatialEntitiesSnapshot,
            encode_world_spatial_entities_snapshot_with_item_handles,
        };
        let position = ActorPosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        let own_identity = id(4);
        let spatial = encode_world_spatial_entities_snapshot_with_item_handles(
            &WorldSpatialEntitiesSnapshot {
                content_generation: [17; 32],
                actor_position: position,
                own_identity,
                entities: vec![WorldSpatialEntity {
                    motion: None,
                    display_name: None,
                    kind: EntityKind::Player,
                    entity: EntityRef {
                        identity: own_identity,
                        generation: 1,
                    },
                    position,
                    detail: EntityDetail::Actor {
                        direction: StepDirection::South,
                        appearance_ref: 1,
                        health_percent: 100,
                    },
                }],
            },
        )
        .map_err(|error| format!("{error:?}"))?;
        let objects = wire::world_object::encode_world_object_overlay_snapshot(&[])
            .map_err(|error| format!("{error:?}"))?;
        let inventory = wire::item_view::encode_character_inventory(&Default::default())
            .map_err(|error| format!("{error:?}"))?;
        let container = wire::item_view::encode_open_container(&Default::default())
            .map_err(|error| format!("{error:?}"))?;
        let domains = [
            (
                1,
                SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                spatial.as_slice(),
            ),
            (2, 1, objects.as_slice()),
            (9, 1, inventory.as_slice()),
            (11, 1, container.as_slice()),
        ]
        .map(|(domain_id, snapshot_type, payload)| wire::DomainSnapshot {
            domain_id,
            revision: 1,
            snapshot_type,
            payload,
        });
        for frame in wire::encode_single_chunk_snapshot(1, 1, 0, &domains)? {
            write_frame(&mut peer, &frame).await?;
        }
        Ok::<(), Error>(())
    };
    let (session, joined) = tokio::join!(admission, join);
    joined?;
    let (link, commands, outbox) = play_channel();
    let task = tokio::spawn(crate::play::run_session(session?, commands, outbox));
    while !link.depot_panel().selected {
        tokio::task::yield_now().await;
    }
    assert_eq!(link.depot_panel().cross_town_selected, cross_town);
    for (index, &(intent, expected, town)) in exchanges.iter().enumerate() {
        link.depot_request(intent);
        let frame = loop {
            let frame = read_frame(&mut peer).await?;
            if wire::decode_wire_envelope(&frame)?.message_type() != wire::MessageType::LivenessAck
            {
                break frame;
            }
        };
        let command = wire::decode_wire_envelope(&frame)?.client_command(1)?;
        assert_eq!(command.command_type, COMMAND_TYPE_DEPOT_INTENT);
        assert_eq!(command.payload, expected);
        let received = wire::depot::decode_depot_request(command.payload, caps)
            .map_err(|error| format!("{error:?}"))?;
        let reply = DepotReply {
            outcome: DepotOutcome::Opened,
            town: Some(town),
            box_number: received.intent.box_number,
            page: received.intent.page,
            total: 1,
            entries: vec![DepotEntry {
                ordinal: 1,
                definition: 3031,
                count: 12,
            }],
        };
        let reply_bytes =
            wire::depot::encode_depot_reply(&reply).map_err(|error| format!("{error:?}"))?;
        let sequence = u64::try_from(index)? + 1;
        write_frame(
            &mut peer,
            &wire::encode_command_result(
                1,
                sequence,
                command.command_id,
                wire::CommandStatus::Accepted,
                &reply_bytes,
            )?,
        )
        .await?;
        while link.depot_panel().pending {
            tokio::task::yield_now().await;
        }
        assert_eq!(link.depot_panel().reply, Some(reply));
    }
    drop(link);
    task.await?;
    Ok(())
}
