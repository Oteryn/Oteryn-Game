//! Canary 1ff6943a: unavailable reclaim subjects never authorize an item sink.
use super::*;
use crate::durability::spell_owner_commit::SpellLanePermit;

pub(super) fn case(tag: &'static str, nested: bool, full: bool) -> TestResult {
    if native::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let admin = native::configured_admin().ok_or("PostgreSQL route disappeared")?;
            native::runtime()
                .map_err(|e| e.to_string())?
                .block_on(async {
                    let h = native::Harness::create(admin, tag).await?;
                    let result = check(&h, nested, full).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup
                })
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "House donor fixture thread panicked")?
        .map_err(Into::into)
}

async fn check(h: &native::Harness, nested: bool, full: bool) -> TestResult {
    let seal = h.recovery.seal_current().map_err(native::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(native::debug)?;
    let parent = h
        .mint(
            &authority,
            if nested {
                native::BACKPACK
            } else {
                native::STONE
            },
            1,
        )
        .await?;
    let child = h.mint(&authority, native::STONE, 1).await?;
    let mut connection = PgConnection::connect(&h.database.url).await?;
    let subject = house_test_fixture::seed_inactive_reclaim_subject(
        &mut connection,
        native::id(native::CHARACTER).as_slice(),
    )
    .await?;
    let sessions: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_reconnect_sessions WHERE character_id=encode($1,'hex')::uuid").bind(&subject).fetch_one(&h.pool).await?;
    assert_eq!(sessions, 0, "recipient is genuinely offline");
    let mut tx = h.pool.begin().await?;
    // Initial historical custody DATA only. The actual operation, checkpoint,
    // deliveries, deferred conservation guards and audits use production writers.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(include_str!("../owned_house_fixture.sql"))
        .bind(native::id(native::WORLD).as_slice())
        .bind(HOUSE)
        .bind(native::id(native::CHARACTER).as_slice())
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_house_bed_revisions(world_id,house_key,revision) VALUES(encode($1,'hex')::uuid,$2,1)")
        .bind(native::id(native::WORLD).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
    for (index, item) in [parent, child].into_iter().enumerate() {
        sqlx::query(
            "DELETE FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(item.as_slice())
        .execute(&mut *tx)
        .await?;
        if nested && index == 1 {
            sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
                .bind(item.as_slice()).bind(native::id(native::WORLD).as_slice()).bind(&subject).bind(parent.as_slice()).bind(native::id(91 + index as u8).as_slice()).execute(&mut *tx).await?;
        } else {
            sqlx::query("INSERT INTO game_item_house_interior_locations(item_instance_id,world_id,house_key,spatial_position,stack_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,'\\x01'::bytea,$4,encode($5,'hex')::uuid)")
                .bind(item.as_slice()).bind(native::id(native::WORLD).as_slice()).bind(HOUSE).bind(index as i32 + 1).bind(native::id(91 + index as u8).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1)")
                .bind(item.as_slice()).bind(native::id(native::WORLD).as_slice()).bind(HOUSE).bind(&subject).bind(native::id(91 + index as u8).as_slice()).execute(&mut *tx).await?;
        }
    }
    if full {
        // Inbox exhaustion is the actual u64 ordinal bound, not an invented slot limit.
        sqlx::query("INSERT INTO game_character_inbox_counters(character_id,next_ordinal) VALUES(encode($1,'hex')::uuid,18446744073709551615)").bind(&subject).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_house_dispositions(world_id,house_key,owner_character_id,ownership_revision,state,noticed_at,due_at) SELECT world_id,house_key,owner_character_id,ownership_revision,1,floor(extract(epoch FROM clock_timestamp())*1000)::bigint-86400001,floor(extract(epoch FROM clock_timestamp())*1000)::bigint-1 FROM game_house_ownership WHERE house_key=$1")
        .bind(HOUSE).execute(&mut *tx).await?;
    tx.commit().await?;
    let before = custody(h).await?;
    assert_eq!(before.len(), 2);
    let mut expected_ids = vec![parent.to_vec(), child.to_vec()];
    expected_ids.sort();
    assert_eq!(
        before.iter().map(|r| r.0.clone()).collect::<Vec<_>>(),
        expected_ids
    );
    let scope = native::scope()?;
    let permit = SpellLanePermit::of_fresh_scope(scope).await?;
    // First pass freezes a bounded census; the second attempts delivery.
    h.root
        .drain_house_dispositions(&permit, &authority, &h.node, scope, 1)
        .await
        .map_err(native::debug)?;
    assert_eq!(state(h).await?, 2);
    let result = h
        .root
        .drain_house_dispositions(&permit, &authority, &h.node, scope, 1)
        .await;
    if nested || full {
        assert!(
            matches!(
                result,
                Err(crate::durability::house_disposition::HouseDispositionError::Unavailable(_))
            ),
            "typed pending/abort: {result:?}"
        );
        assert_eq!(state(h).await?, 2, "operation remains pending");
        assert_eq!(
            custody(h).await?,
            before,
            "abort restores every identity, quantity and custody endpoint"
        );
        assert_eq!(h.count("game_house_disposition_steps").await?, 0);
        assert_eq!(h.count("game_item_character_inbox_locations").await?, 0);
    } else {
        result.map_err(native::debug)?;
        assert_eq!(state(h).await?, 3);
        let after = custody(h).await?;
        assert_eq!(
            after.iter().map(|r| r.0.clone()).collect::<Vec<_>>(),
            expected_ids
        );
        assert!(
            after
                .iter()
                .all(|r| r.1 == 1 && r.2 == "inbox" && r.3 == subject)
        );
        assert_eq!(h.count("game_house_disposition_steps").await?, 2);
        assert_eq!(h.count("game_item_house_interior_locations").await?, 0);
    }
    eprintln!(
        "POSTGRES_STARTED {}: exact 2 identities; nested={nested} full={full}",
        if nested { "f30" } else { "f29" }
    );
    Ok(())
}

async fn state(h: &native::Harness) -> TestResult<i16> {
    Ok(
        sqlx::query_scalar("SELECT state FROM game_house_dispositions WHERE house_key=$1")
            .bind(HOUSE)
            .fetch_one(&h.pool)
            .await?,
    )
}

// UNION ALL preserves duplicates: equality checks identity, multiplicity, quantity,
// and exact custody. A count alone would miss one lost ID plus one duplicate.
async fn custody(h: &native::Harness) -> TestResult<Vec<(Vec<u8>, i64, String, Vec<u8>)>> {
    Ok(sqlx::query_as("SELECT uuid_send(i.item_instance_id),i.quantity,l.kind,l.endpoint FROM game_item_instances i JOIN (SELECT item_instance_id,'house'::text kind,convert_to(house_key,'UTF8') endpoint FROM game_item_house_interior_locations UNION ALL SELECT item_instance_id,'nested',uuid_send(parent_item_instance_id) FROM game_item_container_entries UNION ALL SELECT item_instance_id,'inbox',uuid_send(character_id) FROM game_item_character_inbox_locations) l USING(item_instance_id) ORDER BY i.item_instance_id,l.kind,l.endpoint").fetch_all(&h.pool).await?)
}

#[test]
fn f29_offline_subject_receives_exact_items() -> TestResult {
    case("house_interior_f29_offline", false, false)
}

#[test]
fn f29_full_inbox_retains_pending_custody() -> TestResult {
    case("house_interior_f29_full", false, true)
}
