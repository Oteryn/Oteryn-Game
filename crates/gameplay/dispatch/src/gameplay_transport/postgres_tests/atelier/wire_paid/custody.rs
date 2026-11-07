//! Real carried/stash fixtures and committed read-back for the existing wire cases.
use super::*;

pub(super) async fn set_quantity(h: &Harness, input: u8, quantity: i64) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    assert_eq!(sqlx::query("UPDATE game_item_instances SET quantity=$2 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(input).as_slice()).bind(quantity).execute(&mut *tx).await?.rows_affected(), 1);
    tx.commit().await?;
    Ok(())
}

pub(super) async fn clone_carried(
    h: &Harness,
    source: u8,
    input: u8,
    quantity: i64,
    ordinal: i64,
) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT encode($1,'hex')::uuid,world_id,definition_family,definition_production_key,definition_revision_ref,$3,1,encode($1,'hex')::uuid FROM game_item_instances WHERE item_instance_id=encode($2,'hex')::uuid")
        .bind(id(input).as_slice()).bind(id(source).as_slice()).bind(quantity).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,encode($1,'hex')::uuid)")
        .bind(id(input).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice())
        .bind(id(210).as_slice()).bind(ordinal).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn deepen_gem(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT encode($1,'hex')::uuid,world_id,definition_family,definition_production_key,definition_revision_ref,1,1,encode($1,'hex')::uuid FROM game_item_instances WHERE item_instance_id=encode($2,'hex')::uuid")
        .bind(id(251).as_slice()).bind(id(210).as_slice()).execute(&mut *tx).await?;
    assert_eq!(sqlx::query("UPDATE game_item_nested_entries SET parent_item_instance_id=encode($2,'hex')::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(220).as_slice()).bind(id(251).as_slice()).execute(&mut *tx).await?.rows_affected(), 1);
    sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($1,'hex')::uuid)")
        .bind(id(251).as_slice()).bind(id(42).as_slice()).bind(id(250).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn remove_inventory(h: &Harness) -> TestResult {
    // Fixture retirement, not a production mutation: leave only the real stash
    // balance and bank. No disconnected live bag pretends to be ground custody.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=coalesce(last_transaction_id,minted_transaction_id) WHERE item_instance_id IN (SELECT item_instance_id FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid UNION SELECT item_instance_id FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid)")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query(
        "DELETE FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn remove_gold(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=coalesce(last_transaction_id,minted_transaction_id) WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(235).as_slice()).execute(&mut *tx).await?;
    sqlx::query(
        "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(235).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE game_account_bank_balances SET balance=0 WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
        .bind(id(40).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn single_nested_stacks(h: &Harness, source: u8, count: u8) -> TestResult {
    // Each native bag holds fifteen single-item stacks. This crosses the old
    // twenty-input limit without exceeding any real bag's twenty-slot capacity.
    for bag in 0..count.div_ceil(15) {
        clone_carried(h, 210, 250 + bag, 1, i64::from(8 + bag)).await?;
    }
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for offset in 0..count {
        let input = 100 + offset;
        let parent = 250 + offset / 15;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT encode($1,'hex')::uuid,world_id,definition_family,definition_production_key,definition_revision_ref,1,1,encode($1,'hex')::uuid FROM game_item_instances WHERE item_instance_id=encode($2,'hex')::uuid")
            .bind(id(input).as_slice()).bind(id(source).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,encode($1,'hex')::uuid)")
            .bind(id(input).as_slice()).bind(id(42).as_slice()).bind(id(parent).as_slice())
            .bind(i64::from(offset % 15 + 1)).execute(&mut *tx).await?;
    }
    sqlx::query("UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=coalesce(last_transaction_id,minted_transaction_id) WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(source).as_slice()).execute(&mut *tx).await?;
    sqlx::query(
        "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(source).as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn prepare(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    case: Case,
    before: &wire::AtelierReply,
) -> TestResult {
    match case {
        Case::NestedReveal
        | Case::DeepNestedReveal
        | Case::WholeNestedReveal
        | Case::NestedRevealOrder => {
            let quantity = if matches!(case, Case::WholeNestedReveal) {
                1
            } else {
                2
            };
            if quantity == 1 {
                set_quantity(h, 220, quantity).await?;
            }
            nest_asset(h, 220, quantity).await?;
            if matches!(case, Case::DeepNestedReveal) {
                deepen_gem(h).await?;
            } else if matches!(case, Case::NestedRevealOrder) {
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica")
                    .execute(&mut *tx)
                    .await?;
                sqlx::query("UPDATE game_item_container_entries SET placement_ordinal=9 WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(250).as_slice()).execute(&mut *tx).await?;
                tx.commit().await?;
                // Native main-container display is descending ordinal. The nested
                // bag comes first, despite the direct leaf's lower ItemInstance id.
                clone_carried(h, 220, 219, 2, 8).await?;
            }
        }
        Case::NestedGrade => nest_asset(h, 229, 100).await?,
        Case::NestedGold | Case::WholeNestedGold => {
            let quantity = if matches!(case, Case::WholeNestedGold) {
                13
            } else {
                100
            };
            if quantity == 13 {
                set_quantity(h, 235, quantity).await?;
            }
            nest_asset(h, 235, quantity).await?;
        }
        Case::ManyNestedCoins | Case::ManyBurnedCoins => single_nested_stacks(h, 235, 75).await?,
        Case::StashGrade
        | Case::SupremeStashGrade
        | Case::MixedStashGrade
        | Case::EmptyStashGrade
        | Case::NoBackpackStashGrade
        | Case::InsufficientFragments
        | Case::InsufficientGold
        | Case::StashedReveal => {
            let input = match case {
                Case::SupremeStashGrade => 232,
                Case::StashedReveal => 220,
                _ => 229,
            };
            let quantity = match case {
                Case::EmptyStashGrade => 5,
                Case::InsufficientFragments | Case::StashedReveal => 2,
                _ => 100,
            };
            if matches!(case, Case::EmptyStashGrade | Case::InsufficientFragments) {
                set_quantity(h, input, i64::from(quantity)).await?;
            }
            stow_asset(owner, admitted, input, quantity).await?;
            if matches!(case, Case::MixedStashGrade | Case::InsufficientFragments) {
                clone_carried(h, input, 219, 2, 8).await?;
            } else if matches!(case, Case::NoBackpackStashGrade) {
                remove_inventory(h).await?;
                assert!(
                    owner
                        .root
                        .read_character_backpack(
                            owner.character,
                            admitted.item_fence.ok_or("item fence")?.character_id,
                        )
                        .await
                        .map_err(debug)?
                        .is_none()
                );
            } else if matches!(case, Case::InsufficientGold) {
                remove_gold(h).await?;
            }
        }
        Case::ThirtyFragments => {
            let gem = before
                .gems
                .iter()
                .find(|gem| gem.domain == 0 && gem.quality == 1)
                .ok_or("regular green gem")?;
            let modifier = *gem.modifiers.first().ok_or("basic modifier")?;
            let mut revision = before.revision;
            for (occurrence, grade) in [(176, 1), (177, 2)] {
                advance_ex_action(owner)?;
                let payload = wire::encode_mutate(&wire::MutateRequest {
                    occurrence: id(occurrence),
                    expected_revision: revision,
                    mutation: wire::Mutation::GradeUp {
                        kind: 0,
                        mod_id: modifier,
                    },
                })
                .map_err(debug)?;
                let reply = wire::decode_reply(
                    &send_atelier(owner, admitted, wire::COMMAND_TYPE_ATELIER_MUTATE, &payload)
                        .await?,
                )
                .map_err(debug)?;
                assert!(!reply.refused, "ordinary grade {grade} preparation");
                assert!(reply.grades.iter().any(|value| value.kind == 0
                    && value.mod_id == modifier
                    && value.grade == grade));
                revision = reply.revision;
            }
            single_nested_stacks(h, 229, 30).await?;
        }
        _ => {}
    }
    Ok(())
}

async fn assert_item(h: &Harness, input: u8, quantity: i64, lifecycle: i16) -> TestResult {
    let actual: (i64, i16) = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(input).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(actual, (quantity, lifecycle), "item {input}");
    Ok(())
}

async fn assert_nested(h: &Harness, input: u8, parent: Option<u8>) -> TestResult {
    let actual: Option<String> = sqlx::query_scalar("SELECT parent_item_instance_id::text FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(input).as_slice()).fetch_optional(&h.pool).await?;
    let expected = if let Some(parent) = parent {
        Some(
            sqlx::query_scalar::<_, String>("SELECT encode($1,'hex')::uuid::text")
                .bind(id(parent).as_slice())
                .fetch_one(&h.pool)
                .await?,
        )
    } else {
        None
    };
    assert_eq!(actual, expected, "nested custody of item {input}");
    Ok(())
}

async fn assert_stash_debit(h: &Harness, key: &str, before: i64, amount: i64) -> TestResult {
    let row: (i64, i64, i64, bool, bool) = sqlx::query_as("SELECT e.amount,e.quantity_before,e.quantity_after,e.previous_entry_id IS NOT NULL,e.created_xact_id=r.created_xact_id AND e.created_xact_id=a.created_xact_id AND e.created_xact_id=f.created_xact_id FROM game_character_stash_balances b JOIN game_character_stash_entries e ON e.entry_id=b.last_entry_id JOIN game_atelier_item_records r ON r.character_id=b.character_id AND r.occurrence=encode($3,'hex')::uuid JOIN game_character_atelier_receipts a ON a.character_id=r.character_id AND a.occurrence=r.occurrence JOIN game_item_fee_burns f ON f.transaction_id=r.transaction_id WHERE b.character_id=encode($1,'hex')::uuid AND b.world_id=encode($2,'hex')::uuid AND b.definition_key=$4 AND b.quantity=e.quantity_after")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(180).as_slice()).bind(key)
        .fetch_one(&h.pool).await?;
    assert_eq!(
        row,
        (amount, before, before - amount, true, true),
        "stash ledger, fee, custody and Atelier receipt share one commit"
    );
    Ok(())
}

async fn reject_foreign_nested_proof(h: &Harness) -> TestResult {
    let line: serde_json::Value = sqlx::query_scalar(
        "SELECT inputs->0 FROM game_atelier_item_records WHERE occurrence=encode($1,'hex')::uuid",
    )
    .bind(id(180).as_slice())
    .fetch_one(&h.pool)
    .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let valid: bool = sqlx::query_scalar("SELECT game_atelier_input_present(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4)")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(210).as_slice()).bind(&line)
        .fetch_one(&mut *tx).await?;
    assert!(
        valid,
        "the remaining nested input is still correctly located"
    );
    for (character, world) in [(99, 42), (41, 99)] {
        let accepted: bool = sqlx::query_scalar("SELECT game_atelier_input_present(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4)")
            .bind(id(character).as_slice()).bind(id(world).as_slice()).bind(id(210).as_slice()).bind(&line)
            .fetch_one(&mut *tx).await?;
        assert!(
            !accepted,
            "nested proof cannot cross Character or World ownership"
        );
    }
    let mut changed_parent = line.clone();
    changed_parent["parent"] = line["item"].clone();
    let accepted: bool = sqlx::query_scalar("SELECT game_atelier_input_present(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4)")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(210).as_slice()).bind(changed_parent)
        .fetch_one(&mut *tx).await?;
    assert!(
        !accepted,
        "an old or invented parent cannot redirect consumption"
    );
    tx.rollback().await?;
    Ok(())
}

async fn reject_historical_stash_debit(h: &Harness) -> TestResult {
    let before = scope_footprint(h).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let error = sqlx::query("UPDATE game_character_stash_balances SET quantity=quantity-1 WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND definition_key='oteryn:item.tibia.i46625'")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await
        .expect_err("a historical Atelier stash entry cannot authorize another debit");
    assert_eq!(
        error
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("23514")
    );
    tx.rollback().await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let error = sqlx::query("INSERT INTO game_character_stash_entries(entry_id,atelier_transaction_id,character_id,world_id,definition_key,definition_revision,amount,quantity_before,quantity_after,previous_entry_id) SELECT encode($3,'hex')::uuid,e.atelier_transaction_id,b.character_id,b.world_id,b.definition_key,b.definition_revision,1,b.quantity,b.quantity-1,b.last_entry_id FROM game_character_stash_balances b JOIN game_character_stash_entries e ON e.entry_id=b.last_entry_id WHERE b.character_id=encode($1,'hex')::uuid AND b.world_id=encode($2,'hex')::uuid AND b.definition_key='oteryn:item.tibia.i46625'")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(190).as_slice())
        .execute(&mut *tx).await.expect_err("a new ledger entry cannot reuse a historical Atelier receipt");
    assert_eq!(
        error
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("23514")
    );
    tx.rollback().await?;
    assert_eq!(scope_footprint(h).await?, before);
    Ok(())
}

async fn reject_historical_coin_proof(h: &Harness) -> TestResult {
    let before = scope_footprint(h).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let line: serde_json::Value = sqlx::query_scalar("SELECT r.inputs->0 FROM game_atelier_fee_coin_records r JOIN game_item_fee_burns f USING(transaction_id) WHERE f.cause_occurrence_id=encode($1,'hex')::uuid")
        .bind(id(180).as_slice()).fetch_one(&mut *tx).await?;
    let valid: bool = sqlx::query_scalar(
        "SELECT game_atelier_coin_source_present(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3)",
    )
    .bind(id(41).as_slice())
    .bind(id(42).as_slice())
    .bind(&line)
    .fetch_one(&mut *tx)
    .await?;
    assert!(valid);
    for (character, world) in [(99, 42), (41, 99)] {
        let accepted: bool = sqlx::query_scalar("SELECT game_atelier_coin_source_present(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3)")
            .bind(id(character).as_slice()).bind(id(world).as_slice()).bind(&line).fetch_one(&mut *tx).await?;
        assert!(
            !accepted,
            "coin proof cannot cross Character or World ownership"
        );
    }
    let error = sqlx::query("INSERT INTO game_atelier_fee_coin_records(transaction_id,character_id,world_id,backpack_item_instance_id,backpack_capacity,backpack_ordinals,inputs,equipment_before) SELECT r.transaction_id,r.character_id,r.world_id,r.backpack_item_instance_id,r.backpack_capacity,r.backpack_ordinals,r.inputs,r.equipment_before FROM game_atelier_fee_coin_records r JOIN game_item_fee_burns f USING(transaction_id) WHERE f.cause_occurrence_id=encode($1,'hex')::uuid")
        .bind(id(180).as_slice()).execute(&mut *tx).await
        .expect_err("reusing a historical fee proof must fail before its duplicate key");
    assert_eq!(
        error
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("23514")
    );
    tx.rollback().await?;
    assert_eq!(scope_footprint(h).await?, before);
    Ok(())
}

pub(super) async fn assert_result(h: &Harness, case: Case, bank_before: i64) -> TestResult {
    match case {
        Case::NestedReveal | Case::NestedRevealOrder | Case::DeepNestedReveal => {
            assert_item(h, 220, 1, 1).await?;
            assert_nested(
                h,
                220,
                Some(if matches!(case, Case::DeepNestedReveal) {
                    251
                } else {
                    250
                }),
            )
            .await?;
            if matches!(case, Case::NestedRevealOrder) {
                assert_item(h, 219, 2, 1).await?;
            }
        }
        Case::WholeNestedReveal => {
            assert_item(h, 220, 0, 2).await?;
            assert_nested(h, 220, None).await?;
        }
        Case::NestedGrade => assert_nested(h, 229, Some(250)).await?,
        Case::NestedGold | Case::WholeNestedGold => {
            let whole = matches!(case, Case::WholeNestedGold);
            assert_item(
                h,
                235,
                if whole { 0 } else { 87 },
                if whole { 2 } else { 1 },
            )
            .await?;
            assert_nested(h, 235, (!whole).then_some(250)).await?;
            assert_eq!(
                footprint(h).await?.2,
                bank_before,
                "carried coins precede bank"
            );
            let change: (i64, i64) = sqlx::query_as("SELECT f.change_gold_units,i.quantity FROM game_item_fee_burns f JOIN game_item_instances i ON i.minted_transaction_id=f.transaction_id JOIN game_item_container_entries e USING(item_instance_id) WHERE f.cause_occurrence_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i3035' AND e.parent_item_instance_id=encode($2,'hex')::uuid")
                .bind(id(180).as_slice()).bind(id(210).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(change, (5_000, 50));
        }
        Case::StashGrade | Case::EmptyStashGrade | Case::NoBackpackStashGrade => {
            let before = if matches!(case, Case::EmptyStashGrade) {
                5
            } else {
                100
            };
            assert_stash_debit(h, "oteryn:item.tibia.i46625", before, 5).await?;
            assert_item(h, 229, 0, 2).await?;
            if matches!(case, Case::NoBackpackStashGrade) {
                assert_eq!(bank_before - footprint(h).await?.2, 2_000_000);
            }
        }
        Case::SupremeStashGrade => {
            assert_stash_debit(h, "oteryn:item.tibia.i46626", 100, 5).await?;
            assert_item(h, 232, 0, 2).await?;
            assert_item(h, 229, 100, 1).await?;
        }
        Case::MixedStashGrade => {
            assert_stash_debit(h, "oteryn:item.tibia.i46625", 100, 3).await?;
            assert_item(h, 219, 0, 2).await?;
            let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(219).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                remaining, 0,
                "carried fragments are consumed before the stash remainder"
            );
        }
        Case::ThirtyFragments => {
            for input in 100..130 {
                assert_item(h, input, 0, 2).await?;
                assert_nested(h, input, None).await?;
            }
            let count: i32 = sqlx::query_scalar("SELECT jsonb_array_length(inputs) FROM game_atelier_item_records WHERE occurrence=encode($1,'hex')::uuid")
                .bind(id(180).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(count, 30);
        }
        Case::ManyNestedCoins | Case::ManyBurnedCoins => {
            let all_burned = matches!(case, Case::ManyBurnedCoins);
            let proof: (i64, i64, i64, i64) = sqlx::query_as("SELECT f.line_count::bigint,jsonb_array_length(r.inputs)::bigint,f.burned_gold_units,f.change_gold_units FROM game_item_fee_burns f JOIN game_atelier_fee_coin_records r USING(transaction_id) WHERE f.cause_occurrence_id=encode($1,'hex')::uuid")
                .bind(id(180).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                proof,
                if all_burned {
                    (75, 75, 750_000, 0)
                } else {
                    (25, 75, 250_000, 0)
                },
                "the complete carried scan must not truncate at twenty sources"
            );
            let quantities: (i64, i64, i64, i64) = sqlx::query_as("SELECT count(*) FILTER(WHERE i.quantity=0 AND i.lifecycle=2),count(*) FILTER(WHERE i.quantity=1 AND i.lifecycle=1),coalesce(sum(i.quantity),0)::bigint,count(n.item_instance_id) FROM game_atelier_fee_coin_records r JOIN game_item_fee_burns f USING(transaction_id) CROSS JOIN LATERAL jsonb_array_elements(r.inputs) l JOIN game_item_instances i ON i.item_instance_id=(l->>'item')::uuid LEFT JOIN game_item_nested_entries n ON n.item_instance_id=i.item_instance_id WHERE f.cause_occurrence_id=encode($1,'hex')::uuid")
                .bind(id(180).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                quantities,
                if all_burned {
                    (75, 0, 0, 0)
                } else {
                    (25, 50, 50, 50)
                }
            );
            assert_eq!(
                bank_before - footprint(h).await?.2,
                if all_burned { 1_250_000 } else { 0 },
                "every selected stack precedes the exact bank remainder"
            );
            if all_burned {
                let bytes: i32 = sqlx::query_scalar("SELECT octet_length(a.envelope) FROM game_item_audit_outbox a JOIN game_item_fee_burns f USING(transaction_id) WHERE f.cause_occurrence_id=encode($1,'hex')::uuid")
                    .bind(id(180).as_slice()).fetch_one(&h.pool).await?;
                assert!(
                    bytes > 25_712 && bytes <= 262_144,
                    "the legal seventy-five-line audit must exercise the raised paid shape bound: {bytes}"
                );
                assert_item(h, 229, 95, 1).await?;
                super::audit::reject_size_forgery(h).await?;
            }
            assert_item(h, 235, 0, 2).await?;
            for bag in 250..255 {
                let (quantity, lifecycle, revision): (i64, i16, String) = sqlx::query_as("SELECT quantity,lifecycle,state_revision::text FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(bag).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!((quantity, lifecycle, revision.as_str()), (1, 1, "1"));
            }
        }
        _ => {}
    }
    if matches!(
        case,
        Case::NestedReveal
            | Case::DeepNestedReveal
            | Case::WholeNestedReveal
            | Case::NestedRevealOrder
            | Case::NestedGrade
            | Case::NestedGold
            | Case::WholeNestedGold
            | Case::ThirtyFragments
    ) {
        let (quantity, lifecycle, revision): (i64, i16, String) = sqlx::query_as("SELECT quantity,lifecycle,state_revision::text FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(250).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            (quantity, lifecycle, revision.as_str()),
            (1, 1, "1"),
            "the containing bag is not burned or rewritten with its child"
        );
    }
    match case {
        Case::NestedReveal => reject_foreign_nested_proof(h).await?,
        Case::StashGrade => reject_historical_stash_debit(h).await?,
        Case::NestedGold => reject_historical_coin_proof(h).await?,
        _ => {}
    }
    Ok(())
}
