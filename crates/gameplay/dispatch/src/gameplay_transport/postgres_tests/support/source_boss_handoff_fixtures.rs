//! Shared successor qualification for the two paid Channel boss levers.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult};
use crate::durability::boss_encounter::{
    SourceBossAdmission, SourceBossLever, SourceBossReservation,
};
use crate::durability::item_transfer::{
    CurrentCharacterItemFence, character_item_fence_is_current,
};
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentReceipt, AssignmentRequest, ControlActor,
    NodeIncarnationProof, OperationKey, RuntimeScopeAssignmentWriter,
};
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, ScopeOwnershipGeneration};
use crate::gameplay_transport::{
    ComposedFreshAdmission, source_item_cycle::SOURCE_OWNER_SWEEP_COMPLETE,
};
use std::sync::Arc;
use std::time::Duration;

pub(super) async fn with_unrelated_login<T>(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    operation: impl std::future::Future<Output = T>,
) -> TestResult<T> {
    use crate::foundation::{GameSessionId, MovementFacing, MovementLocalPosition};
    use crate::gameplay_transport::quest_callbacks::PRE_SOURCE_BOSS_CHARGE;
    let moving_session = GameSessionId::decode(&id(245)).map_err(debug)?;
    let leaving_session = GameSessionId::decode(&id(246)).map_err(debug)?;
    let outside = MovementLocalPosition { x: 100, y: 100, floor: 0 };
    let (moving, start, leaving) = {
        let mut runtime = owner.runtime.lock().await;
        let slot = runtime.reserve_fresh_session(moving_session).map_err(debug)?;
        let moving = runtime.commit_fresh_session(slot).map_err(debug)?;
        let start = runtime.initialize_pinned_test_position(moving, outside).map_err(debug)?;
        let slot = runtime.reserve_fresh_session(leaving_session).map_err(debug)?;
        let leaving = runtime.commit_fresh_session(slot).map_err(debug)?;
        runtime.initialize_pinned_test_position(leaving, outside).map_err(debug)?;
        (moving, start, leaving)
    };
    let barrier = Arc::new((tokio::sync::Notify::new(), tokio::sync::Notify::new()));
    let operation = PRE_SOURCE_BOSS_CHARGE.scope(barrier.clone(), operation);
    tokio::pin!(operation);
    tokio::select! {
        _ = &mut operation => return Err("encounter ended before durable write".into()),
        () = barrier.0.notified() => {}
    }
    let session = GameSessionId::decode(&id(247)).map_err(debug)?;
    let (newcomer, expected, logout_fence) = {
        let mut runtime = owner.runtime.lock().await;
        let next = MovementLocalPosition { x: outside.x + 1, ..outside };
        let mut movement = runtime.borrow_movement_position();
        let moved = movement.commit_cardinal(start, next).map_err(debug)?;
        let expected = movement.commit_turn(moved, MovementFacing::North).map_err(debug)?;
        let newcomer = runtime.reserve_fresh_session(session).map_err(debug)?;
        let logout_fence = runtime.mint_transition_fence().map_err(debug)?;
        runtime.remove_terminal_session(leaving_session, leaving).map_err(debug)?;
        (newcomer, expected, logout_fence)
    };
    barrier.1.notify_one();
    let outcome = operation.await;
    let mut runtime = owner.runtime.lock().await;
    assert_eq!(runtime.read_actor_position(moving).map_err(debug)?, expected);
    assert!(runtime.read_actor_position(leaving).is_err());
    assert!(runtime.mint_transition_fence().map_err(debug)? > logout_fence);
    runtime.remove_terminal_session(moving_session, moving).map_err(debug)?;
    let actor = runtime.commit_fresh_session(newcomer).map_err(debug)?;
    runtime.remove_terminal_session(session, actor).map_err(debug)?;
    Ok(outcome)
}

pub(super) async fn assert_paid_handoff(
    h: &Harness,
    deps: &Dependencies,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    lever: SourceBossLever,
    party: Vec<CurrentCharacterItemFence>,
) -> TestResult {
    let cooldowns = active_cooldowns(h, lever).await?;
    assert_eq!(cooldowns.len(), party.len());
    let admission: [u8; 16] = cooldowns
        .first()
        .ok_or("paid cooldown")?
        .1
        .as_slice()
        .try_into()?;
    assert!(cooldowns.iter().all(|(_, id, _)| id == &admission));
    assert_paid_receipt(h, &admission).await?;
    let scope = party.first().ok_or("staging party")?.runtime_scope;
    let request = SourceBossReservation {
        id: admission,
        lever,
        party,
    };

    sqlx::query(
        "INSERT INTO game_control_scope_grants VALUES
        (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 2)",
    )
    .bind(owner.world_id.as_bytes().as_slice())
    .bind(owner.channel_id.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    let predecessor = h
        .root
        .read_runtime_scope_predecessor(scope)
        .await
        .map_err(debug)?
        .ok_or("source boss predecessor")?;
    let successor = super::fixture::registered_node(&h.root, 2).await?;
    let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "sequencer-writer")
        .await
        .map_err(debug)?;
    let moved = writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([203; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Replace {
                scope,
                predecessor,
                target: successor.fact(),
            },
        })
        .await
        .map_err(debug)?;
    let AssignmentOutcome::Committed(moved) = moved else {
        return Err("source boss replacement refused".into());
    };
    assert_eq!(moved.assignment.ownership_generation, 2);
    publish_source_boss_successor_readiness(h, &successor, &moved).await?;

    assert!(
        !h.root
            .source_boss_lever_admission(
                owner.character,
                owner.holder,
                request.clone(),
                owner.reset_epoch,
                SourceBossAdmission::Reconcile,
            )
            .await?,
        "the old owner is fenced out after replacement"
    );
    let content = owner.runtime.lock().await.content_pin().clone();
    let assignment = moved.assignment;
    let fact = successor.fact();
    let runtime = ChannelRuntimeV1::from_committed_scope_assignment(
        scope,
        fact.node_id(),
        fact.registration_revision(),
        assignment.ownership_generation,
        assignment.source_revision,
        &assignment.decision_identity,
        2,
        content,
    )
    .map_err(debug)?;
    let runtime = tokio::sync::Mutex::new(runtime);
    let mut replacement = deps.owner_from(&h.root, &successor, owner.character);
    replacement.runtime = (&runtime).into();
    replacement.active_generation = owner.active_generation;
    assert!(
        replacement
            .runtime
            .lock()
            .await
            .positioned_actor_census()
            .map_err(debug)?
            .is_empty()
    );

    // Drive the real owner loop past the old recovery slot twice, with a fresh carrier.
    let stop = oteryn_foundation::CancellationToken::new();
    let swept = Arc::new(tokio::sync::Notify::new());
    let mut cycle = Box::pin(SOURCE_OWNER_SWEEP_COMPLETE.scope(
        Arc::clone(&swept),
        replacement.run_source_owner_cycles(&stop),
    ));
    for _ in 0..2 {
        tokio::time::timeout(Duration::from_secs(15), async {
            tokio::select! {
                () = &mut cycle => Err("successor owner loop stopped before completing its sweep"),
                () = swept.notified() => Ok(()),
            }
        })
        .await??;
        assert_eq!(active_cooldowns(h, lever).await?, cooldowns);
        assert_paid_receipt(h, &admission).await?;
    }
    stop.cancel();
    cycle.await;

    assert_fresh_admission_refused(h, &replacement, request, assignment.ownership_generation)
        .await?;
    assert_eq!(active_cooldowns(h, lever).await?, cooldowns);
    assert_paid_receipt(h, &admission).await?;
    Ok(())
}

pub(super) async fn active_cooldowns(
    h: &Harness,
    lever: SourceBossLever,
) -> TestResult<Vec<(Vec<u8>, Vec<u8>, i64)>> {
    let boss_key = match lever {
        SourceBossLever::CourtWarlock => {
            oteryn_gameplay_quests::quest::world_callbacks::court_warlock::BOSS
        }
        SourceBossLever::Arbaziloth => "oteryn:boss.arbaziloth",
    };
    Ok(sqlx::query_as(
        "SELECT uuid_send(character_id),uuid_send(admission),available_at
           FROM game_character_boss_cooldowns
          WHERE boss_key=$1
            AND available_at > floor(extract(epoch FROM statement_timestamp())*1000)::bigint
          ORDER BY character_id",
    )
    .bind(boss_key)
    .fetch_all(&h.pool)
    .await?)
}

pub(super) async fn assert_paid_receipt(h: &Harness, admission: &[u8; 16]) -> TestResult {
    let receipts: Vec<Vec<u8>> =
        sqlx::query_scalar("SELECT uuid_send(admission) FROM game_source_boss_admissions")
            .fetch_all(&h.pool)
            .await?;
    assert_eq!(
        receipts,
        [admission.to_vec()],
        "only the original paid receipt remains"
    );
    Ok(())
}

async fn assert_fresh_admission_refused(
    h: &Harness,
    replacement: &ComposedFreshAdmission<'_, '_, '_>,
    mut request: SourceBossReservation,
    generation: u64,
) -> TestResult {
    let generation = ScopeOwnershipGeneration::new(generation).map_err(debug)?;
    request.id = id(204);
    for member in &mut request.party {
        // Rebind the seeded sessions so the fresh command reaches the cooldown gate.
        let changed = sqlx::query(
            "UPDATE game_durability_reconnect_sessions SET scope_ownership_generation=$2::text::numeric
              WHERE game_session_id=encode($1,'hex')::uuid AND scope_ownership_generation=$3::text::numeric",
        )
        .bind(member.game_session_id.as_bytes().as_slice())
        .bind(generation.get().to_string())
        .bind(member.scope_ownership_generation.get().to_string())
        .execute(&h.pool).await?;
        assert_eq!(changed.rows_affected(), 1);
        member.scope_ownership_generation = generation;
    }
    let mut tx = h.pool.begin().await?;
    for member in &request.party {
        assert!(
            character_item_fence_is_current(
                &mut tx,
                replacement.holder,
                member,
                CommandRef::new(member.game_session_id, CommandId::new(2).map_err(debug)?),
                member.character_id,
                *replacement.world_id.as_bytes(),
                *replacement.channel_id.as_bytes(),
            )
            .await?,
            "fresh admission has current party fences"
        );
    }
    tx.rollback().await?;
    assert!(
        !replacement
            .root
            .source_boss_lever_admission(
                replacement.character,
                replacement.holder,
                request,
                replacement.reset_epoch,
                SourceBossAdmission::Commit,
            )
            .await?,
        "the same party cannot pay for a new attempt within the cooldown"
    );
    Ok(())
}

pub(super) async fn publish_source_boss_successor_readiness(
    h: &Harness,
    successor: &NodeIncarnationProof,
    moved: &AssignmentReceipt,
) -> TestResult {
    use crate::durability::admission_authority_guards::{
        AdmissionGuardStore, GuardPublicationDisposition,
    };
    use crate::durability::runtime_scope_assignment::AssignmentError;
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
        AdmissionAuthorityPublicationV1, AdmissionPublicationPreconditionV1,
    };

    let generation = moved.assignment.ownership_generation;
    assert_eq!(moved.assignment.holder, Some(successor.fact()));
    let key = AdmissionAuthorityGuardKeyV1::Runtime(moved.assignment.scope);
    let previous = AdmissionGuardStore::from_root(h.root.clone())
        .load(std::slice::from_ref(&key))
        .await
        .map_err(debug)?
        .pop()
        .flatten()
        .ok_or("successor readiness predecessor")?;
    assert_eq!(
        moved.fenced_publication_revision,
        Some(previous.publication_revision)
    );
    assert!(matches!(
        previous.state,
        AdmissionAuthorityGuardStateV1::Runtime {
            ownership_generation,
            ready: false,
            ..
        } if ownership_generation == generation
    ));

    let now = sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
        .fetch_one(&h.pool)
        .await?;
    let mut publisher = super::fresh_fixture::fresh::Source::new(now)?;
    publisher.rows.retain(|row| row.key == key);
    let [change] = publisher.rows.as_mut_slice() else {
        return Err("successor runtime publisher".into());
    };
    let AdmissionAuthorityGuardStateV1::Runtime {
        ownership_generation,
        ..
    } = &mut change.state
    else {
        return Err("successor runtime state".into());
    };
    *ownership_generation = generation;
    change.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
        expected_publication_revision: previous.publication_revision,
    };
    change.publication_revision = previous
        .publication_revision
        .checked_add(1)
        .ok_or("successor publication revision")?;
    change.source.source_revision = previous
        .source
        .source_revision
        .checked_add(1)
        .ok_or("successor source revision")?;
    change.source.decision_identity = format!("source-boss-runtime-ready-{generation}");

    // Isolate the authority mismatch in the previous readiness helper.
    let mut wrong_publisher = change.clone();
    wrong_publisher.source.authority = "game-runtime-publisher".into();
    let wrong_publication =
        crate::bestiary_postgres_harness::ground_native_readiness_enforcement::prepare(
            wrong_publisher,
            now,
        )?;
    assert_eq!(
        h.root
            .publish_runtime_readiness(successor, &wrong_publication)
            .await
            .map_err(debug)?,
        GuardPublicationDisposition::Conflict,
    );

    let publication = AdmissionAuthorityPublicationV1::prepare(&publisher, now).map_err(debug)?;
    assert!(matches!(
        h.root
            .publish_runtime_readiness(&h.node, &publication)
            .await,
        Err(AssignmentError::NotCurrentHolder)
    ));
    assert_eq!(
        h.root
            .publish_runtime_readiness(successor, &publication)
            .await
            .map_err(debug)?,
        GuardPublicationDisposition::Applied,
    );
    Ok(())
}
