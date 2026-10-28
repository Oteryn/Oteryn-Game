//! Fenced writer qualification with explicit custody/Content fixtures. No TEXT
//! ingress is installed, and these cases do not qualify the pending trade guard.
use super::*;
use crate::bestiary_postgres_harness::{self as native, Harness, configured_admin, runtime};

mod ground_tests;
mod native_ground_tests;

fn carried_request(item: u8, command: u64, text: &str) -> TestResult<ItemTextWriteRequest> {
    Ok(ItemTextWriteRequest {
        command: CommandRef::new(
            GameSessionId::decode(&native::id(native::SESSION))?,
            CommandId::new(command)?,
        ),
        item_instance_id: id(item),
        definition: TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2815".into(),
            revision_ref: "definition-r1".into(),
        },
        text: text.into(),
    })
}

async fn write(
    h: &Harness,
    item: u8,
    command: u64,
    text: &str,
    fence: CurrentCharacterItemFence,
) -> TestResult<std::result::Result<ItemTextWriteOutcome, ItemTextWriteError>> {
    write_with_depot(h, item, command, text, fence, None).await
}

async fn write_with_depot(
    h: &Harness,
    item: u8,
    command: u64,
    text: &str,
    fence: CurrentCharacterItemFence,
    open_depot: Option<(u16, u8)>,
) -> TestResult<std::result::Result<ItemTextWriteOutcome, ItemTextWriteError>> {
    write_with_custody(h, item, command, text, fence, open_depot, None).await
}

async fn write_with_custody(
    h: &Harness,
    item: u8,
    command: u64,
    text: &str,
    fence: CurrentCharacterItemFence,
    open_depot: Option<(u16, u8)>,
    ground: Option<LoopGround>,
) -> TestResult<std::result::Result<ItemTextWriteOutcome, ItemTextWriteError>> {
    let seal = h.recovery.seal_current().map_err(native::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(native::debug)?;
    let request = carried_request(item, command, text)?;
    let policy = MailTextPolicy::from_qualified_content(request.definition.clone(), [1; 32], 511);
    let permit = super::super::super::spell_owner_commit::SpellLanePermit::of_fresh_scope(
        fence.runtime_scope,
    )
    .await?;
    Ok(h.root
        .commit_item_text_rewrite(
            &permit, &authority, &h.node, fence, policy, request, open_depot, ground,
        )
        .await)
}

async fn fixture(h: &Harness) -> TestResult {
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHANNEL).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHANNEL).as_slice())
        .bind([1u8;32].as_slice()).bind([2u8;32].as_slice()).bind([3u8;32].as_slice()).execute(&h.pool).await?;
    // Seed only explicit immutable Content/custody test data. No runtime path
    // uses trigger bypass; all tested writes use the actual fenced public writer.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for tag in 100..=111 {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i2815','definition-r1',1,1,game_character_uuid_v7())")
            .bind(id(tag).as_slice()).bind(native::id(native::WORLD).as_slice()).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_item_container_slots VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,game_character_uuid_v7())")
        .bind(native::id(native::CHARACTER).as_slice()).bind(id(100).as_slice()).bind(native::id(native::WORLD).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,game_character_uuid_v7())")
        .bind(id(101).as_slice()).bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHARACTER).as_slice()).bind(id(100).as_slice()).execute(&mut *tx).await?;
    for tag in 102..=109 {
        sqlx::query("INSERT INTO game_item_nested_entries VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,game_character_uuid_v7())")
            .bind(id(tag).as_slice()).bind(native::id(native::WORLD).as_slice()).bind(id(tag-1).as_slice()).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_character_equipment_slots VALUES(encode($1,'hex')::uuid,1,encode($2,'hex')::uuid,encode($3,'hex')::uuid,game_character_uuid_v7())")
        .bind(native::id(native::CHARACTER).as_slice()).bind(id(110).as_slice()).bind(native::id(native::WORLD).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let mut connection = h.root.try_acquire_ready().map_err(native::debug)?;
    sqlx::query("SET ROLE oteryn_game_runtime")
        .execute(&mut *connection)
        .await?;
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT current_user")
            .fetch_one(&mut *connection)
            .await?,
        "oteryn_game_runtime"
    );
    assert!(matches!(
        connection
            .return_to_pool_observed_until(
                std::time::Instant::now() + std::time::Duration::from_secs(5)
            )
            .await,
        sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle
    ));
    Ok(())
}

async fn depot_fixture(h: &Harness) -> TestResult {
    fixture(h).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_character_depot_locations VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,7,17,1)")
        .bind(id(111).as_slice()).bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHARACTER).as_slice()).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_character_depot_box_counters VALUES(encode($1,'hex')::uuid,7,17,2,1)",
    )
    .bind(native::id(native::CHARACTER).as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

#[test]
fn write_postgres_depot_custody_binding_and_historical_replay() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "write_depot", false).await?;
        let result = async {
            depot_fixture(&h).await?;
            let seal = h.recovery.seal_current().map_err(native::debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(native::debug)?;
            let fence = native::item_fence()?;
            let (world, channel) = scope_of(&fence)?;
            for (lane_world, lane_channel) in [
                (WorldId::decode(&id(90))?, channel),
                (world, crate::foundation::ChannelId::decode(&id(91))?),
            ] {
                let permit = super::super::super::spell_owner_commit::SpellLanePermit::of_fresh_lane(lane_world, lane_channel).await;
                let request = carried_request(111, 20, "refused")?;
                let policy = MailTextPolicy::from_qualified_content(request.definition.clone(), [1; 32], 511);
                assert!(matches!(h.root.commit_item_text_rewrite(&permit, &authority, &h.node, fence, policy, request, Some((7, 17)), None).await, Err(ItemTextWriteError::AuthorityRejected)));
            }
            // Owner-observed open box data is an explicit fixture here. These
            // cases qualify storage custody, not the uninstalled TEXT/view route.
            for (command, access) in [(1, None), (2, Some((8, 17))), (3, Some((7, 1))), (4, Some((7, 0))), (5, Some((7, 18)))] {
                assert!(matches!(write_with_depot(&h, 111, command, "refused", native::item_fence()?, access).await?, Err(ItemTextWriteError::NotReachable)));
            }
            assert_eq!(h.count("game_item_text_write_receipts").await?, 0);
            let written = write_with_depot(&h, 111, 6, "depot\ntext", native::item_fence()?, Some((7, 17))).await??;
            assert_eq!(written, ItemTextWriteOutcome { text_revision: 1, changed: true, replayed: false });
            // A current request requires the open view, but a historical retry
            // still returns its first outcome after that view is closed.
            assert!(matches!(write(&h, 111, 7, "replacement", native::item_fence()?).await?, Err(ItemTextWriteError::NotReachable)));
            let replay = write(&h, 111, 6, "depot\ntext", native::item_fence()?).await??;
            assert_eq!(replay, ItemTextWriteOutcome { replayed: true, ..written });
            let binding = carried_request(111, 6, "depot\ntext")?.freeze_receipt_binding(fence.character_id);
            assert_eq!(h.root.reconcile_item_text_rewrite(&authority, &binding).await?, Some(replay));
            let mut stale = native::item_fence()?;
            stale.connection_generation = crate::foundation::ConnectionGeneration::new(2)?;
            assert!(matches!(write_with_depot(&h, 111, 8, "refused", stale, Some((7, 17))).await?, Err(ItemTextWriteError::AuthorityRejected)));
            for (command, mutation, restore) in [
                (9, "UPDATE game_item_character_depot_locations SET character_id=game_character_uuid_v7()", "UPDATE game_item_character_depot_locations SET character_id=(SELECT character_id FROM game_character_roots LIMIT 1)"),
                (10, "UPDATE game_item_character_depot_locations SET world_id=game_character_uuid_v7()", "UPDATE game_item_character_depot_locations SET world_id=(SELECT world_id FROM game_character_roots LIMIT 1)"),
                (11, "UPDATE game_item_instances SET definition_revision_ref='other-revision'", "UPDATE game_item_instances SET definition_revision_ref='definition-r1'"),
            ] {
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::raw_sql(mutation).execute(&mut *tx).await?;
                tx.commit().await?;
                assert!(matches!(write_with_depot(&h, 111, command, "refused", native::item_fence()?, Some((7, 17))).await?, Err(ItemTextWriteError::NotReachable)));
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::raw_sql(restore).execute(&mut *tx).await?;
                tx.commit().await?;
            }
            // An open box cannot grant custody of a detached item or somebody
            // else's tree, and the depot seam does not admit nested depot bags.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            sqlx::query("DELETE FROM game_item_container_slots").execute(&mut *tx).await?;
            tx.commit().await?;
            assert!(matches!(write_with_depot(&h, 108, 12, "refused", native::item_fence()?, Some((7, 17))).await?, Err(ItemTextWriteError::NotReachable)));
            let text: (String, String) = sqlx::query_as("SELECT text,text_revision::text FROM game_item_texts WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(111).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(text, ("depot\ntext".into(), "1".into()));
            let counters: (String, i32) = sqlx::query_as("SELECT next_ordinal::text,committed FROM game_character_depot_box_counters")
                .fetch_one(&h.pool).await?;
            assert_eq!(counters, ("2".into(), 1));
            assert_eq!(h.count("game_item_text_write_receipts").await?, 1);
            assert_eq!(h.root_revision().await?, "1");
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        h.cleanup().await?;
        if result.is_ok() { eprintln!("WRITE-NATIVE-PG depot custody, closed view data, fences and historical replay PASS"); }
        result
    })
}

#[test]
fn write_postgres_carried_nested_equipment_fences_and_replay() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "write_carried", false).await?;
        let result = async {
            fixture(&h).await?;
            // A closed carried bag requires no open-container view. Depth 8 is
            // admitted, while the same tree's ninth edge is independently refused.
            let nested = write(&h, 108, 1, "nested\ntext", native::item_fence()?).await??;
            assert!(nested.changed);
            assert_eq!(nested.text_revision, 1);
            let equipment = write(&h, 110, 2, "equipped text", native::item_fence()?).await??;
            assert!(equipment.changed);
            // An unchanged write consumes no token, even after the two changes.
            assert!(!write(&h, 108, 3, "nested\ntext", native::item_fence()?).await??.changed);
            assert!(!write(&h, 101, 4, "", native::item_fence()?).await??.changed);
            for (item, command) in [(109, 5), (111, 6)] {
                assert!(matches!(write(&h, item, command, "refused", native::item_fence()?).await?, Err(ItemTextWriteError::NotReachable)));
            }
            let mut stale = native::item_fence()?;
            stale.character_lease_generation += 1;
            assert!(matches!(write(&h, 108, 7, "refused", stale).await?, Err(ItemTextWriteError::AuthorityRejected)));
            let mut stale = native::item_fence()?;
            stale.connection_generation = crate::foundation::ConnectionGeneration::new(2)?;
            assert!(matches!(write(&h, 108, 8, "refused", stale).await?, Err(ItemTextWriteError::AuthorityRejected)));
            let mut stale = native::item_fence()?;
            stale.scope_ownership_generation = crate::foundation::ScopeOwnershipGeneration::new(2)?;
            assert!(matches!(write(&h, 108, 9, "refused", stale).await?, Err(ItemTextWriteError::AuthorityRejected)));
            // Each privileged mutation changes one custody fact; restore it
            // before the next case. Failed requests must leave no receipts/text.
            for (command, mutation, restore) in [
                (20, "UPDATE game_item_container_slots SET character_id=game_character_uuid_v7()", "UPDATE game_item_container_slots SET character_id=(SELECT character_id FROM game_character_roots LIMIT 1)"),
                (21, "UPDATE game_character_equipment_slots SET character_id=game_character_uuid_v7()", "UPDATE game_character_equipment_slots SET character_id=(SELECT character_id FROM game_character_roots LIMIT 1)"),
                (22, "UPDATE game_item_instances SET lifecycle=2,quantity=0,last_transaction_id=minted_transaction_id WHERE item_instance_id IN (SELECT parent_item_instance_id FROM game_item_nested_entries WHERE placement_ordinal=1) AND item_instance_id NOT IN (SELECT item_instance_id FROM game_item_container_entries)", "UPDATE game_item_instances SET lifecycle=1,quantity=1,last_transaction_id=NULL WHERE lifecycle=2"),
                (23, "UPDATE game_item_instances SET definition_revision_ref='other-revision'", "UPDATE game_item_instances SET definition_revision_ref='definition-r1'"),
                (25, "UPDATE game_item_instances SET world_id='00000000-0000-7000-8000-000000000003'::uuid", "UPDATE game_item_instances SET world_id=(SELECT world_id FROM game_character_roots LIMIT 1)"),
                (26, "UPDATE game_item_instances SET quantity=2", "UPDATE game_item_instances SET quantity=1"),
                (24, "UPDATE game_item_nested_entries SET parent_item_instance_id=(SELECT item_instance_id FROM game_item_nested_entries ORDER BY item_instance_id DESC LIMIT 1) WHERE item_instance_id=(SELECT item_instance_id FROM game_item_nested_entries ORDER BY item_instance_id LIMIT 1)", "UPDATE game_item_nested_entries SET parent_item_instance_id=(SELECT item_instance_id FROM game_item_container_entries LIMIT 1) WHERE item_instance_id=(SELECT item_instance_id FROM game_item_nested_entries ORDER BY item_instance_id LIMIT 1)"),
            ] {
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::raw_sql(mutation).execute(&mut *tx).await?;
                tx.commit().await?;
                let item = if command == 21 { 110 } else { 108 };
                assert!(matches!(write(&h, item, command, "refused", native::item_fence()?).await?, Err(ItemTextWriteError::NotReachable)), "custody case {command}");
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::raw_sql(restore).execute(&mut *tx).await?;
                tx.commit().await?;
            }
            // Detach the first edge through privileged fixture mutation. The
            // still-live target can no longer borrow its former ancestor's owner.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(101).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
            assert!(matches!(write(&h, 108, 10, "refused", native::item_fence()?).await?, Err(ItemTextWriteError::NotReachable)));
            // Exact replay and reconciliation remain historical after custody
            // disappears. Changed bytes under that CommandRef never replay it.
            let replay = write(&h, 108, 1, "nested\ntext", native::item_fence()?).await??;
            assert_eq!(replay, ItemTextWriteOutcome { replayed: true, ..nested });
            assert!(matches!(write(&h, 108, 1, "replacement", native::item_fence()?).await?, Err(ItemTextWriteError::ConflictingCommand)));
            let seal = h.recovery.seal_current().map_err(native::debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(native::debug)?;
            let binding = carried_request(108, 1, "nested\ntext")?.freeze_receipt_binding(native::item_fence()?.character_id);
            assert_eq!(h.root.reconcile_item_text_rewrite(&authority, &binding).await?, Some(replay));
            let state: (String, bool, bool, String) = sqlx::query_as("SELECT text,writer_character_id=encode($1,'hex')::uuid,isfinite(written_at),text_revision::text FROM game_item_texts WHERE item_instance_id=encode($2,'hex')::uuid")
                .bind(native::id(native::CHARACTER).as_slice()).bind(id(108).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(state, ("nested\ntext".into(), true, true, "1".into()));
            assert_eq!(h.count("game_item_text_write_receipts").await?, 4);
            assert_eq!(h.root_revision().await?, "1");
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        h.cleanup().await?;
        if result.is_ok() { eprintln!("WRITE-NATIVE-PG carried nested/equipment writer, fences, metadata and historical replay PASS"); }
        result
    })
}

#[test]
fn write_postgres_carried_custody_revalidates_after_item_lock() -> TestResult {
    custody_revalidates_after_item_lock(false)
}

#[test]
fn write_postgres_depot_custody_revalidates_after_item_lock() -> TestResult {
    custody_revalidates_after_item_lock(true)
}

fn custody_revalidates_after_item_lock(depot: bool) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "write_custody_wait", false).await?;
        let result = async {
            if depot { depot_fixture(&h).await?; } else { fixture(&h).await?; }
            let mut blocker = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *blocker).await?;
            let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()").fetch_one(&mut *blocker).await?;
            sqlx::query("SELECT 1 FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(id(if depot { 111 } else { 105 }).as_slice()).execute(&mut *blocker).await?;
            let writing = write_with_depot(&h, if depot { 111 } else { 108 }, 1, "must not commit", native::item_fence()?, depot.then_some((7, 17)));
            tokio::pin!(writing);
            // Observe a real waiter on this exact blocking backend, without a
            // timing-based assumption that the writer has already read ancestry.
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
                    return Poll::Ready(Err(format!("writer completed before the held item lock: {early:?}").into()));
                }
                waiting.as_mut().poll(context)
            }).await?;
            // Only a privileged fixture may change custody while the real
            // writer holds its Character root. This models post-wait drift;
            // ordinary runtime moves serialize on that root as well.
            if depot {
                sqlx::query("UPDATE game_item_character_depot_locations SET box_number=16 WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(111).as_slice()).execute(&mut *blocker).await?;
            } else {
                sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(101).as_slice()).execute(&mut *blocker).await?;
            }
            blocker.commit().await?;
            assert!(matches!(writing.await?, Err(ItemTextWriteError::NotReachable)));
            assert_eq!(h.count("game_item_texts").await?, 0);
            assert_eq!(h.count("game_item_text_revisions").await?, 0);
            assert_eq!(h.count("game_item_text_write_receipts").await?, 0);
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        h.cleanup().await?;
        if result.is_ok() {
            eprintln!("WRITE-NATIVE-PG actual item lock wait, fresh custody refusal and rollback PASS");
        }
        result
    })
}
