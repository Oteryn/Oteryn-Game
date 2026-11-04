//! WDESTR real PostgreSQL execution, including deferred custody and timer proofs.
use super::house_podium::{current_house, house_fence, placed_podiums};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::item_transfer_postgres_cases::{
    CHARACTER, Harness, TestResult, WORLD, configured_admin, id, runtime,
};
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection};
use std::time::Instant;

fn active_definition(item: u32) -> TestResult<TypedDefinitionRef> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../");
    let index: Value =
        serde_json::from_slice(&std::fs::read(root.join("content/items/index.json"))?)?;
    let key = format!("oteryn:item.tibia.i{item}");
    for shard in index["shards"].as_array().ok_or("shards")? {
        let doc: Value =
            serde_json::from_slice(&std::fs::read(root.join(shard.as_str().ok_or("shard")?))?)?;
        for record in doc["records"].as_array().ok_or("records")? {
            let identity = &record["definition"]["identity"];
            if identity["key"].as_str() == Some(&key) {
                return Ok(TypedDefinitionRef {
                    family: identity["family"].as_str().ok_or("family")?.into(),
                    production_key: key,
                    revision_ref: identity["revision"].as_str().ok_or("revision")?.into(),
                });
            }
        }
    }
    Err("missing active definition".into())
}
fn definition(def: &TypedDefinitionRef) -> Value {
    json!({"family":def.family,"key":def.production_key,"revision":def.revision_ref})
}
fn hex(bytes: &[u8; 16]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
async fn call(h: &Harness, command: u64, input: Value) -> TestResult<Value> {
    let house = current_house()?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let result = sqlx::query_scalar("SELECT game_house_furniture_destroy(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,2,1,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid,$5,$6::text::numeric,$7::jsonb)")
        .bind(house_fence()?.game_session_id.as_bytes().as_slice()).bind(id(CHARACTER).as_slice())
        .bind(id(1).as_slice()).bind(id(WORLD).as_slice()).bind(house.house_key())
        .bind(command.to_string()).bind(input).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(result)
}
async fn role(h: &Harness, role: Option<&str>) -> TestResult {
    sqlx::query("DELETE FROM test_house_grants")
        .execute(&h.pool)
        .await?;
    if let Some(role) = role {
        sqlx::query("INSERT INTO test_house_grants(world_id,house_key,character_id,role) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4)")
            .bind(id(WORLD).as_slice()).bind(current_house()?.house_key()).bind(id(CHARACTER).as_slice()).bind(role).execute(&h.pool).await?;
    }
    Ok(())
}
async fn snapshot(h: &Harness, target: [u8; 16]) -> TestResult<Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('item',to_jsonb(i),'location',(SELECT to_jsonb(l) FROM game_item_house_interior_locations l WHERE l.item_instance_id=i.item_instance_id),'children',(SELECT jsonb_agg(to_jsonb(e) ORDER BY item_instance_id) FROM game_item_nested_entries e WHERE e.parent_item_instance_id=i.item_instance_id)) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(target.as_slice()).fetch_one(&h.pool).await?)
}

#[test]
fn house_furniture_destroy_owner_subowner_acl_replay_roll_and_container_contents() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .name("house-furniture-destroy-pg".into())
        .stack_size(64 << 20)
        .spawn(move || {
            let result: TestResult = (|| runtime()?.block_on(exercise(admin)))();
            result.map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "house furniture destroy PG thread panicked")??;
    Ok(())
}
async fn exercise(admin: String) -> TestResult {
    let started = Instant::now();
    let h = Harness::create(admin, "house_interior_destroy").await?;
    let result = exercise_case(&h, started).await;
    h.cleanup().await?;
    result
}
async fn exercise_case(h: &Harness, started: Instant) -> TestResult {
    let seal = h
        .recovery
        .seal_current()
        .map_err(crate::item_transfer_postgres_cases::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(crate::item_transfer_postgres_cases::debug)?;
    let mut items = Vec::new();
    for item_id in [3264, 4995, 2295, 2296, 4995, 100, 100, 100, 100] {
        items.push(
            h.mint_definition(&authority, active_definition(item_id)?, 1)
                .await?,
        );
    }
    // Reuse the real owned-house, scope assignment, entry and placement fixture.
    placed_podiums(h).await?;
    let house = current_house()?;
    let mut connection = PgConnection::connect(&h.database.url).await?;
    crate::house_test_fixture::install(&mut connection).await?;
    sqlx::query("INSERT INTO test_house_properties VALUES(encode($1,'hex')::uuid,$2,1,false)")
        .bind(id(WORLD).as_slice())
        .bind(house.house_key())
        .execute(&mut connection)
        .await?;
    sqlx::query("INSERT INTO test_house_tiles VALUES(encode($1,'hex')::uuid,$2,'\\x010203')")
        .bind(id(WORLD).as_slice())
        .bind(house.house_key())
        .execute(&mut connection)
        .await?;
    // Historical custody DATA on genuine minted instances, using the established
    // inactive-occupancy fixture idiom. Restore every guard before the tested calls.
    let mut seed = connection.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *seed)
        .await?;
    for (n, item) in items.iter().enumerate() {
        sqlx::query(
            "DELETE FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(item.as_slice())
        .execute(&mut *seed)
        .await?;
        if n == 0 {
            sqlx::query("INSERT INTO game_item_container_entries SELECT encode($1,'hex')::uuid,world_id,character_id,item_instance_id,100,game_character_uuid_v7() FROM game_item_container_slots WHERE character_id=encode($2,'hex')::uuid")
                            .bind(item.as_slice()).bind(id(CHARACTER).as_slice()).execute(&mut *seed).await?;
        } else if n <= 4 {
            sqlx::query("INSERT INTO game_item_house_interior_locations VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,'\\x010203',$4,game_character_uuid_v7())")
                            .bind(item.as_slice()).bind(id(WORLD).as_slice()).bind(house.house_key()).bind(100+n as i64).execute(&mut *seed).await?;
            sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) SELECT item_instance_id,world_id,house_key,encode($2,'hex')::uuid,placed_transaction_id,1 FROM game_item_house_interior_locations WHERE item_instance_id=encode($1,'hex')::uuid")
                            .bind(item.as_slice()).bind(id(CHARACTER).as_slice()).execute(&mut *seed).await?;
            sqlx::query("INSERT INTO game_item_house_placement_transactions SELECT placed_transaction_id,item_instance_id FROM game_item_house_interior_locations WHERE item_instance_id=encode($1,'hex')::uuid")
                            .bind(item.as_slice()).execute(&mut *seed).await?;
        } else {
            let parent = if n <= 6 { items[1] } else { items[4] };
            sqlx::query("INSERT INTO game_item_nested_entries VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,game_character_uuid_v7())")
                            .bind(item.as_slice()).bind(id(WORLD).as_slice()).bind(parent.as_slice()).bind(n as i64).execute(&mut *seed).await?;
        }
    }
    seed.commit().await?;
    assert_eq!(
        sqlx::query_scalar::<_, String>("SHOW session_replication_role")
            .fetch_one(&h.pool)
            .await?,
        "origin"
    );
    let input = |target: usize, id: u32, remains: u32, duration: u32| -> TestResult<Value> {
        Ok(json!({"source":hex(&items[0]),"target":hex(&items[target]),
                        "source_definition":definition(&active_definition(3264)?),"target_definition":definition(&active_definition(id)?),
                        "remains":definition(&active_definition(remains)?),"decay":definition(&active_definition(id)?),
                        "revision":"1","acl":"1","roll":10,"attack":0,"succeeds":true,"duration":duration}))
    };
    role(h, Some("OWNER")).await?;
    let first = input(1, 4995, 4996, 300000)?;
    let reply = call(h, 1, first.clone()).await?;
    assert_eq!(reply["status"], "COMMITTED");
    let retired:(i16,i64)=sqlx::query_as("SELECT lifecycle,quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(items[1].as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(retired, (2, 0));
    let counts:(i64,i64,i64,i64)=sqlx::query_as("SELECT count(*),count(DISTINCT l.placed_transaction_id),count(p.item_instance_id),count(*) FILTER(WHERE l.spatial_position='\\x010203') FROM game_item_house_interior_locations l JOIN game_item_house_reclaim_provenance p ON(p.item_instance_id,p.placement_transaction_id)=(l.item_instance_id,l.placed_transaction_id) WHERE l.item_instance_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::uuid)")
                    .bind(items[5].as_slice()).bind(items[6].as_slice()).bind(reply["remains"].as_str().ok_or("remains")?).fetch_one(&h.pool).await?;
    assert_eq!(counts, (3, 3, 3, 3));
    let timer: i64 = sqlx::query_scalar(
        "SELECT remaining_ms FROM game_item_timed_states WHERE item_instance_id=$1::uuid",
    )
    .bind(reply["remains"].as_str().ok_or("remains")?)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(timer, 300000);
    role(h, None).await?;
    let replay = call(h, 1, first).await?;
    assert_eq!(replay, reply);
    role(h, Some("SUBOWNER")).await?;
    assert_eq!(
        call(h, 2, input(2, 2295, 3145, 120000)?).await?["status"],
        "COMMITTED"
    );
    assert_eq!(
        call(h, 3, input(3, 2296, 3146, 120000)?).await?["status"],
        "COMMITTED"
    );
    let before = snapshot(h, items[4]).await?;
    for (command, access, status) in [(4, Some("GUEST"), "NO_ACCESS"), (5, None, "NO_ACCESS")] {
        role(h, access).await?;
        assert_eq!(
            call(h, command, input(4, 4995, 4996, 300000)?).await?["status"],
            status
        );
        assert_eq!(snapshot(h, items[4]).await?, before);
    }
    role(h, Some("OWNER")).await?;
    for (command, field, value) in [
        (6, "revision", json!("2")),
        (7, "acl", json!("2")),
        (8, "source_definition", definition(&active_definition(100)?)),
    ] {
        let mut stale = input(4, 4995, 4996, 300000)?;
        stale[field] = value;
        assert_eq!(call(h, command, stale).await?["status"], "STALE");
        assert_eq!(snapshot(h, items[4]).await?, before);
    }
    for (command, roll) in [(13, 0), (14, 81)] {
        let mut invalid = input(4, 4995, 4996, 300000)?;
        invalid["roll"] = json!(roll);
        assert_eq!(call(h, command, invalid).await?["status"], "STALE");
        assert_eq!(snapshot(h, items[4]).await?, before);
    }
    for (command, attack, roll) in [(9, 0, 11), (10, 9, 11), (11, 10, 11), (12, 40, 41)] {
        let mut failed = input(4, 4995, 4996, 300000)?;
        failed["attack"] = json!(attack);
        failed["roll"] = json!(roll);
        failed["succeeds"] = json!(false);
        let fail = call(h, command, failed.clone()).await?;
        assert_eq!(fail["status"], "FAILED_ROLL");
        role(h, None).await?;
        let replay = call(h, command, failed).await?;
        assert_eq!(replay["status"], "COMMITTED");
        assert_eq!(replay["outcome"], "FAILED_ROLL");
        assert_eq!(replay["receipt"], fail["receipt"]);
        role(h, Some("OWNER")).await?;
        assert_eq!(snapshot(h, items[4]).await?, before);
    }
    let receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_house_furniture_destroy_receipts")
            .fetch_one(&h.pool)
            .await?;
    let remains:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_timed_states t JOIN game_house_furniture_destroy_receipts r ON t.item_instance_id=(r.remains_item->>'item_instance_id')::uuid WHERE t.remaining_ms=(r.input->>'duration')::bigint AND t.deadline_at IS NOT NULL AND r.input->'decay'->>'key'=r.before_item->>'definition_production_key' AND r.input->'decay'->>'revision'=r.before_item->>'definition_revision_ref'").fetch_one(&h.pool).await?;
    let durations: Vec<i64> =
        sqlx::query_scalar("SELECT remaining_ms FROM game_item_timed_states ORDER BY remaining_ms")
            .fetch_all(&h.pool)
            .await?;
    assert_eq!(receipts, 7);
    assert_eq!(remains, 3);
    assert_eq!(durations, vec![120000, 120000, 300000]);
    assert_eq!(
        h.root
            .expire_house_furniture(&h.node, house_fence()?, house.clone())
            .await
            .map_err(crate::item_transfer_postgres_cases::debug)?,
        0
    );
    // Wait for the actual database deadline, rather than altering immutable evidence
    // or shortening the production duration. This exercises all three real expiries.
    let delay:i64=sqlx::query_scalar("SELECT max(deadline_at)-floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_item_timed_states").fetch_one(&h.pool).await?;
    tokio::time::sleep(std::time::Duration::from_millis(u64::try_from(
        delay.max(0),
    )?))
    .await;
    assert_eq!(
        h.root
            .expire_house_furniture(&h.node, house_fence()?, house.clone())
            .await
            .map_err(crate::item_transfer_postgres_cases::debug)?,
        3
    );
    let expired:i64=sqlx::query_scalar("SELECT count(*) FROM game_house_furniture_destroy_receipts r JOIN game_item_instances i ON i.item_instance_id=(r.remains_item->>'item_instance_id')::uuid JOIN game_item_timed_states t USING(item_instance_id) WHERE i.definition_production_key=r.input->'decay'->>'key' AND i.definition_revision_ref=r.input->'decay'->>'revision' AND i.lifecycle=1 AND i.quantity=1 AND i.state_revision=2 AND t.state_revision=2 AND t.remaining_ms IS NULL AND t.deadline_at IS NULL").fetch_one(&h.pool).await?;
    assert_eq!(expired, 3);
    assert_eq!(
        h.root
            .expire_house_furniture(&h.node, house_fence()?, house.clone())
            .await
            .map_err(crate::item_transfer_postgres_cases::debug)?,
        0
    );
    assert!(started.elapsed().as_millis() >= 300000);
    println!("WDESTR_EXPIRY transformed={expired} repeated_expiry=0");
    println!(
        "WDESTR_POSTGRES receipts={receipts} remains={remains} moved_children=2 distinct_placements=3 provenance=3 timers={durations:?} elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}
