//! M04 replay owns the session state seam used by `tools/dev-client/src/lib.rs`.
//! Raw domain payloads are consumed by `Session::apply_state_delta` in
//! `crates/session/src/domain_dispatch.rs`; entity replacement is validated atomically by
//! `WorldEntities::after` in `crates/session/src/entities.rs`.

use super::*;

#[test]
fn captured_push_delta_updates_visible_player_and_invalid_update_is_atomic() -> Result<(), BoxError>
{
    block_on(async {
        let (payload, metadata, initial, moved) = fixture()?;
        let valid = world_spatial_entities::decode_world_spatial_entities_delta(&payload)
            .map_err(|error| format!("M04 decode: {error:?}"))?;
        assert!(valid.update.contains(&moved));
        let (base, new, sequence) = metadata;
        let (client, peer) = entity_peer(
            replay_snapshot(
                valid.content_generation,
                valid.actor_position,
                vec![initial],
            )?,
            &[6],
            vec![Step::Send(encode_state_delta(
                1,
                sequence,
                STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                base,
                new,
                DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                &payload,
            )?)],
        );
        let mut session = Session::admit(client, entity_admission()?).await?;
        session.service_liveness(Duration::from_millis(100)).await?;
        assert_eq!(
            session
                .world_entities()
                .and_then(|view| view.get(&moved.entity)),
            Some(&moved)
        );

        let before = session.world_entities().cloned().ok_or("entity store")?;
        let invalid = WorldSpatialEntitiesDelta {
            update: vec![actor(EntityKind::Player, 9, 0, at(4, 4))],
            ..valid
        };
        assert!(
            before
                .after(&invalid, &world_spatial_entities::EntityLights::new())
                .is_err()
        );
        assert_eq!(session.world_entities(), Some(&before));
        drop(session);
        peer.await??;
        Ok(())
    })?
}

fn replay_snapshot(
    content_generation: [u8; 32],
    own_position: ActorPosition,
    others: Vec<WorldSpatialEntity>,
) -> Result<(u32, Vec<u8>), BoxError> {
    let mut entities = vec![own(own_position)];
    entities.extend(others);
    let payload = world_spatial_entities::encode_world_spatial_entities_snapshot(
        &WorldSpatialEntitiesSnapshot {
            content_generation,
            actor_position: own_position,
            own_identity: OWN,
            entities,
        },
    )
    .map_err(|error| format!("M04 snapshot encode: {error:?}"))?;
    Ok((SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2, payload))
}

type Fixture = (
    Vec<u8>,
    (u64, u64, u64),
    WorldSpatialEntity,
    WorldSpatialEntity,
);

fn fixture() -> Result<Fixture, BoxError> {
    let initial = actor(EntityKind::Player, 1, 0, at(1, 0));
    let mut moved = initial;
    moved.position = at(1, 1);
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let directory = std::path::Path::new(&directory);
        let payload = std::fs::read(directory.join("m04-spatial.bin"))?;
        let expected = std::fs::read_to_string(directory.join("m04-expected.txt"))?;
        let fields = expected.split_whitespace().collect::<Vec<_>>();
        let [base, new, sequence, identity, generation, x, y, floor] = fields.as_slice() else {
            return Err("invalid M04 expected capture".into());
        };
        let identity = decode_identity(identity)?;
        let reference = EntityRef {
            identity,
            generation: generation.parse()?,
        };
        let decoded = world_spatial_entities::decode_world_spatial_entities_delta(&payload)
            .map_err(|error| format!("M04 decode: {error:?}"))?;
        let moved = decoded
            .update
            .iter()
            .find(|entity| entity.entity == reference)
            .copied()
            .ok_or("M04 expected entity absent")?;
        if moved.position
            != (ActorPosition {
                x: x.parse()?,
                y: y.parse()?,
                floor: floor.parse()?,
            })
        {
            return Err("M04 expected position differs from payload".into());
        }
        let mut initial = moved;
        initial.position.x = initial.position.x.saturating_sub(1);
        Ok((
            payload,
            (base.parse()?, new.parse()?, sequence.parse()?),
            initial,
            moved,
        ))
    } else {
        let payload = world_spatial_entities::encode_world_spatial_entities_delta(
            &WorldSpatialEntitiesDelta {
                content_generation: CONTENT_GENERATION,
                actor_position: at(0, 0),
                enter: vec![],
                update: vec![moved],
                leave: vec![],
            },
        )
        .map_err(|error| format!("M04 encode: {error:?}"))?;
        Ok((payload, (5, 6, 41), initial, moved))
    }
}

fn decode_identity(value: &str) -> Result<[u8; 16], BoxError> {
    if value.len() != 32 {
        return Err("M04 identity length".into());
    }
    let mut identity = [0; 16];
    for (index, byte) in identity.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)?;
    }
    Ok(identity)
}
