//! Synthetic atomicity regression. Live M04 qualification uses `replay_player_push`.
//! Raw domain payloads are consumed by `Session::apply_state_delta` in
//! `crates/session/src/domain_dispatch.rs`; entity replacement is validated atomically by
//! `WorldEntities::after` in `crates/session/src/entities.rs`.

use super::*;

#[test]
fn push_delta_updates_visible_player_and_invalid_update_is_atomic() -> Result<(), BoxError> {
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
    let payload =
        world_spatial_entities::encode_world_spatial_entities_delta(&WorldSpatialEntitiesDelta {
            content_generation: CONTENT_GENERATION,
            actor_position: at(0, 0),
            enter: vec![],
            update: vec![moved],
            leave: vec![],
        })
        .map_err(|error| format!("M04 encode: {error:?}"))?;
    Ok((payload, (5, 6, 41), initial, moved))
}
