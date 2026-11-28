//! Qualification replay of the original inbound frames for a player push.
use super::*;
use oteryn_protocol_oteryn::creature_push::{
    CAPABILITY_CREATURE_PUSH_V1, COMMAND_TYPE_CREATURE_PUSH_INTENT, CreaturePushIntent,
    encode_creature_push_intent,
};
use std::io::Cursor;

/// Verify the occupied refusal and the exact visible player movement by feeding the original
/// inbound frames through the ordinary admission and command decoders.
pub async fn replay_player_push(
    frames: &[u8],
    character: CharacterId,
    target: EntityRef,
    before: ActorPosition,
    landing: ActorPosition,
    delta_sequence: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let stream = tokio::io::join(Cursor::new(frames), tokio::io::sink());
    let mut session = Session::admit(
        stream,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: character,
            admission_material: b"offline-replay",
            client_build_id: "m04-replay",
            supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
            deadline: Duration::from_secs(5),
        },
    )
    .await?;
    let occupied = session.world_spatial().actor_position;
    if before == landing || occupied == landing || occupied == before {
        return Err("M04 replay positions are not discriminating".into());
    }
    for (destination, disposition) in [(occupied, 5), (landing, 1)] {
        let payload = encode_creature_push_intent(&CreaturePushIntent {
            target,
            destination,
        })
        .map_err(|error| format!("M04 replay push encode: {error:?}"))?;
        let reply = session
            .gated_command(
                CAPABILITY_CREATURE_PUSH_V1,
                COMMAND_TYPE_CREATURE_PUSH_INTENT,
                &payload,
            )
            .await?;
        if reply.status != CommandStatus::Accepted || reply.payload != [8, disposition] {
            return Err(format!("M04 replay push reply: {reply:?}").into());
        }
        if disposition == 5
            && session
                .world_entities()
                .and_then(|view| view.get(&target))
                .is_none_or(|entity| entity.kind != EntityKind::Player || entity.position != before)
        {
            return Err("M04 replay refusal changed the target".into());
        }
    }

    while session.last_server_sequence() < delta_sequence {
        let (sequence, payload) = session
            .read_sequenced("M04 push replay delta", MessageType::StateDelta)
            .await?;
        session.apply_state_delta(sequence, &payload)?;
    }
    let captured = session
        .raw_world_entity_deltas()
        .iter()
        .find(|delta| delta.server_sequence == delta_sequence)
        .ok_or("M04 replay omitted the captured push delta")?;
    let fields = world_spatial_entities::EntityFields::selected(session.selected_capabilities());
    let (delta, _) = world_spatial_entities::decode_world_spatial_entities_delta_with_fields(
        &captured.payload, fields,
    )
    .map_err(|error| format!("M04 replay delta: {error:?}"))?;
    if !delta.update.iter().any(|entity| {
        entity.entity == target && entity.kind == EntityKind::Player && entity.position == landing
    }) || session
        .world_entities()
        .and_then(|view| view.get(&target))
        .is_none_or(|entity| entity.kind != EntityKind::Player || entity.position != landing)
    {
        return Err("M04 replay did not apply the pushed player at the captured sequence".into());
    }
    Ok(())
}
