//! Bounded external-tree storage qualification. Reach/open views remain the
//! transport compositor's responsibility; this fixture does not activate TEXT.
use super::*;

async fn tree_fixture(h: &Harness, map_mint: bool) -> TestResult<LoopGround> {
    let placement = if map_mint {
        map_ground_fixture(h).await?;
        map_ground()
    } else {
        ground_fixture(h).await?;
        ground()
    };
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    // Explicit location fixture: move the existing eight-level branch below a
    // nonwritable Ground bag. Its leaf is the writable scroll, not its root.
    sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(101).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_nested_entries VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,game_character_uuid_v7())")
        .bind(id(101).as_slice()).bind(native::id(native::WORLD).as_slice()).bind(id(111).as_slice()).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_item_instances SET definition_production_key='oteryn:item.tibia.i2854' WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(111).as_slice()).execute(&mut *tx).await?;
    if map_mint {
        sqlx::query("UPDATE game_map_item_mint_receipts SET definition_production_key='oteryn:item.tibia.i2854' WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(111).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(placement)
}

async fn write_tree(
    h: &Harness,
    item: u8,
    command: u64,
    text: &str,
    placement: Option<LoopGround>,
) -> TestResult<std::result::Result<ItemTextWriteOutcome, ItemTextWriteError>> {
    write_with_custody(h, item, command, text, native::item_fence()?, None, placement).await
}

#[test]
fn write_postgres_ground_tree_metadata_limits_and_history() -> TestResult {
    tree_metadata_limits_and_history(false)
}

#[test]
fn write_postgres_map_ground_tree_metadata_limits_and_history() -> TestResult {
    tree_metadata_limits_and_history(true)
}

fn tree_metadata_limits_and_history(map_mint: bool) -> TestResult {
    let Some(admin) = configured_admin() else { return Ok(()); };
    runtime()?.block_on(async move {
        let tag = if map_mint { "write_map_tree" } else { "write_drop_tree" };
        let h = Harness::create(admin, tag, false).await?;
        let result = async {
            let placement = tree_fixture(&h, map_mint).await?;
            let before: serde_json::Value = sqlx::query_scalar("SELECT jsonb_build_array((SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i),(SELECT jsonb_agg(to_jsonb(e) ORDER BY item_instance_id) FROM game_item_nested_entries e),(SELECT to_jsonb(g) FROM game_item_ground_locations g))")
                .fetch_one(&h.pool).await?;
            // Ground data alone grants no current leaf definition. Depth eight
            // is accepted below; the ninth-level descendant must be refused.
            for (item, ground) in [(108, None), (109, Some(placement.clone())), (111, Some(placement.clone()))] {
                assert!(matches!(write_tree(&h, item, 1, "refused", ground).await?, Err(ItemTextWriteError::NotReachable)));
            }
            let mut wrong = placement.clone();
            wrong.placement_context[0] ^= 1;
            assert!(matches!(write_tree(&h, 108, 1, "refused", Some(wrong)).await?, Err(ItemTextWriteError::NotReachable)));
            // Change one fixture invariant at a time and restore it before
            // the successful write. The root proof never substitutes for the
            // target definition or for every live ancestor in the same World.
            for (item, mutation, restore) in [
                (108, "UPDATE game_item_instances SET definition_revision_ref='other' WHERE item_instance_id=encode($1,'hex')::uuid", "UPDATE game_item_instances SET definition_revision_ref='definition-r1' WHERE item_instance_id=encode($1,'hex')::uuid"),
                (104, "UPDATE game_item_instances SET lifecycle=2,quantity=0,last_transaction_id=minted_transaction_id WHERE item_instance_id=encode($1,'hex')::uuid", "UPDATE game_item_instances SET lifecycle=1,quantity=1,last_transaction_id=NULL WHERE item_instance_id=encode($1,'hex')::uuid"),
                (104, "UPDATE game_item_instances SET world_id=game_character_uuid_v7() WHERE item_instance_id=encode($1,'hex')::uuid", "UPDATE game_item_instances SET world_id=encode($2,'hex')::uuid WHERE item_instance_id=encode($1,'hex')::uuid"),
                (101, "UPDATE game_item_nested_entries SET parent_item_instance_id=encode($2,'hex')::uuid WHERE item_instance_id=encode($1,'hex')::uuid", "UPDATE game_item_nested_entries SET parent_item_instance_id=encode($2,'hex')::uuid WHERE item_instance_id=encode($1,'hex')::uuid"),
            ] {
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                // The cycle fixture binds its new parent, while the World
                // fixture restores the actual World below.
                let argument = if item == 101 { id(109) } else { native::id(native::WORLD) };
                let query = sqlx::query(mutation).bind(id(item).as_slice());
                if mutation.contains("$2") { query.bind(argument.as_slice()).execute(&mut *tx).await?; }
                else { query.execute(&mut *tx).await?; }
                tx.commit().await?;
                assert!(matches!(write_tree(&h, 108, 1, "refused", Some(placement.clone())).await?, Err(ItemTextWriteError::NotReachable)));
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                let argument = if item == 101 { id(111) } else { native::id(native::WORLD) };
                let query = sqlx::query(restore).bind(id(item).as_slice());
                if restore.contains("$2") { query.bind(argument.as_slice()).execute(&mut *tx).await?; }
                else { query.execute(&mut *tx).await?; }
                tx.commit().await?;
            }
            if map_mint {
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query("UPDATE game_map_item_mint_receipts SET quantity=2 WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(111).as_slice()).execute(&mut *tx).await?;
                tx.commit().await?;
                assert!(matches!(write_tree(&h, 108, 1, "refused", Some(placement.clone())).await?, Err(ItemTextWriteError::NotReachable)));
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query("UPDATE game_map_item_mint_receipts SET quantity=1 WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(111).as_slice()).execute(&mut *tx).await?;
                tx.commit().await?;
            }
            for table in ["game_item_texts", "game_item_text_revisions", "game_item_text_write_receipts"] {
                assert_eq!(h.count(table).await?, 0);
            }
            let written = write_tree(&h, 108, 1, "tree\ntext", Some(placement.clone())).await??;
            assert_eq!(written, ItemTextWriteOutcome { text_revision: 1, changed: true, replayed: false });
            let stored: (String, bool, bool, String) = sqlx::query_as("SELECT text,writer_character_id=encode($1,'hex')::uuid,isfinite(written_at),text_revision::text FROM game_item_texts WHERE item_instance_id=encode($2,'hex')::uuid")
                .bind(native::id(native::CHARACTER).as_slice()).bind(id(108).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(stored, ("tree\ntext".into(), true, true, "1".into()));
            let unchanged = write_tree(&h, 108, 2, "tree\ntext", Some(placement.clone())).await??;
            assert_eq!(unchanged, ItemTextWriteOutcome { text_revision: 1, changed: false, replayed: false });
            let cleared = write_tree(&h, 108, 3, "", Some(placement.clone())).await??;
            assert_eq!(cleared, ItemTextWriteOutcome { text_revision: 2, changed: true, replayed: false });
            assert_eq!(h.count("game_item_texts").await?, 0);
            let revision: String = sqlx::query_scalar("SELECT last_revision::text FROM game_item_text_revisions WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(108).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(revision, "2");
            let after: serde_json::Value = sqlx::query_scalar("SELECT jsonb_build_array((SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i),(SELECT jsonb_agg(to_jsonb(e) ORDER BY item_instance_id) FROM game_item_nested_entries e),(SELECT to_jsonb(g) FROM game_item_ground_locations g))")
                .fetch_one(&h.pool).await?;
            assert_eq!(before, after);
            // Lose the branch, then resolve an earlier committed request from
            // history without reopening its external view or its Ground tuple.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            sqlx::query("DELETE FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(101).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
            let replay = write_tree(&h, 108, 1, "tree\ntext", None).await??;
            assert_eq!(replay, ItemTextWriteOutcome { replayed: true, ..written });
            let seal = h.recovery.seal_current().map_err(native::debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(native::debug)?;
            let binding = carried_request(108, 1, "tree\ntext")?.freeze_receipt_binding(native::item_fence()?.character_id);
            assert_eq!(h.root.reconcile_item_text_rewrite(&authority, &binding).await?, Some(replay));
            assert!(matches!(write_tree(&h, 108, 1, "different", None).await?, Err(ItemTextWriteError::ConflictingCommand)));
            assert!(matches!(write_tree(&h, 108, 4, "refused", Some(placement)).await?, Err(ItemTextWriteError::NotReachable)));
            assert_eq!(h.count("game_item_text_write_receipts").await?, 3);
            assert_eq!(h.root_revision().await?, "1");
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        h.cleanup().await?;
        if result.is_ok() { eprintln!("WRITE-NATIVE-PG Ground tree depth/leaf/provenance, metadata, tombstone and history PASS"); }
        result
    })
}

#[test]
fn write_postgres_ground_tree_revalidates_ancestry_after_owner_lock() -> TestResult {
    tree_revalidates_ancestry_after_lock(true)
}

#[test]
fn write_postgres_ground_tree_revalidates_ancestry_after_item_lock() -> TestResult {
    tree_revalidates_ancestry_after_lock(false)
}

fn tree_revalidates_ancestry_after_lock(owner: bool) -> TestResult {
    let Some(admin) = configured_admin() else { return Ok(()); };
    runtime()?.block_on(async move {
        let tag = if owner { "write_tree_owner_wait" } else { "write_tree_item_wait" };
        let h = Harness::create(admin, tag, false).await?;
        let result = async {
            let placement = tree_fixture(&h, false).await?;
            let mut blocker = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *blocker).await?;
            let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()").fetch_one(&mut *blocker).await?;
            if owner {
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'),33))")
                    .bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHANNEL).as_slice()).execute(&mut *blocker).await?;
            } else {
                sqlx::query("SELECT 1 FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
                    .bind(id(104).as_slice()).execute(&mut *blocker).await?;
            }
            let writing = write_tree(&h, 108, 1, "must not commit", Some(placement));
            tokio::pin!(writing);
            let waiting = async {
                tokio::time::timeout(std::time::Duration::from_secs(1), async {
                    loop {
                        let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)))")
                            .bind(pid).fetch_one(&h.pool).await?;
                        if blocked { return Ok::<(), sqlx::Error>(()); }
                        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                    }
                }).await??;
                Ok::<(), Box<dyn std::error::Error>>(())
            };
            tokio::pin!(waiting);
            use std::future::Future;
            use std::task::Poll;
            std::future::poll_fn(|context| {
                if let Poll::Ready(early) = writing.as_mut().poll(context) {
                    return Poll::Ready(Err(format!("writer completed before the held tree lock: {early:?}").into()));
                }
                waiting.as_mut().poll(context)
            }).await?;
            // Privileged fixture drift while all ancestor ItemInstances are
            // locked: proving the root again alone must not admit a lost path.
            sqlx::query("DELETE FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(101).as_slice()).execute(&mut *blocker).await?;
            blocker.commit().await?;
            assert!(matches!(writing.await?, Err(ItemTextWriteError::NotReachable)));
            for table in ["game_item_texts", "game_item_text_revisions", "game_item_text_write_receipts"] {
                assert_eq!(h.count(table).await?, 0);
            }
            assert_eq!(h.root_revision().await?, "1");
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        h.cleanup().await?;
        if result.is_ok() { eprintln!("WRITE-NATIVE-PG Ground tree post-lock ancestry refusal and rollback PASS"); }
        result
    })
}
