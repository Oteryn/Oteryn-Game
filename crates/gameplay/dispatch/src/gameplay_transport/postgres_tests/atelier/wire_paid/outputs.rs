//! Dismantle keeps one gem receipt while choosing real carried fragment destinations.
use super::*;

async fn full_containers(h: &Harness, fill_nested: bool) -> TestResult {
    custody::clone_carried(h, 210, 250, 1, 8).await?;
    // The six original leaves plus this nested bag and thirteen full fragment
    // stacks fill all twenty native main-backpack slots without a partial stack.
    for offset in 0..13_u8 {
        custody::clone_carried(h, 229, 100 + offset, 100, i64::from(9 + offset)).await?;
    }
    if fill_nested {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        for offset in 0..20_u8 {
            let item = id(130 + offset);
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT encode($1,'hex')::uuid,world_id,definition_family,definition_production_key,definition_revision_ref,100,1,encode($1,'hex')::uuid FROM game_item_instances WHERE item_instance_id=encode($2,'hex')::uuid")
                .bind(item.as_slice()).bind(id(229).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,encode($1,'hex')::uuid)")
                .bind(item.as_slice()).bind(id(42).as_slice()).bind(id(250).as_slice())
                .bind(i64::from(offset + 1)).execute(&mut *tx).await?;
        }
        tx.commit().await?;
    }
    let counts: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_item_nested_entries WHERE parent_item_instance_id=encode($2,'hex')::uuid)")
        .bind(id(210).as_slice()).bind(id(250).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(counts, (20, if fill_nested { 20 } else { 0 }));
    Ok(())
}

pub(super) async fn prepare(h: &Harness, case: Case) -> TestResult {
    match case {
        Case::NestedDismantle => full_containers(h, false).await?,
        Case::FullDismantle => full_containers(h, true).await?,
        Case::SplitDismantle => {
            // A regular gem always yields 2..=10. Display order fills the first
            // stack's one free unit, then puts the rest into the second stack.
            custody::set_quantity(h, 229, 90).await?;
            custody::clone_carried(h, 229, 219, 99, 8).await?;
        }
        _ => {}
    }
    Ok(())
}

async fn reject_historical_output(h: &Harness) -> TestResult {
    let before = scope_footprint(h).await?;
    for statement in [
        "UPDATE game_item_instances i SET quantity=i.quantity+1,last_transaction_id=r.transaction_id FROM game_atelier_item_records r WHERE r.occurrence=encode($1,'hex')::uuid AND i.item_instance_id=(r.outputs->0->>'item')::uuid",
        "UPDATE game_item_nested_entries e SET placement_ordinal=e.placement_ordinal+1 FROM game_atelier_item_records r WHERE r.occurrence=encode($1,'hex')::uuid AND e.item_instance_id=(r.outputs->0->>'item')::uuid",
    ] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
        let error = match sqlx::query(statement).bind(id(180).as_slice()).execute(&mut *tx).await {
            Ok(result) => {
                assert_eq!(result.rows_affected(), 1);
                tx.commit().await.expect_err("a historical dismantle record cannot mint or move another output")
            }
            Err(error) => {
                tx.rollback().await?;
                error
            }
        };
        assert_eq!(error.as_database_error().and_then(|error| error.code()).as_deref(), Some("23514"));
        assert_eq!(scope_footprint(h).await?, before);
    }
    Ok(())
}

pub(super) async fn assert_result(h: &Harness, case: Case, bank_before: i64) -> TestResult {
    if !matches!(case, Case::NestedDismantle | Case::SplitDismantle) {
        return Ok(());
    }
    assert_eq!(footprint(h).await?.2, bank_before, "dismantle has no gold fee");
    let (count, total): (i32, i64) = sqlx::query_as("SELECT jsonb_array_length(r.outputs),sum((l->>'count')::int)::bigint FROM game_atelier_item_records r CROSS JOIN LATERAL jsonb_array_elements(r.outputs) l WHERE r.occurrence=encode($1,'hex')::uuid GROUP BY r.outputs")
        .bind(id(180).as_slice()).fetch_one(&h.pool).await?;
    assert!((2..=10).contains(&total));
    if matches!(case, Case::NestedDismantle) {
        assert_eq!(count, 1);
        let output: (bool, bool, i64, i16, i64) = sqlx::query_as("SELECT r.outputs->0->>'location'='nested',e.parent_item_instance_id=encode($2,'hex')::uuid,i.quantity,i.lifecycle,(SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=r.parent_item_instance_id) FROM game_atelier_item_records r JOIN game_item_instances i ON i.item_instance_id=(r.outputs->0->>'item')::uuid JOIN game_item_nested_entries e USING(item_instance_id) WHERE r.occurrence=encode($1,'hex')::uuid AND i.minted_transaction_id=r.transaction_id AND e.placed_transaction_id=r.transaction_id")
            .bind(id(180).as_slice()).bind(id(250).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(output, (true, true, total, 1, 20));
        reject_historical_output(h).await?;
    } else {
        assert_eq!(count, 2, "both partial stacks must absorb the same dismantle yield");
        let quantities: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid),(SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($2,'hex')::uuid),(SELECT count(*) FROM game_item_instances i JOIN game_atelier_item_records r ON i.minted_transaction_id=r.transaction_id WHERE r.occurrence=encode($3,'hex')::uuid)")
            .bind(id(219).as_slice()).bind(id(229).as_slice()).bind(id(180).as_slice())
            .fetch_one(&h.pool).await?;
        assert_eq!(quantities, (100, 90 + total - 1, 0), "no new stack is minted while the carried receivers have room");
        let proofs: i64 = sqlx::query_scalar("SELECT count(*) FROM game_atelier_item_records r CROSS JOIN LATERAL jsonb_array_elements(r.outputs) l JOIN game_item_instances i ON i.item_instance_id=(l->>'item')::uuid JOIN game_item_container_entries e USING(item_instance_id) WHERE r.occurrence=encode($1,'hex')::uuid AND l->>'location'='main' AND l->'before' IS NOT NULL AND i.quantity=(l->'before'->>'quantity')::bigint+(l->>'count')::bigint AND i.last_transaction_id=r.transaction_id AND e.parent_item_instance_id=encode($2,'hex')::uuid")
            .bind(id(180).as_slice()).bind(id(210).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(proofs, 2);
    }
    Ok(())
}
