use super::*;

fn id(seed: u8) -> [u8; 16] {
    let mut value = [0; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value[15] = seed;
    value
}
fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    }
}
fn request() -> ItemUseRequest {
    ItemUseRequest {
        kind: ItemUseKind::DromeCube {
            wire_binding: [9; 32],
        },
        command_id: 1,
        content_revision: "content-r1".into(),
        ruleset_revision: "ruleset-r1".into(),
        sim_revision: "sim-r1".into(),
        transaction_id: id(3),
        event_id: id(4),
        occurred_at_unix_ms: 1,
        server_build_id: "test-build".into(),
        source_item_instance_id: id(1),
        source_definition: definition(DROME_CUBE_KEY),
        source_quantity_before: 1,
        flask: ItemUseFlaskPlan::New {
            item_instance_id: id(2),
            definition: definition("oteryn:item.tibia.i36723"),
            placement_ordinal: 2,
        },
    }
}

#[test]
fn cube_requires_an_output_and_never_accepts_burn_only_or_missing_output() {
    let mut request = request();
    assert!(request.validate().is_ok());
    assert_eq!(request.kind.code(), DROME_CUBE_CAUSE);
    for invalid in [ItemUseFlaskPlan::None, ItemUseFlaskPlan::Missing] {
        request.flask = invalid;
        assert!(request.validate().is_err());
    }
}

#[test]
fn cube_transform_rejects_self_output_zero_quantity_and_invalid_uuid() {
    let original = request();
    let mut changed = original.clone();
    changed.source_quantity_before = 0;
    assert!(changed.validate().is_err());
    changed = original.clone();
    changed.source_item_instance_id = id(2);
    assert!(changed.validate().is_err());
    changed = original;
    changed.transaction_id = [0; 16];
    assert!(changed.validate().is_err());
}

#[test]
fn cube_request_binding_covers_wire_source_unit_and_output() {
    let original = request();
    let binding = original.binding();
    let mut changed = original.clone();
    changed.kind = ItemUseKind::DromeCube {
        wire_binding: [8; 32],
    };
    assert_ne!(binding, changed.binding());
    changed = original.clone();
    changed.source_quantity_before = 2;
    assert_ne!(binding, changed.binding());
    changed = original.clone();
    changed.flask = ItemUseFlaskPlan::New {
        item_instance_id: id(2),
        definition: definition("oteryn:item.tibia.i36724"),
        placement_ordinal: 2,
    };
    assert_ne!(binding, changed.binding());
    changed = original.clone();
    changed.kind = ItemUseKind::Potion;
    assert_ne!(binding, changed.binding());
    assert_eq!(binding, original.binding());
}
