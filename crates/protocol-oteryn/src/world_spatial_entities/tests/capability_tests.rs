use super::*;

#[test]
fn delta_lights_cover_entered_and_updated_entries_and_views_gate_on_capability_19() {
    let mut change = delta(1, 1, 1);
    change.enter[0] = actor(EntityKind::Creature, 1);
    change.update[0] = actor(EntityKind::Player, 2);
    let lights = lit(&[&change.enter[0], &change.update[0]], BRIGHTEST);
    let both = EntityFields {
        actor_motion: false,
        display_names: false,
        inventory_capacity: false,
        item_handles: true,
        light: true,
        party_icons: false,
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
            actor_motion: false,
            display_names: false,
            inventory_capacity: false,
            item_handles: true,
            light: true,
            party_icons: false,
        }
    );
}

#[test]
fn capacity_sidecar_is_refused_unless_capability_508_was_selected() -> Result<(), WorldSpatialError>
{
    let header = delta(0, 0, 0);
    let capacity = crate::inventory_capacity::InventoryCapacity {
        capacity: 60_000,
        free: 41_299,
    };
    let bytes = encode_world_spatial_entities_sidecar_delta(
        header.content_generation,
        header.actor_position,
        &DeltaSidecar::InventoryCapacity(capacity),
    )?;
    let selected = EntityFields::selected(&[
        CAPABILITY_WORLD_SPATIAL_ENTITIES,
        crate::inventory_capacity::CAPABILITY_INVENTORY_CAPACITY_V1,
    ]);
    assert!(selected.inventory_capacity);
    assert_eq!(
        decode_world_spatial_entities_delta_with_sidecar(&bytes, selected)?,
        (
            header.clone(),
            EntityLights::new(),
            Some(DeltaSidecar::InventoryCapacity(capacity))
        )
    );
    // A session without 508 keeps the old decoder: field 7 is unknown and fails closed.
    let legacy = EntityFields::selected(&[CAPABILITY_WORLD_SPATIAL_ENTITIES]);
    assert!(!legacy.inventory_capacity);
    assert!(decode_world_spatial_entities_delta_with_sidecar(&bytes, legacy).is_err());
    assert!(decode_world_spatial_entities_delta_with_icons(&bytes, selected).is_err());
    // Its ordinary deltas decode exactly as before, with no sidecar.
    let plain = encode_world_spatial_entities_delta(&header)?;
    assert_eq!(
        decode_world_spatial_entities_delta_with_sidecar(&plain, legacy)?,
        (header.clone(), EntityLights::new(), None)
    );
    // The sidecar cannot ride with entity changes or be followed by anything.
    let mut trailing = bytes.clone();
    trailing.extend_from_slice(&[0x3a, 0]);
    assert!(decode_world_spatial_entities_delta_with_sidecar(&trailing, selected).is_err());
    let changed = delta(1, 0, 0);
    let mut mixed = encode_world_spatial_entities_delta(&changed)?;
    mixed.extend_from_slice(&bytes[plain.len()..]);
    assert!(decode_world_spatial_entities_delta_with_sidecar(&mixed, selected).is_err());
    Ok(())
}
