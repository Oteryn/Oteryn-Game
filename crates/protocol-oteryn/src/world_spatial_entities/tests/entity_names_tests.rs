use super::*;

#[test]
fn entity_names_negotiated_actor_field_decodes() {
    let mut entry = encode_entity(&own(), false).expect("actor");
    push_message(&mut entry, 13, b"Oteryn Player");
    let mut bytes =
        encode_world_spatial_entities_snapshot(&snapshot(vec![own()])).expect("snapshot");
    // Replace the one entry, retaining the snapshot's header.
    let plain = encode_entity(&own(), false).expect("plain actor");
    bytes.truncate(bytes.len() - plain.len() - 2);
    push_message(&mut bytes, 4, &entry);
    assert!(decode_world_spatial_snapshot_view(&[6, 286], 2, &bytes).is_ok());
    assert!(decode_world_spatial_snapshot_view(&[6], 2, &bytes).is_err());
}

use crate::entity_names::{EntityDisplayName, MAX_ENTITY_DISPLAY_NAME_BYTES};

fn named(mut entity: WorldSpatialEntity, name: &str) -> WorldSpatialEntity {
    entity.display_name = Some(EntityDisplayName::new(name).expect("bounded name"));
    entity
}

#[test]
fn entity_names_snapshot_and_delta_round_trip_and_preserve_legacy_bytes() {
    let entities = vec![
        named(own(), "Fixture Hero"),
        named(actor(EntityKind::Creature, 1), "Rat"),
        named(actor(EntityKind::Npc, 2), "Élodie"),
    ];
    let scene = snapshot(entities.clone());
    let fields = EntityFields::selected(&[6, 286]);
    let bytes =
        encode_world_spatial_entities_snapshot_with_fields(&scene, fields, &EntityLights::new())
            .expect("named snapshot");
    assert_eq!(
        decode_world_spatial_entities_snapshot_with_fields(&bytes, fields),
        Ok((scene.clone(), EntityLights::new()))
    );
    let change = WorldSpatialEntitiesDelta {
        content_generation: scene.content_generation,
        actor_position: scene.actor_position,
        enter: entities[1..].to_vec(),
        update: vec![entities[0]],
        leave: vec![],
    };
    let bytes =
        encode_world_spatial_entities_delta_with_fields(&change, fields, &EntityLights::new())
            .expect("named delta");
    assert_eq!(
        decode_world_spatial_entities_delta_with_fields(&bytes, fields),
        Ok((change, EntityLights::new()))
    );
    assert!(decode_world_spatial_entities_delta(&bytes).is_err());
    let mut plain = scene.clone();
    for entity in &mut plain.entities {
        entity.display_name = None;
    }
    assert_eq!(
        encode_world_spatial_entities_snapshot(&scene),
        encode_world_spatial_entities_snapshot(&plain)
    );
    assert!(
        encode_world_spatial_entities_snapshot_with_fields(&plain, fields, &EntityLights::new())
            .is_err()
    );
}

#[test]
fn entity_names_bounds_utf8_duplicates_and_kinds_fail_closed() {
    let fields = EntityFields::selected(&[6, 286]);
    let max = "é".repeat(MAX_ENTITY_DISPLAY_NAME_BYTES / 2);
    let entity = named(own(), &max);
    let bytes = super::super::encode_entity(&entity, fields, None).expect("max name");
    assert_eq!(
        super::super::decode_entity(&bytes, fields),
        Ok((entity, None))
    );
    assert_eq!(
        EntityDisplayName::new(&(max + "x")),
        Err(WorldSpatialError::LimitExceeded)
    );
    let base = encode_entity(&own(), false).expect("actor");
    for name in [
        vec![b'x'; MAX_ENTITY_DISPLAY_NAME_BYTES + 1],
        vec![0xff],
        vec![],
        b"\n".to_vec(),
    ] {
        let mut bytes = base.clone();
        push_message(&mut bytes, 13, &name);
        assert!(super::super::decode_entity(&bytes, fields).is_err());
    }
    let mut repeated = bytes;
    push_message(&mut repeated, 13, b"Other");
    assert!(super::super::decode_entity(&repeated, fields).is_err());
    let mut object = object(EntityKind::Corpse, 4);
    object.display_name = entity.display_name;
    assert!(super::super::encode_entity(&object, fields, None).is_err());
    let mut bytes = encode_entity(&object, false).expect("legacy object");
    push_message(&mut bytes, 13, b"Invalid object name");
    assert!(super::super::decode_entity(&bytes, fields).is_err());
}
