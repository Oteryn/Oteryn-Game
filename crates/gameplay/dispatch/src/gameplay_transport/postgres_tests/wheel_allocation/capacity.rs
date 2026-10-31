//! Actual incoming TRANSFER admission and commit share the current Wheel capacity reader.
use super::*;
use crate::durability::item_transfer::{
    ItemTransferDestination, ItemTransferError, ItemTransferOutcome, ItemTransferRefusal,
    ItemTransferRequest,
};
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};

pub(super) async fn incoming_transfer(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    gate: &SpellPremiumCoordinator,
    free: &SpellPremiumCoordinator,
) -> TestResult {
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|_| "current transfer fence")?.ok_or("transfer fence")?.item_fence();
    let native = owner.active_generation.and_then(|generation| generation.native_gameplay())
        .ok_or("native capacity content")?;
    let index = native.item_index().ok_or("item index")?;
    let definition = |key| -> TestResult<_> {
        crate::gameplay_transport::item_look::typed_definition(
            index,
            index.definition_ref("Item", key, "definition-r1").ok_or("item reference")?,
        ).ok_or_else(|| "item definition".into())
    };
    let bag = generation_item_facts(native, &definition("oteryn:item.tibia.i2854")?).map_err(debug)?;
    let spear = generation_item_facts(native, &definition("oteryn:item.tibia.i3277")?).map_err(debug)?;
    let coin = generation_item_facts(native, &definition("oteryn:item.tibia.i3031")?).map_err(debug)?;
    let base = oteryn_gameplay_items::item_capacity::project_global_capacity_centi_oz(60, "sorcerer")
        .map_err(debug)?;
    let bag_weight = u64::from(bag.weight.ok_or("bag weight")?);
    let spear_weight = u64::from(spear.weight.ok_or("spear weight")?);
    let quantity = u32::try_from((base - bag_weight) / spear_weight)?;
    assert!((1..100).contains(&quantity));
    let carried = bag_weight + u64::from(quantity) * spear_weight;
    // Sorcerer core slot15 grants 2oz per point; the level60 budget is exactly ten points.
    let bonus = 10 * 2 * 100;
    assert!(carried <= base && carried + spear_weight > base);
    assert!(carried + spear_weight + u64::from(coin.weight.ok_or("coin weight")?) <= base + bonus);

    // Retain the existing promotion fixture's backpack, and bind its historical content
    // revision to this test's actual activated generation. Only the initial custody is seeded;
    // every capacity decision and accepted transfer below runs the production writer.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("UPDATE game_item_instances SET definition_revision_ref=$2 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(210).as_slice()).bind(&bag.definition.revision_ref).execute(&mut *tx).await?;
    let entries: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
    assert_eq!(entries, 0, "the promotion consumed its exact coin payment");
    for (seed, facts, count) in [(220, &spear, quantity), (223, &spear, 1), (226, &coin, 1)] {
        sqlx::raw_sql(sqlx::AssertSqlSafe(
            crate::durability::item_fee_burn::test_support::item(
                seed, &facts.definition.production_key, &facts.definition.revision_ref, count,
            ),
        )).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(id(210).as_slice())
        .bind(id(220).as_slice()).bind(owner.world_id.as_bytes().as_slice()).bind(id(222).as_slice())
        .execute(&mut *tx).await?;
    for seed in [223, 226] {
        sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric(20,0),$5,$6,'map-1','content-1',$7)")
            .bind(id(seed).as_slice()).bind(owner.world_id.as_bytes().as_slice())
            .bind(owner.channel_id.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string())
            .bind(crate::durability::map_item_mint_audit::map_item_spatial_position(1, 1, -7))
            .bind(id(3).as_slice()).bind([1_u8].as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now()) ON CONFLICT DO NOTHING")
        .execute(&h.pool).await?;
    let paid_weights = NativeItemWeights(native).with_wheel(Some(gate));
    let free_weights = NativeItemWeights(native).with_wheel(Some(free));
    let session = admitted.game_session_id;
    let request = |seed, command, item| -> TestResult<_> {
        Ok(ItemTransferRequest {
            weight_generation_digest: native.source_digest(),
            command: crate::foundation::CommandRef::new(
                session,
                crate::foundation::CommandId::new(command).map_err(debug)?,
            ),
            source_item_instance_id: id(seed),
            destination: ItemTransferDestination::MainBackpack,
            item,
            backpack: Some(bag.clone()),
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
        })
    };
    let incoming = request(223, 900, spear.clone())?;
    assert!(matches!(h.root.freeze_item_transfer(owner.character, owner.holder, fence, incoming.clone(), &paid_weights).await,
        Err(ItemTransferError::Refused(ItemTransferRefusal::Overweight))));
    let mut points = [0; 36];
    points[14] = 10;
    let save = wire::encode_wheel_request(&wire::WheelAllocationRequest {
        action: 1, occurrence: id(188), expected_revision: 0, points, vessels: None,
    }).map_err(debug)?;
    assert_eq!(send(owner, admitted, &save).await?, reply(false, 1, points)?);
    let mut frozen = h.root.freeze_item_transfer(owner.character, owner.holder, fence, incoming, &paid_weights)
        .await.map_err(debug)?;
    let permit = owner.spell_lane_permit().await.ok_or("capacity transfer lane")?;
    assert!(matches!(h.root.commit_item_transfer(&permit, owner.character, owner.holder, fence, &mut frozen, &free_weights).await,
        Err(ItemTransferError::Refused(ItemTransferRefusal::Overweight))),
        "losing Premium between freeze and commit removes the capacity bonus");
    assert!(matches!(h.root.commit_item_transfer(&permit, owner.character, owner.holder, fence, &mut frozen, &paid_weights).await.map_err(debug)?,
        ItemTransferOutcome::Committed(_)));
    assert!(matches!(h.root.commit_item_transfer(&permit, owner.character, owner.holder, fence, &mut frozen, &free_weights).await.map_err(debug)?,
        ItemTransferOutcome::AlreadyCommitted(_)), "receipt replay retains its completed result after Premium loss");
    let next = request(226, 901, coin)?;
    assert!(matches!(h.root.freeze_item_transfer(owner.character, owner.holder, fence, next.clone(), &free_weights).await,
        Err(ItemTransferError::Refused(ItemTransferRefusal::Overweight))));
    let mut frozen = h.root.freeze_item_transfer(owner.character, owner.holder, fence, next, &paid_weights)
        .await.map_err(debug)?;
    assert!(matches!(h.root.commit_item_transfer(&permit, owner.character, owner.holder, fence, &mut frozen, &paid_weights).await.map_err(debug)?,
        ItemTransferOutcome::Committed(_)));
    let stored_quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(220).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(stored_quantity, i64::from(quantity + 1), "exactly one spear was merged");
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_transfer_receipts WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(receipts, 2);
    Ok(())
}
