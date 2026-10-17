//! Native MAIL Inbox seam qualification, PostgreSQL 17.6 only.
//!
//! Every case requires the exact installed SQLx migration 0101. Fresh positives
//! use the real native reward mint/receipt/audit bridge under the runtime role.
//! Only source-owned claim/line records are a disclosed fixture. Refusals use
//! actual native begin/finish or a genuinely committed Ground MINT; no value
//! seed, private schema copy, fake audit bytes or disabled guard is used.

use super::reward_claim_inbox_mint::{
    IncompleteProof, SourceMode, SourceOwner, footprint, incomplete_proof, plan as source_plan,
    request, reservation, return_connection, run, source_write,
};
use crate::bestiary_postgres_harness as harness;
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_inbox::{
    CharacterInboxDeliveryError, RewardClaimInboxError, RewardClaimInboxMintCandidate,
    RewardClaimInboxOutcome, RewardClaimInboxPlan, RewardClaimInboxTransaction,
};
use crate::durability::item_mint::{
    CommittedItemMint, GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest,
    TypedDefinitionRef,
};
use crate::foundation::{
    ChannelId, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration,
    WorldId,
};
use harness::{CHANNEL, Harness, TestResult, WORLD, debug, id, item_fence};
use sqlx::{Executor, Row};

fn plan(command: u64) -> TestResult<RewardClaimInboxPlan> {
    source_plan(command, 2)
}

fn assert_sql_refusal(error: &sqlx::Error, code: &str, message: &str) {
    let database = error.as_database_error();
    assert_eq!(database.and_then(|e| e.code()).as_deref(), Some(code));
    assert_eq!(database.map(|e| e.message()), Some(message));
}

/// Only the deliberate lower-seam faults use this helper. Public preparation
/// has already frozen the real reservation; its actual runtime-only charge is
/// durable independently of the value transaction, exactly as in the bridge.
async fn charge_native_attempt(
    h: &Harness,
    candidate: &RewardClaimInboxMintCandidate,
) -> TestResult<(Vec<u8>, Vec<u8>, i16)> {
    let (binding, intent, before) = reservation(h).await?;
    let plan = candidate.plan();
    let mut connection = h.root.try_acquire_ready().map_err(debug)?;
    let used: i16 = sqlx::query_scalar("SELECT work_units_used FROM game_reward_claim_inbox_mint_charge(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,$4)")
        .bind(plan.command.game_session_id().as_bytes().as_slice())
        .bind(plan.command.command_id().get().to_string())
        .bind(plan.transaction_id.as_slice()).bind(&binding)
        .fetch_one(&mut *connection).await?;
    return_connection(connection).await?;
    assert_eq!(used, before + 1);
    Ok((binding, intent, used))
}

async fn begin_native_mint(
    context: &mut RewardClaimInboxTransaction<'_>,
    binding: &[u8],
) -> Result<(), RewardClaimInboxError> {
    let plan = context.plan().clone();
    sqlx::query("SELECT game_reward_claim_inbox_mint_begin(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,$4)")
        .bind(plan.command.game_session_id().as_bytes().as_slice())
        .bind(plan.command.command_id().get().to_string())
        .bind(plan.transaction_id.as_slice()).bind(binding)
        .execute(&mut **context.transaction()).await?;
    Ok(())
}

/// This is the existing canonical native Ground MINT vector, using a disclosed
/// test-only Combat death cause. All real MINT/receipt/outbox/location guards
/// execute, and the actual committed transaction identity is retained.
async fn mint_ground(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult<CommittedItemMint> {
    let definition = |family: &str, key: &str| TypedDefinitionRef {
        family: family.into(),
        production_key: key.into(),
        revision_ref: "r1".into(),
    };
    let request = ItemMintRequest {
        cause: ItemMintCause::for_test(
            WorldId::decode(&id(WORLD)).map_err(debug)?,
            ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
            ScopeOwnershipGeneration::new(1).map_err(debug)?,
            1,
            1,
            definition("LootTable", "fixture:loot.b3"),
            "fixture:purpose.drop".into(),
            0,
        ),
        item: definition("Item", "fixture:inbox.ground-refusal"),
        quantity: 1,
        ground: GroundPlacement {
            spatial_position: vec![1, 2, 3],
            corpse_ref: id(3).to_vec(),
            map_revision: "map-1".into(),
            content_revision: "content-1".into(),
            native_room_placement_context: id(6).to_vec(),
        },
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    };
    let mut candidate = h
        .root
        .freeze_item_mint(authority, &h.node, request)
        .await
        .map_err(debug)?;
    match h
        .root
        .commit_item_mint(
            &candidate.fresh_death_lane_permit().await,
            authority,
            &h.node,
            &mut candidate,
        )
        .await
        .map_err(debug)?
    {
        ItemMintOutcome::Committed(result) => Ok(result),
        other => Err(format!("expected native Ground MINT, got {other:?}").into()),
    }
}

#[test]
fn native_inbox_sink_commits_real_mint_source_and_full_plan_once() -> TestResult {
    run("mail_inbox_once", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let plan = plan(u64::MAX)?;
        let request = request(2);
        let mut source = SourceOwner::new(&plan, &request);
        let mut candidate = h
            .root
            .prepare_reward_claim_inbox_mint(&authority, &h.node, item_fence()?, &plan, request)
            .await
            .map_err(debug)?;
        let first = h
            .root
            .commit_reward_claim_inbox_mint(
                &authority,
                &h.node,
                item_fence()?,
                &mut candidate,
                &mut source,
                |context, source| Box::pin(source_write(context, source)),
            )
            .await
            .map_err(debug)?;
        let RewardClaimInboxOutcome::Committed {
            source: written,
            positions,
        } = first
        else {
            return Err("expected an actual native mint + sink commit".into());
        };
        assert_eq!(
            written, positions,
            "same-TX delivery retries retain ordinals"
        );
        assert_eq!(
            positions.iter().map(|p| p.ordinal).collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(source.calls, 1);
        assert_eq!(footprint(h).await?, [2, 0, 1, 1, 2, 2, 2, 1, 1, 2]);
        assert_eq!(h.root_revision().await?, "1");
        assert_eq!(reservation(h).await?.2, 1);
        let same_transaction: bool = sqlx::query_scalar("SELECT bool_and(d.created_xact_id=s.created_xact_id AND d.created_xact_id=a.created_xact_id AND d.created_xact_id=r.created_xact_id AND d.created_xact_id=l.created_xact_id) FROM game_character_inbox_deliveries d CROSS JOIN test_inbox_mint_claims s CROSS JOIN game_item_audit_outbox a CROSS JOIN game_reward_claim_inbox_mint_receipts r CROSS JOIN game_reward_claim_inbox_mint_lines l")
            .fetch_one(&h.pool).await?;
        assert!(same_transaction);
        let mut stale = item_fence()?;
        stale.connection_generation = ConnectionGeneration::new(99).map_err(debug)?;
        let replay = h
            .root
            .commit_reward_claim_inbox_mint(
                &authority,
                &h.node,
                stale,
                &mut candidate,
                &mut source,
                |context, source| Box::pin(source_write(context, source)),
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(replay, RewardClaimInboxOutcome::AlreadyCommitted(ref p) if *p == positions)
        );
        assert_eq!(source.calls, 1);
        assert_eq!(
            reservation(h).await?.2,
            1,
            "committed replay consumes no work"
        );
        // Preparation binds the mint intent into this plan. The original
        // source plan deliberately cannot impersonate that terminal binding.
        assert_eq!(
            h.root
                .reconcile_reward_claim_inbox(&authority, candidate.plan())
                .await
                .map_err(debug)?,
            Some(positions)
        );
        let mut changed = candidate.plan().clone();
        changed.item_instance_ids = vec![id(73), id(74)];
        assert!(matches!(
            h.root
                .with_reward_claim_inbox_transaction(
                    &authority,
                    &h.node,
                    item_fence()?,
                    &changed,
                    &mut source,
                    |context, source| Box::pin(source_write(context, source)),
                )
                .await,
            Err(RewardClaimInboxError::ConflictingDelivery)
        ));
        let mut changed = candidate.plan().clone();
        changed.source_binding[0] ^= 1;
        assert!(matches!(
            h.root
                .reconcile_reward_claim_inbox(&authority, &changed)
                .await,
            Err(RewardClaimInboxError::ConflictingDelivery)
        ));
        assert!(matches!(
            h.root.reconcile_reward_claim_inbox(&authority, &plan).await,
            Err(RewardClaimInboxError::ConflictingDelivery)
        ));
        assert_eq!(source.calls, 1);
        assert_eq!(footprint(h).await?, [2, 0, 1, 1, 2, 2, 2, 1, 1, 2]);
        Ok(())
    })
}

#[test]
fn native_inbox_source_refusal_rolls_back_and_retry_reuses_ordinals() -> TestResult {
    run("mail_inbox_rollback", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let plan = plan(1)?;
        let request = request(2);
        let mut source = SourceOwner::new(&plan, &request);
        source.mode = SourceMode::Refuse;
        let mut candidate = h
            .root
            .prepare_reward_claim_inbox_mint(&authority, &h.node, item_fence()?, &plan, request)
            .await
            .map_err(debug)?;
        let rejected = h
            .root
            .commit_reward_claim_inbox_mint(
                &authority,
                &h.node,
                item_fence()?,
                &mut candidate,
                &mut source,
                |context, source| Box::pin(source_write(context, source)),
            )
            .await;
        assert!(matches!(
            rejected,
            Err(RewardClaimInboxError::SourceRefused)
        ));
        assert_eq!(
            source.calls, 1,
            "complete native mint must reach the refusing source"
        );
        assert_eq!(footprint(h).await?, [0; 10]);
        assert_eq!(
            h.count("game_reward_claim_inbox_mint_reservations").await?,
            1
        );
        assert_eq!(reservation(h).await?.2, 1);
        assert_eq!(
            h.root
                .reconcile_reward_claim_inbox(&authority, candidate.plan())
                .await
                .map_err(debug)?,
            None
        );
        source.mode = SourceMode::Accept;
        let retry = h
            .root
            .commit_reward_claim_inbox_mint(
                &authority,
                &h.node,
                item_fence()?,
                &mut candidate,
                &mut source,
                |context, source| Box::pin(source_write(context, source)),
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(retry, RewardClaimInboxOutcome::Committed {ref positions, ..}
            if positions.iter().map(|p|p.ordinal).collect::<Vec<_>>() == [1,2])
        );
        assert_eq!(source.calls, 2);
        assert_eq!(reservation(h).await?.2, 2);
        assert_eq!(footprint(h).await?, [2, 0, 1, 1, 2, 2, 2, 1, 1, 2]);
        Ok(())
    })
}

#[test]
fn native_inbox_omission_and_caught_refusal_cannot_commit() -> TestResult {
    run("mail_inbox_incomplete", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let plan = plan(1)?;
        let request = request(2);
        let mut source = SourceOwner::new(&plan, &request);
        let mut candidate = h
            .root
            .prepare_reward_claim_inbox_mint(&authority, &h.node, item_fence()?, &plan, request)
            .await
            .map_err(debug)?;
        let (binding, _, used) = charge_native_attempt(h, &candidate).await?;
        assert_eq!(used, 1);
        let mut omission = (binding, false);
        let result = h
            .root
            .with_reward_claim_inbox_transaction(
                &authority,
                &h.node,
                item_fence()?,
                candidate.plan(),
                &mut omission,
                |context, omitted| {
                    Box::pin(async move {
                        begin_native_mint(context, &omitted.0).await?;
                        let first = context.plan().item_instance_ids[0];
                        context.deliver(first).await?;
                        omitted.1 = true;
                        Ok(())
                    })
                },
            )
            .await;
        assert!(
            omission.1,
            "real begin and first delivery must precede omission refusal"
        );
        assert!(matches!(
            result,
            Err(RewardClaimInboxError::IncompleteDelivery)
        ));
        assert_eq!(footprint(h).await?, [0; 10]);
        assert_eq!(reservation(h).await?.2, 1);
        // Public bridge delivers the COMPLETE plan and writes receipt + audit
        // before the source. Catching an unplanned delivery must still poison
        // that otherwise-complete transaction independently of omission.
        source.mode = SourceMode::CatchRefusal;
        let result = h
            .root
            .commit_reward_claim_inbox_mint(
                &authority,
                &h.node,
                item_fence()?,
                &mut candidate,
                &mut source,
                |context, source| Box::pin(source_write(context, source)),
            )
            .await;
        assert_eq!(source.calls, 1);
        assert!(matches!(
            result,
            Err(RewardClaimInboxError::IncompleteDelivery)
        ));
        assert_eq!(footprint(h).await?, [0; 10]);
        assert_eq!(reservation(h).await?.2, 2);
        Ok(())
    })
}

#[test]
fn native_inbox_refuses_wrong_root_world_old_mint_and_real_ground_atomically() -> TestResult {
    run("mail_inbox_refusals", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let minted = mint_ground(h, &authority).await?;
        let before = footprint(h).await?;
        assert_eq!(before, [1, 0, 1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(h.count("game_item_mint_receipts").await?, 1);
        assert_eq!(h.count("game_item_ground_locations").await?, 1);
        let mut plan = source_plan(1, 1)?;
        plan.item_instance_ids = vec![minted.item_instance_id];
        plan.transaction_id = minted.transaction_id;
        let mut reached_world = false;
        let result = h.root.with_reward_claim_inbox_transaction(
            &authority, &h.node, item_fence()?, &plan, &mut reached_world,
            |context, reached| Box::pin(async move {
                let plan = context.plan().clone();
                // Deliberately misbind the narrow SQL World argument while the
                // actual native Root fence remains current. This qualifies the
                // Character-root World check, not the later item-World branch.
                let error = sqlx::query("SELECT * FROM game_character_inbox_reward_claim_mint_deliver(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,encode($6,'hex')::uuid,$7::text::numeric,1::smallint)")
                    .bind(plan.item_instance_ids[0].as_slice())
                    .bind(plan.character_id.as_bytes().as_slice()).bind(id(80).as_slice())
                    .bind(plan.transaction_id.as_slice()).bind(plan.source_binding.as_slice())
                    .bind(plan.command.game_session_id().as_bytes().as_slice())
                    .bind(plan.command.command_id().get().to_string())
                    .fetch_all(&mut **context.transaction()).await
                    .err().ok_or(RewardClaimInboxError::InvalidPlan)?;
                assert_sql_refusal(&error, "OTI02", "reward Inbox Character World mismatch");
                *reached = true;
                Err::<(), _>(error.into())
            }),
        ).await;
        assert!(reached_world);
        assert!(matches!(
            result,
            Err(RewardClaimInboxError::Delivery(
                CharacterInboxDeliveryError::WorldMismatch
            ))
        ));
        assert_eq!(footprint(h).await?, before);
        for old_transaction in [true, false] {
            let mut attempted = plan.clone();
            if old_transaction {
                attempted.transaction_id = id(81);
            }
            let mut reached = false;
            let result = h
                .root
                .with_reward_claim_inbox_transaction(
                    &authority,
                    &h.node,
                    item_fence()?,
                    &attempted,
                    &mut reached,
                    |context, reached| {
                        Box::pin(async move {
                            *reached = true;
                            let item = context.plan().item_instance_ids[0];
                            context.deliver(item).await
                        })
                    },
                )
                .await;
            assert!(reached);
            if old_transaction {
                assert!(matches!(result, Err(RewardClaimInboxError::MintNotProven)));
            } else {
                assert!(matches!(
                    result,
                    Err(RewardClaimInboxError::Delivery(
                        CharacterInboxDeliveryError::ItemInAnotherLocation
                    ))
                ));
            }
            assert_eq!(footprint(h).await?, before);
            assert_eq!(h.count("game_item_ground_locations").await?, 1);
            assert_eq!(h.count("game_item_mint_receipts").await?, 1);
            assert_eq!(h.root_revision().await?, "1");
        }
        assert_eq!(
            h.count("game_reward_claim_inbox_mint_reservations").await?,
            0
        );
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
        // Registration is a real control operation on an independent native Root;
        // the value Root stays under oteryn_game_runtime throughout this case.
        let control_root = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
        assert!(control_root.maintain_ready_once().await?);
        let foreign_node = harness::register(&control_root, 2).await?;
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
        assert_eq!(footprint(harness).await?, [0; 10]);
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
        // Privileged test-only recovery fault: PostgreSQL advances while the
        // real external generation-one seal remains held. This is not a value
        // fixture, a production recovery publisher, or a disabled trigger.
        let predecessor = crate::character_recovery_fence::recovery_record_digest(seal.record())
            .map_err(debug)?;
        sqlx::query("INSERT INTO game_character_recovery_admissions(authority_scope_id,recovery_generation,recovery_event_id,predecessor_generation,predecessor_digest,issued_at,issuer_identity,reconciled_at) VALUES('character-primary',2,encode($1,'hex')::uuid,1,$2,200,'game-ops',200)")
            .bind(id(12).as_slice()).bind(predecessor.as_slice()).execute(&mut *tx).await?;
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
        assert_eq!(footprint(harness).await?, [0; 10]);
        Ok(())
    })
}

#[test]
fn native_inbox_sql_guard_refuses_partial_plan_and_missing_same_tx_audit() -> TestResult {
    run("mail_inbox_guards", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let source = plan(1)?;
        let request = request(2);
        let candidate = h
            .root
            .prepare_reward_claim_inbox_mint(
                &authority,
                &h.node,
                item_fence()?,
                &source,
                request.clone(),
            )
            .await
            .map_err(debug)?;
        let (binding, _, used) = charge_native_attempt(h, &candidate).await?;
        assert_eq!(used, 1);
        let mut partial = (binding, false);
        let result = h.root.with_reward_claim_inbox_transaction(
            &authority, &h.node, item_fence()?, candidate.plan(), &mut partial,
            |context, partial| Box::pin(async move {
                begin_native_mint(context, &partial.0).await?;
                let plan = context.plan().clone();
                context.deliver(plan.item_instance_ids[0]).await?;
                // Deliberately ask the real finisher to complete only one of
                // two actual deliveries. This must fail its specific missing-
                // delivery check before any deferred receipt/audit guard.
                let error = sqlx::query("SELECT game_reward_claim_inbox_mint_finish(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,$4,1,1,$5)")
                    .bind(plan.command.game_session_id().as_bytes().as_slice())
                    .bind(plan.command.command_id().get().to_string())
                    .bind(plan.transaction_id.as_slice()).bind(&partial.0)
                    .bind([0_u8; 32].as_slice())
                    .execute(&mut **context.transaction()).await
                    .err().ok_or(RewardClaimInboxError::InvalidPlan)?;
                assert_sql_refusal(&error, "23514", "Inbox mint completion lacks its exact same-transaction delivery");
                partial.1 = true;
                Err::<(), _>(error.into())
            }),
        ).await;
        assert!(partial.1, "the exact finisher refusal must execute");
        assert!(result.is_err());
        assert_eq!(footprint(h).await?, [0; 10]);
        let (binding, intent, used) = charge_native_attempt(h, &candidate).await?;
        assert_eq!(used, 2);
        let mut missing_audit = (
            IncompleteProof {
                binding,
                intent,
                source_binding: source.source_binding,
                event: *candidate.event_id(),
                occurred: candidate.occurred_at_unix_ms(),
                request,
                omit_receipt: false,
                reached_end: false,
            },
            false,
        );
        let result = h.root.with_reward_claim_inbox_transaction(
            &authority, &h.node, item_fence()?, candidate.plan(), &mut missing_audit,
            |context, owner| Box::pin(async move {
                incomplete_proof(context, &mut owner.0).await?;
                // The entire real plan, placements, immutable receipt and
                // native encoded audit hash exist. Only the outbox is omitted.
                // Ask for this one proof guard so another deferred constraint
                // cannot mask the intended missing-audit refusal.
                // Runtime cannot read the private receipt table. Successful
                // native finish above is its receipt witness; outbox SELECT is
                // an existing runtime grant and confirms only that half is absent.
                let outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_audit_outbox")
                    .fetch_one(&mut **context.transaction()).await?;
                assert_eq!(outbox, 0);
                let error = sqlx::query("SET CONSTRAINTS game_reward_claim_inbox_receipt_proven IMMEDIATE")
                    .execute(&mut **context.transaction()).await
                    .err().ok_or(RewardClaimInboxError::InvalidPlan)?;
                assert_sql_refusal(&error, "23514", "fresh reward Inbox mint, full plan, immutable receipt and native audit must commit together");
                owner.1 = true;
                Err::<(), _>(error.into())
            }),
        ).await;
        assert!(missing_audit.0.reached_end);
        assert!(
            missing_audit.1,
            "the exact missing-audit receipt guard must execute"
        );
        assert!(result.is_err());
        assert_eq!(footprint(h).await?, [0; 10]);
        assert_eq!(reservation(h).await?.2, 2);
        Ok(())
    })
}

#[test]
fn native_inbox_runtime_executes_only_the_narrow_seam() -> TestResult {
    run("mail_inbox_grants", async |h| {
        let row = sqlx::query("SELECT has_function_privilege('oteryn_game_runtime','game_character_inbox_reward_claim_mint_deliver(uuid,uuid,uuid,uuid,bytea,uuid,numeric,smallint)','EXECUTE') AS seam, has_function_privilege('oteryn_game_runtime','game_character_inbox_reward_claim_mint_lookup(uuid,uuid,uuid,bytea,uuid,numeric,smallint)','EXECUTE') AS lookup, has_function_privilege('oteryn_game_runtime','game_character_inbox_deliver(uuid,uuid,uuid,text,text)','EXECUTE') AS generic, has_table_privilege('oteryn_game_runtime','game_character_inbox_deliveries','INSERT,UPDATE,DELETE,SELECT') AS journal")
            .fetch_one(&h.pool).await?;
        assert!(row.try_get::<bool, _>("seam")?);
        assert!(row.try_get::<bool, _>("lookup")?);
        assert!(!row.try_get::<bool, _>("generic")?);
        assert!(!row.try_get::<bool, _>("journal")?);
        for statement in [
            "SELECT * FROM game_character_inbox_deliver(game_character_uuid_v7(),game_character_uuid_v7(),game_character_uuid_v7(),'REWARD_CLAIM_MINT','anything')",
            "SELECT * FROM game_character_inbox_deliveries",
            "SELECT * FROM game_reward_claim_inbox_mint_reservations",
            "SELECT * FROM game_reward_claim_inbox_mint_receipts",
            "SELECT * FROM game_reward_claim_inbox_mint_lines",
            "DELETE FROM game_reward_claim_inbox_mint_reservations",
            "DELETE FROM game_reward_claim_inbox_mint_receipts",
            "DELETE FROM game_reward_claim_inbox_mint_lines",
            "DELETE FROM game_item_character_inbox_locations",
        ] {
            let mut tx = h.pool.begin().await?;
            tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
            let result = sqlx::query(sqlx::AssertSqlSafe(statement.to_owned()))
                .execute(&mut *tx)
                .await;
            assert!(
                matches!(result, Err(ref e)
                if e.as_database_error().and_then(|e|e.code()).as_deref() == Some("42501")),
                "{statement}"
            );
            tx.rollback().await?;
        }
        Ok(())
    })
}
