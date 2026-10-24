#![allow(clippy::expect_used)]

use super::*;
use crate::decoder_fuzz_support::codec;
use oteryn_test_support::property::Rng;

fn scene(rng: &mut Rng, fields: EntityFields) -> (WorldSpatialEntitiesSnapshot, EntityLights) {
    let position = ActorPosition {
        x: rng.u64() as i32,
        y: rng.u64() as i32,
        floor: rng.below(16) as i16,
    };
    let own = WorldSpatialEntity {
        kind: EntityKind::Player,
        entity: EntityRef {
            identity: [1; 16],
            generation: rng.u64().max(1),
        },
        position,
        detail: EntityDetail::Actor {
            direction: StepDirection::North,
            appearance_ref: rng.u64() as u32,
            health_percent: rng.below(101) as u8,
        },
    };
    let object = WorldSpatialEntity {
        kind: EntityKind::GroundItem,
        entity: EntityRef {
            identity: [2; 16],
            generation: 0,
        },
        position,
        detail: EntityDetail::Object {
            item_definition_ref: (rng.u64() as u32).max(1),
            quantity: (rng.u64() as u32).max(1),
            item_handle: fields.item_handles.then_some(ItemHandle::MIN),
        },
    };
    let mut lights = EntityLights::new();
    if fields.light && rng.bool() {
        lights.insert(
            own.entity.identity,
            EntityLight {
                level: (rng.below(255) + 1) as u8,
                color: rng.u64() as u8,
            },
        );
    }
    (
        WorldSpatialEntitiesSnapshot {
            content_generation: [0xab; 32],
            actor_position: position,
            own_identity: own.entity.identity,
            entities: vec![own, object],
        },
        lights,
    )
}

fn delta(rng: &mut Rng, fields: EntityFields) -> (WorldSpatialEntitiesDelta, EntityLights) {
    let (snapshot, lights) = scene(rng, fields);
    (
        WorldSpatialEntitiesDelta {
            content_generation: snapshot.content_generation,
            actor_position: snapshot.actor_position,
            enter: vec![snapshot.entities[0]],
            update: vec![snapshot.entities[1]],
            leave: vec![EntityRef {
                identity: [3; 16],
                generation: rng.u64(),
            }],
        },
        lights,
    )
}

#[test]
fn fuzz_entity_snapshot_and_delta_decoders() {
    codec(
        "decode_world_spatial_entities_snapshot",
        |rng| scene(rng, EntityFields::default()).0,
        decode_world_spatial_entities_snapshot,
        |v| encode_world_spatial_entities_snapshot(v).expect("snapshot"),
    );
    codec(
        "decode_world_spatial_entities_delta",
        |rng| delta(rng, EntityFields::default()).0,
        decode_world_spatial_entities_delta,
        |v| encode_world_spatial_entities_delta(v).expect("delta"),
    );
    codec(
        "decode_world_spatial_entities_snapshot_with_item_handles",
        |rng| scene(rng, EntityFields::handles(true)).0,
        decode_world_spatial_entities_snapshot_with_item_handles,
        |v| encode_world_spatial_entities_snapshot_with_item_handles(v).expect("snapshot"),
    );
    codec(
        "decode_world_spatial_entities_delta_with_item_handles",
        |rng| delta(rng, EntityFields::handles(true)).0,
        decode_world_spatial_entities_delta_with_item_handles,
        |v| encode_world_spatial_entities_delta_with_item_handles(v).expect("delta"),
    );
    for item_handles in [false, true] {
        for light in [false, true] {
            let fields = EntityFields {
                item_handles,
                light,
            };
            codec(
                "decode_world_spatial_entities_snapshot_with_fields",
                |rng| scene(rng, fields),
                |b| decode_world_spatial_entities_snapshot_with_fields(b, fields),
                |v| {
                    encode_world_spatial_entities_snapshot_with_fields(&v.0, fields, &v.1)
                        .expect("snapshot")
                },
            );
            codec(
                "decode_world_spatial_entities_delta_with_fields",
                |rng| delta(rng, fields),
                |b| decode_world_spatial_entities_delta_with_fields(b, fields),
                |v| {
                    encode_world_spatial_entities_delta_with_fields(&v.0, fields, &v.1)
                        .expect("delta")
                },
            );
        }
    }
}

#[test]
fn fuzz_selected_spatial_view_decoders() {
    for caps in [
        &[][..],
        &[6][..],
        &[
            4,  // ITEM_VIEW_MOVE_V1
            6,  // WORLD_SPATIAL_ENTITIES
            19, // WORLD_LIGHT_V1
        ][..],
    ] {
        let fields = EntityFields::selected(caps);
        for kind in [1, 2] {
            if kind == 2 && !caps.contains(&6) {
                let snapshot = encode_world_spatial_entities_snapshot(
                    &scene(&mut Rng::new(1), EntityFields::default()).0,
                )
                .expect("snapshot");
                let delta = encode_world_spatial_entities_delta(
                    &delta(&mut Rng::new(1), EntityFields::default()).0,
                )
                .expect("delta");
                assert!(decode_world_spatial_snapshot_view(caps, kind, &snapshot).is_err());
                assert!(decode_world_spatial_snapshot_view_lit(caps, kind, &snapshot).is_err());
                assert!(decode_world_spatial_delta_view(caps, kind, &delta).is_err());
                assert!(decode_world_spatial_delta_view_lit(caps, kind, &delta).is_err());
                continue;
            }
            let snapshot = |rng: &mut Rng| {
                if kind == 1 {
                    WorldSpatialSnapshotView::OwnActor(WorldSpatialObservation {
                        content_generation: [1; 32],
                        actor_position: scene(rng, fields).0.actor_position,
                    })
                } else {
                    WorldSpatialSnapshotView::Entities(scene(rng, fields).0)
                }
            };
            let encode_snapshot = |v: &WorldSpatialSnapshotView| match v {
                WorldSpatialSnapshotView::OwnActor(own) => {
                    crate::world_spatial::encode_world_spatial(own)
                }
                WorldSpatialSnapshotView::Entities(value) => {
                    encode_world_spatial_entities_snapshot_with_fields(
                        value,
                        fields,
                        &EntityLights::new(),
                    )
                    .expect("snapshot")
                }
            };
            codec(
                "decode_world_spatial_snapshot_view",
                snapshot,
                |b| decode_world_spatial_snapshot_view(caps, kind, b),
                encode_snapshot,
            );
            codec(
                "decode_world_spatial_snapshot_view_lit",
                |rng| (snapshot(rng), EntityLights::new()),
                |b| decode_world_spatial_snapshot_view_lit(caps, kind, b),
                |v| match &v.0 {
                    WorldSpatialSnapshotView::OwnActor(_) => encode_snapshot(&v.0),
                    WorldSpatialSnapshotView::Entities(value) => {
                        encode_world_spatial_entities_snapshot_with_fields(value, fields, &v.1)
                            .expect("lit snapshot")
                    }
                },
            );
            let generate_delta = |rng: &mut Rng| {
                if kind == 1 {
                    WorldSpatialDeltaView::OwnActor(WorldSpatialObservation {
                        content_generation: [1; 32],
                        actor_position: scene(rng, fields).0.actor_position,
                    })
                } else {
                    WorldSpatialDeltaView::Entities(delta(rng, fields).0)
                }
            };
            let encode_delta = |v: &WorldSpatialDeltaView| match v {
                WorldSpatialDeltaView::OwnActor(own) => {
                    crate::world_spatial::encode_world_spatial(own)
                }
                WorldSpatialDeltaView::Entities(value) => {
                    encode_world_spatial_entities_delta_with_fields(
                        value,
                        fields,
                        &EntityLights::new(),
                    )
                    .expect("delta")
                }
            };
            codec(
                "decode_world_spatial_delta_view",
                generate_delta,
                |b| decode_world_spatial_delta_view(caps, kind, b),
                encode_delta,
            );
            codec(
                "decode_world_spatial_delta_view_lit",
                |rng| (generate_delta(rng), EntityLights::new()),
                |b| decode_world_spatial_delta_view_lit(caps, kind, b),
                |v| match &v.0 {
                    WorldSpatialDeltaView::OwnActor(_) => encode_delta(&v.0),
                    WorldSpatialDeltaView::Entities(value) => {
                        encode_world_spatial_entities_delta_with_fields(value, fields, &v.1)
                            .expect("lit delta")
                    }
                },
            );
        }
    }
}
