//! Real MINT invalidates the former empty-tile observation on the next Item transaction.
use super::*;
use crate::durability::item_mint::{
    ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};

pub(super) async fn assert_unknown_mint(
    harness: &Harness,
    deps: &Dependencies,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult {
    assert_eq!(read_cell(harness, deps, owner).await?, Some(true));
    let (target, scope) = {
        let runtime = deps.runtime.lock().await;
        (
            crate::durability::native_world_items::SpellGroundTarget::for_native_tile_read(
                &deps.room,
                &runtime,
                cell(3),
            )
            .map_err(debug)?,
            runtime.binding().scope_generation(),
        )
    };
    let request = ItemMintRequest {
        cause: ItemMintCause::for_test(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            scope,
            91,
            1,
            TypedDefinitionRef {
                family: "LootTable".into(),
                production_key: "fixture:loot.loop-monster".into(),
                revision_ref: "definition-r1".into(),
            },
            "fixture:purpose.loop-monster".into(),
            0,
        ),
        item: TypedDefinitionRef {
            family: "Item".into(),
            production_key: "fixture:item.unknown-movement".into(),
            revision_ref: "definition-r1".into(),
        },
        quantity: 1,
        ground: target.placement(b"loop-monster-item-scope".to_vec()),
        content_revision: target.content_revision.clone(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    };
    let mut frozen = harness
        .root
        .freeze_item_mint(owner.character, owner.holder, request)
        .await
        .map_err(debug)?;
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    let minted = harness
        .root
        .commit_item_mint(&permit, owner.character, owner.holder, &mut frozen)
        .await
        .map_err(debug)?;
    assert!(
        matches!(minted, ItemMintOutcome::Committed(_)),
        "actual Item MINT: {minted:?}"
    );
    drop(permit);
    assert_eq!(
        read_cell(harness, deps, owner).await?,
        None,
        "unqualified current Item movement flags refuse; a former empty tile grants nothing"
    );
    Ok(())
}
async fn read_cell(
    harness: &Harness,
    deps: &Dependencies,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<Option<bool>> {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    let runtime = deps.runtime.lock().await;
    let objects = deps.door.lock().await;
    let mut tx = harness.pool.begin().await?;
    let scope = crate::foundation::RuntimeScopeRefV1::channel(
        runtime.binding().world_id(),
        runtime.binding().channel_id(),
    );
    let authority =
        crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
            &mut tx,
            &permit,
            owner.root,
            owner.character,
            owner.holder,
            scope,
            runtime.binding().scope_generation().get(),
        )
        .await
        .map_err(debug)?;
    let mut map = crate::movement::monster_map::CurrentMonsterMap::new(
        &mut tx, &authority, &runtime, &deps.room, &objects, native,
    );
    let result = map.read(&runtime, cell(3)).await;
    assert!(map.validate_transaction(&runtime).await);
    drop(map);
    tx.rollback().await?;
    Ok(result)
}
