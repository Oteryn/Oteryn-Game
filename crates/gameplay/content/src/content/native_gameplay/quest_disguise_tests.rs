#[test]
fn quest_mummy_disguise_is_an_exact_carried_native_item_without_invented_transform() {
    use super::super::ReferenceItemField as F;
    let world = test_source(1).expect("source world").world_id;
    let baseline = qualify_native_entry_room(world).expect("qualified baseline");
    let mut supplied = input();
    supplied.item_profiles = Some(pinned(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/items/definitions/spell-native-profiles.json"
    ))));
    let artifact =
        compile_native_gameplay(baseline.compiled(), &supplied).expect("qualified native artifact");
    let decoded = decode(&artifact.server_artifact).expect("decoded artifact");
    let record = decoded
        .state
        .item_policy("oteryn:item.tibia.i144", "definition-r1")
        .expect("source-qualified mummy disguise")
        .record();
    assert_eq!(record.production_binding.external_id, "144");
    assert_eq!(
        record.production_binding.identity_namespace,
        "ots/item_server_id"
    );
    assert_eq!(
        record.production_definition.production_key,
        "oteryn:item.tibia.i144"
    );
    assert_eq!(record.production_definition.revision_ref, "definition-r1");
    assert!(record.production_binding_qualification.is_none());
    assert!(record.admission.materializable);
    assert_eq!(
        record.admission.stack_class,
        NativeItemStackClass::NonStackable
    );
    assert_eq!(
        record.admission.legal_destinations,
        [
            NativeItemDestination::CharacterInventory,
            NativeItemDestination::Ground
        ]
    );
    assert!(matches!(&record.semantics.physical, F::Known(physical)
            if physical.weight == F::Known(900)
                && physical.movable == F::Known(true)
                && physical.pickupable == F::Known(true)));
    assert_eq!(record.semantics.use_transform, F::Unknown);
    assert_eq!(record.semantics.trade_restrictions, F::Unknown);
    assert_eq!(record.semantics.charges, F::NotApplicable);
    assert_eq!(record.semantics.temporal, F::NotApplicable);
}
