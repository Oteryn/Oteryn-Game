use super::*;

// BAGS-USE-1: two closed bags below the main backpack. Only setup bypasses guards;
// uses and guard probes below use the real fenced writer and runtime role.
const OUTER_BAG: u8 = 170;
const INNER_BAG: u8 = 180;
const OTHER_CHARACTER: u8 = 190;
const OTHER_BACKPACK: u8 = 200;

async fn nest_use_sources(used: &Use) -> TestResult {
    let mut script = item(OUTER_BAG, "oteryn:item.tibia.i2854", 1);
    script.push_str(&item(INNER_BAG, "oteryn:item.tibia.i2854", 1));
    script.push_str(&entry(OUTER_BAG, BACKPACK, 6));
    for (seed, parent, ordinal) in [
        (INNER_BAG, OUTER_BAG, 1),
        (FOOD, INNER_BAG, 1),
        (POTION, INNER_BAG, 2),
        (LAST_POTION, INNER_BAG, 3),
    ] {
        script.push_str(&format!(
            "DELETE FROM game_item_container_entries WHERE item_instance_id = {}; \
             INSERT INTO game_item_nested_entries(item_instance_id, world_id, \
               parent_item_instance_id, placement_ordinal, placed_transaction_id) \
             VALUES ({}, {}, {}, {ordinal}, {});",
            uuid(seed),
            uuid(seed),
            uuid(WORLD),
            uuid(parent),
            uuid(seed + 2),
        ));
    }
    let mut tx = used.harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(script))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn nested_state(used: &Use, seed: u8) -> TestResult<(i64, i16, Option<i64>)> {
    Ok(sqlx::query_as(
        "SELECT i.quantity, i.lifecycle, (SELECT n.placement_ordinal::bigint \
           FROM game_item_nested_entries n WHERE n.item_instance_id = i.item_instance_id) \
           FROM game_item_instances i WHERE i.item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(id(seed).as_slice())
    .fetch_one(&used.harness.pool)
    .await?)
}

#[test]
fn nested_food_two_bags_deep_burns_once_and_replays() -> TestResult {
    run(async |admin| {
        let used = Use::create(admin, "nested_food").await?;
        nest_use_sources(&used).await?;
        let request = food(1, 150, 3);
        assert_eq!(
            used.use_item(&request).await.map_err(debug)?,
            ItemUseOutcome::Written {
                shape: ItemUseShape::Burn
            }
        );
        assert_eq!(
            used.use_item(&request).await.map_err(debug)?,
            ItemUseOutcome::AlreadyCommitted {
                shape: ItemUseShape::Burn
            }
        );
        assert_eq!(nested_state(&used, FOOD).await?, (2, 1, Some(1)));
        assert_eq!(used.records().await?, 1);
        assert_eq!(used.harness.count("game_item_audit_outbox").await?, 1);
        let envelopes = used.envelopes(151).await?;
        let (_, decoded) = decode_item_use_envelope(&envelopes[0]).map_err(debug)?;
        assert_eq!(
            decoded
                .source_entry
                .ok_or("source entry")?
                .parent_item_instance_id,
            id(INNER_BAG)
        );
        // The complete fenced read includes closed bags and rejects a stale session fence.
        let seal = used.harness.recovery.seal_current().map_err(debug)?;
        let authority = used
            .harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let inventory = used
            .harness
            .root
            .read_item_use_inventory(&authority, &used.harness.node, item_fence()?)
            .await
            .map_err(debug)?;
        assert_eq!(inventory.nested.len(), 4);
        assert!(
            inventory
                .nested
                .iter()
                .any(|(parent, entry)| *parent == id(INNER_BAG)
                    && entry.item.item_instance_id == id(FOOD)
                    && entry.item.quantity == 2)
        );
        let mut stale = item_fence()?;
        stale.character_lease_generation += 1;
        assert!(matches!(
            used.harness
                .root
                .read_item_use_inventory(&authority, &used.harness.node, stale)
                .await,
            Err(ItemUseError::AuthorityRejected)
        ));
        drop(authority);
        drop(seal);
        used.cleanup().await
    })
}

#[test]
fn nested_potion_returns_stacked_and_fresh_flasks_only_to_main_backpack() -> TestResult {
    run(async |admin| {
        let used = Use::create(admin, "nested_flask").await?;
        nest_use_sources(&used).await?;
        assert_eq!(
            used.use_item(&potion(1, 150, POTION, 2, flask_stack(FLASK_STACK, 5)))
                .await
                .map_err(debug)?,
            ItemUseOutcome::Written {
                shape: ItemUseShape::FlaskStack
            }
        );
        assert_eq!(nested_state(&used, POTION).await?, (1, 1, Some(2)));
        assert_eq!(used.item(FLASK_STACK).await?, (6, 1, Some(4)));
        assert_eq!(
            used.use_item(&potion(2, 152, LAST_POTION, 1, flask_new(7)))
                .await
                .map_err(debug)?,
            ItemUseOutcome::Written {
                shape: ItemUseShape::FlaskNew
            }
        );
        assert_eq!(nested_state(&used, LAST_POTION).await?, (0, 2, None));
        assert_eq!(used.item(NEW_FLASK).await?, (1, 1, Some(7)));
        let envelopes = used.envelopes(153).await?;
        let (_, decoded) = decode_item_use_envelope(&envelopes[0]).map_err(debug)?;
        assert_eq!(
            decoded
                .source_entry
                .ok_or("source entry")?
                .parent_item_instance_id,
            id(INNER_BAG)
        );
        assert_eq!(
            decoded
                .flask
                .ok_or("flask")?
                .entry
                .ok_or("flask entry")?
                .parent_item_instance_id,
            id(BACKPACK)
        );
        used.cleanup().await
    })
}

#[test]
fn last_nested_unit_retires_and_unproven_nested_removal_is_rejected() -> TestResult {
    run(async |admin| {
        let used = Use::create(admin, "nested_last").await?;
        nest_use_sources(&used).await?;
        assert_eq!(
            used.runtime_sql(format!(
                "DELETE FROM game_item_nested_entries WHERE item_instance_id = {};",
                uuid(LAST_POTION)
            ))
            .await?
            .as_deref(),
            Some("23514")
        );
        assert_eq!(nested_state(&used, LAST_POTION).await?, (1, 1, Some(3)));
        let request = potion(1, 150, LAST_POTION, 1, ItemUseFlaskPlan::None);
        assert_eq!(
            used.use_item(&request).await.map_err(debug)?,
            ItemUseOutcome::Written {
                shape: ItemUseShape::Burn
            }
        );
        assert_eq!(
            used.use_item(&request).await.map_err(debug)?,
            ItemUseOutcome::AlreadyCommitted {
                shape: ItemUseShape::Burn
            }
        );
        assert_eq!(nested_state(&used, LAST_POTION).await?, (0, 2, None));
        used.cleanup().await
    })
}

#[test]
fn foreign_nested_stack_is_refused_and_a_forged_sql_burn_rolls_back() -> TestResult {
    run(async |admin| {
        let used = Use::create(admin, "nested_foreign").await?;
        nest_use_sources(&used).await?;
        // Keep a valid own-use record as a template. The probe supplies all the ordinary
        // quantity/audit evidence and changes only the source custody to another owner.
        used.use_item(&food(1, 150, 3)).await.map_err(debug)?;
        let mut tx = used.harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(item(
            OTHER_BACKPACK,
            "oteryn:item.tibia.i2854",
            1,
        )))
        .execute(&mut *tx)
        .await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "INSERT INTO game_character_roots(character_id, account_id, world_id, lifecycle, \
               character_revision, profile_revision, ruleset_revision, content_revision, starter_template_revision, name) \
             SELECT {}, account_id, world_id, lifecycle, character_revision, profile_revision, \
               ruleset_revision, content_revision, starter_template_revision, 'Other Owner' \
               FROM game_character_roots WHERE character_id = {}; \
             INSERT INTO game_character_name_reservations(name_key,naming_policy_revision,character_id,reserved_at) \
               SELECT name_key,1,character_id,1 FROM game_character_roots WHERE name='Other Owner'; \
             INSERT INTO game_character_operation_receipts(operation_id,command_binding,account_id,character_id,world_id,character_revision,event_id,occurred_at,server_build_id,transaction_id,issuer_decision_id,intent_source_revision,issued_at_source,expires_at_source) \
             SELECT 'd2000000-0000-7000-8000-000000000001'::uuid, \
               decode('02','hex') || uuid_send('d2000000-0000-7000-8000-000000000002'::uuid) || int8send(2) \
               || uuid_send('d2000000-0000-7000-8000-000000000001'::uuid) || uuid_send(account_id) || uuid_send(world_id) \
               || int8send(1) || int8send(120) \
               || int2send(octet_length(profile_revision)::int2) || convert_to(profile_revision,'UTF8') \
               || int2send(octet_length(ruleset_revision)::int2) || convert_to(ruleset_revision,'UTF8') \
               || int2send(octet_length(content_revision)::int2) || convert_to(content_revision,'UTF8') \
               || int2send(octet_length(starter_template_revision)::int2) || convert_to(starter_template_revision,'UTF8') \
               || int2send(octet_length(name)::int2) || convert_to(name,'UTF8'), \
               account_id,character_id,world_id,1,'d2000000-0000-7000-8000-000000000003'::uuid,1,'test/1', \
               'd2000000-0000-7000-8000-000000000004'::uuid,'d2000000-0000-7000-8000-000000000002'::uuid,2,1,120 \
               FROM game_character_roots WHERE name='Other Owner'; \
             UPDATE game_character_bootstrap_intent_floors SET source_revision=2, \
               issuer_decision_id='d2000000-0000-7000-8000-000000000002'::uuid, \
               intent_binding=(SELECT command_binding FROM game_character_operation_receipts WHERE intent_source_revision=2); \
             INSERT INTO game_item_container_slots(character_id, item_instance_id, world_id, placed_transaction_id) \
               VALUES ({}, {}, {}, {}); \
             UPDATE game_item_container_entries SET character_id = {}, parent_item_instance_id = {} WHERE item_instance_id = {};",
            uuid(OTHER_CHARACTER), uuid(CHARACTER), uuid(OTHER_CHARACTER), uuid(OTHER_BACKPACK), uuid(WORLD), uuid(OTHER_BACKPACK + 2),
            uuid(OTHER_CHARACTER), uuid(OTHER_BACKPACK), uuid(OUTER_BAG),
        ))).execute(&mut *tx).await?;
        tx.commit().await?;
        assert!(matches!(
            used.use_item(&food(2, 152, 2)).await,
            Err(ItemUseError::StaleState)
        ));
        assert_eq!(used.runtime_sql(format!(
            "INSERT INTO game_item_use_records SELECT (jsonb_populate_record(NULL::game_item_use_records, \
               to_jsonb(u) || jsonb_build_object('transaction_id', {}, 'event_id', {}, \
                 'command_id', 2, 'source_quantity_before', 2, 'source_quantity_after', 1, \
                 'created_xact_id', pg_current_xact_id()::text))).* FROM game_item_use_records u; \
             UPDATE game_item_instances SET quantity = 1, last_transaction_id = {} WHERE item_instance_id = {}; \
             INSERT INTO game_item_audit_outbox SELECT (jsonb_populate_record(NULL::game_item_audit_outbox, \
               to_jsonb(a) || jsonb_build_object('transaction_id', {}, 'event_id', {}, \
                 'created_xact_id', pg_current_xact_id()::text))).* FROM game_item_audit_outbox a;",
            uuid(152), uuid(153), uuid(152), uuid(FOOD), uuid(152), uuid(153),
        )).await?.as_deref(), Some("23514"));
        assert_eq!(nested_state(&used, FOOD).await?, (2, 1, Some(1)));
        assert_eq!(used.records().await?, 1);
        assert_eq!(used.harness.count("game_item_audit_outbox").await?, 1);
        // A container with no carried root remains forbidden too.
        let mut tx = used.harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id = encode($1,'hex')::uuid").bind(id(OUTER_BAG).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        assert!(matches!(
            used.use_item(&food(2, 152, 2)).await,
            Err(ItemUseError::StaleState)
        ));
        assert_eq!(nested_state(&used, FOOD).await?, (2, 1, Some(1)));
        assert_eq!(used.records().await?, 1);
        used.cleanup().await
    })
}
