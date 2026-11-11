//! C16c client replay: `Session::apply_state_delta` in `domain_dispatch.rs` decodes the
//! capability-6 payload, and `WorldEntities::after` in `entities.rs` installs the visible
//! Convince Creature update and Animate Dead corpse-leave/skeleton-enter atomically.

use super::*;
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityDetail, EntityKind, WorldSpatialEntitiesDelta, WorldSpatialEntity,
    decode_world_spatial_entities_delta, encode_world_spatial_entities_delta,
};

fn named_actor(
    marker: u8,
    name: &str,
    position: ActorPosition,
) -> Result<WorldSpatialEntity, BoxError> {
    Ok(WorldSpatialEntity {
        display_name: Some(
            oteryn_protocol_oteryn::entity_names::EntityDisplayName::new(name)
                .map_err(|error| format!("{error:?}"))?,
        ),
        kind: EntityKind::Creature,
        entity: EntityRef {
            identity: [marker; 16],
            generation: 1,
        },
        position,
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    })
}

fn fixture() -> Result<(Vec<u8>, Vec<u8>, String), BoxError> {
    let rat = named_actor(2, "Rat", at(1, 0))?;
    let corpse = corpse(3, at(2, 0));
    let convinced = entities_delta(at(0, 0), vec![], vec![rat], vec![]);
    let animated = entities_delta(
        at(0, 0),
        vec![named_actor(4, "Skeleton", at(2, 0))?],
        vec![],
        vec![corpse.entity],
    );
    Ok((
        encode_world_spatial_entities_delta(&convinced).map_err(|error| format!("{error:?}"))?,
        encode_world_spatial_entities_delta(&animated).map_err(|error| format!("{error:?}"))?,
        "Rat 02020202020202020202020202020202 1\nSkeleton 04040404040404040404040404040404 1\n"
            .into(),
    ))
}

fn visible(session: &Session<DuplexStream>) -> Result<String, BoxError> {
    let mut values = session
        .world_entities()
        .ok_or("capability 6 selected")?
        .others()
        .filter_map(|entry| {
            let name = match entry.entity.identity[0] {
                2 => "Rat",
                4 => "Skeleton",
                _ => return None,
            };
            Some(format!(
                "{name} {} {}\n",
                entry
                    .entity
                    .identity
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>(),
                entry.entity.generation
            ))
        })
        .collect::<Vec<_>>();
    values.sort();
    Ok(values.concat())
}

#[test]
fn e2e_c16c_replays_real_companion_visibility_payloads() -> Result<(), BoxError> {
    block_on(async {
        if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            return replay_captures(std::path::Path::new(&root)).await;
        }
        let (convince, animate, expected) = fixture()?;
        let rat = named_actor(2, "Rat", at(1, 0))?;
        let dead = corpse(3, at(2, 0));
        let (client, peer) = entity_peer(
            entity_snapshot(vec![rat, dead]),
            &[6],
            vec![
                entities_push(
                    41,
                    5,
                    6,
                    &decode_world_spatial_entities_delta(&convince)
                        .map_err(|error| format!("{error:?}"))?,
                )?,
                entities_push(
                    42,
                    6,
                    7,
                    &decode_world_spatial_entities_delta(&animate)
                        .map_err(|error| format!("{error:?}"))?,
                )?,
            ],
        );
        let mut session = Session::admit(client, entity_admission()?).await?;
        session.service_liveness(Duration::from_millis(300)).await?;
        assert_eq!(visible(&session)?, expected);

        let before = visible(&session)?;
        let skeleton = session
            .world_entities()
            .and_then(|entities| {
                entities
                    .others()
                    .find(|entry| entry.entity.identity[0] == 4)
            })
            .copied()
            .ok_or("skeleton visible")?;
        let invalid = WorldSpatialEntitiesDelta {
            content_generation: CONTENT_GENERATION,
            actor_position: at(0, 0),
            enter: vec![skeleton],
            update: vec![],
            leave: vec![],
        };
        let frame = encode_state_delta(
            1,
            43,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            7,
            8,
            DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
            &encode_world_spatial_entities_delta(&invalid).map_err(|error| format!("{error:?}"))?,
        )?;
        let error = session
            .apply_state_delta(43, decode_wire_envelope(&frame)?.payload())
            .err()
            .ok_or("duplicate enter must be refused")?;
        assert!(matches!(
            error,
            SessionError::EntityStoreInconsistent { .. }
        ));
        assert_eq!(visible(&session)?, before);
        drop(session);
        peer.await??;
        Ok(())
    })?
}

async fn replay_captures(root: &std::path::Path) -> Result<(), BoxError> {
    for (stage, expected_name) in [("convince", "Rat"), ("animate", "Skeleton")] {
        let snapshot = std::fs::read(root.join(format!("c16c-{stage}-before.bin")))?;
        let payload = std::fs::read(root.join(format!("c16c-{stage}.bin")))?;
        let (delta, _) =
            oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_entities_delta_with_fields(&payload, oteryn_protocol_oteryn::world_spatial_entities::EntityFields::selected(&[6, 286])).map_err(|error| format!("{error:?}"))?;
        let affected = if stage == "convince" {
            &delta.update
        } else {
            &delta.enter
        };
        let entity = affected
            .iter()
            .find(|entity| {
                entity
                    .display_name
                    .as_ref()
                    .and_then(|name| name.as_str().ok())
                    == Some(expected_name)
            })
            .ok_or("real companion update absent")?;
        let (client, peer) = entity_peer(
            (SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2, snapshot),
            &[6, 286],
            vec![],
        );
        let mut session = Session::admit(client, entity_admission()?).await?;
        if stage == "convince" {
            assert!(
                session
                    .world_entities()
                    .is_some_and(|entities| entities.get(&entity.entity).is_some())
            );
        } else {
            assert!(!delta.leave.is_empty());
            for corpse in &delta.leave {
                assert!(
                    session
                        .world_entities()
                        .is_some_and(|entities| entities.get(corpse).is_some())
                );
            }
        }
        let frame = encode_state_delta(
            1,
            41,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            5,
            6,
            DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
            &payload,
        )?;
        session.apply_state_delta(41, decode_wire_envelope(&frame)?.payload())?;
        assert_eq!(
            session
                .world_entities()
                .and_then(|entities| entities.get(&entity.entity)),
            Some(entity)
        );
        for corpse in &delta.leave {
            assert!(
                session
                    .world_entities()
                    .is_some_and(|entities| entities.get(corpse).is_none())
            );
        }
        let before = session
            .world_entities()
            .map(|entities| entities.iter().copied().collect::<Vec<_>>());
        let invalid = WorldSpatialEntitiesDelta {
            content_generation: delta.content_generation,
            actor_position: delta.actor_position,
            enter: vec![*entity],
            update: vec![],
            leave: vec![],
        };
        let invalid =
            oteryn_protocol_oteryn::world_spatial_entities::encode_world_spatial_entities_delta_with_fields(&invalid, oteryn_protocol_oteryn::world_spatial_entities::EntityFields::selected(&[6, 286]), &oteryn_protocol_oteryn::world_spatial_entities::EntityLights::new()).map_err(|error| format!("{error:?}"))?;
        let frame = encode_state_delta(
            1,
            42,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            6,
            7,
            DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
            &invalid,
        )?;
        assert!(matches!(
            session.apply_state_delta(42, decode_wire_envelope(&frame)?.payload()),
            Err(SessionError::EntityStoreInconsistent { .. })
        ));
        assert_eq!(
            session
                .world_entities()
                .map(|entities| entities.iter().copied().collect::<Vec<_>>()),
            before
        );
        drop(session);
        peer.await??;
    }
    Ok(())
}
