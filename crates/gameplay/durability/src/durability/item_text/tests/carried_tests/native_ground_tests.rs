//! Native-map producer + fenced TEXT storage qualification, not TEXT ingress.
use super::*;
use crate::durability::native_map_items_abi::{
    NativeMapInitializationProof, NativeMapItemPlacement, NativeMapOwnerBinding,
    map_initializer_seal,
};

struct MapFixture {
    binding: NativeMapOwnerBinding,
    placements: Vec<NativeMapItemPlacement>,
}
impl map_initializer_seal::Sealed for MapFixture {}
impl NativeMapInitializationProof for MapFixture {
    fn current_binding(&self) -> std::result::Result<NativeMapOwnerBinding, &'static str> {
        Ok(self.binding.clone())
    }
    fn placements(&self) -> &[NativeMapItemPlacement] {
        &self.placements
    }
}

fn placement() -> LoopGround {
    LoopGround {
        spatial_position: [
            10_i32.to_be_bytes().as_slice(),
            20_i32.to_be_bytes().as_slice(),
            (-7_i16).to_be_bytes().as_slice(),
        ]
        .concat(),
        map_revision: format!("sha256:{}", "03".repeat(32)),
        content_revision: format!("sha256:{}", "01".repeat(32)),
        placement_context: vec![4; 32],
    }
}

async fn native_fixture(h: &Harness) -> TestResult<[u8; 16]> {
    fixture(h).await?;
    let fence = native::item_fence()?;
    let (world, channel) = scope_of(&fence)?;
    let ground = placement();
    let proof = MapFixture {
        binding: NativeMapOwnerBinding {
            world,
            channel,
            scope_generation: 1,
            content_digest: [1; 32],
            map_digest: [3; 32],
            frame_digest: [4; 32],
        },
        placements: vec![NativeMapItemPlacement {
            placement_key: "write-scroll".into(),
            definition: carried_request(111, 1, "")?.definition,
            quantity: 1,
            ground: crate::durability::item_mint::GroundPlacement {
                spatial_position: ground.spatial_position,
                corpse_ref: b"native-map-item".to_vec(),
                map_revision: ground.map_revision,
                content_revision: ground.content_revision,
                native_room_placement_context: ground.placement_context,
            },
            source_binding: vec![1],
            source_attributes: serde_json::json!({}),
            blocks_movement: false,
            blocks_projectile: false,
            immovable_block_solid: false,
            owner_kind: None,
        }],
    };
    let seal = h.recovery.seal_current().map_err(native::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(native::debug)?;
    let permit =
        crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(fence.runtime_scope)
            .await?;
    // Use the actual native producer, including all immediate/deferred guards
    // and runtime-role grants. Only the active-map proof itself is a fixture.
    let items = h
        .root
        .initialize_native_map_current_owner(&permit, &authority, &h.node, &proof)
        .await
        .map_err(native::debug)?;
    assert_eq!(items.len(), 1);
    assert!(!items[0].replayed);
    Ok(items[0].item_instance_id)
}

fn request(item: [u8; 16], command: u64, text: &str) -> TestResult<ItemTextWriteRequest> {
    let mut request = carried_request(111, command, text)?;
    request.item_instance_id = item;
    Ok(request)
}

async fn native_write(
    h: &Harness,
    item: [u8; 16],
    command: u64,
    text: &str,
    ground: Option<LoopGround>,
) -> TestResult<std::result::Result<ItemTextWriteOutcome, ItemTextWriteError>> {
    native_write_fenced(h, item, command, text, ground, native::item_fence()?).await
}

async fn native_write_fenced(
    h: &Harness,
    item: [u8; 16],
    command: u64,
    text: &str,
    ground: Option<LoopGround>,
    fence: CurrentCharacterItemFence,
) -> TestResult<std::result::Result<ItemTextWriteOutcome, ItemTextWriteError>> {
    let seal = h.recovery.seal_current().map_err(native::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(native::debug)?;
    let request = request(item, command, text)?;
    let policy = MailTextPolicy::from_qualified_content(request.definition.clone(), [1; 32], 511);
    let permit =
        crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(fence.runtime_scope)
            .await?;
    Ok(h.root
        .commit_item_text_rewrite(
            &permit, &authority, &h.node, fence, policy, request, None, ground,
        )
        .await)
}

#[test]
fn write_postgres_native_ground_requires_current_scope_adoption() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "write_native_adoption", false).await?;
        let result = async {
            let item = native_fixture(&h).await?;
            // Explicit current-session/assignment fixture, not a claim to run
            // the live relocation lifecycle. Leave the original map receipt at
            // generation 1 and the original Ground row at generation 1.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            sqlx::query("UPDATE game_runtime_scope_assignments SET ownership_generation=2").execute(&mut *tx).await?;
            // Keep the synthetic assignment and its retained receipt coherent:
            // the session fence validates history before checking Ground custody.
            sqlx::query("UPDATE game_runtime_scope_assignment_receipts r SET ownership_generation=a.ownership_generation FROM game_runtime_scope_assignments a WHERE r.operation_key=a.operation_key").execute(&mut *tx).await?;
            sqlx::query("UPDATE game_durability_admission_runtime_guards SET ownership_generation=2").execute(&mut *tx).await?;
            sqlx::query("UPDATE game_durability_reconnect_sessions SET scope_ownership_generation=2").execute(&mut *tx).await?;
            tx.commit().await?;
            assert!(sqlx::query_scalar::<_, bool>("SELECT game_runtime_scope_assignment_history_valid()").fetch_one(&h.pool).await?);
            let mut fence = native::item_fence()?;
            fence.scope_ownership_generation = crate::foundation::ScopeOwnershipGeneration::new(2)?;
            assert!(matches!(native_write_fenced(&h, item, 1, "refused", Some(placement()), fence).await?, Err(ItemTextWriteError::NotReachable)));
            // Use the native owner's real adoption trigger to retain exactly
            // this Ground placement and advance only its ownership generation.
            let fact = h.node.fact();
            sqlx::query("INSERT INTO game_native_map_scope_adoptions(source_transaction_id,ownership_generation,holder_node_id,holder_registration_revision,ground_before) SELECT r.transaction_id,2,encode($2,'hex')::uuid,$3::text::numeric,to_jsonb(g) FROM game_native_map_item_receipts r JOIN game_item_ground_locations g USING(item_instance_id) WHERE r.item_instance_id=encode($1,'hex')::uuid")
                .bind(item.as_slice()).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).execute(&h.pool).await?;
            let written = native_write_fenced(&h, item, 1, "adopted", Some(placement()), fence).await??;
            assert_eq!(written, ItemTextWriteOutcome { text_revision: 1, changed: true, replayed: false });
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            sqlx::query("DELETE FROM game_native_map_scope_adoptions").execute(&mut *tx).await?;
            tx.commit().await?;
            // Ground alone at generation 2 does not prove native adoption.
            assert!(matches!(native_write_fenced(&h, item, 2, "refused", Some(placement()), fence).await?, Err(ItemTextWriteError::NotReachable)));
            assert_eq!(h.count("game_item_text_write_receipts").await?, 1);
            assert_eq!(h.root_revision().await?, "1");
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        h.cleanup().await?;
        if result.is_ok() { eprintln!("WRITE-NATIVE-PG native-map current scope adoption PASS"); }
        result
    })
}

#[test]
fn write_postgres_native_ground_metadata_tombstone_and_history() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "write_native_ground", false).await?;
        let result = async {
            let item = native_fixture(&h).await?;
            assert!(matches!(native_write(&h, item, 1, "refused", None).await?, Err(ItemTextWriteError::NotReachable)));
            let before: serde_json::Value = sqlx::query_scalar("SELECT jsonb_build_array(to_jsonb(i),to_jsonb(g),to_jsonb(r),to_jsonb(a)) FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) JOIN game_native_map_item_receipts r USING(item_instance_id) JOIN game_native_map_item_audit a USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid")
                .bind(item.as_slice()).fetch_one(&h.pool).await?;
            let written = native_write(&h, item, 2, "native\ntext", Some(placement())).await??;
            assert_eq!(written, ItemTextWriteOutcome { text_revision: 1, changed: true, replayed: false });
            let stored: (String, bool, bool, String) = sqlx::query_as("SELECT text,writer_character_id=encode($1,'hex')::uuid,isfinite(written_at),text_revision::text FROM game_item_texts WHERE item_instance_id=encode($2,'hex')::uuid")
                .bind(native::id(native::CHARACTER).as_slice()).bind(item.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(stored, ("native\ntext".into(), true, true, "1".into()));
            let unchanged = native_write(&h, item, 3, "native\ntext", Some(placement())).await??;
            assert_eq!(unchanged, ItemTextWriteOutcome { changed: false, ..written });
            let cleared = native_write(&h, item, 4, "", Some(placement())).await??;
            assert_eq!(cleared, ItemTextWriteOutcome { text_revision: 2, changed: true, replayed: false });
            assert_eq!(h.count("game_item_texts").await?, 0);
            let revision: String = sqlx::query_scalar("SELECT last_revision::text FROM game_item_text_revisions WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(item.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(revision, "2");
            let after: serde_json::Value = sqlx::query_scalar("SELECT jsonb_build_array(to_jsonb(i),to_jsonb(g),to_jsonb(r),to_jsonb(a)) FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) JOIN game_native_map_item_receipts r USING(item_instance_id) JOIN game_native_map_item_audit a USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid")
                .bind(item.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(before, after);
            assert_eq!(h.root_revision().await?, "1");
            // Historical receipt resolution must survive lost current source
            // authority, even when a fresh write would now be refused.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            sqlx::query("DELETE FROM game_native_map_item_audit WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
            let replay = native_write(&h, item, 2, "native\ntext", None).await??;
            assert_eq!(replay, ItemTextWriteOutcome { replayed: true, ..written });
            let seal = h.recovery.seal_current().map_err(native::debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(native::debug)?;
            let binding = request(item, 2, "native\ntext")?.freeze_receipt_binding(native::item_fence()?.character_id);
            assert_eq!(h.root.reconcile_item_text_rewrite(&authority, &binding).await?, Some(replay));
            assert!(matches!(native_write(&h, item, 2, "conflict", None).await?, Err(ItemTextWriteError::ConflictingCommand)));
            assert!(matches!(native_write(&h, item, 5, "refused", Some(placement())).await?, Err(ItemTextWriteError::NotReachable)));
            assert_eq!(h.count("game_item_text_write_receipts").await?, 3);
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        h.cleanup().await?;
        if result.is_ok() { eprintln!("WRITE-NATIVE-PG native-map metadata, preserved custody, tombstone and history PASS"); }
        result
    })
}

#[test]
fn write_postgres_native_ground_rejects_unproven_source() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "write_native_proof", false).await?;
        let result = async {
            let item = native_fixture(&h).await?;
            // Each committed privileged mutation removes one proof invariant.
            // Restore exact bounded snapshots after the runtime-role refusal.
            for mutation in [
                "DELETE FROM game_native_map_item_receipts",
                "DELETE FROM game_native_map_item_audit",
                "UPDATE game_native_map_item_receipts SET transaction_id=game_character_uuid_v7()",
                "UPDATE game_native_map_item_receipts SET world_id=game_character_uuid_v7()",
                "UPDATE game_native_map_item_receipts SET channel_id=game_character_uuid_v7()",
                "UPDATE game_native_map_item_receipts SET definition_revision='other'",
                "UPDATE game_native_map_item_receipts SET quantity=2",
                "UPDATE game_native_map_item_receipts SET scope_generation=2",
                "UPDATE game_native_map_item_receipts SET reset_epoch=1",
                "UPDATE game_native_map_item_receipts SET owner_kind='door'",
                "UPDATE game_native_map_item_receipts SET map_digest=decode(repeat('05',32),'hex'),map_revision='sha256:' || repeat('05',32)",
                "UPDATE game_native_map_item_receipts SET content_digest=decode(repeat('05',32),'hex'),content_revision='sha256:' || repeat('05',32)",
                "UPDATE game_native_map_item_receipts SET frame_digest=decode(repeat('05',32),'hex'),placement_context=decode(repeat('05',32),'hex')",
                "UPDATE game_native_map_item_audit SET event_id=game_character_uuid_v7()",
                "UPDATE game_native_map_item_audit SET transaction_id=game_character_uuid_v7()",
                "UPDATE game_native_map_item_audit SET envelope=convert_to('other','UTF8'),envelope_digest=sha256(convert_to('other','UTF8'))",
                "UPDATE game_native_map_item_audit SET created_xact_id=pg_current_xact_id()",
                "UPDATE game_item_instances SET state_revision=2 WHERE item_instance_id IN(SELECT item_instance_id FROM game_native_map_item_receipts)",
                "UPDATE game_item_instances SET last_transaction_id=game_character_uuid_v7() WHERE item_instance_id IN(SELECT item_instance_id FROM game_native_map_item_receipts)",
            ] {
                let mut connection = h.pool.acquire().await?;
                let mut tx = sqlx::Acquire::begin(&mut connection).await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query("CREATE TEMP TABLE proof_item ON COMMIT PRESERVE ROWS AS SELECT * FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).execute(&mut *tx).await?;
                sqlx::query("CREATE TEMP TABLE proof_native ON COMMIT PRESERVE ROWS AS SELECT * FROM game_native_map_item_receipts").execute(&mut *tx).await?;
                sqlx::query("CREATE TEMP TABLE proof_audit ON COMMIT PRESERVE ROWS AS SELECT * FROM game_native_map_item_audit").execute(&mut *tx).await?;
                sqlx::query(mutation).execute(&mut *tx).await?;
                tx.commit().await?;
                assert!(matches!(native_write(&h, item, 1, "refused", Some(placement())).await?, Err(ItemTextWriteError::NotReachable)), "mutation: {mutation}");
                let mut tx = sqlx::Acquire::begin(&mut connection).await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query("DELETE FROM game_native_map_item_audit").execute(&mut *tx).await?;
                sqlx::query("DELETE FROM game_native_map_item_receipts").execute(&mut *tx).await?;
                sqlx::query("DELETE FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_item_instances SELECT * FROM proof_item").execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_native_map_item_receipts SELECT * FROM proof_native").execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_native_map_item_audit SELECT * FROM proof_audit").execute(&mut *tx).await?;
                sqlx::query("DROP TABLE proof_item,proof_native,proof_audit").execute(&mut *tx).await?;
                tx.commit().await?;
            }
            for field in 0..4 {
                let mut wrong = placement();
                match field {
                    0 => wrong.spatial_position[3] += 1,
                    1 => wrong.map_revision.push('x'),
                    2 => wrong.content_revision.push('x'),
                    _ => wrong.placement_context[0] += 1,
                }
                assert!(matches!(native_write(&h, item, 1, "refused", Some(wrong)).await?, Err(ItemTextWriteError::NotReachable)));
            }
            for table in ["game_item_texts", "game_item_text_revisions", "game_item_text_write_receipts"] {
                assert_eq!(h.count(table).await?, 0);
            }
            assert_eq!(h.root_revision().await?, "1");
            // The restored valid source is usable after every refused attempt.
            assert_eq!(native_write(&h, item, 1, "valid", Some(placement())).await??.text_revision, 1);
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        h.cleanup().await?;
        if result.is_ok() { eprintln!("WRITE-NATIVE-PG native-map provenance refusals and rollback PASS"); }
        result
    })
}

#[test]
fn write_postgres_native_ground_revalidates_after_item_lock() -> TestResult {
    native_revalidates_after_lock(false)
}

#[test]
fn write_postgres_native_ground_revalidates_after_owner_lock() -> TestResult {
    native_revalidates_after_lock(true)
}

fn native_revalidates_after_lock(owner: bool) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let tag = if owner { "write_native_owner_wait" } else { "write_native_item_wait" };
        let h = Harness::create(admin, tag, false).await?;
        let result = async {
            let item = native_fixture(&h).await?;
            let mut blocker = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *blocker).await?;
            let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()").fetch_one(&mut *blocker).await?;
            if owner {
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'),33))")
                    .bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHANNEL).as_slice()).execute(&mut *blocker).await?;
            } else {
                sqlx::query("SELECT 1 FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
                    .bind(item.as_slice()).execute(&mut *blocker).await?;
            }
            let writing = native_write(&h, item, 1, "must not commit", Some(placement()));
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
                    return Poll::Ready(Err(format!("writer completed before the held native-map lock: {early:?}").into()));
                }
                waiting.as_mut().poll(context)
            }).await?;
            // Ground and receipt remain exact. Only the source audit changes
            // while the actual writer is observed waiting on the physical lock.
            sqlx::query("DELETE FROM game_native_map_item_audit WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(item.as_slice()).execute(&mut *blocker).await?;
            blocker.commit().await?;
            assert!(matches!(writing.await?, Err(ItemTextWriteError::NotReachable)));
            for table in ["game_item_texts", "game_item_text_revisions", "game_item_text_write_receipts"] {
                assert_eq!(h.count(table).await?, 0);
            }
            assert_eq!(h.root_revision().await?, "1");
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        h.cleanup().await?;
        if result.is_ok() { eprintln!("WRITE-NATIVE-PG native-map post-wait provenance revalidation PASS (owner={owner})"); }
        result
    })
}
