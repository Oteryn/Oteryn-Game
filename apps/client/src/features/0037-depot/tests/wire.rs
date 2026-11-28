//! Shared wire peer for play-loop and native Depot control tests.
use oteryn_protocol_oteryn as wire;
use oteryn_session::{Session, read_frame, write_frame};
use tokio::io::DuplexStream;
use wire::depot::{
    CAPABILITY_CROSS_TOWN_DEPOT_V1, CAPABILITY_DEPOT_V1, COMMAND_TYPE_DEPOT_INTENT, DepotEntry,
    DepotOutcome, DepotReply,
};

pub(crate) type Error = Box<dyn std::error::Error + Send + Sync>;

pub(crate) async fn admit(
    cross_town: bool,
) -> Result<(Session<DuplexStream>, DuplexStream), Error> {
    use oteryn_session::{Admission, CLIENT_SUPPORTED_CAPABILITIES};
    let caps = if cross_town {
        &[4, 6, CAPABILITY_DEPOT_V1, CAPABILITY_CROSS_TOWN_DEPOT_V1][..]
    } else {
        &[4, 6, CAPABILITY_DEPOT_V1][..]
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
        assert!(
            hello
                .supported_capabilities
                .contains(&CAPABILITY_CROSS_TOWN_DEPOT_V1)
        );
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
    Ok((session?, peer))
}

pub(crate) async fn reply(
    peer: &mut DuplexStream,
    selected: &[u32],
    sequence: u64,
    expected: &[u8],
    town: u16,
) -> Result<DepotReply, Error> {
    let frame = loop {
        let frame = read_frame(peer).await?;
        if wire::decode_wire_envelope(&frame)?.message_type() != wire::MessageType::LivenessAck {
            break frame;
        }
    };
    let command = wire::decode_wire_envelope(&frame)?.client_command(1)?;
    assert_eq!(command.command_type, COMMAND_TYPE_DEPOT_INTENT);
    assert_eq!(command.payload, expected);
    let received = wire::depot::decode_depot_request(command.payload, selected)
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
    write_frame(
        peer,
        &wire::encode_command_result(
            1,
            sequence,
            command.command_id,
            wire::CommandStatus::Accepted,
            &reply_bytes,
        )?,
    )
    .await?;
    Ok(reply)
}
