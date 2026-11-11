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

fn decode_identity(value: &str) -> Result<[u8; 16], BoxError> {
    if value.len() != 32 || !value.is_ascii() {
        return Err("invalid C17b target identity length".into());
    }
    let mut identity = [0_u8; 16];
    for (index, byte) in identity.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)?;
    }
    Ok(identity)
}

#[test]
fn invisibility_visibility_payload_removes_target_and_rejects_replay() -> Result<(), BoxError> {
    block_on(async {
        let (
            snapshot_payload,
            delta_payload,
            expected_identity,
            expected_generation,
            sequence,
            selected,
        ) = if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let directory = std::path::Path::new(&directory);
            let expected = std::fs::read_to_string(directory.join("c17b-expected.txt"))?;
            let values = expected.split_whitespace().collect::<Vec<_>>();
            let [_, _, sequence, identity, generation] = values.as_slice() else {
                return Err("invalid C17b expected capture".into());
            };
            (
                std::fs::read(directory.join("c17b-snapshot.bin"))?,
                std::fs::read(directory.join("c17b-invisible.bin"))?,
                decode_identity(identity)?,
                generation.parse::<u64>()?,
                sequence.parse::<u64>()?,
                &[6_u32, 286] as &'static [u32],
            )
        } else {
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

#[test]
fn captured_identity_rejects_malformed_and_non_ascii_text() -> Result<(), BoxError> {
    assert!(decode_identity("").is_err());
    assert!(decode_identity(&"é".repeat(16)).is_err());
    assert!(decode_identity(&"gg".repeat(16)).is_err());
    assert_eq!(decode_identity(&"ff".repeat(16))?, [255; 16]);
    Ok(())
}
