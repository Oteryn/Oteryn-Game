// Shared D3-6 DUR-03 corpse DECAY_RETIRE cases (DUR-03 §39.1/§39.4;
// decisions D135/D136): a corpse decays at its durable deadline
// `materialized_at + 60 s`, as N+1 separate one-item steps (each live entry,
// then the corpse once no live entry remains), each with its own receipt and
// its own admitted `CorpseDecay` audit event; decay resumes after a partial
// drain from durable state alone, and an entry picked up before decay reaches
// it is never retired. Both wrappers provide the same path-loaded crate root;
// the PostgreSQL bootstrap, the corpse MINT, the loot entries and the
// database-clock time travel are the D3-4 fixtures.

use crate::corpse_transfer_postgres_cases::{
    GENERATION, equip_backpack, in_corpse, locations, mint_corpse, put_loot, run_statements,
    set_materialized_ago, take,
};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_decay_retire::{
    CORPSE_DECAY_AFTER_MS, CommittedDecayRetire, CorpseDecayFence, DecayRetireError,
    DecayRetireOutcome, DecayRetireRefusal, DecayRetireStep,
};
use crate::durability::item_decay_retire_audit::{DecayRetireShape, decode_decay_retire_envelope};
use crate::durability::item_transfer::ItemTransferOutcome;
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, NodeIncarnationProof,
    OperationKey, RuntimeScopeAssignmentWriter,
};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};
use crate::item_transfer_postgres_cases::{
    CHANNEL, CHARACTER, Harness, SESSION, TestResult, WORLD, configured_admin, debug, fence, id,
    register, runtime, scope, uuid_text,
};

fn decay_fence(generation: u64) -> TestResult<CorpseDecayFence> {
    Ok(CorpseDecayFence {
        world_id: WorldId::decode(&id(WORLD)).map_err(debug)?,
        channel_id: ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
        scope_ownership_generation: ScopeOwnershipGeneration::new(generation).map_err(debug)?,
    })
}

/// A corpse's committed `materialized_at` (database-clock unix ms).
async fn materialized_at(harness: &Harness, corpse: [u8; 16]) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT materialized_at FROM game_item_mint_receipts \
          WHERE item_instance_id = encode($1,'hex')::uuid \
            AND loot_purpose_key = 'CORPSE_MATERIALIZATION'",
    )
    .bind(corpse.as_slice())
    .fetch_one(&harness.pool)
    .await?)
}

/// Row counts of every relation a DECAY_RETIRE step could write.
async fn decay_footprint(harness: &Harness) -> TestResult<Vec<i64>> {
    let mut counts = Vec::new();
    for relation in [
        "game_item_decay_retire_reservations",
        "game_item_decay_retire_receipts",
        "game_item_audit_outbox",
        "game_item_ground_locations",
        "game_item_corpse_container_entries",
        "game_item_instances WHERE lifecycle = 2",
    ] {
        counts.push(harness.count(relation).await?);
    }
    Ok(counts)
}

/// One item's receipt: (owner generation, creating physical transaction).
async fn receipt_of(harness: &Harness, item: [u8; 16]) -> TestResult<Option<(u64, u64)>> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT fence_scope_ownership_generation::text, created_xact_id::text \
           FROM game_item_decay_retire_receipts WHERE item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(item.as_slice())
    .fetch_optional(&harness.pool)
    .await?;
    let Some((generation, xact)) = row else {
        return Ok(None);
    };
    Ok(Some((generation.parse()?, xact.parse()?)))
}

/// The committed audit event of `committed`, through the registered gate.
async fn audited_shape(
    harness: &Harness,
    committed: &CommittedDecayRetire,
) -> TestResult<DecayRetireShape> {
    let envelope: Vec<u8> = sqlx::query_scalar(
        "SELECT envelope FROM game_item_audit_outbox \
          WHERE event_id = encode($1,'hex')::uuid AND transaction_id = encode($2,'hex')::uuid \
            AND item_instance_id = encode($3,'hex')::uuid AND publication_state = 1",
    )
    .bind(committed.event_id.as_slice())
    .bind(committed.transaction_id.as_slice())
    .bind(committed.item_instance_id.as_slice())
    .fetch_one(&harness.pool)
    .await?;
    let (event, retire) = decode_decay_retire_envelope(&envelope).map_err(debug)?;
    assert_eq!(event.occurred_at_unix_ms, committed.occurred_at_unix_ms);
    assert!(committed.occurred_at_unix_ms >= committed.deadline_unix_ms);
    let cause = retire.cause.as_ref().ok_or("cause")?;
    assert_eq!(
        cause.corpse_item_instance_id,
        committed.corpse_item_instance_id.to_vec()
    );
    assert_eq!(
        i64::try_from(cause.deadline_unix_ms)?,
        committed.deadline_unix_ms
    );
    let before = retire.before.as_ref().ok_or("before")?;
    assert_eq!(before.item_instance_id, committed.item_instance_id.to_vec());
    crate::durability::item_decay_retire_audit::check_decay_retire(&retire)
        .map_err(|error| debug(error).into())
}

async fn retire_step(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    generation: u64,
    step: DecayRetireStep,
) -> Result<DecayRetireOutcome, DecayRetireError> {
    let fence = decay_fence(generation).map_err(|_| DecayRetireError::InvalidInput)?;
    let mut candidate = harness
        .root
        .freeze_decay_retire(authority, node, fence, step)
        .await?;
    harness
        .root
        .commit_decay_retire(authority, node, fence, &mut candidate)
        .await
}

fn refused<T: std::fmt::Debug>(
    result: Result<T, DecayRetireError>,
    expected: DecayRetireRefusal,
) -> TestResult {
    match result {
        Err(DecayRetireError::Refused(refusal)) if refusal == expected => Ok(()),
        other => Err(format!("expected {expected:?}, got {other:?}").into()),
    }
}

fn committed(outcome: DecayRetireOutcome) -> TestResult<CommittedDecayRetire> {
    match outcome {
        DecayRetireOutcome::Committed(result) => Ok(result),
        other => Err(format!("expected a fresh commit, got {other:?}").into()),
    }
}

/// Retired to exactly quantity 0 with no location left.
async fn assert_retired(harness: &Harness, item: [u8; 16]) -> TestResult {
    assert_eq!(harness.item_state(item).await?, (0, 2));
    assert_eq!(locations(harness, item).await?, 0);
    Ok(())
}

/// D135/D136: at `materialized_at + 60 s` (never before, judged by the database
/// clock) the corpse retires as separate one-item steps, each entry in ordinal
/// order and then the corpse, each its own physical transaction with its own
/// receipt and its own admitted `CorpseDecay` event naming the exact deadline.
/// The recovery query reports that exact deadline; a younger corpse in the
/// same scope is untouched; a replay returns the original results.
#[test]
fn decay_retires_each_entry_then_the_corpse_at_sixty_seconds() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpsedecay").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let node = &harness.node;

        let corpse = mint_corpse(&harness, &authority, 1000, id(CHARACTER)).await?;
        let a = put_loot(&harness, corpse, 1000, 1, 100).await?;
        let b = put_loot(&harness, corpse, 1000, 2, 104).await?;
        let c = put_loot(&harness, corpse, 1000, 3, 108).await?;
        let young = mint_corpse(&harness, &authority, 1001, id(CHARACTER)).await?;
        let y = put_loot(&harness, young, 1001, 1, 112).await?;

        // One second before the deadline: refused, nothing written.
        set_materialized_ago(&harness, corpse, CORPSE_DECAY_AFTER_MS - 1_000).await?;
        let before = decay_footprint(&harness).await?;
        refused(
            harness
                .root
                .retire_decayed_corpse(&authority, node, decay_fence(GENERATION)?, corpse)
                .await,
            DecayRetireRefusal::NotYetDue,
        )?;
        assert_eq!(decay_footprint(&harness).await?, before);
        for item in [a, b, c] {
            assert!(in_corpse(&harness, item).await?);
        }

        // The recovery query reports every live corpse with its exact deadline.
        let schedule = harness
            .root
            .read_corpse_decay_schedule(
                &authority,
                WorldId::decode(&id(WORLD)).map_err(debug)?,
                ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
            )
            .await
            .map_err(debug)?;
        assert_eq!(schedule.corpses.len(), 2);
        for entry in &schedule.corpses {
            let materialized = materialized_at(&harness, entry.corpse_item_instance_id).await?;
            assert_eq!(entry.materialized_at_unix_ms, materialized);
            assert_eq!(entry.decay_at_unix_ms, materialized + CORPSE_DECAY_AFTER_MS);
            assert!(entry.decay_at_unix_ms > schedule.database_now_unix_ms);
        }

        // Exactly at the deadline: admitted.
        set_materialized_ago(&harness, corpse, CORPSE_DECAY_AFTER_MS).await?;
        let deadline = materialized_at(&harness, corpse).await? + CORPSE_DECAY_AFTER_MS;
        let report = harness
            .root
            .retire_decayed_corpse(&authority, node, decay_fence(GENERATION)?, corpse)
            .await
            .map_err(debug)?;
        let retired: Vec<[u8; 16]> = report
            .entries
            .iter()
            .map(|entry| entry.item_instance_id)
            .collect();
        assert_eq!(retired, vec![a, b, c]);
        assert_eq!(report.corpse.item_instance_id, corpse);

        let mut xacts = Vec::new();
        let mut transactions = Vec::new();
        for (step, shape) in report
            .entries
            .iter()
            .map(|entry| (entry, DecayRetireShape::Entry))
            .chain([(&report.corpse, DecayRetireShape::Corpse)])
        {
            assert_eq!(step.corpse_item_instance_id, corpse);
            assert_eq!(step.deadline_unix_ms, deadline);
            assert_eq!(audited_shape(&harness, step).await?, shape);
            let (generation, xact) = receipt_of(&harness, step.item_instance_id)
                .await?
                .ok_or("receipt")?;
            assert_eq!(generation, GENERATION);
            xacts.push(xact);
            transactions.push(step.transaction_id);
            assert_retired(&harness, step.item_instance_id).await?;
        }
        // N+1 separate physical transactions, the corpse's own step last.
        let mut distinct = xacts.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), 4);
        assert!(xacts[..3].iter().all(|entry| *entry < xacts[3]));
        transactions.sort_unstable();
        transactions.dedup();
        assert_eq!(transactions.len(), 4);
        assert_eq!(harness.count("game_item_decay_retire_receipts").await?, 4);
        assert!(!harness.on_ground(corpse).await?);

        // The younger corpse and its entry are untouched; only it stays scheduled.
        assert!(harness.on_ground(young).await?);
        assert!(in_corpse(&harness, y).await?);
        let schedule = harness
            .root
            .read_corpse_decay_schedule(
                &authority,
                WorldId::decode(&id(WORLD)).map_err(debug)?,
                ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
            )
            .await
            .map_err(debug)?;
        let listed: Vec<[u8; 16]> = schedule
            .corpses
            .iter()
            .map(|entry| entry.corpse_item_instance_id)
            .collect();
        assert_eq!(listed, vec![young]);

        // A replay retires nothing twice and returns the original corpse result.
        let settled = decay_footprint(&harness).await?;
        let again = harness
            .root
            .retire_decayed_corpse(&authority, node, decay_fence(GENERATION)?, corpse)
            .await
            .map_err(debug)?;
        assert!(again.entries.is_empty());
        assert_eq!(again.corpse, report.corpse);
        assert_eq!(decay_footprint(&harness).await?, settled);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// The corpse's own step is admitted only once zero live entries remain; each
/// step replays and reconciles to its original result; a non-corpse, an
/// unassigned generation and a foreign entry are refused with nothing written.
#[test]
fn the_corpse_step_waits_for_every_live_entry() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpsedecaysteps").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let node = &harness.node;

        let corpse = mint_corpse(&harness, &authority, 1000, id(CHARACTER)).await?;
        let a = put_loot(&harness, corpse, 1000, 1, 100).await?;
        let b = put_loot(&harness, corpse, 1000, 2, 104).await?;
        let other = mint_corpse(&harness, &authority, 1001, id(CHARACTER)).await?;
        let foreign = put_loot(&harness, other, 1001, 1, 108).await?;
        let plain = harness.mint(&authority, "fixture:plain", 1).await?;
        set_materialized_ago(&harness, corpse, CORPSE_DECAY_AFTER_MS + 1_000).await?;
        set_materialized_ago(&harness, other, CORPSE_DECAY_AFTER_MS + 1_000).await?;

        let untouched = decay_footprint(&harness).await?;
        refused(
            retire_step(
                &harness,
                &authority,
                node,
                GENERATION,
                DecayRetireStep::corpse(corpse),
            )
            .await,
            DecayRetireRefusal::EntriesRemain,
        )?;
        // Not a corpse: a plain Ground item, and an entry of another corpse.
        refused(
            retire_step(
                &harness,
                &authority,
                node,
                GENERATION,
                DecayRetireStep::corpse(plain),
            )
            .await,
            DecayRetireRefusal::NotACorpse,
        )?;
        refused(
            retire_step(
                &harness,
                &authority,
                node,
                GENERATION,
                DecayRetireStep::entry(corpse, foreign),
            )
            .await,
            DecayRetireRefusal::NotInCorpse,
        )?;
        // A generation this node does not currently own.
        assert!(matches!(
            retire_step(
                &harness,
                &authority,
                node,
                2,
                DecayRetireStep::entry(corpse, a)
            )
            .await,
            Err(DecayRetireError::AuthorityRejected)
        ));
        assert_eq!(decay_footprint(&harness).await?, untouched);

        let first = committed(
            retire_step(
                &harness,
                &authority,
                node,
                GENERATION,
                DecayRetireStep::entry(corpse, a),
            )
            .await
            .map_err(debug)?,
        )?;
        assert_retired(&harness, a).await?;
        // One live entry still remains: the corpse still waits.
        refused(
            retire_step(
                &harness,
                &authority,
                node,
                GENERATION,
                DecayRetireStep::corpse(corpse),
            )
            .await,
            DecayRetireRefusal::EntriesRemain,
        )?;
        assert!(harness.on_ground(corpse).await?);

        let fence = decay_fence(GENERATION)?;
        let mut candidate = harness
            .root
            .freeze_decay_retire(&authority, node, fence, DecayRetireStep::entry(corpse, b))
            .await
            .map_err(debug)?;
        let second = committed(
            harness
                .root
                .commit_decay_retire(&authority, node, fence, &mut candidate)
                .await
                .map_err(debug)?,
        )?;
        assert_ne!(second.transaction_id, first.transaction_id);
        // Replay and reconciliation return the original result.
        assert_eq!(
            harness
                .root
                .commit_decay_retire(&authority, node, fence, &mut candidate)
                .await
                .map_err(debug)?,
            DecayRetireOutcome::AlreadyCommitted(second.clone())
        );
        assert_eq!(
            harness
                .root
                .reconcile_decay_retire(&authority, &mut candidate)
                .await
                .map_err(debug)?,
            Some(second.clone())
        );
        // Now no live entry remains: the corpse's own step is admitted.
        let last = committed(
            retire_step(
                &harness,
                &authority,
                node,
                GENERATION,
                DecayRetireStep::corpse(corpse),
            )
            .await
            .map_err(debug)?,
        )?;
        assert_eq!(
            audited_shape(&harness, &last).await?,
            DecayRetireShape::Corpse
        );
        assert_retired(&harness, corpse).await?;
        // The other corpse and its entry are untouched.
        assert!(in_corpse(&harness, foreign).await?);
        assert!(harness.on_ground(other).await?);
        assert_eq!(harness.count("game_item_decay_retire_receipts").await?, 3);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// D135 resume: a decay interrupted after some entries retired (and with one
/// step frozen but never committed) is completed after a scope handoff by the
/// new owner, from the durable recovery query alone: the frozen step of the
/// ended generation can never commit, the remaining entries and then the
/// corpse retire under the new generation, and no entry retires twice.
#[test]
fn a_partial_decay_resumes_from_durable_state_after_a_handoff() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpsedecayresume").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        let corpse = mint_corpse(&harness, &authority, 1000, id(CHARACTER)).await?;
        let a = put_loot(&harness, corpse, 1000, 1, 100).await?;
        let b = put_loot(&harness, corpse, 1000, 2, 104).await?;
        let c = put_loot(&harness, corpse, 1000, 3, 108).await?;
        set_materialized_ago(&harness, corpse, CORPSE_DECAY_AFTER_MS + 1_000).await?;
        let deadline = materialized_at(&harness, corpse).await? + CORPSE_DECAY_AFTER_MS;

        // Generation 1 retires one entry and freezes the next, then "crashes".
        committed(
            retire_step(
                &harness,
                &authority,
                &harness.node,
                GENERATION,
                DecayRetireStep::entry(corpse, a),
            )
            .await
            .map_err(debug)?,
        )?;
        let old_fence = decay_fence(GENERATION)?;
        let mut stranded = harness
            .root
            .freeze_decay_retire(
                &authority,
                &harness.node,
                old_fence,
                DecayRetireStep::entry(corpse, b),
            )
            .await
            .map_err(debug)?;

        // The scope moves to another node: generation 1 ends. The B3-1
        // harness granted only the initial assignment; grant the replacement.
        sqlx::query(
            "INSERT INTO game_control_scope_grants \
             (control_role, world_id, channel_id, operation) \
             VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 2)",
        )
        .bind(id(WORLD).as_slice())
        .bind(id(CHANNEL).as_slice())
        .execute(&harness.pool)
        .await?;
        let predecessor = harness
            .root
            .read_runtime_scope_predecessor(scope()?)
            .await
            .map_err(debug)?
            .ok_or("expected a live assignment predecessor")?;
        let node2 = register(&harness.root, 2).await?;
        let writer =
            RuntimeScopeAssignmentWriter::open(harness.root.clone(), "item-transfer-writer")
                .await
                .map_err(debug)?;
        let moved = writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([9_u8; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Replace {
                    scope: scope()?,
                    predecessor,
                    target: node2.fact(),
                },
            })
            .await
            .map_err(debug)?;
        let AssignmentOutcome::Committed(moved) = moved else {
            return Err(format!("unexpected replacement outcome: {moved:?}").into());
        };
        assert_eq!(moved.assignment.ownership_generation, 2);

        // The ended generation's frozen step can never commit.
        assert!(matches!(
            harness
                .root
                .commit_decay_retire(&authority, &harness.node, old_fence, &mut stranded)
                .await,
            Err(DecayRetireError::AuthorityRejected)
        ));
        assert!(in_corpse(&harness, b).await?);

        // The new owner's recovery query still lists the partly drained corpse
        // with its unchanged durable deadline ...
        let schedule = harness
            .root
            .read_corpse_decay_schedule(
                &authority,
                WorldId::decode(&id(WORLD)).map_err(debug)?,
                ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
            )
            .await
            .map_err(debug)?;
        assert_eq!(schedule.corpses.len(), 1);
        assert_eq!(schedule.corpses[0].corpse_item_instance_id, corpse);
        assert_eq!(schedule.corpses[0].decay_at_unix_ms, deadline);
        assert!(schedule.database_now_unix_ms >= deadline);

        // ... and draining it completes only the remaining steps.
        let report = harness
            .root
            .retire_decayed_corpse(&authority, &node2, decay_fence(2)?, corpse)
            .await
            .map_err(debug)?;
        let retired: Vec<[u8; 16]> = report
            .entries
            .iter()
            .map(|entry| entry.item_instance_id)
            .collect();
        assert_eq!(retired, vec![b, c]);
        assert_eq!(report.corpse.item_instance_id, corpse);
        assert_eq!(receipt_of(&harness, a).await?.map(|r| r.0), Some(1));
        for item in [b, c, corpse] {
            assert_eq!(receipt_of(&harness, item).await?.map(|r| r.0), Some(2));
        }
        for item in [a, b, c, corpse] {
            assert_retired(&harness, item).await?;
        }
        assert_eq!(harness.count("game_item_decay_retire_receipts").await?, 4);
        // b holds two reservations (one per generation) but one receipt.
        assert_eq!(
            harness.count("game_item_decay_retire_reservations").await?,
            5
        );
        // The stranded step now resolves to the one committed result, never a
        // second retirement.
        match harness
            .root
            .commit_decay_retire(&authority, &harness.node, old_fence, &mut stranded)
            .await
            .map_err(debug)?
        {
            DecayRetireOutcome::AlreadyCommitted(result) => {
                assert_eq!(result.item_instance_id, b);
                assert_ne!(&result.transaction_id, stranded.transaction_id());
            }
            other => return Err(format!("expected the committed result, got {other:?}").into()),
        }
        assert!(
            harness
                .root
                .read_corpse_decay_schedule(
                    &authority,
                    WorldId::decode(&id(WORLD)).map_err(debug)?,
                    ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
                )
                .await
                .map_err(debug)?
                .corpses
                .is_empty()
        );

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// An entry legitimately picked up before decay reaches it is never retired,
/// including when the pickup wins the race against an already frozen decay
/// step; decay retires only what is still live in the corpse.
#[test]
fn an_entry_picked_up_before_decay_is_never_retired() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpsedecaypickup").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let node = &harness.node;
        equip_backpack(&harness, &authority, fence()?, SESSION).await?;

        let corpse = mint_corpse(&harness, &authority, 1000, id(CHARACTER)).await?;
        let a = put_loot(&harness, corpse, 1000, 1, 100).await?;
        let b = put_loot(&harness, corpse, 1000, 2, 104).await?;
        let c = put_loot(&harness, corpse, 1000, 3, 108).await?;
        // The top-damage Character picks `a` up inside its window.
        set_materialized_ago(&harness, corpse, 1_000).await?;
        assert!(matches!(
            harness
                .transfer(&authority, fence()?, take(SESSION, 2, a)?)
                .await
                .map_err(debug)?,
            ItemTransferOutcome::Committed(_)
        ));

        set_materialized_ago(&harness, corpse, CORPSE_DECAY_AFTER_MS + 1_000).await?;
        let untouched = decay_footprint(&harness).await?;
        refused(
            retire_step(
                &harness,
                &authority,
                node,
                GENERATION,
                DecayRetireStep::entry(corpse, a),
            )
            .await,
            DecayRetireRefusal::NotInCorpse,
        )?;
        assert_eq!(decay_footprint(&harness).await?, untouched);

        // A decay step for `b` is frozen, then the pickup commits first.
        let fence_1 = decay_fence(GENERATION)?;
        let mut raced = harness
            .root
            .freeze_decay_retire(&authority, node, fence_1, DecayRetireStep::entry(corpse, b))
            .await
            .map_err(debug)?;
        assert!(matches!(
            harness
                .transfer(&authority, fence()?, take(SESSION, 3, b)?)
                .await
                .map_err(debug)?,
            ItemTransferOutcome::Committed(_)
        ));
        refused(
            harness
                .root
                .commit_decay_retire(&authority, node, fence_1, &mut raced)
                .await,
            DecayRetireRefusal::NotInCorpse,
        )?;

        let report = harness
            .root
            .retire_decayed_corpse(&authority, node, fence_1, corpse)
            .await
            .map_err(debug)?;
        let retired: Vec<[u8; 16]> = report
            .entries
            .iter()
            .map(|entry| entry.item_instance_id)
            .collect();
        assert_eq!(retired, vec![c]);
        assert_eq!(report.corpse.item_instance_id, corpse);
        // The picked-up items stay live in exactly one place, with no receipt.
        for item in [a, b] {
            assert_eq!(harness.item_state(item).await?, (1, 1));
            assert_eq!(locations(&harness, item).await?, 1);
            assert!(receipt_of(&harness, item).await?.is_none());
        }
        assert_retired(&harness, c).await?;
        assert_retired(&harness, corpse).await?;
        assert_eq!(harness.count("game_item_decay_retire_receipts").await?, 2);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// Raw-SQL statements forging one whole DECAY_RETIRE step (reservation, audit
/// event, retirement, `removal`, receipt) under generation 1.
struct Forged<'a> {
    item: [u8; 16],
    corpse: [u8; 16],
    removal: &'a str,
    ordinal: Option<u64>,
    deadline: i64,
    ids: (u8, u8),
}

fn forge_decay(forged: &Forged<'_>) -> Vec<String> {
    let item = uuid_text(forged.item);
    let corpse = uuid_text(forged.corpse);
    let world = uuid_text(id(WORLD));
    let channel = uuid_text(id(CHANNEL));
    let tx = uuid_text(id(forged.ids.0));
    let ev = uuid_text(id(forged.ids.1));
    let node = uuid_text(id(forged.ids.1 + 1));
    let ordinal = forged
        .ordinal
        .map_or_else(|| "NULL".to_owned(), |value| value.to_string());
    let deadline = forged.deadline;
    vec![
        format!(
            "INSERT INTO game_item_decay_retire_reservations \
               (item_instance_id, fence_scope_ownership_generation, corpse_item_instance_id, \
                world_id, channel_id, transaction_id, event_id, quantity_before, \
                placement_ordinal, deadline, occurred_at, envelope, fence_holder_node_id, \
                fence_holder_registration_revision, work_units_used, reserved_at) \
             VALUES ('{item}', 1, '{corpse}', '{world}', '{channel}', '{tx}', '{ev}', 1, \
                     {ordinal}, {deadline}, {deadline}, decode(repeat('ab',16),'hex'), \
                     '{node}', 0, 0, 900)"
        ),
        format!(
            "INSERT INTO game_item_audit_outbox \
               (event_id, transaction_id, transaction_ordinal, transaction_count, \
                event_type_id, schema_revision, retention_profile_id, item_instance_id, \
                occurred_at, expires_at, envelope, envelope_sha256, publication_state) \
             VALUES ('{ev}', '{tx}', 1, 1, 2, 1, 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1', \
                     '{item}', {deadline}, {deadline} + 7776000000, \
                     decode(repeat('ab',16),'hex'), sha256(decode(repeat('ab',16),'hex')), 1)"
        ),
        format!(
            "UPDATE game_item_instances SET quantity = 0, lifecycle = 2, \
                    last_transaction_id = '{tx}' \
              WHERE item_instance_id = '{item}' AND lifecycle = 1 AND quantity = 1"
        ),
        forged.removal.to_owned(),
        format!(
            "INSERT INTO game_item_decay_retire_receipts \
               (item_instance_id, corpse_item_instance_id, world_id, channel_id, \
                fence_scope_ownership_generation, transaction_id, event_id, quantity_before, \
                placement_ordinal, deadline, occurred_at, envelope_sha256, committed_at) \
             VALUES ('{item}', '{corpse}', '{world}', '{channel}', 1, '{tx}', '{ev}', 1, \
                     {ordinal}, {deadline}, {deadline}, \
                     sha256(decode(repeat('ab',16),'hex')), 1000)"
        ),
    ]
}

fn remove_entry(item: [u8; 16]) -> String {
    format!(
        "DELETE FROM game_item_corpse_container_entries WHERE item_instance_id = '{}'",
        uuid_text(item)
    )
}

fn remove_ground(item: [u8; 16]) -> String {
    format!(
        "DELETE FROM game_item_ground_locations WHERE item_instance_id = '{}'",
        uuid_text(item)
    )
}

/// The database gate, independent of the Rust admission: a raw-SQL retirement
/// is rejected at commit when it is early by the database clock, names a
/// deadline other than `materialized_at + 60 s`, retires the corpse while an
/// entry remains, or changes the item without its receipt; the identical,
/// correct forgeries are admitted, and a corpse's Ground row still cannot be
/// removed by anything else.
#[test]
fn database_admits_decay_only_as_its_own_proven_step() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpsedecaydbgate").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        let corpse = mint_corpse(&harness, &authority, 1000, id(CHARACTER)).await?;
        let a = put_loot(&harness, corpse, 1000, 1, 100).await?;
        let young = mint_corpse(&harness, &authority, 1001, id(CHARACTER)).await?;
        let y = put_loot(&harness, young, 1001, 1, 104).await?;
        set_materialized_ago(&harness, corpse, CORPSE_DECAY_AFTER_MS + 1_000).await?;
        let deadline = materialized_at(&harness, corpse).await? + CORPSE_DECAY_AFTER_MS;
        let young_deadline = materialized_at(&harness, young).await? + CORPSE_DECAY_AFTER_MS;
        let receipts = harness.count("game_item_decay_retire_receipts").await?;

        let entry_a = |deadline, ids| Forged {
            item: a,
            corpse,
            removal: "",
            ordinal: Some(1),
            deadline,
            ids,
        };
        let remove_a = remove_entry(a);
        let remove_corpse = remove_ground(corpse);

        // A bare retirement without any receipt.
        assert!(
            run_statements(
                &harness.pool,
                vec![format!(
                    "UPDATE game_item_instances SET quantity = 0, lifecycle = 2, \
                            last_transaction_id = '{}' WHERE item_instance_id = '{}'",
                    uuid_text(id(200)),
                    uuid_text(a)
                )],
            )
            .await
            .is_err()
        );
        // The corpse while its entry `a` remains.
        let early_corpse = forge_decay(&Forged {
            item: corpse,
            corpse,
            removal: &remove_corpse,
            ordinal: None,
            deadline,
            ids: (202, 203),
        });
        assert!(run_statements(&harness.pool, early_corpse).await.is_err());
        assert!(harness.on_ground(corpse).await?);
        // An entry of the young corpse before its deadline (database clock).
        let not_due = forge_decay(&Forged {
            item: y,
            corpse: young,
            removal: &remove_entry(y),
            ordinal: Some(1),
            deadline: young_deadline,
            ids: (206, 207),
        });
        assert!(run_statements(&harness.pool, not_due).await.is_err());
        assert!(in_corpse(&harness, y).await?);
        // A deadline other than materialized_at + 60 s.
        let wrong_deadline = forge_decay(&Forged {
            removal: &remove_a,
            ..entry_a(deadline - 1, (210, 211))
        });
        assert!(run_statements(&harness.pool, wrong_deadline).await.is_err());
        assert!(in_corpse(&harness, a).await?);
        assert_eq!(
            harness.count("game_item_decay_retire_receipts").await?,
            receipts
        );

        // The correct entry step, then the correct corpse step, are admitted.
        let entry = forge_decay(&Forged {
            removal: &remove_a,
            ..entry_a(deadline, (214, 215))
        });
        run_statements(&harness.pool, entry).await?;
        assert_retired(&harness, a).await?;
        let corpse_step = forge_decay(&Forged {
            item: corpse,
            corpse,
            removal: &remove_corpse,
            ordinal: None,
            deadline,
            ids: (218, 219),
        });
        run_statements(&harness.pool, corpse_step).await?;
        assert_retired(&harness, corpse).await?;

        // The young corpse's Ground row still cannot be removed without its
        // own DECAY_RETIRE step (D134 unchanged).
        assert!(
            run_statements(&harness.pool, vec![remove_ground(young)])
                .await
                .is_err()
        );
        assert!(harness.on_ground(young).await?);
        assert_eq!(
            harness.count("game_item_decay_retire_receipts").await?,
            receipts + 2
        );

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
