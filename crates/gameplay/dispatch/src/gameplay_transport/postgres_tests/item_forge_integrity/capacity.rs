//! Complete, otherwise legal conversions must retain the native custody ceilings.
use super::*;

#[derive(Clone, Copy, Debug)]
enum Ceiling {
    Main,
    Nested,
    Tree,
    BeforeConsumption,
}

pub(super) async fn exercise(h: &Harness, admitted: &AdmittedSession) -> TestResult {
    for ceiling in [
        Ceiling::Main,
        Ceiling::Nested,
        Ceiling::Tree,
        Ceiling::BeforeConsumption,
    ] {
        let before = snapshot(h).await?;
        let mut tx = h.pool.begin().await?;
        // Only the initial, legal carried tree is fixture data. Every attempted
        // Forge effect below runs as the runtime role with all guards enabled.
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        let (parent, kind, ordinal, size) = seed(&mut tx, ceiling).await?;
        sqlx::query("SET LOCAL session_replication_role=origin")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let attack = conversion(h, &mut tx, &parent, kind, ordinal, ceiling).await?;
        attack.write(&mut tx, admitted).await?;
        let complete: (i64, i64, i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_forge_operations WHERE applied),(SELECT count(*) FROM game_forge_history),(SELECT count(*) FROM game_item_audit_outbox),(SELECT balance FROM game_character_forge_dust WHERE character_id=encode($1,'hex')::uuid),game_item_loop_tree_size(encode($1,'hex')::uuid)")
            .bind(id(41).as_slice()).fetch_one(&mut *tx).await?;
        let retired = i64::from(matches!(ceiling, Ceiling::BeforeConsumption));
        assert_eq!(
            complete,
            (1, 1, 1, 1000 - attack.dust_spent, size + 1 - retired)
        );
        if matches!(ceiling, Ceiling::BeforeConsumption) {
            let delta: (i64, i64) = sqlx::query_as("SELECT game_forge_resource_delta(p,'oteryn:item.tibia.i37109'),game_forge_resource_delta(p,'oteryn:item.tibia.i37110') FROM game_forge_operations p")
                .fetch_one(&mut *tx).await?;
            assert_eq!(delta, (-50, 1));
            assert_eq!(
                complete.4, 20,
                "the final count fits; only the original lack of a slot rejects this conversion"
            );
        }
        let error = match tx.commit().await {
            Ok(()) => return Err(format!("forged {ceiling:?} overflow committed").into()),
            Err(error) => error,
        };
        let database = error.as_database_error().ok_or("native SQL guard error")?;
        assert_eq!(database.code().as_deref(), Some("23514"));
        assert!(
            database
                .message()
                .contains("Forge resource destination exceeds"),
            "the custody ceiling must reject the complete {ceiling:?} conversion: {database}"
        );
        assert_eq!(
            snapshot(h).await?,
            before,
            "conversion and dust debit roll back"
        );
    }
    Ok(())
}

async fn seed(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ceiling: Ceiling,
) -> TestResult<(String, i16, i64, i64)> {
    let root: String = sqlx::query_scalar("SELECT encode($1,'hex')::uuid::text")
        .bind(id(70).as_slice())
        .fetch_one(&mut **tx)
        .await?;
    let (parent, kind, ordinal, expected) = match ceiling {
        Ceiling::Main => {
            children(tx, &root, 1, 18, false).await?;
            (root, 1, 21, 20)
        }
        Ceiling::BeforeConsumption => {
            children(tx, &root, 1, 17, false).await?;
            let slivers = children(tx, &root, 1, 1, false).await?.remove(0);
            sqlx::query("UPDATE game_item_instances SET definition_production_key='oteryn:item.tibia.i37109',quantity=50 WHERE item_instance_id=$1::uuid")
                .bind(slivers).execute(&mut **tx).await?;
            (root, 1, 21, 20)
        }
        Ceiling::Nested => {
            let bag = children(tx, &root, 1, 1, true).await?.remove(0);
            children(tx, &bag, 2, 20, false).await?;
            (bag, 2, 21, 23)
        }
        Ceiling::Tree => {
            let branch = children(tx, &root, 1, 1, true).await?.remove(0);
            for bag in children(tx, &branch, 2, 20, true).await? {
                children(tx, &bag, 2, 20, false).await?;
            }
            let branch = children(tx, &root, 1, 1, true).await?.remove(0);
            for bag in children(tx, &branch, 2, 4, true).await? {
                children(tx, &bag, 2, 18, false).await?;
            }
            // Four direct entries leave a real main-backpack slot; only the
            // global 500-item ceiling rejects this otherwise valid placement.
            (root, 1, 5, 500)
        }
    };
    let count: i64 = sqlx::query_scalar("SELECT game_item_loop_tree_size(encode($1,'hex')::uuid)")
        .bind(id(41).as_slice())
        .fetch_one(&mut **tx)
        .await?;
    assert_eq!(count, expected);
    let counts: Vec<i64> = sqlx::query_scalar("SELECT count(*) FROM (SELECT parent_item_instance_id FROM game_item_container_entries UNION ALL SELECT parent_item_instance_id FROM game_item_nested_entries) e GROUP BY parent_item_instance_id")
        .fetch_all(&mut **tx).await?;
    assert!(counts.into_iter().all(|count| count <= 20));
    Ok((parent, kind, ordinal, count))
}

async fn children(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    parent: &str,
    kind: i16,
    count: i32,
    bags: bool,
) -> TestResult<Vec<String>> {
    let statement = if kind == 1 {
        "WITH children AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT game_character_uuid_v7(),encode($1,'hex')::uuid,'Item',$4,'definition-r1',1,1,encode($6,'hex')::uuid FROM generate_series(1,$5::integer) RETURNING item_instance_id) INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::uuid,row_number() OVER(ORDER BY item_instance_id)+(SELECT coalesce(max(placement_ordinal),0) FROM game_item_container_entries WHERE parent_item_instance_id=$3::uuid),encode($6,'hex')::uuid FROM children RETURNING item_instance_id::text"
    } else {
        "WITH children AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT game_character_uuid_v7(),encode($1,'hex')::uuid,'Item',$4,'definition-r1',1,1,encode($6,'hex')::uuid FROM generate_series(1,$5::integer) RETURNING item_instance_id) INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,encode($1,'hex')::uuid,$3::uuid,row_number() OVER(ORDER BY item_instance_id)+(SELECT coalesce(max(placement_ordinal),0) FROM game_item_nested_entries WHERE parent_item_instance_id=$3::uuid),encode($6,'hex')::uuid FROM children WHERE encode($2,'hex')::uuid IS NOT NULL RETURNING item_instance_id::text"
    };
    Ok(sqlx::query_scalar(statement)
        .bind(id(42).as_slice())
        .bind(id(41).as_slice())
        .bind(parent)
        .bind(if bags {
            "oteryn:item.tibia.i2854"
        } else {
            "oteryn:item.tibia.i3031"
        })
        .bind(count)
        .bind(id(72).as_slice())
        .fetch_all(&mut **tx)
        .await?)
}

async fn conversion(
    h: &Harness,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    parent: &str,
    kind: i16,
    ordinal: i64,
    ceiling: Ceiling,
) -> TestResult<ForgedOperation> {
    let mut attack = ForgedOperation::prepare(h).await?;
    let (item, bytes): (String, Vec<u8>) = sqlx::query_as(
        "SELECT item::text,uuid_send(item) FROM (SELECT game_character_uuid_v7() AS item) output",
    )
    .fetch_one(&h.pool)
    .await?;
    let core = matches!(ceiling, Ceiling::BeforeConsumption);
    attack.request = request(
        if core {
            Operation::SliversToCores
        } else {
            Operation::DustToSlivers
        },
        201,
    );
    attack.inputs = if core {
        sqlx::query_scalar("SELECT jsonb_build_array(jsonb_build_object('item',i.item_instance_id,'before',to_jsonb(i),'location',jsonb_build_object('kind',1,'parent',e.parent_item_instance_id,'ordinal',e.placement_ordinal::text,'depth',1),'quantity_after',0,'tier_before',0,'tier_after',0)) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.parent_item_instance_id=$1::uuid AND i.definition_production_key='oteryn:item.tibia.i37109' AND i.quantity=50")
            .bind(parent).fetch_one(&mut **tx).await?
    } else {
        json!([])
    };
    if core {
        // Equivalent UUID spelling must not hide the consumed input's parent
        // from the original-room proof; the native custody owner compares UUIDs.
        attack.inputs[0]["location"]["parent"] = json!(format!("{{{parent}}}"));
    }
    attack.outputs = json!([{"item":item,"key":if core { "oteryn:item.tibia.i37110" } else { "oteryn:item.tibia.i37109" },"revision":"definition-r1","count":if core { 1 } else { 3 },
        "location":{"kind":kind,"parent":parent,"ordinal":ordinal.to_string()}}]);
    attack.dust_spent = if core { 0 } else { 60 };
    attack.audit_item = bytes;
    attack.reply = json!({"operation":if core { "slivers_to_cores" } else { "dust_to_slivers" },"succeeded":true,"applied":true,"reason":null,"bonus":0,
        "result":{"first_item":null,"first_tier":null,"second_item":null,"second_tier":null,
            "dust":1000-attack.dust_spent,"dust_limit":1000,"gold_spent":0,"cores_spent":0,
            "slivers_spent":if core { 50 } else { 0 },"slivers_created":if core { 0 } else { 3 },"cores_created":u8::from(core)}});
    let mut proof: Value = serde_json::from_slice(&attack.proof)?;
    proof["operation"] = json!(attack.request.operation as u8);
    proof["reply"] = attack.reply.clone();
    proof["inputs_hash"] = json!(Sha256::digest(serde_json::to_vec(&attack.inputs)?).to_vec());
    proof["outputs_hash"] = json!(Sha256::digest(serde_json::to_vec(&attack.outputs)?).to_vec());
    proof["inputs"] = json!(u8::from(core));
    proof["retired"] = json!(u8::from(core));
    proof["outputs"] = json!(1);
    proof["tier_changes"] = json!(0);
    proof["dust_spent"] = json!(attack.dust_spent);
    attack.proof = serde_json::to_vec(&proof)?;
    Ok(attack)
}

pub(super) async fn write_effects(
    attack: &ForgedOperation,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> TestResult {
    for input in attack.inputs.as_array().ok_or("Forge input array")? {
        sqlx::query("UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=encode($2,'hex')::uuid WHERE item_instance_id=($1->>'item')::uuid")
            .bind(input).bind(id(202).as_slice()).execute(&mut **tx).await?;
        sqlx::query(
            "DELETE FROM game_item_container_entries WHERE item_instance_id=($1->>'item')::uuid",
        )
        .bind(input)
        .execute(&mut **tx)
        .await?;
    }
    let output = &attack.outputs[0];
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,'Item',$1->>'key',$1->>'revision',($1->>'count')::bigint,1,encode($3,'hex')::uuid)")
        .bind(output).bind(id(42).as_slice()).bind(id(202).as_slice()).execute(&mut **tx).await?;
    if output["location"]["kind"] == 1 {
        sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,($1->'location'->>'parent')::uuid,($1->'location'->>'ordinal')::numeric,encode($4,'hex')::uuid)")
            .bind(output).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(202).as_slice()).execute(&mut **tx).await?;
    } else {
        sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,($1->'location'->>'parent')::uuid,($1->'location'->>'ordinal')::numeric,encode($3,'hex')::uuid)")
            .bind(output).bind(id(42).as_slice()).bind(id(202).as_slice()).execute(&mut **tx).await?;
    }
    if attack.dust_spent > 0 {
        sqlx::query("INSERT INTO game_character_forge_dust_entries(entry_id,character_id,previous_entry_id,kind,cause,cause_occurrence_id,transaction_id,amount,lost_amount,balance_before,balance_after,dust_limit_before,dust_limit_after) SELECT encode($1,'hex')::uuid,character_id,last_entry_id,'CONVERT','forge',encode($2,'hex')::uuid,encode($3,'hex')::uuid,60,0,balance,balance-60,dust_limit,dust_limit FROM game_character_forge_dust WHERE character_id=encode($4,'hex')::uuid")
            .bind(id(204).as_slice()).bind(attack.request.occurrence.as_slice()).bind(id(202).as_slice()).bind(id(41).as_slice()).execute(&mut **tx).await?;
        sqlx::query("UPDATE game_character_forge_dust SET balance=balance-60,last_entry_id=encode($1,'hex')::uuid WHERE character_id=encode($2,'hex')::uuid")
            .bind(id(204).as_slice()).bind(id(41).as_slice()).execute(&mut **tx).await?;
    }
    Ok(())
}
