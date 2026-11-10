#[path = "tests/account_achievements.rs"]
mod account_achievements_tests;
#[path = "tests/activity_handoff.rs"]
mod activity_handoff_tests;
#[path = "tests/analyser.rs"]
mod analyser_tests;
mod appearance_session_tests;
#[path = "cast_chat/tests.rs"]
mod cast_chat_tests;
#[path = "features/0083-channel-list/tests.rs"]
mod channel_list_tests;
#[path = "container_tree_tests.rs"]
mod container_tree_tests;
#[path = "tests/depot_map_tests.rs"]
mod depot_map_tests;
#[cfg(test)]
#[path = "tests/e2e_c01.rs"]
mod e2e_c01;
#[path = "tests/e2e_c02.rs"]
mod e2e_c02;
#[path = "tests/e2e_c07.rs"]
mod e2e_c07;
#[path = "tests/e2e_c08.rs"]
mod e2e_c08;
#[path = "tests/e2e_c09.rs"]
mod e2e_c09;
#[path = "tests/e2e_c12.rs"]
mod e2e_c12;
#[path = "tests/e2e_c13.rs"]
mod e2e_c13;
#[path = "tests/e2e_c16a.rs"]
mod e2e_c16a;
#[path = "tests/e2e_c16c.rs"]
mod e2e_c16c;
#[path = "tests/e2e_c17b.rs"]
mod e2e_c17b;
#[path = "tests/e2e_c17c.rs"]
mod e2e_c17c;
#[path = "tests/e2e_c17d.rs"]
mod e2e_c17d;
#[path = "tests/e2e_c18b.rs"]
mod e2e_c18b;
#[path = "tests/e2e_c19a.rs"]
mod e2e_c19a;
#[path = "tests/e2e_c20b.rs"]
mod e2e_c20b;
#[path = "tests/e2e_c20d.rs"]
mod e2e_c20d;
#[path = "tests/e2e_e02.rs"]
mod e2e_e02;
#[path = "tests/e2e_e03.rs"]
mod e2e_e03;
#[path = "tests/e2e_i06.rs"]
mod e2e_i06;
#[path = "tests/e2e_i08.rs"]
mod e2e_i08;
#[path = "tests/e2e_i09.rs"]
mod e2e_i09;
#[path = "tests/e2e_i12.rs"]
mod e2e_i12;
#[path = "tests/e2e_i15.rs"]
mod e2e_i15;
#[cfg(test)]
#[path = "tests/e2e_m02.rs"]
mod e2e_m02;
#[path = "tests/e2e_m03.rs"]
mod e2e_m03;
#[cfg(test)]
#[path = "tests/e2e_m04.rs"]
mod e2e_m04;
#[path = "tests/e2e_m05.rs"]
mod e2e_m05;
#[path = "tests/e2e_m06.rs"]
mod e2e_m06;
#[path = "tests/e2e_m07.rs"]
mod e2e_m07;
#[path = "tests/e2e_m08.rs"]
mod e2e_m08;
#[path = "tests/e2e_m09.rs"]
mod e2e_m09;
#[path = "tests/e2e_m11a.rs"]
mod e2e_m11a;
#[path = "tests/e2e_p11.rs"]
mod e2e_p11;
#[cfg(test)]
#[path = "tests/e2e_s04c.rs"]
mod e2e_s04c;
#[cfg(test)]
#[path = "tests/e2e_s09.rs"]
mod e2e_s09;
#[cfg(test)]
#[path = "tests/e2e_s10.rs"]
mod e2e_s10;
#[cfg(test)]
#[path = "tests/e2e_s13a.rs"]
mod e2e_s13a;
#[cfg(test)]
#[path = "tests/e2e_s13b.rs"]
mod e2e_s13b;
#[cfg(test)]
#[path = "tests/e2e_s15b.rs"]
mod e2e_s15b;
#[cfg(test)]
#[path = "tests/e2e_s16a.rs"]
mod e2e_s16a;
#[path = "tests/e2e_w01a.rs"]
mod e2e_w01a;
#[path = "tests/e2e_w01b.rs"]
mod e2e_w01b;
#[path = "tests/e2e_w01c.rs"]
mod e2e_w01c;
#[path = "tests/e2e_w05a.rs"]
mod e2e_w05a;
#[cfg(test)]
#[path = "tests/e2e_w15b.rs"]
mod e2e_w15b;
#[cfg(test)]
#[path = "tests/e2e_w16.rs"]
mod e2e_w16;
#[cfg(test)]
#[path = "tests/e2e_w19a.rs"]
mod e2e_w19a;
#[cfg(test)]
#[path = "tests/e2e_w20a.rs"]
mod e2e_w20a;
#[cfg(test)]
#[path = "tests/e2e_w20b.rs"]
mod e2e_w20b;
#[cfg(test)]
#[path = "tests/e2e_w22a.rs"]
mod e2e_w22a;
#[cfg(test)]
#[path = "tests/e2e_w23.rs"]
mod e2e_w23;
#[cfg(test)]
#[path = "tests/e2e_w24.rs"]
mod e2e_w24;
#[path = "features/0004-item-view-move/tests.rs"]
mod item_movement_tests;
#[path = "features/0004-item-view-move/tests/e2e_i05.rs"]
mod item_replay_i05_tests;
#[path = "tests/npc_conversation_replay.rs"]
mod npc_conversation_replay;
#[path = "tests/property_services.rs"]
mod property_services_tests;
#[path = "tests/pvp_expert.rs"]
mod pvp_expert_tests;
#[path = "quest_log_transport/tests.rs"]
mod quest_log_tests;
#[path = "tests/replacement_snapshot.rs"]
mod replacement_snapshot_tests;
#[path = "features/0287-actor-spell-catalog/tests.rs"]
mod spell_catalog_tests;
#[path = "tests/status_projection_tests.rs"]
mod status_projection_tests;
#[path = "features/0064-target-move/tests.rs"]
mod target_move_tests;
include!("tutorial_notices_tests.rs");
include!("presentation_tests.rs");
#[path = "features/0011-world-interaction/tests.rs"]
mod world_context_tests;
#[path = "features/0019-world-light/tests.rs"]
mod world_light_tests;
#[path = "features/0026-world-weather/tests.rs"]
mod world_weather_tests;
include!("achievement_notices_tests.rs");
include!("server_messages_tests.rs");
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
include!("session_tests_recovery.rs");
include!("session_tests_combat_logout.rs");
include!("session_tests_vitals.rs");
include!("session_tests_capabilities.rs");
include!("session_tests_market_details.rs");
include!("session_tests_admission.rs");
#[path = "property_services/house_transfer_tests.rs"]
mod pc04_house_transfer;
include!("session_tests_capability_movement.rs");
include!("session_tests_capability_native.rs");
include!("session_tests_push.rs");
include!("session_tests_entities.rs");
#[path = "features/0004-item-view-move/tests/e2e_i13.rs"]
mod e2e_i13;
include!("session_tests_chat.rs");
include!("session_tests_chat_messages.rs");
#[path = "tests/e2e_i14.rs"]
mod e2e_i14;
#[path = "tests/e2e_s04b.rs"]
mod e2e_s04b;
#[path = "tests/e2e_s18.rs"]
mod e2e_s18;
include!("session_tests_items.rs");
include!("session_tests_refusal_details.rs");
include!("session_tests_browse_field.rs");

include!("session_tests_player_trade.rs");

#[path = "tests/gem_presentation_gated.rs"]
mod gem_presentation_gated;

#[path = "tests/wheel_presentation_gated.rs"]
mod wheel_presentation_gated;

#[cfg(test)]
#[path = "e2e_c16d.rs"]
mod e2e_c16d;
