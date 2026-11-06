//! Regenerate the NX-4 native play recording using the production wire encoders:
//! cargo run -p oteryn-protocol-oteryn --example nx4_npc_voice_recording > apps/client/src/features/0007-chat/tests/npc-voice-wire.json
use oteryn_protocol_oteryn::chat::{
    ChatLine, ChatRoomSet, ChatSpeaker, ChatSpeechMode, encode_chat_line, encode_chat_rooms,
};
use oteryn_protocol_oteryn::world_object::{
    SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1, STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
    encode_world_object_overlay_snapshot,
};
use oteryn_protocol_oteryn::world_spatial::{
    ActorPosition, STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, WorldSpatialObservation,
    encode_world_spatial,
};
use oteryn_protocol_oteryn::{
    ChannelId, DomainSnapshot, GameSessionId, ServerAcceptedValue, WorldId, encode_server_accepted,
    encode_single_chunk_snapshot, encode_state_delta,
};

fn uuid(marker: u8) -> [u8; 16] {
    [
        1, 0x93, 0x4f, 0x10, 0x7c, 0, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        marker,
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let at = ActorPosition {
        x: 32310,
        y: 32210,
        floor: 6,
    };
    let mut frames = vec![encode_server_accepted(&ServerAcceptedValue {
        game_session_id: GameSessionId::decode(&uuid(1))?,
        world_id: WorldId::decode(&uuid(2))?,
        channel_id: ChannelId::decode(&uuid(3))?,
        connection_generation: 1,
        current_server_sequence: 0,
        next_command_id: 7,
        schema_revision: 1,
        selected_capabilities: &[7],
    })?];
    let spatial = encode_world_spatial(&WorldSpatialObservation {
        content_generation: [0x11; 32],
        actor_position: at,
    });
    let rooms = encode_chat_rooms(ChatRoomSet::default());
    let objects = encode_world_object_overlay_snapshot(&[]).map_err(|e| format!("{e:?}"))?;
    frames.extend(encode_single_chunk_snapshot(
        1,
        1,
        40,
        &[
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                revision: 1,
                snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload: &objects,
            },
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 5,
                snapshot_type: 1,
                payload: &spatial,
            },
            DomainSnapshot {
                domain_id: 12,
                revision: 1,
                snapshot_type: 1,
                payload: &rooms,
            },
        ],
    )?);
    for (offset, (mode, text)) in [
        // Crystal 201189f8 data-global/npc/captain_bluebear.lua:26-30.
        (ChatSpeechMode::Say, "Passages to Carlin, Ab'Dendriel, Edron, Venore, Port Hope, Liberty Bay, Yalahar, Roshamuul, Krailos, Oramond and Svargrond."),
        // Codec/rendition probe: an NPC yell must preserve the source's case.
        (ChatSpeechMode::Yell, "Come closer, traveller."),
    ].into_iter().enumerate() {
        let offset = u64::try_from(offset)?;
        let payload = encode_chat_line(&ChatLine::Local {
            speaker: ChatSpeaker { identity: [42; 16], generation: std::num::NonZeroU64::MIN },
            speaker_name: "Captain Bluebear".into(), mode, text: text.into(), position: at,
        }).map_err(|e| format!("{e:?}"))?;
        frames.push(encode_state_delta(1, 41 + offset, 12, 1 + offset, 2 + offset, 1, &payload)?);
    }
    println!("{}", serde_json::to_string_pretty(&frames)?);
    Ok(())
}
