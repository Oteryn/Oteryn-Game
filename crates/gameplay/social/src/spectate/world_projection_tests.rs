#![allow(clippy::expect_used)]
use super::spectate_test_support::{facts, grant, limits, observer};
use super::*;
use oteryn_protocol_oteryn::{
    spectate::SpectateMessage,
    world_map::{MapViewHeader, WorldMapViewSnapshot, encode_world_map_snapshot},
    world_spatial::{ActorPosition, StepDirection},
    world_spatial_entities::{
        EntityDetail, EntityFields, EntityKind, EntityLight, EntityLights, EntityRef,
        WorldSpatialEntitiesSnapshot, WorldSpatialEntity,
        encode_world_spatial_entities_snapshot_with_fields,
    },
};

#[test]
fn spectate_native_map_and_spatial_payloads_preserve_presentation_fields() {
    let current = facts(1, 1, 1);
    let mut owner = SpectateRuntime::from(limits(10, 40));
    let (revision, code) = grant(&mut owner, current);
    assert_eq!(
        owner.apply(Request::Admit {
            facts: current,
            observer: observer(1),
            code: code.to_vec(),
            grant_revision: revision
        }),
        Reply::Admitted
    );
    let position = ActorPosition {
        x: 100,
        y: 100,
        floor: 0,
    };
    let own_identity = *current.authority.commit().character_id().as_bytes();
    let spatial = WorldSpatialEntitiesSnapshot {
        content_generation: [1; 32],
        actor_position: position,
        own_identity,
        entities: vec![WorldSpatialEntity {
            kind: EntityKind::Player,
            entity: EntityRef {
                identity: own_identity,
                generation: 1,
            },
            position,
            detail: EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 17,
                health_percent: 42,
            },
        }],
    };
    let fields = EntityFields::selected(&[6, 19]);
    let lights = EntityLights::from([(own_identity, EntityLight { level: 7, color: 9 })]);
    let spatial_bytes =
        encode_world_spatial_entities_snapshot_with_fields(&spatial, fields, &lights)
            .expect("native spatial");
    let map = WorldMapViewSnapshot {
        header: MapViewHeader {
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 1,
            origin: position,
            first_visible_floor: 0,
        },
        tiles: vec![],
    };
    let map_bytes = encode_world_map_snapshot(&map).expect("native map");
    owner.apply(Request::Publish {
        facts: current,
        projection: WorldProjection::SpatialSnapshot {
            value: spatial,
            fields,
            lights,
        },
    });
    owner.apply(Request::Publish {
        facts: current,
        projection: WorldProjection::MapSnapshot(map),
    });
    assert_eq!(
        owner.apply(Request::Drain(observer(1))),
        Reply::Frames(vec![
            SpectateMessage::Frame {
                domain: 1,
                payload_type: 2,
                snapshot: true,
                payload: spatial_bytes
            },
            SpectateMessage::Frame {
                domain: 17,
                payload_type: 1,
                snapshot: true,
                payload: map_bytes
            },
        ])
    );
}
