//! A real bounded440-row carried tree, then actual command7/500gold and native audit decode.
use super::*;
pub(super) async fn seed(h: &Harness) -> TestResult {
    let main:sqlx::types::Uuid=sqlx::query_scalar("SELECT item_instance_id FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    // Controlled initial inventory only. All actual burns/change/receipts below
    // must come from the native NPC command and existing current source owner.
    sqlx::query("DELETE FROM game_item_instances WHERE item_instance_id IN(SELECT item_instance_id FROM game_item_container_entries WHERE parent_item_instance_id=$1)").bind(main).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM game_item_container_entries WHERE parent_item_instance_id=$1")
        .bind(main)
        .execute(&mut *tx)
        .await?;
    let mut bags = Vec::new();
    for n in 0..21 {
        let bag: sqlx::types::Uuid = sqlx::query_scalar("SELECT game_character_uuid_v7()")
            .fetch_one(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)VALUES($1,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i2854','definition-r1',1,1,encode($3,'hex')::uuid)").bind(bag).bind(id(42).as_slice()).bind(id(199).as_slice()).execute(&mut *tx).await?;
        if n < 18 {
            sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id)VALUES($1,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,encode($6,'hex')::uuid)").bind(bag).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(main).bind(i64::from(n+1)).bind(id(199).as_slice()).execute(&mut *tx).await?;
        } else {
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id)VALUES($1,encode($2,'hex')::uuid,$3,$4,encode($5,'hex')::uuid)").bind(bag).bind(id(42).as_slice()).bind(bags[0]).bind(i64::from(n)).bind(id(199).as_slice()).execute(&mut *tx).await?;
        }
        bags.push(bag);
    }
    let mut coins = 0;
    for (n, bag) in bags.into_iter().enumerate() {
        let count = if n == 0 { 17 } else { 20 };
        for ordinal in 1..=count {
            let item: sqlx::types::Uuid = sqlx::query_scalar("SELECT game_character_uuid_v7()")
                .fetch_one(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)VALUES($1,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i3031','definition-r1',1,1,encode($3,'hex')::uuid)").bind(item).bind(id(42).as_slice()).bind(id(199).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id)VALUES($1,encode($2,'hex')::uuid,$3,$4,encode($5,'hex')::uuid)").bind(item).bind(id(42).as_slice()).bind(bag).bind(i64::from(ordinal)).bind(id(199).as_slice()).execute(&mut *tx).await?;
            coins += 1;
        }
    }
    assert_eq!(coins, 417);
    let item: sqlx::types::Uuid = sqlx::query_scalar("SELECT game_character_uuid_v7()")
        .fetch_one(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)VALUES($1,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i3035','definition-r1',11,1,encode($3,'hex')::uuid)").bind(item).bind(id(42).as_slice()).bind(id(199).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id)VALUES($1,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,19,encode($5,'hex')::uuid)").bind(item).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(main).bind(id(199).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*)FROM game_atelier_carried_coin_sources(encode($1,'hex')::uuid,encode($2,'hex')::uuid)").bind(id(41).as_slice()).bind(id(42).as_slice()).fetch_one(&h.pool).await?,418);
    Ok(())
}
pub(super) async fn verify(h: &Harness) -> TestResult {
    let row:(i64,i64,i64,Vec<u8>)=sqlx::query_as("SELECT f.line_count,f.bank_debit_gold_units,f.change_gold_units,a.envelope FROM game_item_fee_burns f JOIN game_item_audit_outbox a ON a.transaction_id=f.transaction_id WHERE f.cause_kind=473").fetch_one(&h.pool).await?;
    assert_eq!(
        (row.0, row.1, row.2),
        (418, 0, 17),
        "actual native source spends all417nestedgold, oneplatinum, then returns17gold"
    );
    let (_, decoded) =
        crate::durability::item_fee_burn_audit::decode_fee_burn_envelope(&row.3).map_err(debug)?;
    assert_eq!(decoded.lines.len(), 418);
    assert!(row.3.len() <= 262144, "closed source envelope bound");
    let usage = crate::durability::item_fee_burn_audit::fee_burn_usage(&decoded);
    assert!(usage.effect_work_units <= 1620 && usage.touched_item_instances <= 540);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*)FROM game_item_instances WHERE minted_transaction_id=(SELECT transaction_id FROM game_item_fee_burns WHERE cause_kind=473)").fetch_one(&h.pool).await?,1);
    eprintln!(
        "SRV7_HORSE_RECURSIVE_CASH_NATIVE_EXECUTED tree_rows=440 coin_inputs=418 envelope_bytes={} work_units={} fresh_change_stacks=1",
        row.3.len(),
        usage.effect_work_units
    );
    Ok(())
}
