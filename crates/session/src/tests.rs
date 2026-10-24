mod appearance_session_tests;
#[path = "tests/depot_map_tests.rs"]
mod depot_map_tests;
include!("tutorial_notices_tests.rs");
use super::*;
use oteryn_protocol_oteryn::world_object::{
    SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1, STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
    WorldObjectOverlayEntry as WireOverlayEntry, encode_world_object_overlay_snapshot,
};
use oteryn_protocol_oteryn::world_spatial::{
    ActorPosition, DELTA_TYPE_WORLD_SPATIAL_V1, SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
    STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, WorldSpatialObservation as WireSpatialObservation,
    encode_step_result, encode_world_spatial,
};
use oteryn_protocol_oteryn::world_spatial_entities::{
    DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2, SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
    WorldSpatialEntitiesSnapshot, encode_world_spatial_entities_delta,
    encode_world_spatial_entities_snapshot,
};
use oteryn_protocol_oteryn::{
    ChannelId, DomainSnapshot, ServerAcceptedValue, WorldId, encode_command_result,
    encode_server_accepted, encode_single_chunk_snapshot, encode_state_delta,
};
use tokio::io::DuplexStream;

pub(super) type BoxError = Box<dyn StdError + Send + Sync>;

const CONTENT_GENERATION: [u8; 32] = [0x11; 32];
const DEADLINE: Duration = Duration::from_secs(5);

fn uuid_v7(marker: u8) -> [u8; 16] {
    [
        0x01, 0x93, 0x4f, 0x10, 0x7c, 0x00, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        marker,
    ]
}

pub(super) fn spatial(x: i32) -> Vec<u8> {
    encode_world_spatial(&WireSpatialObservation {
        content_generation: CONTENT_GENERATION,
        actor_position: ActorPosition { x, y: 0, floor: 0 },
    })
}

/// A scripted peer over an in-memory duplex: no TLS, no TCP, no socket. Reads the
/// `ClientBootstrap`, admits, sends the join snapshot, then answers one `step` with `Moved`
/// and its one `WORLD_SPATIAL` delta.
async fn scripted_server(mut stream: DuplexStream) -> Result<(), BoxError> {
    let bootstrap = read_frame(&mut stream).await?;
    assert_eq!(
        decode_wire_envelope(&bootstrap)?
            .client_bootstrap()?
            .admission_material,
        b"grant"
    );
    write_frame(
        &mut stream,
        &encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&uuid_v7(1))?,
            world_id: WorldId::decode(&uuid_v7(2))?,
            channel_id: ChannelId::decode(&uuid_v7(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 7,
            schema_revision: 1,
            selected_capabilities: &[],
        })?,
    )
    .await?;
    let spatial_payload = spatial(0);
    let overlay_payload = encode_world_object_overlay_snapshot(&[WireOverlayEntry {
        content_generation: CONTENT_GENERATION,
        placement: b"oteryn:cell/entry-door".to_vec(),
        state: b"oteryn:reference.state.closed".to_vec(),
        revision: 0,
    }])
    .map_err(|error| format!("overlay snapshot: {error:?}"))?;
    for frame in encode_single_chunk_snapshot(
        1,
        1,
        40,
        &[
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 5,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &spatial_payload,
            },
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                revision: 2,
                snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload: &overlay_payload,
            },
        ],
    )? {
        write_frame(&mut stream, &frame).await?;
    }

    let command = read_frame(&mut stream).await?;
    let command = decode_wire_envelope(&command)?;
    assert_eq!(command.client_command(1)?.command_id, 7);
    write_frame(
        &mut stream,
        &encode_command_result(
            1,
            41,
            7,
            CommandStatus::Accepted,
            &encode_step_result(StepDisposition::Moved),
        )?,
    )
    .await?;
    write_frame(
        &mut stream,
        &encode_state_delta(
            1,
            42,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            5,
            6,
            DELTA_TYPE_WORLD_SPATIAL_V1,
            &spatial(1),
        )?,
    )
    .await?;
    Ok(())
}

fn admission() -> Result<Admission<'static>, BoxError> {
    Ok(Admission {
        appearance_items: None,
        schema_revision: 1,
        character_id: CharacterId::decode(&uuid_v7(4))?,
        admission_material: b"grant",
        client_build_id: "oteryn-session-test",
        supported_capabilities: &[13],
        deadline: DEADLINE,
    })
}

pub(super) fn block_on<F: Future>(future: F) -> Result<F::Output, BoxError> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(future))
}

// Include topic files in this module to preserve test paths and private fixture access.
include!("session_tests_admission_cast.rs");
include!("session_tests_combat_logout.rs");
include!("session_tests_vitals.rs");
include!("session_tests_capabilities.rs");
include!("session_tests_push.rs");
include!("session_tests_entities.rs");
include!("session_tests_chat.rs");
include!("session_tests_items.rs");
