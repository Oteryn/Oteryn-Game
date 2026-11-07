//! The existing session appearance fixture, shared without a second registration.
use super::*;

type JoinedAppearance = (appearance::Snapshot, WorldSpatialEntity, (u32, Vec<u8>));

pub(super) fn snapshot() -> Result<JoinedAppearance, BoxError> {
    let mut snapshot = appearance::Snapshot::decode(GOLDEN).map_err(|e| format!("{e:?}"))?;
    snapshot.catalogue.context.content_generation = CONTENT_GENERATION;
    snapshot.bindings[0].context = snapshot.catalogue.context;
    snapshot.bindings[0].entity.identity = OWN;
    let mut entity = own(at(0, 0));
    let EntityDetail::Actor { appearance_ref, .. } = &mut entity.detail else {
        return Err("actor".into());
    };
    *appearance_ref = 1;
    let spatial_snapshot = WorldSpatialEntitiesSnapshot {
        content_generation: CONTENT_GENERATION,
        actor_position: at(0, 0),
        own_identity: OWN,
        entities: vec![entity],
    };
    let spatial = (
        SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
        encode_world_spatial_entities_snapshot(&spatial_snapshot).map_err(|e| format!("{e:?}"))?,
    );
    Ok((snapshot, entity, spatial))
}
