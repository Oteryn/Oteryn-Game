//! C17b client replay.
//!
//! The owning client state is `Session::apply_state_delta` in
//! `crates/session/src/domain_dispatch.rs`; its visibility branch calls
//! `WorldEntities::after` in `crates/session/src/entities.rs` and publishes the result through
//! `Session::world_entities` in `crates/session/src/session_access.rs` (the state read by
//! `tools/dev-client/src/lib.rs::DevClientSession::world_entities`).
use super::*;
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityFields, EntityLights, WorldSpatialEntitiesDelta, WorldSpatialEntitiesSnapshot,
    encode_world_spatial_entities_delta_with_fields,
    encode_world_spatial_entities_snapshot_with_fields,
};

fn replay_live(directory: &std::path::Path, stale_revision: bool) -> Result<(), BoxError> {
    use serde_json::Value;
    let expected: Value =
        serde_json::from_slice(&std::fs::read(directory.join("c17b-expected.json"))?)?;
    let cast: Value = serde_json::from_slice(&std::fs::read(directory.join("c17b-cast.json"))?)?;
    assert_eq!(
        expected["run"].as_str(),
        directory.file_name().and_then(|name| name.to_str())
    );
    let keys = cast.as_object().ok_or("invalid C17b cast envelope")?;
    assert_eq!(keys.len(), 5);
    for key in [
        "command_id",
        "game_session_id",
        "payload",
        "server_sequence",
        "status",
    ] {
        assert!(keys.contains_key(key));
    }
    assert_eq!(cast["game_session_id"], expected["caster"]);
    assert_ne!(cast["game_session_id"], expected["observer"]);
    assert_eq!(cast["command_id"], expected["cast_command"]);
    assert_eq!(cast["server_sequence"], expected["cast_sequence"]);
    assert!(cast["command_id"].as_u64().is_some_and(|id| id > 0));
    assert!(
        cast["server_sequence"]
            .as_u64()
            .is_some_and(|sequence| sequence > 0)
    );
    assert_eq!(
        cast["status"].as_u64(),
        Some(CommandStatus::Accepted as u64)
    );
    let cast_payload: Vec<u8> = serde_json::from_value(cast["payload"].clone())?;
    assert_eq!(
        actor_spell::decode_spell_cast_result(&cast_payload)
            .map_err(|error| format!("{error:?}"))?,
        SpellCastDisposition::Cast
    );
    let cast_frame: Vec<u8> = serde_json::from_value(expected["cast_frame"].clone())?;
    let cast_frame = decode_wire_envelope(&cast_frame)?;
    cast_frame.validate(Direction::ServerToClient, true)?;
    assert_eq!(cast_frame.message_type(), MessageType::CommandResult);
    let reply = decode_command_result(cast_frame.payload())?;
    assert_eq!(reply.status, CommandStatus::Accepted);
    assert_eq!(Some(reply.command_id), cast["command_id"].as_u64());
    assert_eq!(
        Some(cast_frame.server_sequence()),
        cast["server_sequence"].as_u64()
    );
    assert_eq!(reply.payload, cast_payload);
    let received: Vec<Vec<u8>> = serde_json::from_value(expected["received"].clone())?;
    let commands: Vec<Vec<u8>> = serde_json::from_value(expected["commands"].clone())?;
    let character: [u8; 16] = serde_json::from_value(expected["observer_character"].clone())?;
    let target = EntityRef {
        identity: serde_json::from_value(expected["identity"].clone())?,
        generation: expected["generation"]
            .as_u64()
            .ok_or("missing C17b generation")?,
    };
    let last_sequence = expected["observer_sequence"]
        .as_u64()
        .ok_or("missing C17b observer sequence")?;
    assert!(!commands.is_empty());
    let accepted = decode_wire_envelope(received.first().ok_or("no original C17b admission")?)?;
    assert_eq!(accepted.message_type(), MessageType::ServerAccepted);
    let accepted = decode_server_accepted(accepted.payload())?;
    let session = accepted.game_session_id.as_bytes();
    let hex = |bytes: &[u8]| {
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    let session = format!(
        "{}-{}-{}-{}-{}",
        hex(&session[..4]),
        hex(&session[4..6]),
        hex(&session[6..8]),
        hex(&session[8..10]),
        hex(&session[10..])
    );
    assert_eq!(expected["observer"].as_str(), Some(session.as_str()));
    let directions = commands
        .iter()
        .map(|bytes| -> Result<_, BoxError> {
            let envelope = decode_wire_envelope(bytes)?;
            let command = envelope.client_command(accepted.connection_generation)?;
            assert_eq!(
                command.command_type,
                world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT
            );
            Ok(world_spatial::decode_step_intent_selected(
                &accepted.selected_capabilities,
                command.payload,
            )
            .map_err(|error| format!("{error:?}"))?)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let original = received
        .iter()
        .rev()
        .find(|bytes| {
            decode_wire_envelope(bytes).is_ok_and(|frame| {
                frame.message_type() == MessageType::StateDelta
                    && frame.server_sequence() <= last_sequence
                    && (!stale_revision
                        || decode_state_delta(frame.payload()).is_ok_and(|delta| {
                            delta.domain_id == STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY
                        }))
            })
        })
        .ok_or("missing original C17b delta for negative replay")?;
    let duplicate = if stale_revision {
        // Only this negative injection changes the sequence. The payload and both
        // visibility revisions are the original live delta.
        let frame = decode_wire_envelope(original)?;
        let delta = decode_state_delta(frame.payload())?;
        encode_state_delta(
            accepted.connection_generation,
            last_sequence
                .checked_add(1)
                .ok_or("C17b replay sequence overflow")?,
            delta.domain_id,
            delta.base_revision,
            delta.new_revision,
            delta.delta_type,
            delta.payload,
        )?
    } else {
        original.clone()
    };
    block_on(async move {
        let (client, server) = tokio::io::duplex(256 * 1024);
        let (mut input, mut output) = tokio::io::split(server);
        let (duplicate_now, send_duplicate) = tokio::sync::oneshot::channel();
        let pump = tokio::spawn(async move {
            let writer = tokio::spawn(async move {
                for frame in received {
                    write_frame(&mut output, &frame).await?;
                }
                send_duplicate.await?;
                write_frame(&mut output, &duplicate).await?;
                Ok::<(), BoxError>(())
            });
            let reader = async {
                let bootstrap = read_frame(&mut input).await?;
                assert_eq!(
                    decode_wire_envelope(&bootstrap)?.message_type(),
                    MessageType::ClientBootstrap
                );
                for expected in commands {
                    loop {
                        let sent = read_frame(&mut input).await?;
                        if decode_wire_envelope(&sent)?.message_type() == MessageType::LivenessAck {
                            continue;
                        }
                        assert_eq!(sent, expected);
                        break;
                    }
                }
                // Drain liveness acknowledgements while the original tail is dispatched.
                while read_frame(&mut input).await.is_ok() {}
                Ok::<(), BoxError>(())
            };
            reader.await?;
            writer.await??;
            Ok::<(), BoxError>(())
        });
        let admission = Admission {
            character_id: CharacterId::decode(&character)?,
            ..entity_admission()?
        };
        let mut observer = Session::admit(client, admission).await?;
        for direction in directions {
            let step = observer.step(direction).await?;
            assert_eq!(step.disposition, world_spatial::StepDisposition::Moved);
            observer.take_events();
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while observer.last_server_sequence() < last_sequence {
            assert!(
                tokio::time::Instant::now() < deadline,
                "C17b replay did not reach its final sequence"
            );
            observer.service_liveness(Duration::from_millis(5)).await?;
            assert!(!observer.take_events().into_iter().any(|event| matches!(event,
                SessionEvent::WorldSpatialEntities(delta) if delta.value.leave.iter().any(|entity| entity.identity == target.identity))));
        }
        assert_eq!(observer.last_server_sequence(), last_sequence);
        assert!(
            observer
                .world_entities()
                .is_some_and(|view| view.get(&target).is_some())
        );
        duplicate_now
            .send(())
            .map_err(|_| "C17b duplicate frame sender closed")?;
        let result = observer.service_liveness(Duration::from_secs(1)).await;
        if stale_revision {
            assert!(
                matches!(
                    result,
                    Err(SessionError::StateRevisionMismatch {
                        domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                        ..
                    })
                ),
                "{result:?}"
            );
        } else {
            assert!(
                matches!(result, Err(SessionError::ServerSequenceMismatch { .. })),
                "{result:?}"
            );
        }
        assert!(
            observer
                .world_entities()
                .is_some_and(|view| view.get(&target).is_some())
        );
        drop(observer);
        pump.await??;
        Ok::<(), BoxError>(())
    })?
}

#[test]
fn e2e_c17b_invisibility_keeps_player_visible_and_rejects_duplicate_frame() -> Result<(), BoxError>
{
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let directory = std::path::Path::new(&directory);
        replay_live(directory, false)?;
        return replay_live(directory, true);
    }
    block_on(async {
        let target = actor(EntityKind::Player, 9, 3, at(1, 0));
        let snapshot = WorldSpatialEntitiesSnapshot {
            content_generation: CONTENT_GENERATION,
            actor_position: at(0, 0),
            own_identity: OWN,
            entities: vec![own(at(0, 0)), target],
        };
        let delta = WorldSpatialEntitiesDelta {
            content_generation: CONTENT_GENERATION,
            actor_position: at(0, 0),
            enter: vec![],
            update: vec![target],
            leave: vec![],
        };
        let fields = EntityFields::selected(&[6]);
        let snapshot = encode_world_spatial_entities_snapshot_with_fields(
            &snapshot,
            fields,
            &EntityLights::new(),
        )
        .map_err(|error| format!("{error:?}"))?;
        let payload =
            encode_world_spatial_entities_delta_with_fields(&delta, fields, &EntityLights::new())
                .map_err(|error| format!("{error:?}"))?;
        let frame = encode_state_delta(
            1,
            41,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            5,
            6,
            DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
            &payload,
        )?;
        let (client, peer) = entity_peer(
            (SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2, snapshot),
            &[6],
            vec![Step::SendTogether(vec![frame.clone(), frame])],
        );
        let mut session = Session::admit(client, entity_admission()?).await?;
        assert!(
            session
                .world_entities()
                .is_some_and(|view| view.get(&target.entity).is_some())
        );
        let result = session.service_liveness(Duration::from_secs(1)).await;
        assert!(matches!(
            result,
            Err(SessionError::ServerSequenceMismatch {
                expected: 42,
                actual: 41
            })
        ));
        assert_eq!(session.last_server_sequence(), 41);
        assert!(
            session
                .world_entities()
                .is_some_and(|view| view.get(&target.entity).is_some())
        );
        drop(session);
        peer.await??;
        Ok(())
    })?
}
