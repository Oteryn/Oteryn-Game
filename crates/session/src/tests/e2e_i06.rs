//! I06 replay owner: `Session::apply_state_delta` in `domain_dispatch.rs` decodes entity deltas,
//! applies `WorldEntities::after` in `entities.rs`, and exposes the result through
//! `Session::world_entities` in `session_access.rs`. The server capture is made by
//! `apps/game-server/src/qualification/client_phases/client_row_i06.rs`.
use super::*;
use oteryn_protocol_oteryn::world_spatial_entities::{
    WorldSpatialEntitiesDelta, decode_world_spatial_entities_delta,
};

fn fixture() -> Result<(Vec<u8>, Vec<u8>, String), BoxError> {
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let directory = std::path::Path::new(&directory);
        return Ok((
            std::fs::read(directory.join("i06-corpse.bin"))?,
            std::fs::read(directory.join("i06-decay.bin"))?,
            std::fs::read_to_string(directory.join("i06-expected.txt"))?,
        ));
    }
    let corpse = corpse(0x6, at(3, 0));
    let introduced = entities_delta(at(0, 0), vec![corpse], vec![], vec![]);
    let retired = entities_delta(at(0, 0), vec![], vec![], vec![corpse.entity]);
    Ok((
        encode_world_spatial_entities_delta(&introduced).map_err(|error| format!("{error:?}"))?,
        encode_world_spatial_entities_delta(&retired).map_err(|error| format!("{error:?}"))?,
        format!("{} 1\n", "06".repeat(16)),
    ))
}

fn decode(payload: &[u8]) -> Result<WorldSpatialEntitiesDelta, BoxError> {
    decode_world_spatial_entities_delta(payload).map_err(|error| format!("{error:?}").into())
}

#[test]
fn e2e_i06_corpse_decay_replay() -> Result<(), BoxError> {
    block_on(async {
        let (introduced_bytes, retired_bytes, expected) = fixture()?;
        let introduced = decode(&introduced_bytes)?;
        let retired = decode(&retired_bytes)?;
        let corpse = introduced
            .enter
            .iter()
            .find(|entity| entity.kind == EntityKind::Corpse)
            .copied()
            .ok_or("I06 capture has no introduced corpse")?;
        if !retired.leave.contains(&corpse.entity) {
            return Err("I06 retirement payload does not leave the introduced corpse".into());
        }
        let mut expected_fields = expected.split_whitespace();
        let expected_identity = expected_fields
            .next()
            .ok_or("I06 expected identity missing")?;
        let remaining_loot = expected_fields
            .next()
            .ok_or("I06 expected remaining-loot count missing")?
            .parse::<usize>()?;
        assert!(remaining_loot > 0);
        let actual_identity = corpse
            .entity
            .identity
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(actual_identity, expected_identity);

        let snapshot = WorldSpatialEntitiesSnapshot {
            content_generation: introduced.content_generation,
            actor_position: introduced.actor_position,
            own_identity: OWN,
            entities: vec![own(introduced.actor_position)],
        };
        let snapshot = (
            SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
            encode_world_spatial_entities_snapshot(&snapshot)
                .map_err(|error| format!("{error:?}"))?,
        );
        let (client, peer) = entity_peer(
            snapshot,
            &[6],
            vec![
                entities_push(41, 5, 6, &introduced)?,
                entities_push(42, 6, 7, &retired)?,
            ],
        );
        let mut session = Session::admit(client, entity_admission()?).await?;
        session.service_liveness(Duration::from_millis(300)).await?;
        assert!(
            session
                .world_entities()
                .is_some_and(|entities| entities.get(&corpse.entity).is_none())
        );
        drop(session);
        peer.await??;

        let (client, peer) = entity_peer(
            entity_snapshot(vec![corpse]),
            &[6],
            vec![entities_push(41, 4, 6, &retired)?],
        );
        let mut session = Session::admit(client, entity_admission()?).await?;
        let before = session
            .world_entities()
            .cloned()
            .ok_or("I06 entity store absent")?;
        assert!(matches!(
            session.service_liveness(Duration::from_millis(200)).await,
            Err(SessionError::StateRevisionMismatch { domain_id: 1, .. })
        ));
        assert_eq!(session.world_entities(), Some(&before));
        drop(session);
        peer.await??;
        Ok(())
    })?
}
