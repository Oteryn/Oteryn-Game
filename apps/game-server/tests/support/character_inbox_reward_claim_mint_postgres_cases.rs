//! Native PostgreSQL 18.6 fresh reward MINT -> Inbox qualification.
//!
//! The real Root, fences, runtime role, fresh instances, key rows, Inbox writer,
//! immutable receipts, registered tag-4 encoding and all deferred guards execute.
//! Only source-owned claim/line tables are a disclosed fixture: DAILY's claim
//! producer is a separate consumer. No item/audit seed or disabled guard is used.

use crate::bestiary_postgres_harness as harness;
use crate::durability::character_inbox::{
    RewardClaimInboxError, RewardClaimInboxMintCandidate, RewardClaimInboxMintOutput,
    RewardClaimInboxMintRequest, RewardClaimInboxOutcome, RewardClaimInboxPlan,
    RewardClaimInboxPosition, RewardClaimInboxTransaction,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_mint_audit::{
    self as common, OneItemStateV1, OneItemTypedDefinitionRevisionV1,
};
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass};
use crate::durability::item_transfer_audit::OneItemCommandRefV1;
use crate::durability::reward_claim_mint_audit::{
    self as audit, OneItemRewardClaimCauseV1, OneItemRewardClaimMintV1,
    RewardClaimInboxMintOutputV1, RewardClaimInboxMintV1, RewardClaimMintEventIdentity,
};
use crate::foundation::{
    ChannelId, CommandId, CommandRef, ConnectionGeneration, GameSessionId,
    ScopeOwnershipGeneration, WorldId,
};
use harness::{CHANNEL, CHARACTER, Harness, SESSION, TestResult, WORLD, debug, id, item_fence};
use sha2::{Digest, Sha256};
use sqlx::Row;
use sqlx::pool::{PoolConnection, PoolConnectionReturnDisposition};
use std::time::{Duration, Instant};

pub(super) async fn return_connection(
    mut connection: PoolConnection<sqlx::Postgres>,
) -> TestResult {
    let disposition = connection
        .return_to_pool_observed_until(Instant::now() + Duration::from_secs(5))
        .await;
    if !matches!(disposition, PoolConnectionReturnDisposition::ReturnedToIdle) {
        return Err(format!("native runtime holder was not returned: {disposition:?}").into());
    }
    Ok(())
}

pub(super) fn plan(command: u64, count: u8) -> TestResult<RewardClaimInboxPlan> {
    Ok(RewardClaimInboxPlan {
        command: CommandRef::new(
            GameSessionId::decode(&id(SESSION)).map_err(debug)?,
            CommandId::new(command).map_err(debug)?,
        ),
        character_id: crate::domain::CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?,
        world_id: WorldId::decode(&id(WORLD)).map_err(debug)?,
        channel_id: ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
        transaction_id: id(100),
        source_binding: [5; 32],
        item_instance_ids: (0..count).map(|i| id(110 + i)).collect(),
    })
}

pub(super) fn request(count: u8) -> RewardClaimInboxMintRequest {
    RewardClaimInboxMintRequest {
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: "fixture:source.claim".into(),
            revision_ref: "claim-r1".into(),
        },
        content_revision: "content-r1".into(),
        ruleset_revision: "ruleset-r1".into(),
        sim_revision: "sim-r1".into(),
        outputs: (0..count)
            .map(|i| RewardClaimInboxMintOutput {
                source_line: u32::from(i),
                item: ItemDefinitionFacts {
                    weight: None,
                    definition: TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: format!("fixture:reward.{i}"),
                        revision_ref: "item-r1".into(),
                    },
                    stack: ItemStackClass::Stackable {
                        proven_maximum: Some(100),
                    },
                    container_capacity: None,
                    container_slot_equip_pattern: false,
                },
                quantity: u32::from(i) + 1,
                key_number: None,
            })
            .collect(),
    }
}

pub(super) fn run(tag: &'static str, case: impl AsyncFnOnce(&Harness) -> TestResult) -> TestResult {
    let Some(admin) = harness::configured_admin() else {
        return Ok(());
    };
    harness::runtime()?.block_on(async move {
        let h = Harness::create(admin, tag, true).await?;
        let result = async {
            super::require_mail_migration(&h.pool).await?;
            sqlx::raw_sql(
                r#"
                CREATE TABLE test_inbox_mint_claims (
                    transaction_id UUID PRIMARY KEY, source_binding BYTEA NOT NULL,
                    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id());
                CREATE TABLE test_inbox_mint_bindings (
                    item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances,
                    transaction_id UUID NOT NULL REFERENCES test_inbox_mint_claims,
                    source_line BIGINT NOT NULL, ordinal NUMERIC(20,0) NOT NULL,
                    created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id());
                GRANT SELECT, INSERT ON test_inbox_mint_claims, test_inbox_mint_bindings
                    TO oteryn_game_runtime;
            "#,
            )
            .execute(&h.pool)
            .await?;
            // The native test Root has one physical connection. Apply the real
            // runtime role before preparation, budget charge and every value write.
            let mut connection = h.root.try_acquire_ready().map_err(debug)?;
            sqlx::query("SET ROLE oteryn_game_runtime")
                .execute(&mut *connection)
                .await?;
            let role: String = sqlx::query_scalar("SELECT current_user")
                .fetch_one(&mut *connection)
                .await?;
            assert_eq!(role, "oteryn_game_runtime");
            return_connection(connection).await?;
            case(&h).await
        }
        .await;
        h.cleanup().await?;
        result?;
        eprintln!(
            "native MAIL migration0101 body + cleanup PASS; source claim fixture only: {tag}"
        );
        Ok(())
    })
}

#[derive(Clone, Copy)]
pub(super) enum SourceMode {
    Accept,
    Refuse,
    CatchRefusal,
}

pub(super) struct SourceOwner {
    pub(super) calls: usize,
    binding: [u8; 32],
    lines: Vec<u32>,
    pub(super) mode: SourceMode,
}

impl SourceOwner {
    pub(super) fn new(plan: &RewardClaimInboxPlan, request: &RewardClaimInboxMintRequest) -> Self {
        Self {
            calls: 0,
            binding: plan.source_binding,
            lines: request.outputs.iter().map(|o| o.source_line).collect(),
            mode: SourceMode::Accept,
        }
    }
}

pub(super) async fn source_write(
    context: &mut RewardClaimInboxTransaction<'_>,
    owner: &mut SourceOwner,
) -> Result<Vec<RewardClaimInboxPosition>, RewardClaimInboxError> {
    let role: String = sqlx::query_scalar("SELECT current_user")
        .fetch_one(&mut **context.transaction())
        .await?;
    assert_eq!(role, "oteryn_game_runtime");
    owner.calls += 1;
    let plan = context.plan().clone();
    sqlx::query("INSERT INTO test_inbox_mint_claims(transaction_id,source_binding) VALUES(encode($1,'hex')::uuid,$2)")
        .bind(plan.transaction_id.as_slice()).bind(owner.binding.as_slice())
        .execute(&mut **context.transaction()).await?;
    let mut positions = Vec::new();
    for (item, line) in plan.item_instance_ids.iter().zip(&owner.lines) {
        let position = context.deliver(*item).await?;
        sqlx::query("INSERT INTO test_inbox_mint_bindings(item_instance_id,transaction_id,source_line,ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4::text::numeric)")
            .bind(item.as_slice()).bind(plan.transaction_id.as_slice()).bind(i64::from(*line))
            .bind(position.ordinal.to_string()).execute(&mut **context.transaction()).await?;
        positions.push(position);
    }
    match owner.mode {
        SourceMode::Accept => {}
        SourceMode::Refuse => return Err(RewardClaimInboxError::SourceRefused),
        SourceMode::CatchRefusal => {
            // All planned items, receipt, audit and source bindings exist first.
            // A caught unplanned delivery is independently poisoned, not omission.
            assert!(matches!(
                context.deliver(id(79)).await,
                Err(RewardClaimInboxError::IncompleteDelivery)
            ));
        }
    }
    Ok(positions)
}

pub(super) async fn footprint(h: &Harness) -> TestResult<Vec<i64>> {
    let mut counts = Vec::new();
    for table in [
        "game_item_instances",
        "game_item_keys",
        "game_item_audit_outbox",
        "game_reward_claim_inbox_mint_receipts",
        "game_reward_claim_inbox_mint_lines",
        "game_character_inbox_deliveries",
        "game_item_character_inbox_locations",
        "game_character_inbox_counters",
        "test_inbox_mint_claims",
        "test_inbox_mint_bindings",
    ] {
        counts.push(h.count(table).await?);
    }
    Ok(counts)
}

pub(super) async fn reservation(h: &Harness) -> TestResult<(Vec<u8>, Vec<u8>, i16)> {
    let row = sqlx::query("SELECT plan_binding,mint_intent_sha256,work_units_used FROM game_reward_claim_inbox_mint_reservations")
        .fetch_one(&h.pool).await?;
    Ok((
        row.try_get("plan_binding")?,
        row.try_get("mint_intent_sha256")?,
        row.try_get("work_units_used")?,
    ))
}

#[test]
fn native_inbox_fresh_twenty_keyed_outputs_audit_and_replay_are_one_commit() -> TestResult {
    run("mail_mint_twenty", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let plan = plan(u64::MAX, 20)?;
        let mut request = request(20);
        for (index, key) in [(18, 1), (19, u16::MAX)] {
            request.outputs[index].item.stack = ItemStackClass::NonStackable;
            request.outputs[index].quantity = 1;
            request.outputs[index].key_number = Some(key);
        }
        request.outputs[19].source_line = u32::MAX;
        let mut owner = SourceOwner::new(&plan, &request);
        let mut candidate = h
            .root
            .prepare_reward_claim_inbox_mint(
                &authority,
                &h.node,
                item_fence()?,
                &plan,
                request.clone(),
            )
            .await
            .map_err(debug)?;
        assert_eq!(candidate.source_binding(), &plan.source_binding);
        assert_eq!(candidate.work_units_used(), 0);
        assert_eq!(footprint(h).await?, [0; 10]);
        let again = h
            .root
            .prepare_reward_claim_inbox_mint(
                &authority,
                &h.node,
                item_fence()?,
                &plan,
                request.clone(),
            )
            .await
            .map_err(debug)?;
        assert_eq!(again.event_id(), candidate.event_id());
        assert_eq!(again.occurred_at_unix_ms(), candidate.occurred_at_unix_ms());
        let outcome = h
            .root
            .commit_reward_claim_inbox_mint(
                &authority,
                &h.node,
                item_fence()?,
                &mut candidate,
                &mut owner,
                |context, owner| Box::pin(source_write(context, owner)),
            )
            .await
            .map_err(debug)?;
        let RewardClaimInboxOutcome::Committed { source, positions } = outcome else {
            return Err("expected actual fresh Inbox mint".into());
        };
        assert_eq!(source, positions);
        assert_eq!(
            positions.iter().map(|p| p.ordinal).collect::<Vec<_>>(),
            (1..=20).collect::<Vec<_>>()
        );
        assert_eq!(footprint(h).await?, [20, 2, 1, 1, 20, 20, 20, 1, 1, 20]);
        assert_eq!(owner.calls, 1);
        assert_eq!(h.root_revision().await?, "1");
        let row = sqlx::query("SELECT envelope,envelope_sha256,schema_revision,retention_profile_id FROM game_item_audit_outbox")
            .fetch_one(&h.pool).await?;
        let wire: Vec<u8> = row.try_get("envelope")?;
        let stored_hash: Vec<u8> = row.try_get("envelope_sha256")?;
        assert_eq!(Sha256::digest(&wire).to_vec(), stored_hash);
        let (envelope, payload) = audit::decode_reward_claim_mint_envelope(&wire).map_err(debug)?;
        assert_eq!(envelope.event_id, candidate.event_id().as_slice());
        assert_eq!(
            envelope.occurred_at_unix_ms,
            candidate.occurred_at_unix_ms()
        );
        assert_eq!(
            i64::from(envelope.event_schema_revision),
            row.try_get::<i64, _>("schema_revision")?
        );
        assert_eq!(
            envelope.retention_profile_id,
            row.try_get::<String, _>("retention_profile_id")?
        );
        assert_eq!(envelope.command_id, Some(u64::MAX));
        assert_eq!(envelope.connection_generation, Some(1));
        let inbox = payload.inbox.ok_or("missing native Inbox audit payload")?;
        let (binding, intent, used) = reservation(h).await?;
        assert_eq!(used, 1);
        assert_eq!(inbox.plan_binding, binding);
        assert_eq!(inbox.mint_intent_sha256, intent);
        assert_eq!(inbox.source_binding, plan.source_binding);
        assert_eq!(inbox.expected_character_lease_generation, 1);
        assert_eq!(inbox.outputs.len(), 20);
        for (index, output) in inbox.outputs.iter().enumerate() {
            let after = output.after.as_ref().ok_or("missing actual minted state")?;
            assert_eq!(after.item_instance_id, plan.item_instance_ids[index]);
            assert_eq!(after.quantity, request.outputs[index].quantity);
            assert_eq!(
                after.key_number,
                request.outputs[index]
                    .key_number
                    .map(u32::from)
                    .unwrap_or(0)
            );
            assert_eq!(output.source_line, request.outputs[index].source_line);
            assert_eq!(output.ordinal, positions[index].ordinal);
        }
        let xacts: i64 = sqlx::query_scalar("SELECT count(DISTINCT x) FROM (SELECT created_xact_id x FROM game_item_audit_outbox UNION ALL SELECT created_xact_id FROM game_item_keys UNION ALL SELECT created_xact_id FROM game_reward_claim_inbox_mint_receipts UNION ALL SELECT created_xact_id FROM game_reward_claim_inbox_mint_lines UNION ALL SELECT created_xact_id FROM game_character_inbox_deliveries UNION ALL SELECT created_xact_id FROM test_inbox_mint_claims UNION ALL SELECT created_xact_id FROM test_inbox_mint_bindings) v")
            .fetch_one(&h.pool).await?;
        assert_eq!(
            xacts, 1,
            "mint keys, receipt, audit, Inbox and source share the physical transaction"
        );
        let mut stale = item_fence()?;
        stale.connection_generation = ConnectionGeneration::new(99).map_err(debug)?;
        let replay = h
            .root
            .commit_reward_claim_inbox_mint(
                &authority,
                &h.node,
                stale,
                &mut candidate,
                &mut owner,
                |context, owner| Box::pin(source_write(context, owner)),
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(replay, RewardClaimInboxOutcome::AlreadyCommitted(ref p) if *p == positions)
        );
        assert_eq!(owner.calls, 1);
        assert_eq!(reservation(h).await?.2, 1);
        let replay = h
            .root
            .prepare_reward_claim_inbox_mint(&authority, &h.node, stale, &plan, request.clone())
            .await
            .map_err(debug)?;
        assert_eq!(replay.event_id(), candidate.event_id());
        request.outputs[0].quantity += 1;
        assert!(
            h.root
                .prepare_reward_claim_inbox_mint(&authority, &h.node, item_fence()?, &plan, request)
                .await
                .is_err()
        );
        assert_eq!(footprint(h).await?, [20, 2, 1, 1, 20, 20, 20, 1, 1, 20]);
        Ok(())
    })
}

#[test]
fn native_inbox_source_refusal_and_caught_delivery_roll_back_complete_mint() -> TestResult {
    run("mail_mint_refusal", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let plan = plan(9, 2)?;
        let request = request(2);
        let mut owner = SourceOwner::new(&plan, &request);
        let mut candidate = h
            .root
            .prepare_reward_claim_inbox_mint(&authority, &h.node, item_fence()?, &plan, request)
            .await
            .map_err(debug)?;
        let event = *candidate.event_id();
        let occurred = candidate.occurred_at_unix_ms();
        for (index, mode) in [SourceMode::Refuse, SourceMode::CatchRefusal]
            .into_iter()
            .enumerate()
        {
            owner.mode = mode;
            let error = h
                .root
                .commit_reward_claim_inbox_mint(
                    &authority,
                    &h.node,
                    item_fence()?,
                    &mut candidate,
                    &mut owner,
                    |context, owner| Box::pin(source_write(context, owner)),
                )
                .await;
            assert!(if index == 0 {
                matches!(error, Err(RewardClaimInboxError::SourceRefused))
            } else {
                matches!(error, Err(RewardClaimInboxError::IncompleteDelivery))
            });
            assert_eq!(
                owner.calls,
                index + 1,
                "source ran after the full native mint path"
            );
            assert_eq!(footprint(h).await?, [0; 10]);
            assert_eq!(usize::from(candidate.work_units_used()), index + 1);
            assert_eq!(usize::try_from(reservation(h).await?.2)?, index + 1);
        }
        owner.mode = SourceMode::Accept;
        let outcome = h
            .root
            .commit_reward_claim_inbox_mint(
                &authority,
                &h.node,
                item_fence()?,
                &mut candidate,
                &mut owner,
                |context, owner| Box::pin(source_write(context, owner)),
            )
            .await
            .map_err(debug)?;
        let RewardClaimInboxOutcome::Committed { positions, .. } = outcome else {
            return Err("third bounded attempt must actually commit".into());
        };
        assert_eq!(
            positions.iter().map(|p| p.ordinal).collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(candidate.event_id(), &event);
        assert_eq!(candidate.occurred_at_unix_ms(), occurred);
        assert_eq!(candidate.work_units_used(), 3);
        assert_eq!(owner.calls, 3);
        assert_eq!(footprint(h).await?, [2, 0, 1, 1, 2, 2, 2, 1, 1, 2]);
        Ok(())
    })
}

#[test]
fn native_inbox_prepare_and_commit_require_the_real_current_item_fence() -> TestResult {
    run("mail_mint_fence", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let plan = plan(10, 2)?;
        let request = request(2);
        for mutation in 0..4 {
            let mut stale = item_fence()?;
            match mutation {
                0 => stale.game_session_id = GameSessionId::decode(&id(90)).map_err(debug)?,
                1 => stale.connection_generation = ConnectionGeneration::new(2).map_err(debug)?,
                2 => stale.character_lease_generation = 2,
                _ => {
                    stale.scope_ownership_generation =
                        ScopeOwnershipGeneration::new(2).map_err(debug)?
                }
            }
            assert!(matches!(
                h.root
                    .prepare_reward_claim_inbox_mint(
                        &authority,
                        &h.node,
                        stale,
                        &plan,
                        request.clone()
                    )
                    .await,
                Err(RewardClaimInboxError::AuthorityRejected)
            ));
            assert_eq!(
                h.count("game_reward_claim_inbox_mint_reservations").await?,
                0
            );
        }
        let mut owner = SourceOwner::new(&plan, &request);
        let mut candidate = h
            .root
            .prepare_reward_claim_inbox_mint(&authority, &h.node, item_fence()?, &plan, request)
            .await
            .map_err(debug)?;
        let mut stale = item_fence()?;
        stale.character_lease_generation = 2;
        assert!(matches!(
            h.root
                .commit_reward_claim_inbox_mint(
                    &authority,
                    &h.node,
                    stale,
                    &mut candidate,
                    &mut owner,
                    |context, owner| Box::pin(source_write(context, owner))
                )
                .await,
            Err(RewardClaimInboxError::AuthorityRejected)
        ));
        assert_eq!(owner.calls, 0);
        assert_eq!(footprint(h).await?, [0; 10]);
        assert_eq!(
            reservation(h).await?.2,
            1,
            "a failed value attempt still consumes its durable work unit"
        );
        Ok(())
    })
}

#[test]
fn native_inbox_retry_budget_survives_reprepare_and_blocks_the_fourth_attempt() -> TestResult {
    run("mail_mint_budget", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let plan = plan(11, 2)?;
        let request = request(2);
        let mut owner = SourceOwner::new(&plan, &request);
        owner.mode = SourceMode::Refuse;
        let mut candidate = h
            .root
            .prepare_reward_claim_inbox_mint(
                &authority,
                &h.node,
                item_fence()?,
                &plan,
                request.clone(),
            )
            .await
            .map_err(debug)?;
        for used in 1..=3 {
            assert!(matches!(
                h.root
                    .commit_reward_claim_inbox_mint(
                        &authority,
                        &h.node,
                        item_fence()?,
                        &mut candidate,
                        &mut owner,
                        |context, owner| Box::pin(source_write(context, owner))
                    )
                    .await,
                Err(RewardClaimInboxError::SourceRefused)
            ));
            assert_eq!(candidate.work_units_used(), used);
            assert_eq!(footprint(h).await?, [0; 10]);
        }
        let old_event = *candidate.event_id();
        candidate = h
            .root
            .prepare_reward_claim_inbox_mint(&authority, &h.node, item_fence()?, &plan, request)
            .await
            .map_err(debug)?;
        assert_eq!(candidate.event_id(), &old_event);
        assert_eq!(candidate.work_units_used(), 3);
        owner.mode = SourceMode::Accept;
        assert!(matches!(
            h.root
                .commit_reward_claim_inbox_mint(
                    &authority,
                    &h.node,
                    item_fence()?,
                    &mut candidate,
                    &mut owner,
                    |context, owner| Box::pin(source_write(context, owner))
                )
                .await,
            Err(RewardClaimInboxError::CapacityExceeded)
        ));
        assert_eq!(owner.calls, 3);
        assert_eq!(reservation(h).await?.2, 3);
        assert_eq!(footprint(h).await?, [0; 10]);
        Ok(())
    })
}

fn code(error: &sqlx::Error) -> Option<String> {
    error
        .as_database_error()
        .and_then(|e| e.code())
        .map(|code| code.into_owned())
}

async fn legacy_reserve(h: &Harness, command: u64) -> Result<(), sqlx::Error> {
    // Boundary-only legacy reservation: no item, RewardClaim or audit is seeded.
    sqlx::query("INSERT INTO game_reward_claim_mint_reservations(game_session_id,command_id,character_id,world_id,channel_id,claim_family,claim_production_key,claim_revision_ref,intent_binding,transaction_id,event_id,item_instance_id,occurred_at,work_units_used,reserved_at) VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,'RewardClaim','fixture:legacy.claim','r1',$6,encode($7,'hex')::uuid,encode($8,'hex')::uuid,encode($9,'hex')::uuid,1700000000000,0,1700000000000)")
        .bind(id(SESSION).as_slice()).bind(command.to_string()).bind(id(CHARACTER).as_slice())
        .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).bind([1_u8; 33].as_slice())
        .bind(id(80).as_slice()).bind(id(81).as_slice()).bind(id(82).as_slice())
        .execute(&h.pool).await?;
    Ok(())
}

#[test]
fn native_inbox_runtime_grants_immutability_and_legacy_command_exclusion() -> TestResult {
    run("mail_mint_grants", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let source = plan(u64::MAX, 2)?;
        let _candidate = h
            .root
            .prepare_reward_claim_inbox_mint(
                &authority,
                &h.node,
                item_fence()?,
                &source,
                request(2),
            )
            .await
            .map_err(debug)?;
        let mut connection = h.root.try_acquire_ready().map_err(debug)?;
        for sql in [
            "SELECT * FROM game_reward_claim_inbox_mint_reservations",
            "SELECT * FROM game_reward_claim_inbox_mint_receipts",
            "SELECT * FROM game_reward_claim_inbox_mint_lines",
            "UPDATE game_reward_claim_inbox_mint_reservations SET work_units_used=1",
            "DELETE FROM game_reward_claim_inbox_mint_reservations",
            "DELETE FROM game_item_character_inbox_locations",
        ] {
            let error = sqlx::raw_sql(sql)
                .execute(&mut *connection)
                .await
                .err()
                .ok_or("private runtime table access unexpectedly succeeded")?;
            assert_eq!(code(&error).as_deref(), Some("42501"));
        }
        let raw_grant: bool = sqlx::query_scalar("SELECT has_function_privilege(current_user,'game_character_inbox_deliver(uuid,uuid,uuid,text,text)','EXECUTE')")
            .fetch_one(&mut *connection).await?;
        assert!(
            !raw_grant,
            "runtime can execute the reward seam, not the generic raw Inbox writer"
        );
        return_connection(connection).await?;
        for sql in [
            "UPDATE game_reward_claim_inbox_mint_reservations SET source_binding=decode(repeat('06',32),'hex')",
            "DELETE FROM game_reward_claim_inbox_mint_reservations",
        ] {
            let error = sqlx::raw_sql(sql)
                .execute(&h.pool)
                .await
                .err()
                .ok_or("immutable reservation changed")?;
            assert_eq!(code(&error).as_deref(), Some("23514"));
        }
        let error = legacy_reserve(h, u64::MAX)
            .await
            .err()
            .ok_or("legacy destination reused Inbox CommandRef")?;
        assert_eq!(code(&error).as_deref(), Some("OTI05"));
        legacy_reserve(h, u64::MAX - 1).await?;
        let mut reverse = source;
        reverse.command = CommandRef::new(
            reverse.command.game_session_id(),
            CommandId::new(u64::MAX - 1).map_err(debug)?,
        );
        reverse.transaction_id = id(101);
        assert!(matches!(
            h.root
                .prepare_reward_claim_inbox_mint(
                    &authority,
                    &h.node,
                    item_fence()?,
                    &reverse,
                    request(2)
                )
                .await,
            Err(RewardClaimInboxError::ConflictingDelivery)
        ));
        assert_eq!(footprint(h).await?, [0; 10]);
        assert_eq!(
            h.count("game_reward_claim_inbox_mint_reservations").await?,
            1
        );
        Ok(())
    })
}

pub(super) struct IncompleteProof {
    pub(super) binding: Vec<u8>,
    pub(super) intent: Vec<u8>,
    pub(super) source_binding: [u8; 32],
    pub(super) event: [u8; 16],
    pub(super) occurred: i64,
    pub(super) request: RewardClaimInboxMintRequest,
    pub(super) omit_receipt: bool,
    pub(super) reached_end: bool,
}

fn definition(value: &TypedDefinitionRef) -> OneItemTypedDefinitionRevisionV1 {
    OneItemTypedDefinitionRevisionV1 {
        family: value.family.clone(),
        production_key: value.production_key.clone(),
        revision_ref: value.revision_ref.clone(),
    }
}

pub(super) async fn incomplete_proof(
    context: &mut RewardClaimInboxTransaction<'_>,
    owner: &mut IncompleteProof,
) -> Result<(), RewardClaimInboxError> {
    let plan = context.plan().clone();
    sqlx::query("SELECT game_reward_claim_inbox_mint_begin(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,$4)")
        .bind(plan.command.game_session_id().as_bytes().as_slice()).bind(plan.command.command_id().get().to_string())
        .bind(plan.transaction_id.as_slice()).bind(&owner.binding).execute(&mut **context.transaction()).await?;
    let mut outputs = Vec::new();
    for (item, source) in plan.item_instance_ids.iter().zip(&owner.request.outputs) {
        let position = context.deliver(*item).await?;
        outputs.push(RewardClaimInboxMintOutputV1 {
            after: Some(OneItemStateV1 {
                item_instance_id: item.to_vec(),
                world_id: plan.world_id.as_bytes().to_vec(),
                definition: Some(definition(&source.item.definition)),
                quantity: source.quantity,
                lifecycle: common::ITEM_LIFECYCLE_LIVE,
                key_number: source.key_number.map(u32::from).unwrap_or(0),
            }),
            ordinal: position.ordinal,
            source_line: source.source_line,
        });
    }
    // Encode the actual source tuple and freshly delivered states. This is the
    // registered native codec, not a byte witness standing in for an audit.
    let tuple = context.selected_type2_tuple();
    let envelope = audit::encode_reward_claim_mint_event(
        RewardClaimMintEventIdentity {
            event_id: owner.event,
            transaction_id: plan.transaction_id,
            occurred_at_unix_ms: owner.occurred,
            channel_id: *plan.channel_id.as_bytes(),
            server_build_id: crate::durability::character_authority::SERVER_BUILD_ID,
        },
        OneItemRewardClaimMintV1 {
            after: None,
            destination: None,
            before_semantically_absent: true,
            source: Some(OneItemRewardClaimCauseV1 {
                typed_cause: context.typed_cause().into(),
                content_revision: owner.request.content_revision.clone(),
                ruleset_revision: owner.request.ruleset_revision.clone(),
                sim_revision: owner.request.sim_revision.clone(),
                reward_claim: Some(definition(&owner.request.claim)),
                command_ref: Some(OneItemCommandRefV1 {
                    game_session_id: plan.command.game_session_id().as_bytes().to_vec(),
                    command_id: plan.command.command_id().get(),
                }),
            }),
            inbox: Some(RewardClaimInboxMintV1 {
                character_id: plan.character_id.as_bytes().to_vec(),
                world_id: plan.world_id.as_bytes().to_vec(),
                expected_session_generation: 1,
                expected_character_lease_generation: 1,
                source_binding: owner.source_binding.to_vec(),
                plan_binding: owner.binding.clone(),
                mint_intent_sha256: owner.intent.clone(),
                outputs,
            }),
        },
        tuple.tuple(),
    )
    .map_err(|_| RewardClaimInboxError::InvalidPlan)?;
    if owner.omit_receipt {
        sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)")
            .bind(owner.event.as_slice()).bind(plan.transaction_id.as_slice()).bind(i64::from(tuple.schema_revision()))
            .bind(tuple.retention_profile_id()).bind(plan.item_instance_ids[0].as_slice()).bind(owner.occurred)
            .bind(common::AUDIT_RETENTION_P90D_MS).bind(envelope).execute(&mut **context.transaction()).await?;
    } else {
        let hash: [u8; 32] = Sha256::digest(&envelope).into();
        sqlx::query("SELECT game_reward_claim_inbox_mint_finish(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,$4,1,1,$5)")
            .bind(plan.command.game_session_id().as_bytes().as_slice()).bind(plan.command.command_id().get().to_string())
            .bind(plan.transaction_id.as_slice()).bind(&owner.binding).bind(hash.as_slice())
            .execute(&mut **context.transaction()).await?;
    }
    owner.reached_end = true;
    Ok(())
}

#[test]
fn native_inbox_real_mint_without_receipt_or_without_audit_cannot_commit() -> TestResult {
    run("mail_mint_proof", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let source = plan(12, 2)?;
        let request = request(2);
        let candidate: RewardClaimInboxMintCandidate = h
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
        let (binding, intent, _) = reservation(h).await?;
        for omit_receipt in [true, false] {
            let mut connection = h.root.try_acquire_ready().map_err(debug)?;
            sqlx::query("SELECT work_units_used FROM game_reward_claim_inbox_mint_charge(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,$4)")
                .bind(source.command.game_session_id().as_bytes().as_slice()).bind(source.command.command_id().get().to_string())
                .bind(source.transaction_id.as_slice()).bind(&binding).fetch_one(&mut *connection).await?;
            return_connection(connection).await?;
            let mut owner = IncompleteProof {
                binding: binding.clone(),
                intent: intent.clone(),
                source_binding: source.source_binding,
                event: *candidate.event_id(),
                occurred: candidate.occurred_at_unix_ms(),
                request: request.clone(),
                omit_receipt,
                reached_end: false,
            };
            let result = h
                .root
                .with_reward_claim_inbox_transaction(
                    &authority,
                    &h.node,
                    item_fence()?,
                    candidate.plan(),
                    &mut owner,
                    |context, owner| Box::pin(incomplete_proof(context, owner)),
                )
                .await;
            assert!(
                owner.reached_end,
                "actual mint, all placements and the retained half of the proof must execute before the deferred refusal"
            );
            assert!(
                result.is_err(),
                "neither incomplete native proof may commit"
            );
            assert_eq!(footprint(h).await?, [0; 10]);
        }
        Ok(())
    })
}
