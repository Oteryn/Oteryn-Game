use super::*;

#[test]
fn delta_lights_cover_entered_and_updated_entries_and_views_gate_on_capability_19() {
    let mut change = delta(1, 1, 1);
    change.enter[0] = actor(EntityKind::Creature, 1);
    change.update[0] = actor(EntityKind::Player, 2);
    let lights = lit(&[&change.enter[0], &change.update[0]], BRIGHTEST);
    let both = EntityFields {
        item_handles: true,
        light: true,
    };
    let bytes =
        encode_world_spatial_entities_delta_with_fields(&change, both, &lights).expect("delta");
    assert_eq!(
        decode_world_spatial_entities_delta_with_fields(&bytes, both),
        Ok((change.clone(), lights.clone()))
    );
    assert!(decode_world_spatial_entities_delta_with_item_handles(&bytes).is_err());
    let left = lit(&[&actor(EntityKind::Creature, 3)], BRIGHTEST);
    assert!(encode_world_spatial_entities_delta_with_fields(&change, both, &left).is_err());

    let with = [
        CAPABILITY_WORLD_SPATIAL_ENTITIES, // WORLD_SPATIAL_ENTITIES
        CAPABILITY_ITEM_VIEW_MOVE_V1,      // ITEM_VIEW_MOVE_V1
        CAPABILITY_WORLD_LIGHT_V1,         // WORLD_LIGHT_V1
    ];
    assert_eq!(
        decode_world_spatial_delta_view_lit(&with, 2, &bytes),
        Ok((WorldSpatialDeltaView::Entities(change.clone()), lights))
    );
    assert_eq!(
        decode_world_spatial_delta_view(&with, 2, &bytes),
        Ok(WorldSpatialDeltaView::Entities(change))
    );
    assert!(decode_world_spatial_delta_view(&with[..2], 2, &bytes).is_err());

    let one = snapshot(vec![own()]);
    let own_light = lit(&[&one.entities[0]], BRIGHTEST);
    let bytes = encode_world_spatial_entities_snapshot_with_fields(&one, LIT, &own_light)
        .expect("snapshot");
    let selected = [
        CAPABILITY_WORLD_SPATIAL_ENTITIES, // WORLD_SPATIAL_ENTITIES
        CAPABILITY_WORLD_LIGHT_V1,         // WORLD_LIGHT_V1
    ];
    assert_eq!(
        decode_world_spatial_snapshot_view_lit(&selected, 2, &bytes),
        Ok((WorldSpatialSnapshotView::Entities(one), own_light))
    );
    assert!(decode_world_spatial_snapshot_view(&selected[..1], 2, &bytes).is_err());
    assert_eq!(
        EntityFields::selected(&with),
        EntityFields {
            item_handles: true,
            light: true
        }
    );
}
