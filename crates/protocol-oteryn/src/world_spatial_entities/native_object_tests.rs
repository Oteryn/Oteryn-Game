//! Native object metadata is optional and never inferred from an opaque Item reference.
use super::*;
fn corpse() -> Result<WorldSpatialEntity, WorldSpatialError> {
    Ok(WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Corpse,
        entity: EntityRef {
            identity: [9; 16],
            generation: 0,
        },
        position: ActorPosition {
            x: 33562,
            y: 32760,
            floor: -15,
        },
        detail: EntityDetail::ObjectWithAppearance {
            item_definition_ref: 24388,
            quantity: 1,
            item_handle: NonZeroU64::new(2),
            appearance_id: NonZeroU16::new(39529).ok_or(WorldSpatialError::Malformed)?,
        },
    })
}
#[test]
fn srx3_native_object_appearance_roundtrips_exact_resource_under_386()
-> Result<(), WorldSpatialError> {
    let entity = corpse()?;
    let selected = EntityFields::selected(&[4, 6, 386]);
    assert!(selected.item_handles && selected.native_objects);
    let bytes = encode_entity(&entity, selected, None)?;
    let decoded = decode_entity(&bytes, selected)?.0;
    assert_eq!(
        decoded, entity,
        "selected field15 must preserve the actual native object resource"
    );
    Ok(())
}
#[test]
fn srx3_native_object_appearance_raw_field15_requires_selection_and_nonzero_u16()
-> Result<(), WorldSpatialError> {
    let mut entity = corpse()?;
    entity.detail = EntityDetail::Object {
        item_definition_ref: 24388,
        quantity: 1,
        item_handle: NonZeroU64::new(2),
    };
    let fields = EntityFields::selected(&[4, 6]);
    let legacy = encode_entity(&entity, fields, None)?;
    let mut native = legacy.clone();
    push_tag(&mut native, 15, 0);
    push_varint(&mut native, 39529);
    let decoded = decode_entity(&native, EntityFields::selected(&[4, 6, 386]));
    assert!(decoded.is_ok(), "actual selected field15 bytes must decode");
    assert!(decode_entity(&native, fields).is_err());
    assert!(
        decode_entity(&native, EntityFields::selected(&[6, 386])).is_err(),
        "the native object extension must not bypass Item-view dependency4"
    );
    let mut duplicate = native.clone();
    push_tag(&mut duplicate, 15, 0);
    push_varint(&mut duplicate, 39529);
    assert!(decode_entity(&duplicate, EntityFields::selected(&[4, 6, 386])).is_err());
    for id in [0, 65536] {
        let mut invalid = legacy.clone();
        push_tag(&mut invalid, 15, 0);
        push_varint(&mut invalid, id);
        assert!(decode_entity(&invalid, EntityFields::selected(&[4, 6, 386])).is_err());
    }
    Ok(())
}
#[test]
fn srx3_native_object_appearance_legacy_bytes_and_actor_shape_stay_strict()
-> Result<(), WorldSpatialError> {
    let native = corpse()?;
    let mut legacy = native;
    legacy.detail = EntityDetail::Object {
        item_definition_ref: 24388,
        quantity: 1,
        item_handle: NonZeroU64::new(2),
    };
    let fields = EntityFields::selected(&[4, 6]);
    assert_eq!(
        encode_entity(&native, fields, None)?,
        encode_entity(&legacy, fields, None)?
    );
    let mut actor = native;
    actor.kind = EntityKind::Creature;
    actor.entity.generation = 1;
    assert!(encode_entity(&actor, EntityFields::selected(&[4, 6, 386]), None).is_err());
    Ok(())
}
