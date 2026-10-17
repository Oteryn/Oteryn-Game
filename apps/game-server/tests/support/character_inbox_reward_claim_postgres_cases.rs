//! Native MAIL Inbox seam qualification, PostgreSQL 17.6 only.
//!
//! The real native Root, recovery proof, node registration/assignment, admission
//! guards and Character fence come from the existing canonical harness. The
//! source item/audit fixture below deliberately stands in for the unintegrated
//! reward-mint Inbox destination bridge: it disables item-seeding triggers only
//! inside the test-owned seed function. Every Inbox function/constraint and the
//! transaction runner execute normally, including runtime-role grants. These
//! cases qualify the Inbox sink and composition, not production DAILY/CHEST mint.

use crate::bestiary_postgres_harness as harness;
use crate::durability::DurabilityRoot;
use crate::durability::character_inbox::{
    CharacterInboxDeliveryError, REWARD_CLAIM_INBOX_SCHEMA_CANDIDATE, RewardClaimInboxError,
    RewardClaimInboxOutcome, RewardClaimInboxPlan, RewardClaimInboxPosition,
    RewardClaimInboxTransaction,
};
use crate::foundation::{
    ChannelId, CommandId, CommandRef, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1,
    ScopeOwnershipGeneration, WorldId,
};
use harness::{CHANNEL, CHARACTER, Harness, SESSION, TestResult, WORLD, debug, id, item_fence};
use sqlx::{Executor, Row};

fn plan(command_id: u64) -> TestResult<RewardClaimInboxPlan> {
    Ok(RewardClaimInboxPlan {
        command: CommandRef::new(
            GameSessionId::decode(&id(SESSION)).map_err(debug)?,
            CommandId::new(command_id).map_err(debug)?,
        ),
        character_id: crate::domain::CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?,
        world_id: WorldId::decode(&id(WORLD)).map_err(debug)?,
        channel_id: ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
        transaction_id: id(70),
        source_binding: [5; 32],
        item_instance_ids: vec![id(71), id(72)],
    })
}

fn run(tag: &'static str, case: impl AsyncFnOnce(&Harness) -> TestResult) -> TestResult {
    // Match the canonical target's PRE-ROUTING / NONCANONICAL path. Only a
    // configured PostgreSQL 17.6 run that executes the body qualifies this seam.
    let Some(admin) = harness::configured_admin() else {
        return Ok(());
    };
    harness::runtime()?.block_on(async move {
        let harness = Harness::create(admin, tag, true).await?;
        let result = async {
            install_source_fixture(&harness).await?;
            case(&harness).await
        }
        .await;
        harness.cleanup().await?;
        result
    })
}

async fn install_source_fixture(harness: &Harness) -> TestResult {
    sqlx::raw_sql(REWARD_CLAIM_INBOX_SCHEMA_CANDIDATE)
        .execute(&harness.pool)
        .await?;
    sqlx::raw_sql(r#"
      CREATE TABLE test_reward_inbox_source_writes (
        transaction_id UUID PRIMARY KEY, source_binding BYTEA NOT NULL,
        created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id()
      );
      GRANT SELECT,INSERT ON test_reward_inbox_source_writes TO oteryn_game_runtime;
      CREATE FUNCTION test_reward_inbox_seed_item(p_item UUID, p_world UUID, p_transaction UUID)
      RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER
      SET search_path=pg_catalog,public,pg_temp SET session_replication_role=replica AS $$
      BEGIN
        INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,
          definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
        VALUES(p_item,p_world,'Item','fixture:inbox.reward','fixture-r1',1,1,p_transaction);
      END $$;
      CREATE FUNCTION test_reward_inbox_add_ground(p_item UUID, p_world UUID)
      RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER
      SET search_path=pg_catalog,public,pg_temp SET session_replication_role=replica AS $$
      BEGIN
        INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,
          runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,
          content_revision,native_room_placement_context)
        VALUES(p_item,p_world,game_character_uuid_v7(),1,decode('01','hex'),decode('01','hex'),
          'm1','c1',decode('01','hex'));
      END $$;
      REVOKE ALL ON FUNCTION test_reward_inbox_seed_item(UUID,UUID,UUID),
        test_reward_inbox_add_ground(UUID,UUID) FROM PUBLIC;
      GRANT EXECUTE ON FUNCTION test_reward_inbox_seed_item(UUID,UUID,UUID),
        test_reward_inbox_add_ground(UUID,UUID) TO oteryn_game_runtime;
    "#).execute(&harness.pool).await?;
    Ok(())
}

#[derive(Clone, Copy)]
enum FixtureFault {
    None,
    WrongWorld,
    OldMint,
    Ground,
    MissingAudit,
}

async fn stage_source(
    context: &mut RewardClaimInboxTransaction<'_>,
    fault: FixtureFault,
) -> Result<(), RewardClaimInboxError> {
    let plan = context.plan().clone();
    let tuple = context.selected_type2_tuple();
    assert_eq!(context.typed_cause(), "reward_claim_mint");
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut **context.transaction())
        .await?;
    let role: String = sqlx::query_scalar("SELECT current_user")
        .fetch_one(&mut **context.transaction())
        .await?;
    assert_eq!(role, "oteryn_game_runtime");
    sqlx::query("INSERT INTO test_reward_inbox_source_writes(transaction_id,source_binding) VALUES(encode($1,'hex')::uuid,$2)")
        .bind(plan.transaction_id.as_slice()).bind(plan.source_binding.as_slice())
        .execute(&mut **context.transaction()).await?;
    for (index, item) in plan.item_instance_ids.iter().enumerate() {
        let is_last = index + 1 == plan.item_instance_ids.len();
        let world = if is_last && matches!(fault, FixtureFault::WrongWorld) {
            id(80)
        } else {
            *plan.world_id.as_bytes()
        };
        let transaction = if is_last && matches!(fault, FixtureFault::OldMint) {
            id(81)
        } else {
            plan.transaction_id
        };
        sqlx::query("SELECT test_reward_inbox_seed_item(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
            .bind(item.as_slice()).bind(world.as_slice()).bind(transaction.as_slice())
            .execute(&mut **context.transaction()).await?;
        if is_last && matches!(fault, FixtureFault::Ground) {
            sqlx::query("SELECT test_reward_inbox_add_ground(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
                .bind(item.as_slice()).bind(world.as_slice())
                .execute(&mut **context.transaction()).await?;
        }
    }
    if !matches!(fault, FixtureFault::MissingAudit) {
        // Test-owned upstream witness only: this payload intentionally does not
        // claim to be a production reward-claim audit. The native Inbox trigger
        // checks its real transaction stamp; the shared mint/audit adapter has
        // separate qualification and is not bypassed in production by this test.
        let envelope = b"TEST-ONLY-INBOX-SOURCE-WITNESS";
        sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state,published_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,1700000000000,1707776000000,$6,sha256($6),1,NULL)")
            .bind(id(90).as_slice()).bind(plan.transaction_id.as_slice())
            .bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id())
            .bind(plan.item_instance_ids[0].as_slice()).bind(envelope.as_slice())
            .execute(&mut **context.transaction()).await?;
    }
    Ok(())
}

async fn deliver_all(
    context: &mut RewardClaimInboxTransaction<'_>,
) -> Result<Vec<RewardClaimInboxPosition>, RewardClaimInboxError> {
    let items = context.plan().item_instance_ids.clone();
    let mut positions = Vec::with_capacity(items.len());
    for item in items {
        let first = context.deliver(item).await?;
        assert_eq!(
            context.deliver(item).await?,
            first,
            "same-TX retry must retain its ordinal"
        );
        positions.push(first);
    }
    Ok(positions)
}

async fn footprint(harness: &Harness) -> TestResult<Vec<i64>> {
    let mut counts = Vec::new();
    for table in [
        "test_reward_inbox_source_writes",
        "game_item_instances",
        "game_item_audit_outbox",
        "game_character_inbox_deliveries",
        "game_item_character_inbox_locations",
        "game_character_inbox_counters",
    ] {
        counts.push(harness.count(table).await?);
    }
    Ok(counts)
}

#[test]
fn native_inbox_sink_commits_fixture_source_and_full_plan_once() -> TestResult {
    run("mail_inbox_once", async |harness| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let plan = plan(u64::MAX)?;
        let mut calls = 0_usize;
        let first = harness
            .root
            .with_reward_claim_inbox_transaction(
                &authority,
                &harness.node,
                item_fence()?,
                &plan,
                &mut calls,
                |context, calls| {
                    Box::pin(async move {
                        *calls += 1;
                        stage_source(context, FixtureFault::None).await?;
                        deliver_all(context).await
                    })
                },
            )
            .await
            .map_err(debug)?;
        let RewardClaimInboxOutcome::Committed { source, positions } = first else {
            return Err("expected a native sink commit".into());
        };
        assert_eq!(source, positions);
        assert_eq!(
            positions.iter().map(|p| p.ordinal).collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(calls, 1);
        assert_eq!(footprint(harness).await?, [1, 2, 1, 2, 2, 1]);
        assert_eq!(harness.root_revision().await?, "1");
        let same_transaction: bool = sqlx::query_scalar("SELECT bool_and(d.created_xact_id=s.created_xact_id AND d.created_xact_id=a.created_xact_id) FROM game_character_inbox_deliveries d CROSS JOIN test_reward_inbox_source_writes s CROSS JOIN game_item_audit_outbox a")
            .fetch_one(&harness.pool).await?;
        assert!(same_transaction);

        let mut stale = item_fence()?;
        stale.connection_generation = ConnectionGeneration::new(99).map_err(debug)?;
        let replay = harness
            .root
            .with_reward_claim_inbox_transaction(
                &authority,
                &harness.node,
                stale,
                &plan,
                &mut calls,
                |_, calls| {
                    Box::pin(async move {
                        *calls += 1;
                        Ok(())
                    })
                },
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(replay, RewardClaimInboxOutcome::AlreadyCommitted(ref p) if *p == positions)
        );
        assert_eq!(calls, 1, "replay must not invoke the source");
        assert_eq!(
            harness
                .root
                .reconcile_reward_claim_inbox(&authority, &plan)
                .await
                .map_err(debug)?,
            Some(positions)
        );

        let mut changed = plan.clone();
        changed.item_instance_ids = vec![id(73), id(74)];
        assert!(matches!(
            harness
                .root
                .with_reward_claim_inbox_transaction(
                    &authority,
                    &harness.node,
                    item_fence()?,
                    &changed,
                    &mut calls,
                    |_, calls| Box::pin(async move {
                        *calls += 1;
                        Ok(())
                    }),
                )
                .await,
            Err(RewardClaimInboxError::ConflictingDelivery)
        ));
        let mut changed = plan.clone();
        changed.source_binding[0] ^= 1;
        assert!(matches!(
            harness
                .root
                .reconcile_reward_claim_inbox(&authority, &changed)
                .await,
            Err(RewardClaimInboxError::ConflictingDelivery)
        ));
        assert_eq!(calls, 1);
        assert_eq!(footprint(harness).await?, [1, 2, 1, 2, 2, 1]);
        Ok(())
    })
}

#[test]
fn native_inbox_source_refusal_rolls_back_and_retry_reuses_ordinals() -> TestResult {
    run("mail_inbox_rollback", async |harness| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let plan = plan(1)?;
        let mut calls = 0_usize;
        let rejected = harness
            .root
            .with_reward_claim_inbox_transaction(
                &authority,
                &harness.node,
                item_fence()?,
                &plan,
                &mut calls,
                |context, calls| {
                    Box::pin(async move {
                        *calls += 1;
                        stage_source(context, FixtureFault::None).await?;
                        deliver_all(context).await?;
                        Err::<(), _>(RewardClaimInboxError::InvalidPlan)
                    })
                },
            )
            .await;
        assert!(matches!(rejected, Err(RewardClaimInboxError::InvalidPlan)));
        assert_eq!(footprint(harness).await?, [0, 0, 0, 0, 0, 0]);
        assert_eq!(
            harness
                .root
                .reconcile_reward_claim_inbox(&authority, &plan)
                .await
                .map_err(debug)?,
            None
        );
        let retry = harness
            .root
            .with_reward_claim_inbox_transaction(
                &authority,
                &harness.node,
                item_fence()?,
                &plan,
                &mut calls,
                |context, calls| {
                    Box::pin(async move {
                        *calls += 1;
                        stage_source(context, FixtureFault::None).await?;
                        deliver_all(context).await
                    })
                },
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(retry, RewardClaimInboxOutcome::Committed {ref positions, ..}
            if positions.iter().map(|p|p.ordinal).collect::<Vec<_>>() == [1,2])
        );
        assert_eq!(calls, 2);
        assert_eq!(footprint(harness).await?, [1, 2, 1, 2, 2, 1]);
        Ok(())
    })
}

#[test]
fn native_inbox_omission_and_caught_refusal_cannot_commit() -> TestResult {
    run("mail_inbox_incomplete", async |harness| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let plan = plan(1)?;
        for caught_refusal in [false, true] {
            let mut mode = caught_refusal;
            let result = harness
                .root
                .with_reward_claim_inbox_transaction(
                    &authority,
                    &harness.node,
                    item_fence()?,
                    &plan,
                    &mut mode,
                    |context, caught| {
                        Box::pin(async move {
                            stage_source(context, FixtureFault::None).await?;
                            if *caught {
                                deliver_all(context).await?;
                                assert!(context.deliver(id(79)).await.is_err());
                            } else {
                                let first = context.plan().item_instance_ids[0];
                                context.deliver(first).await?;
                            }
                            Ok(())
                        })
                    },
                )
                .await;
            assert!(matches!(
                result,
                Err(RewardClaimInboxError::IncompleteDelivery)
            ));
            assert_eq!(footprint(harness).await?, [0, 0, 0, 0, 0, 0]);
        }
        Ok(())
    })
}

#[test]
fn native_inbox_refuses_world_old_mint_and_other_location_atomically() -> TestResult {
    run("mail_inbox_refusals", async |harness| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let plan = plan(1)?;
        for mut fault in [
            FixtureFault::WrongWorld,
            FixtureFault::OldMint,
            FixtureFault::Ground,
        ] {
            let result = harness
                .root
                .with_reward_claim_inbox_transaction(
                    &authority,
                    &harness.node,
                    item_fence()?,
                    &plan,
                    &mut fault,
                    |context, fault| {
                        Box::pin(async move {
                            stage_source(context, *fault).await?;
                            deliver_all(context).await
                        })
                    },
                )
                .await;
            match fault {
                FixtureFault::WrongWorld => assert!(matches!(
                    result,
                    Err(RewardClaimInboxError::Delivery(
                        CharacterInboxDeliveryError::WorldMismatch
                    ))
                )),
                FixtureFault::OldMint => {
                    assert!(matches!(result, Err(RewardClaimInboxError::MintNotProven)))
                }
                FixtureFault::Ground => assert!(matches!(
                    result,
                    Err(RewardClaimInboxError::Delivery(
                        CharacterInboxDeliveryError::ItemInAnotherLocation
                    ))
                )),
                _ => unreachable!(),
            }
            assert_eq!(footprint(harness).await?, [0, 0, 0, 0, 0, 0]);
            assert_eq!(harness.root_revision().await?, "1");
        }
        Ok(())
    })
}

#[test]
fn native_inbox_fence_rejects_foreign_session_scope_node_and_root() -> TestResult {
    run("mail_inbox_fences", async |harness| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let plan = plan(1)?;
        let mut calls = 0_usize;
        let valid = harness
            .root
            .with_reward_claim_inbox_transaction(
                &authority,
                &harness.node,
                item_fence()?,
                &plan,
                &mut calls,
                |_, calls| {
                    Box::pin(async move {
                        *calls += 1;
                        Err::<(), _>(RewardClaimInboxError::InvalidPlan)
                    })
                },
            )
            .await;
        assert!(matches!(valid, Err(RewardClaimInboxError::InvalidPlan)));
        assert_eq!(calls, 1, "positive authority control must reach the source");
        let base = item_fence()?;
        let mut cases = Vec::new();
        let mut stale = base;
        stale.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        cases.push(stale);
        let mut stale = base;
        stale.character_lease_generation = 2;
        cases.push(stale);
        let mut stale = base;
        stale.scope_ownership_generation = ScopeOwnershipGeneration::new(2).map_err(debug)?;
        cases.push(stale);
        let mut stale = base;
        stale.game_session_id = GameSessionId::decode(&id(51)).map_err(debug)?;
        cases.push(stale);
        let mut stale = base;
        stale.character_id = crate::domain::CharacterId::from_bytes(id(44)).map_err(debug)?;
        cases.push(stale);
        let mut stale = base;
        stale.runtime_scope =
            RuntimeScopeRefV1::channel(plan.world_id, ChannelId::decode(&id(45)).map_err(debug)?);
        cases.push(stale);
        let mut stale = base;
        stale.runtime_scope =
            RuntimeScopeRefV1::channel(WorldId::decode(&id(46)).map_err(debug)?, plan.channel_id);
        cases.push(stale);
        for stale in cases {
            let result = harness
                .root
                .with_reward_claim_inbox_transaction(
                    &authority,
                    &harness.node,
                    stale,
                    &plan,
                    &mut calls,
                    |_, calls| {
                        Box::pin(async move {
                            *calls += 1;
                            Ok(())
                        })
                    },
                )
                .await;
            assert!(matches!(
                result,
                Err(RewardClaimInboxError::AuthorityRejected)
            ));
        }
        let foreign_node = harness::register(&harness.root, 2).await?;
        assert!(matches!(
            harness
                .root
                .with_reward_claim_inbox_transaction(
                    &authority,
                    &foreign_node,
                    base,
                    &plan,
                    &mut calls,
                    |_, calls| Box::pin(async move {
                        *calls += 1;
                        Ok(())
                    }),
                )
                .await,
            Err(RewardClaimInboxError::AuthorityRejected)
        ));
        let other_root = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
        assert!(matches!(
            other_root
                .with_reward_claim_inbox_transaction(
                    &authority,
                    &harness.node,
                    base,
                    &plan,
                    &mut calls,
                    |_, calls| Box::pin(async move {
                        *calls += 1;
                        Ok(())
                    }),
                )
                .await,
            Err(RewardClaimInboxError::AuthorityRejected)
        ));
        assert_eq!(calls, 1);
        assert_eq!(footprint(harness).await?, [0, 0, 0, 0, 0, 0]);
        Ok(())
    })
}

#[test]
fn native_inbox_stale_recovery_blocks_delivery_and_reconciliation() -> TestResult {
    run("mail_inbox_recovery", async |harness| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let plan = plan(1)?;
        let mut tx = harness.pool.begin().await?;
        tx.execute("SET LOCAL session_replication_role=replica")
            .await?;
        sqlx::query("INSERT INTO game_character_recovery_admissions(authority_scope_id,recovery_generation,recovery_event_id,predecessor_generation,predecessor_digest,issued_at,issuer_identity,reconciled_at) VALUES('character-primary',2,encode($1,'hex')::uuid,1,decode(repeat('ab',32),'hex'),200,'game-ops',200)")
            .bind(id(12).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let mut calls = 0_usize;
        assert!(matches!(
            harness
                .root
                .with_reward_claim_inbox_transaction(
                    &authority,
                    &harness.node,
                    item_fence()?,
                    &plan,
                    &mut calls,
                    |_, calls| Box::pin(async move {
                        *calls += 1;
                        Ok(())
                    }),
                )
                .await,
            Err(RewardClaimInboxError::Unavailable(_))
        ));
        assert!(matches!(
            harness
                .root
                .reconcile_reward_claim_inbox(&authority, &plan)
                .await,
            Err(RewardClaimInboxError::Unavailable(_))
        ));
        assert_eq!(calls, 0);
        assert_eq!(footprint(harness).await?, [0, 0, 0, 0, 0, 0]);
        Ok(())
    })
}

#[test]
fn native_inbox_sql_guard_refuses_partial_plan_and_missing_same_tx_audit() -> TestResult {
    run("mail_inbox_guards", async |harness| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let plan = plan(1)?;
        for mut partial in [true, false] {
            let result = harness.root.with_reward_claim_inbox_transaction(
                &authority, &harness.node, item_fence()?, &plan, &mut partial,
                |context, partial| Box::pin(async move {
                    stage_source(context, if *partial { FixtureFault::None } else { FixtureFault::MissingAudit }).await?;
                    let first = context.plan().item_instance_ids[0];
                    context.deliver(first).await?;
                    if !*partial { deliver_all(context).await?; }
                    let checked = sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
                        .execute(&mut **context.transaction()).await;
                    assert!(matches!(checked, Err(ref e) if e.as_database_error().and_then(|e| e.code()).as_deref() == Some("23514")));
                    Err::<(),_>(RewardClaimInboxError::IncompleteDelivery)
                }),
            ).await;
            assert!(matches!(
                result,
                Err(RewardClaimInboxError::IncompleteDelivery)
            ));
            assert_eq!(footprint(harness).await?, [0, 0, 0, 0, 0, 0]);
        }
        Ok(())
    })
}

#[test]
fn native_inbox_runtime_executes_only_the_narrow_seam() -> TestResult {
    run("mail_inbox_grants", async |harness| {
        let row = sqlx::query("SELECT has_function_privilege('oteryn_game_runtime','game_character_inbox_reward_claim_mint_deliver(uuid,uuid,uuid,uuid,bytea,uuid,numeric,smallint)','EXECUTE') AS seam, has_function_privilege('oteryn_game_runtime','game_character_inbox_deliver(uuid,uuid,uuid,text,text)','EXECUTE') AS generic, has_table_privilege('oteryn_game_runtime','game_character_inbox_deliveries','INSERT,UPDATE,DELETE,SELECT') AS journal")
            .fetch_one(&harness.pool).await?;
        assert!(row.try_get::<bool, _>("seam")?);
        assert!(!row.try_get::<bool, _>("generic")?);
        assert!(!row.try_get::<bool, _>("journal")?);
        for statement in [
            "SELECT * FROM game_character_inbox_deliver(game_character_uuid_v7(),game_character_uuid_v7(),game_character_uuid_v7(),'REWARD_CLAIM_MINT','anything')",
            "SELECT * FROM game_character_inbox_deliveries",
            "DELETE FROM game_item_character_inbox_locations",
        ] {
            let mut tx = harness.pool.begin().await?;
            tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
            let result = sqlx::query(sqlx::AssertSqlSafe(statement.to_owned()))
                .execute(&mut *tx)
                .await;
            assert!(
                matches!(result, Err(ref e) if e.as_database_error().and_then(|e|e.code()).as_deref() == Some("42501")),
                "{statement}"
            );
            tx.rollback().await?;
        }
        Ok(())
    })
}
