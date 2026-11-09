use oteryn_gameplay_durability::durability::{
    item_mint::TypedDefinitionRef,
    item_transfer::{ItemDefinitionFacts, ItemStackClass},
    map_item_mint::FlatMapCrateProof,
};

fn crate_facts() -> ItemDefinitionFacts {
    ItemDefinitionFacts {
        definition: TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2471".into(),
            revision_ref: "definition-r1".into(),
        },
        weight: Some(8000),
        stack: ItemStackClass::NonStackable,
        container_capacity: Some(15),
        container_slot_equip_pattern: false,
    }
}
#[test]
fn only_a_flat_empty_native_crate_gets_a_bound_map_source_proof() -> Result<(), String> {
    let key =
        oteryn_world_bundle::bundle::placement_key(-9, 33218, 32854, 1).ok_or("source key")?;
    let item = crate_facts();
    assert!(
        FlatMapCrateProof::qualify(&item, [1; 32], key, &[0, 0, 0, 0]).is_some(),
        "the source hideout's three empty crates need materialization"
    );
    for depths in [vec![], vec![0], vec![0, 0, 1], vec![0; 65]] {
        assert!(FlatMapCrateProof::qualify(&item, [1; 32], key, &depths).is_none());
    }
    let mut other = item.clone();
    other.definition.production_key = "oteryn:item.tibia.i2854".into();
    assert!(FlatMapCrateProof::qualify(&other, [1; 32], key, &[0, 0]).is_none());
    other = item.clone();
    other.container_capacity = Some(16);
    assert!(FlatMapCrateProof::qualify(&other, [1; 32], key, &[0, 0]).is_none());
    Ok(())
}

#[test]
fn map_mint_owner_consumes_the_exact_crate_scope_and_preserves_legacy_refusal() -> Result<(), String>
{
    use oteryn_gameplay_durability::durability::map_item_mint::{
        MapItemMintRefusal, MapItemMintRequest, plan_map_item_mint, plan_map_item_mint_request,
    };
    use oteryn_gameplay_entities::{CommandId, CommandRef, GameSessionId};
    let item = crate_facts();
    let key = oteryn_world_bundle::bundle::placement_key(-9, 33218, 32854, 1).ok_or("key")?;
    let proof = FlatMapCrateProof::qualify(&item, [1; 32], key, &[0, 0, 0, 0])
        .ok_or("empty crate proof")?;
    let session =
        GameSessionId::decode(&[50, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 50])
            .map_err(|e| format!("{e:?}"))?;
    let mut request = MapItemMintRequest {
        flat_crate: Some(proof),
        command: CommandRef::new(session, CommandId::new(7).map_err(|e| format!("{e:?}"))?),
        base_bundle_digest: [1; 32],
        placement_key: key,
        reset_epoch: 0,
        item: item.clone(),
        quantity: 1,
        content_revision: "content-1".into(),
        ruleset_revision: "r1".into(),
        sim_revision: "r1".into(),
    };
    assert_eq!(
        plan_map_item_mint(&item, 1, false),
        Err(MapItemMintRefusal::ItemIsContainer)
    );
    assert_eq!(plan_map_item_mint_request(&request, false), Ok(()));
    assert_eq!(
        oteryn_gameplay_durability::durability::item_loop_move::plan_map_item_loop_mint(
            &request,
            Some(&item)
        ),
        Ok(()),
        "the live loop preflight must admit the same qualified empty source crate"
    );
    use oteryn_gameplay_durability::durability::item_loop_move::{
        LoopMoveRefusal, plan_map_item_loop_mint,
    };
    assert_eq!(
        plan_map_item_loop_mint(&request, None),
        Err(LoopMoveRefusal::NotSupported)
    );
    let mut changed = item.clone();
    changed.weight = Some(1);
    assert_eq!(
        plan_map_item_loop_mint(&request, Some(&changed)),
        Err(LoopMoveRefusal::NotSupported)
    );
    request.flat_crate = None;
    assert_eq!(
        plan_map_item_loop_mint(&request, Some(&item)),
        Err(LoopMoveRefusal::NotSupported)
    );
    assert_eq!(
        plan_map_item_mint_request(&request, false),
        Err(MapItemMintRefusal::ItemIsContainer)
    );
    request.flat_crate = Some(proof);
    request.base_bundle_digest = [2; 32];
    assert_eq!(
        plan_map_item_mint_request(&request, false),
        Err(MapItemMintRefusal::ItemIsContainer)
    );
    assert_eq!(
        plan_map_item_mint_request(&request, true),
        Err(MapItemMintRefusal::AlreadyTaken)
    );
    request.base_bundle_digest = [1; 32];
    request.placement_key += 1;
    assert_eq!(
        plan_map_item_mint_request(&request, false),
        Err(MapItemMintRefusal::ItemIsContainer)
    );
    request.placement_key = key;
    request.quantity = 2;
    assert_eq!(
        plan_map_item_mint_request(&request, false),
        Err(MapItemMintRefusal::QuantityAboveStackMaximum)
    );
    Ok(())
}
