//! Ground storage qualification with explicit location/reach fixtures, not ingress.
use super::*;

fn ground() -> LoopGround {
    LoopGround {
        spatial_position: [
            10_i32.to_be_bytes().as_slice(),
            20_i32.to_be_bytes().as_slice(),
            7_i16.to_be_bytes().as_slice(),
        ]
        .concat(),
        map_revision: "map-r1".into(),
        content_revision: "content-r1".into(),
        placement_context: vec![3; 32],
    }
}

async fn ground_fixture(h: &Harness) -> TestResult {
    fixture(h).await?;
    let ground = ground();
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$5,$6,$7,$8)")
        .bind(id(111).as_slice()).bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHANNEL).as_slice())
        .bind(&ground.spatial_position).bind(super::super::super::super::item_loop_move::ITEM_LOOP_DROP_REF_PREFIX)
        .bind(&ground.map_revision).bind(&ground.content_revision).bind(&ground.placement_context).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn write_ground(
    h: &Harness,
    command: u64,
    text: &str,
    ground: Option<LoopGround>,
) -> TestResult<std::result::Result<ItemTextWriteOutcome, ItemTextWriteError>> {
    write_with_custody(h, 111, command, text, native::item_fence()?, None, ground).await
}

fn map_ground() -> LoopGround {
    let mut placement = ground();
    // The map-item placement key encodes a nonpositive floor.
    placement.spatial_position[8..].copy_from_slice(&(-7_i16).to_be_bytes());
    placement.map_revision = format!("sha256:{}", "03".repeat(32));
    placement
}

async fn map_ground_fixture(h: &Harness) -> TestResult {
    ground_fixture(h).await?;
    let placement = map_ground();
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_ground_locations SET spatial_position=$1, map_revision=$2, corpse_ref=convert_to('map-item-materialization-v1','UTF8'), stack_ordinal=1 WHERE item_instance_id=encode($3,'hex')::uuid")
        .bind(&placement.spatial_position).bind(&placement.map_revision).bind(id(111).as_slice()).execute(&mut *tx).await?;
    // Explicit storage fixture: this does not claim to execute the map-MINT or
    // TEXT dispatch routes. Keep every source-proof field internally consistent.
    sqlx::query("INSERT INTO game_map_item_mint_receipts(game_session_id,command_id,character_id,world_id,channel_id,base_bundle_digest,placement_key,reset_epoch,intent_binding,transaction_id,event_id,item_instance_id,definition_family,definition_production_key,definition_revision_ref,quantity,runtime_scope_ownership_generation,spatial_position,map_revision,content_revision,occurred_at,envelope_sha256,committed_at) SELECT encode($1,'hex')::uuid,1000,encode($2,'hex')::uuid,i.world_id,g.channel_id,$3,$4,1,$5,i.minted_transaction_id,game_character_uuid_v7(),i.item_instance_id,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,g.runtime_scope_ownership_generation,g.spatial_position,g.map_revision,g.content_revision,1,$3,1 FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.item_instance_id=encode($6,'hex')::uuid")
        .bind(native::id(native::SESSION).as_slice()).bind(native::id(native::CHARACTER).as_slice())
        .bind([3_u8; 32].as_slice()).bind((10_i64 << 32) + (20_i64 << 16) + (7_i64 << 8))
        .bind([1_u8; 33].as_slice()).bind(id(111).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

#[test]
fn write_postgres_map_ground_custody_metadata_tombstone_and_replay() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "write_map_ground", false).await?;
        let result = async {
            map_ground_fixture(&h).await?;
            assert!(matches!(write_ground(&h, 1, "refused", None).await?, Err(ItemTextWriteError::NotReachable)));
            assert!(matches!(write_ground(&h, 2, "refused", Some(ground())).await?, Err(ItemTextWriteError::NotReachable)));
            let before: serde_json::Value = sqlx::query_scalar("SELECT jsonb_build_array(to_jsonb(i),to_jsonb(g),to_jsonb(m)) FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) JOIN game_map_item_mint_receipts m USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid")
                .bind(id(111).as_slice()).fetch_one(&h.pool).await?;
            let written = write_ground(&h, 3, "map\ntext", Some(map_ground())).await??;
            assert_eq!(written, ItemTextWriteOutcome { text_revision: 1, changed: true, replayed: false });
            let stored: (String, bool, bool, String) = sqlx::query_as("SELECT text,writer_character_id=encode($1,'hex')::uuid,isfinite(written_at),text_revision::text FROM game_item_texts")
                .bind(native::id(native::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(stored, ("map\ntext".into(), true, true, "1".into()));
            let unchanged = write_ground(&h, 4, "map\ntext", Some(map_ground())).await??;
            assert_eq!(unchanged, ItemTextWriteOutcome { text_revision: 1, changed: false, replayed: false });
            let cleared = write_ground(&h, 5, "", Some(map_ground())).await??;
            assert_eq!(cleared, ItemTextWriteOutcome { text_revision: 2, changed: true, replayed: false });
            assert_eq!(h.count("game_item_texts").await?, 0);
            let revision: String = sqlx::query_scalar("SELECT last_revision::text FROM game_item_text_revisions WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(111).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(revision, "2");
            let after: serde_json::Value = sqlx::query_scalar("SELECT jsonb_build_array(to_jsonb(i),to_jsonb(g),to_jsonb(m)) FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) JOIN game_map_item_mint_receipts m USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid")
                .bind(id(111).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(before, after);
            assert_eq!(h.root_revision().await?, "1");
            // Present source provenance is irrelevant to an exact committed
            // request: replay and reconciliation must still resolve it.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            sqlx::query("DELETE FROM game_map_item_mint_receipts").execute(&mut *tx).await?;
            tx.commit().await?;
            let replay = write_ground(&h, 3, "map\ntext", None).await??;
            assert_eq!(replay, ItemTextWriteOutcome { replayed: true, ..written });
            let seal = h.recovery.seal_current().map_err(native::debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(native::debug)?;
            let binding = carried_request(111, 3, "map\ntext")?.freeze_receipt_binding(native::item_fence()?.character_id);
            assert_eq!(h.root.reconcile_item_text_rewrite(&authority, &binding).await?, Some(replay));
            assert!(matches!(write_ground(&h, 3, "different", None).await?, Err(ItemTextWriteError::ConflictingCommand)));
            assert!(matches!(write_ground(&h, 6, "refused", Some(map_ground())).await?, Err(ItemTextWriteError::NotReachable)));
            assert_eq!(h.count("game_item_text_write_receipts").await?, 3);
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        h.cleanup().await?;
        if result.is_ok() { eprintln!("WRITE-NATIVE-PG map Ground metadata, preserved custody, tombstone and history PASS"); }
        result
    })
}

#[test]
fn write_postgres_map_ground_rejects_unproven_source() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "write_map_proof", false).await?;
        let result = async {
            map_ground_fixture(&h).await?;
            // Restore each privileged one-field fixture mutation after the
            // refusal. No source or placement string alone grants custody.
            for mutation in [
                "DELETE FROM game_map_item_mint_receipts",
                "UPDATE game_map_item_mint_receipts SET transaction_id=game_character_uuid_v7()",
                "UPDATE game_map_item_mint_receipts SET world_id=game_character_uuid_v7()",
                "UPDATE game_map_item_mint_receipts SET channel_id=game_character_uuid_v7()",
                "UPDATE game_map_item_mint_receipts SET definition_revision_ref='other'",
                "UPDATE game_map_item_mint_receipts SET quantity=2",
                "UPDATE game_map_item_mint_receipts SET runtime_scope_ownership_generation=2",
                "UPDATE game_map_item_mint_receipts SET base_bundle_digest=decode(repeat('04',32),'hex'),map_revision='sha256:' || repeat('04',32)",
                "UPDATE game_map_item_mint_receipts SET content_revision='other'",
                "UPDATE game_item_instances SET state_revision=2",
                "UPDATE game_item_instances SET last_transaction_id=game_character_uuid_v7()",
                "UPDATE game_item_ground_locations SET stack_ordinal=NULL",
                "UPDATE game_item_ground_locations SET corpse_ref='ordinary-corpse'::bytea",
            ] {
                // The real writer needs a separate connection, so commit the
                // fixture and restore its exact rows from bounded snapshots.
                let mut connection = h.pool.acquire().await?;
                let mut tx = sqlx::Acquire::begin(&mut connection).await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query("CREATE TEMP TABLE proof_item ON COMMIT PRESERVE ROWS AS SELECT * FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(111).as_slice()).execute(&mut *tx).await?;
                sqlx::query("CREATE TEMP TABLE proof_ground ON COMMIT PRESERVE ROWS AS SELECT * FROM game_item_ground_locations").execute(&mut *tx).await?;
                sqlx::query("CREATE TEMP TABLE proof_mint ON COMMIT PRESERVE ROWS AS SELECT * FROM game_map_item_mint_receipts").execute(&mut *tx).await?;
                sqlx::query(mutation).execute(&mut *tx).await?;
                // Retain this admin connection: the snapshots are session-local.
                tx.commit().await?;
                assert!(matches!(write_ground(&h, 1, "refused", Some(map_ground())).await?, Err(ItemTextWriteError::NotReachable)));
                let mut tx = sqlx::Acquire::begin(&mut connection).await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query("DELETE FROM game_map_item_mint_receipts").execute(&mut *tx).await?;
                sqlx::query("DELETE FROM game_item_ground_locations").execute(&mut *tx).await?;
                sqlx::query("DELETE FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(111).as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_item_instances SELECT * FROM proof_item").execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_item_ground_locations SELECT * FROM proof_ground").execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_map_item_mint_receipts SELECT * FROM proof_mint").execute(&mut *tx).await?;
                sqlx::query("DROP TABLE proof_item,proof_ground,proof_mint").execute(&mut *tx).await?;
                tx.commit().await?;
            }
            for table in ["game_item_texts", "game_item_text_revisions", "game_item_text_write_receipts"] {
                assert_eq!(h.count(table).await?, 0);
            }
            assert_eq!(h.root_revision().await?, "1");
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        h.cleanup().await?;
        if result.is_ok() { eprintln!("WRITE-NATIVE-PG map Ground exact provenance refusals and rollback PASS"); }
        result
    })
}

#[test]
fn write_postgres_ground_custody_pins_and_historical_replay() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "write_ground", false).await?;
        let result = async {
            ground_fixture(&h).await?;
            assert!(matches!(write_ground(&h, 1, "refused", None).await?, Err(ItemTextWriteError::NotReachable)));
            for field in 0..5 {
                let mut wrong = ground();
                match field {
                    0 => wrong.spatial_position[3] += 1,
                    1 => wrong.map_revision.push('x'),
                    2 => wrong.content_revision.push('x'),
                    3 => wrong.placement_context[0] += 1,
                    _ => { wrong.spatial_position.pop(); },
                }
                assert!(matches!(write_ground(&h, 2 + field, "refused", Some(wrong)).await?, Err(ItemTextWriteError::NotReachable)));
            }
            // Each privileged fixture changes one current custody invariant.
            for (command, mutation, restore) in [
                (10, "UPDATE game_item_ground_locations SET channel_id=game_character_uuid_v7()", "UPDATE game_item_ground_locations SET channel_id=encode($1,'hex')::uuid"),
                (11, "UPDATE game_item_ground_locations SET world_id=game_character_uuid_v7()", "UPDATE game_item_ground_locations SET world_id=encode($1,'hex')::uuid"),
                (12, "UPDATE game_item_ground_locations SET runtime_scope_ownership_generation=2", "UPDATE game_item_ground_locations SET runtime_scope_ownership_generation=1 WHERE world_id=encode($1,'hex')::uuid"),
                (13, "UPDATE game_item_ground_locations SET corpse_ref='ordinary-corpse'::bytea", "UPDATE game_item_ground_locations SET corpse_ref='OTERYN_ITEM_LOOP_DROP/v1:'::bytea WHERE world_id=encode($1,'hex')::uuid"),
                (14, "UPDATE game_item_instances SET definition_revision_ref='other'", "UPDATE game_item_instances SET definition_revision_ref='definition-r1' WHERE world_id=encode($1,'hex')::uuid"),
            ] {
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query(mutation).execute(&mut *tx).await?;
                tx.commit().await?;
                assert!(matches!(write_ground(&h, command, "refused", Some(ground())).await?, Err(ItemTextWriteError::NotReachable)));
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query(restore).bind(native::id(if command == 10 { native::CHANNEL } else { native::WORLD }).as_slice()).execute(&mut *tx).await?;
                tx.commit().await?;
            }
            assert_eq!(h.count("game_item_text_write_receipts").await?, 0);
            let location_before: serde_json::Value = sqlx::query_scalar("SELECT to_jsonb(g) FROM game_item_ground_locations g WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(111).as_slice()).fetch_one(&h.pool).await?;
            let written = write_ground(&h, 20, "ground\ntext", Some(ground())).await??;
            assert_eq!(written, ItemTextWriteOutcome { text_revision: 1, changed: true, replayed: false });
            // Location is unchanged; text does not consume a Character revision.
            let stored: (String, bool, bool, String) = sqlx::query_as("SELECT text,writer_character_id=encode($1,'hex')::uuid,isfinite(written_at),text_revision::text FROM game_item_texts")
                .bind(native::id(native::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(stored, ("ground\ntext".into(), true, true, "1".into()));
            let location_after: serde_json::Value = sqlx::query_scalar("SELECT to_jsonb(g) FROM game_item_ground_locations g WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(111).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(location_after, location_before);
            assert_eq!(h.count("game_item_ground_locations").await?, 1);
            assert_eq!(h.root_revision().await?, "1");
            let replay = write_ground(&h, 20, "ground\ntext", None).await??;
            assert_eq!(replay, ItemTextWriteOutcome { replayed: true, ..written });
            assert!(matches!(write_ground(&h, 20, "changed", None).await?, Err(ItemTextWriteError::ConflictingCommand)));
            let seal = h.recovery.seal_current().map_err(native::debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(native::debug)?;
            let binding = carried_request(111, 20, "ground\ntext")?.freeze_receipt_binding(native::item_fence()?.character_id);
            assert_eq!(h.root.reconcile_item_text_rewrite(&authority, &binding).await?, Some(replay));
            assert_eq!(h.count("game_item_text_write_receipts").await?, 1);
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        h.cleanup().await?;
        if result.is_ok() { eprintln!("WRITE-NATIVE-PG Ground custody, pins, metadata and historical replay PASS"); }
        result
    })
}

#[test]
fn write_postgres_ground_custody_revalidates_after_item_lock() -> TestResult {
    ground_revalidates_after_lock(false, false)
}

#[test]
fn write_postgres_ground_custody_revalidates_after_owner_lock() -> TestResult {
    ground_revalidates_after_lock(true, false)
}

#[test]
fn write_postgres_map_ground_revalidates_after_item_lock() -> TestResult {
    ground_revalidates_after_lock(false, true)
}

#[test]
fn write_postgres_map_ground_revalidates_after_owner_lock() -> TestResult {
    ground_revalidates_after_lock(true, true)
}

fn ground_revalidates_after_lock(owner: bool, map_mint: bool) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "write_ground_wait", false).await?;
        let result = async {
            let placement = if map_mint {
                map_ground_fixture(&h).await?;
                map_ground()
            } else {
                ground_fixture(&h).await?;
                ground()
            };
            let mut blocker = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *blocker).await?;
            let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()").fetch_one(&mut *blocker).await?;
            if owner {
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'),33))")
                    .bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHANNEL).as_slice()).execute(&mut *blocker).await?;
            } else {
                sqlx::query("SELECT 1 FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
                    .bind(id(111).as_slice()).execute(&mut *blocker).await?;
            }
            let writing = write_ground(&h, 1, "must not commit", Some(placement.clone()));
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
                    return Poll::Ready(Err(format!("writer completed before the held Ground lock: {early:?}").into()));
                }
                waiting.as_mut().poll(context)
            }).await?;
            // Only a privileged fixture changes custody/provenance outside the
            // runtime's locks and mutation lane. Map placement stays identical:
            // its receipt proof must also use the post-wait snapshot.
            if map_mint {
                sqlx::query("UPDATE game_map_item_mint_receipts SET quantity=2 WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(111).as_slice()).execute(&mut *blocker).await?;
            } else {
                let mut moved = placement.spatial_position;
                moved[..4].copy_from_slice(&11_i32.to_be_bytes());
                sqlx::query("UPDATE game_item_ground_locations SET spatial_position=$1 WHERE item_instance_id=encode($2,'hex')::uuid")
                    .bind(moved)
                    .bind(id(111).as_slice()).execute(&mut *blocker).await?;
            }
            blocker.commit().await?;
            assert!(matches!(writing.await?, Err(ItemTextWriteError::NotReachable)));
            for table in ["game_item_texts", "game_item_text_revisions", "game_item_text_write_receipts"] {
                assert_eq!(h.count(table).await?, 0);
            }
            assert_eq!(h.root_revision().await?, "1");
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        h.cleanup().await?;
        if result.is_ok() { eprintln!("WRITE-NATIVE-PG actual Ground lock wait, fresh custody refusal and rollback PASS"); }
        result
    })
}
