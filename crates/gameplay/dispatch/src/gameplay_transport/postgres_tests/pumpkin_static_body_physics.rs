//! Native static reward-object physics never grants an Item MINT entitlement.
#[test]
fn srv7_pumpkin_static_body_native_physics_never_admits_mint() {
    let world = crate::foundation::WorldId::decode(&super::fixture::id(42)).expect("fixture World");
    let (controller, _) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let active = controller.active().expect("current qualified Native");
    let native = active.native_gameplay().expect("current native rules");
    let policy = native
        .item_policy_for_source_id(12816)
        .expect("source-physical-only static12816 profile must exist");
    let record = policy.record();
    assert_eq!(record.attributes.blocks_movement, Some(true));
    assert_eq!(record.attributes.blocks_projectile, Some(false));
    assert_eq!(record.attributes.has_height, Some(false));
    assert!(
        !record.admission.materializable,
        "a static reward object has no Item MINT authority"
    );
    assert_eq!(
        record.admission.legal_destinations,
        [crate::content::native_gameplay::NativeItemDestination::Ground]
    );
}
