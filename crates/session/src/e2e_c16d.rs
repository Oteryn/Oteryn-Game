//! C16d client replay.
//!
//! The owning client seam is `Session::admit` in `snapshot_state.rs`, which decodes the
//! domain-1 snapshot into `Session::entities`; `Session::world_entities` in
//! `session_access.rs` exposes the client-visible actor appearance reference.

use super::*;
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityDetail, WorldSpatialEntitiesSnapshot, encode_world_spatial_entities_snapshot,
};

#[test]
fn e2e_c16d_item_appearance_snapshot_replay() -> Result<(), BoxError> {
    block_on(async {
        let (payload, expected) = if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let path = std::path::Path::new(&path);
            let expected = std::fs::read_to_string(path.join("c16d-expected.txt"))?
                .trim()
                .parse::<u32>()?;
            (std::fs::read(path.join("c16d-entities.bin"))?, expected)
        } else {
            let expected = 2_854;
            let mut actor = own(at(0, 0));
            actor.detail = EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: expected,
                health_percent: 100,
            };
            let fixture = WorldSpatialEntitiesSnapshot {
                content_generation: CONTENT_GENERATION,
                actor_position: at(0, 0),
                own_identity: OWN,
                entities: vec![actor],
            };
            (
                encode_world_spatial_entities_snapshot(&fixture)
                    .map_err(|error| format!("{error:?}"))?,
                expected,
            )
        };
        let (client, peer) = entity_peer(
            (SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2, payload.clone()),
            &[6],
            vec![],
        );
        let session = Session::admit(client, entity_admission()?).await?;
        let own = session.world_entities().ok_or("C16d entities absent")?;
        let actor = own
            .iter()
            .find(|entity| &entity.entity.identity == own.own_identity())
            .ok_or("C16d own actor absent")?;
        assert!(
            matches!(actor.detail, EntityDetail::Actor { appearance_ref, .. } if appearance_ref == expected)
        );
        drop(session);
        peer.await??;

        let mut invalid = payload;
        invalid.truncate(invalid.len().saturating_sub(1));
        let (client, peer) = entity_peer(
            (SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2, invalid),
            &[6],
            vec![],
        );
        assert!(Session::admit(client, entity_admission()?).await.is_err());
        peer.await??;
        Ok(())
    })?
}
