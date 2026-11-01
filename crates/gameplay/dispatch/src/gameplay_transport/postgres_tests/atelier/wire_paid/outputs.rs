//! Dismantle keeps one gem receipt while choosing real carried fragment destinations.
use super::*;

async fn nested_clone(
    h: &Harness,
    source: u8,
    item: u8,
    quantity: i64,
    parent: u8,
    ordinal: i64,
) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT encode($1,'hex')::uuid,world_id,definition_family,definition_production_key,definition_revision_ref,$3,1,encode($1,'hex')::uuid FROM game_item_instances WHERE item_instance_id=encode($2,'hex')::uuid")
        .bind(id(item).as_slice()).bind(id(source).as_slice()).bind(quantity).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,encode($1,'hex')::uuid)")
        .bind(id(item).as_slice()).bind(id(42).as_slice()).bind(id(parent).as_slice())
        .bind(ordinal).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn managed_container(h: &Harness, case: Case) -> TestResult {
    custody::clone_carried(h, 210, 250, 1, 8).await?;
    if matches!(case, Case::ManagedMainFallbackDismantle) {
        // Preferred bag 250 and main are full. Main's first ordinary child 251
        // is full but owns empty bag 252; sibling 253 has room. Managed DFS must
        // select 252, whereas the ordinary breadth-first fallback would take 253.
        custody::clone_carried(h, 210, 251, 1, 10).await?;
        custody::clone_carried(h, 210, 253, 1, 9).await?;
        for offset in 0..11_u8 {
            custody::clone_carried(h, 229, 100 + offset, 100, i64::from(11 + offset)).await?;
        }
        for offset in 0..20_u8 {
            nested_clone(h, 229, 130 + offset, 100, 250, i64::from(offset + 1)).await?;
        }
        nested_clone(h, 210, 252, 1, 251, 20).await?;
        for offset in 0..19_u8 {
            nested_clone(h, 229, 150 + offset, 100, 251, i64::from(offset + 1)).await?;
        }
    } else if matches!(case, Case::ManagedNestedDismantle) {
        nested_clone(h, 210, 251, 1, 250, 20).await?;
        nested_clone(h, 229, 252, 90, 251, 1).await?;
        for offset in 0..19_u8 {
            nested_clone(h, 229, 100 + offset, 100, 250, i64::from(offset + 1)).await?;
        }
    } else if matches!(case, Case::ManagedFallbackDismantle) {
        for offset in 0..20_u8 {
            nested_clone(h, 229, 100 + offset, 100, 250, i64::from(offset + 1)).await?;
        }
    } else {
        nested_clone(h, 229, 252, 90, 250, 1).await?;
    }
    // These are valid persisted Character preferences, not a second obtain store.
    // fallback_to_main=false proves that internalPlayerAddItem's ordinary fallback
    // remains available even when the preferred collection subtree cannot fit.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_quick_loot_settings(character_id,unassigned_container_id,unassigned_gold_pouch,fallback_to_main,committed_character_revision,accept_only) SELECT character_id,encode($2,'hex')::uuid,false,$3,character_revision,false FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).bind(id(250).as_slice())
        .bind(matches!(case, Case::ManagedMainFallbackDismantle)).execute(&mut *tx).await?;
    if matches!(case, Case::FilteredManagedDismantle) {
        sqlx::query("INSERT INTO game_character_quick_loot_filters(character_id,accepted,item_production_key) VALUES(encode($1,'hex')::uuid,false,'oteryn:item.tibia.i46625')")
            .bind(id(41).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let main_count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid")
        .bind(id(210).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        main_count,
        if matches!(case, Case::ManagedMainFallbackDismantle) {
            20
        } else {
            7
        }
    );
    Ok(())
}

async fn full_containers(h: &Harness, fill_nested: bool) -> TestResult {
    custody::clone_carried(h, 210, 250, 1, 8).await?;
    // The six original leaves plus this nested bag and thirteen full fragment
    // stacks fill all twenty native main-backpack slots without a partial stack.
    for offset in 0..13_u8 {
        custody::clone_carried(h, 229, 100 + offset, 100, i64::from(9 + offset)).await?;
    }
    if fill_nested {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
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
        Case::ManagedDismantle
        | Case::ManagedNestedDismantle
        | Case::ManagedFallbackDismantle
        | Case::ManagedMainFallbackDismantle
        | Case::FilteredManagedDismantle => {
            managed_container(h, case).await?;
        }
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
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let error = match sqlx::query(statement)
            .bind(id(180).as_slice())
            .execute(&mut *tx)
            .await
        {
            Ok(result) => {
                assert_eq!(result.rows_affected(), 1);
                tx.commit()
                    .await
                    .expect_err("a historical dismantle record cannot mint or move another output")
            }
            Err(error) => {
                tx.rollback().await?;
                error
            }
        };
        assert_eq!(
            error
                .as_database_error()
                .and_then(|error| error.code())
                .as_deref(),
            Some("23514")
        );
        assert_eq!(scope_footprint(h).await?, before);
    }
    Ok(())
}

pub(super) async fn assert_result(h: &Harness, case: Case, bank_before: i64) -> TestResult {
    if matches!(
        case,
        Case::ManagedDismantle
            | Case::ManagedNestedDismantle
            | Case::ManagedFallbackDismantle
            | Case::ManagedMainFallbackDismantle
            | Case::FilteredManagedDismantle
    ) {
        assert_eq!(footprint(h).await?.2, bank_before);
        let (lines, amount, nested, partial, parent): (i32, i64, bool, bool, String) = sqlx::query_as("SELECT jsonb_array_length(outputs),(outputs->0->>'count')::bigint,outputs->0->>'location'='nested',outputs->0->'before' IS NOT NULL,outputs->0->>'parent' FROM game_atelier_item_records WHERE occurrence=encode($1,'hex')::uuid")
            .bind(id(180).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(lines, 1);
        assert!((2..=10).contains(&amount));
        let preferred = matches!(case, Case::ManagedDismantle | Case::ManagedNestedDismantle);
        let main_subtree = matches!(case, Case::ManagedMainFallbackDismantle);
        assert_eq!((nested, partial), (preferred || main_subtree, preferred));
        let expected_parent = if main_subtree {
            252
        } else if matches!(case, Case::ManagedNestedDismantle) {
            251
        } else if preferred {
            250
        } else {
            210
        };
        let expected_parent: String = sqlx::query_scalar("SELECT encode($1,'hex')::uuid::text")
            .bind(id(expected_parent).as_slice())
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(parent, expected_parent);
        if !matches!(
            case,
            Case::ManagedFallbackDismantle | Case::ManagedMainFallbackDismantle
        ) {
            let quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(252).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(quantity, if preferred { 90 + amount } else { 90 });
        }
        let main_count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid")
            .bind(id(210).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            main_count,
            if main_subtree {
                20
            } else if preferred {
                7
            } else {
                8
            }
        );
        return Ok(());
    }
    if !matches!(case, Case::NestedDismantle | Case::SplitDismantle) {
        return Ok(());
    }
    assert_eq!(
        footprint(h).await?.2,
        bank_before,
        "dismantle has no gold fee"
    );
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
        assert_eq!(
            count, 2,
            "both partial stacks must absorb the same dismantle yield"
        );
        let quantities: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid),(SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($2,'hex')::uuid),(SELECT count(*) FROM game_item_instances i JOIN game_atelier_item_records r ON i.minted_transaction_id=r.transaction_id WHERE r.occurrence=encode($3,'hex')::uuid)")
            .bind(id(219).as_slice()).bind(id(229).as_slice()).bind(id(180).as_slice())
            .fetch_one(&h.pool).await?;
        assert_eq!(
            quantities,
            (100, 90 + total - 1, 0),
            "no new stack is minted while the carried receivers have room"
        );
        let proofs: i64 = sqlx::query_scalar("SELECT count(*) FROM game_atelier_item_records r CROSS JOIN LATERAL jsonb_array_elements(r.outputs) l JOIN game_item_instances i ON i.item_instance_id=(l->>'item')::uuid JOIN game_item_container_entries e USING(item_instance_id) WHERE r.occurrence=encode($1,'hex')::uuid AND l->>'location'='main' AND l->'before' IS NOT NULL AND i.quantity=(l->'before'->>'quantity')::bigint+(l->>'count')::bigint AND i.last_transaction_id=r.transaction_id AND e.parent_item_instance_id=encode($2,'hex')::uuid")
            .bind(id(180).as_slice()).bind(id(210).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(proofs, 2);
    }
    Ok(())
}
