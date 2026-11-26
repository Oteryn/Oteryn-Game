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

fn replay_live(directory: &std::path::Path) -> Result<(), BoxError> {
    use serde_json::Value;
    use world_spatial_entities::{
        decode_world_spatial_entities_delta_with_sidecar,
        decode_world_spatial_entities_snapshot_with_fields,
    };
    let read = |name: &str| -> Result<Value, BoxError> {
        Ok(serde_json::from_slice(&std::fs::read(
            directory.join(name),
        )?)?)
    };
    let expected = read("c17b-expected.json")?;
    assert_eq!(
        expected["run"].as_str(),
        directory.file_name().and_then(|name| name.to_str())
    );
    let caster = expected["caster"]
        .as_str()
        .ok_or("missing caster session")?;
    let observer = expected["observer"]
        .as_str()
        .ok_or("missing observer session")?;
    assert_ne!(caster, observer);
    for id in [caster, observer] {
        assert_eq!(
            id.split('-').map(str::len).collect::<Vec<_>>(),
            [8, 4, 4, 4, 12]
        );
        assert!(
            id.bytes()
                .all(|byte| byte == b'-' || byte.is_ascii_hexdigit())
        );
    }
    let number = |value: &Value| -> Result<u64, BoxError> {
        value.as_u64().ok_or_else(|| "missing integer".into())
    };
    let envelope = |value: &Value,
                    session: &str,
                    command: Option<u64>,
                    status: Option<u64>|
     -> Result<(u64, Vec<u8>), BoxError> {
        let keys = value.as_object().ok_or("invalid C17b envelope")?;
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
        assert_eq!(value["game_session_id"].as_str(), Some(session));
        assert_eq!(value["command_id"], serde_json::json!(command));
        assert_eq!(value["status"], serde_json::json!(status));
        Ok((
            number(&value["server_sequence"])?,
            serde_json::from_value(value["payload"].clone())?,
        ))
    };
    let cast_command = number(&expected["cast_command"])?;
    assert!(cast_command > 0);
    let (cast_sequence, cast_payload) = envelope(
        &read("c17b-cast.json")?,
        caster,
        Some(cast_command),
        Some(CommandStatus::Accepted as u64),
    )?;
    assert!(cast_sequence > 0 && cast_sequence == number(&expected["cast_sequence"])?);
    assert_eq!(
        actor_spell::decode_spell_cast_result(&cast_payload)
            .map_err(|error| format!("C17b capture decode: {error:?}"))?,
        SpellCastDisposition::Cast
    );
    let (mut previous_sequence, payload) =
        envelope(&read("c17b-snapshot.json")?, observer, None, None)?;
    let selected: Vec<u32> = serde_json::from_value(expected["selected"].clone())?;
    let fields = EntityFields::selected(&selected);
    let (snapshot, lights) = decode_world_spatial_entities_snapshot_with_fields(&payload, fields)
        .map_err(|error| format!("C17b capture decode: {error:?}"))?;
    let mut entities = WorldEntities::from_snapshot(&snapshot, lights);
    let target = EntityRef {
        identity: serde_json::from_value(expected["identity"].clone())?,
        generation: number(&expected["generation"])?,
    };
    let mut revision = number(&expected["baseline_revision"])?;
    let leave_sequence = number(&expected["leave_sequence"])?;
    let deltas = expected["deltas"]
        .as_array()
        .ok_or("missing visibility deltas")?;
    assert!(!deltas.is_empty());
    for record in deltas {
        let (sequence, payload) = envelope(&record["frame"], observer, None, None)?;
        assert!(previous_sequence < sequence && sequence <= leave_sequence);
        assert_eq!(number(&record["base"])?, revision);
        let next_revision = number(&record["revision"])?;
        assert!(next_revision > revision);
        assert_eq!(
            number(&record["delta_type"])?,
            u64::from(DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2)
        );
        let (delta, lights, _) = decode_world_spatial_entities_delta_with_sidecar(&payload, fields)
            .map_err(|error| format!("C17b capture decode: {error:?}"))?;
        assert_eq!(delta.content_generation, snapshot.content_generation);
        if sequence == leave_sequence {
            assert!(entities.get(&target).is_some());
            assert!(delta.leave.contains(&target));
            entities = entities.after(&delta, &lights)?;
            assert!(entities.get(&target).is_none());
            assert!(entities.after(&delta, &lights).is_err());
        } else {
            entities = entities.after(&delta, &lights)?;
        }
        revision = next_revision;
        previous_sequence = sequence;
    }
    assert_eq!(previous_sequence, leave_sequence);
    Ok(())
}

#[test]
fn e2e_c17b_invisibility_visibility_payload_removes_target_and_rejects_replay()
-> Result<(), BoxError> {
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        return replay_live(std::path::Path::new(&directory));
    }
    block_on(async {
        let (
            snapshot_payload,
            delta_payload,
            expected_identity,
            expected_generation,
            sequence,
            selected,
        ) = {
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
                update: vec![],
                leave: vec![target.entity],
            };
            let fields = EntityFields::selected(&[6]);
            (
                encode_world_spatial_entities_snapshot_with_fields(
                    &snapshot,
                    fields,
                    &EntityLights::new(),
                )
                .map_err(|error| format!("{error:?}"))?,
                encode_world_spatial_entities_delta_with_fields(
                    &delta,
                    fields,
                    &EntityLights::new(),
                )
                .map_err(|error| format!("{error:?}"))?,
                target.entity.identity,
                target.entity.generation,
                41,
                &[6_u32] as &'static [u32],
            )
        };
        let target = EntityRef {
            identity: expected_identity,
            generation: expected_generation,
        };
        let (client, peer) = entity_peer(
            (SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2, snapshot_payload),
            selected,
            vec![],
        );
        let mut session = Session::admit(client, entity_admission()?).await?;
        assert!(
            session
                .world_entities()
                .is_some_and(|state| state.get(&target).is_some())
        );
        let frame = encode_state_delta(
            1,
            sequence,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            5,
            6,
            DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
            &delta_payload,
        )?;
        let payload = decode_wire_envelope(&frame)?.payload().to_vec();
        let event = session.apply_state_delta(sequence, &payload)?;
        assert!(matches!(event, SessionEvent::WorldSpatialEntities(_)));
        assert!(
            session
                .world_entities()
                .is_some_and(|state| state.get(&target).is_none())
        );
        let visible_after = session.world_entities().map(WorldEntities::len);
        assert!(session.apply_state_delta(sequence + 1, &payload).is_err());
        assert_eq!(
            session.world_entities().map(WorldEntities::len),
            visible_after
        );
        drop(session);
        peer.await??;
        Ok(())
    })?
}
