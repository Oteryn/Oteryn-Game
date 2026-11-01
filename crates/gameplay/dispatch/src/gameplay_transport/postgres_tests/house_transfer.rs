//! HOUSE-TRANSFER-1 exercises the production composition through framed dispatch.
use super::*;
use crate::durability::house_scope_handoff::{
    HouseId, HouseScopeAssignmentCommand, HouseScopeAssignmentOutcome, HouseScopeAssignmentRequest,
};
use crate::durability::runtime_scope_assignment::{ControlActor, OperationKey};
use crate::foundation::{MessageType, MovementLocalPosition, decode_wire_envelope};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    ConnectionEnd, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use oteryn_protocol_oteryn::house_transfer as wire;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
const HOUSE: &str = "oteryn:content.house.admiral_s_avenue_1";

#[test]
fn house_transfer_postgres_real_dispatch_entry_exit_recovery_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "House transfer thread panicked")??;
    Ok(())
}
fn scenario() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    let rt = crate::bestiary_postgres_harness::runtime()?;
    let h = rt.block_on(Harness::create_with_admission_seed(
        admin,
        "house_transfer_dispatch",
        true,
        false,
    ))?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rt.block_on(case(&h))));
    let cleanup = rt.block_on(h.cleanup());
    match result {
        Ok(result) => {
            result?;
            cleanup
        }
        Err(panic) => std::panic::resume_unwind(panic),
    }
}
/// NODEBOOT-1: a same-session resume whose PREPARE meets a busy holder waits for it instead
/// of closing once it may have written (the node-boot `same-session resume diverged: Closed`).
#[test]
fn a_same_session_resume_waits_for_a_busy_holder_at_prepare_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| resume_scenario(false, false, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "busy-holder resume thread panicked")??;
    Ok(())
}
#[test]
fn a_resumed_loss_retries_an_unreadable_history_without_releasing_the_session_postgres()
-> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| resume_scenario(true, false, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "resumed-loss thread panicked")??;
    Ok(())
}
#[test]
fn c08_conditions_postgres_reconnect_preserves_poison_and_haste() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| resume_scenario(false, true, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "condition reconnect thread panicked")??;
    Ok(())
}

#[test]
fn c08_conditions_postgres_reconnect_with_delayed_pre_prepare_reads() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| resume_scenario(false, true, true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "delayed condition reconnect thread panicked")??;
    Ok(())
}

fn resume_scenario(history_read_fails: bool, conditions: bool, delayed_reads: bool) -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    let rt = crate::bestiary_postgres_harness::runtime()?;
    let h = rt.block_on(Harness::create_with_admission_seed(
        admin,
        "resume_busy_holder",
        true,
        false,
    ))?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        rt.block_on(resume_case(
            &h,
            history_read_fails,
            conditions,
            delayed_reads,
        ))
    }));
    let cleanup = rt.block_on(h.cleanup());
    match result {
        Ok(result) => {
            result?;
            cleanup
        }
        Err(panic) => std::panic::resume_unwind(panic),
    }
}
/// Drive the real resume until it is pending at the signalled boundary, with a deadline.
async fn wait_for_resume_barrier(
    mut resume: std::pin::Pin<&mut impl std::future::Future<Output = TestResult<AdmittedSession>>>,
    signal: &tokio::sync::Notify,
) -> TestResult {
    let mut notified = std::pin::pin!(signal.notified());
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        std::future::poll_fn(|cx| {
            use std::future::Future;
            use std::task::Poll;
            if let Poll::Ready(early) = resume.as_mut().poll(cx) {
                return Poll::Ready(Err(format!("resume ended before barrier: {early:?}")));
            }
            notified.as_mut().poll(cx).map(Ok)
        }),
    )
    .await
    .map_err(|_| "resume did not reach barrier before deadline")??;
    Ok(())
}

async fn resume_case(
    h: &Harness,
    history_read_fails: bool,
    conditions: bool,
    delayed_reads: bool,
) -> TestResult {
    let recovery_source = RecoverySource::start(h).await?;
    prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let mut owner = deps.owner(h, &authority);
    owner.evidence = &recovery_source.evidence;
    deps.initialize(&owner).await;
    owner
        .runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(deps.admitted.game_session_id)
        .await?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[86]).ok_or("selection")?;
    let before_conditions = if conditions {
        Some(install_reconnect_conditions(&owner, &deps).await?)
    } else {
        None
    };
    owner.states.command_ingress.admit(admitted.game_session_id);
    // The served connection ends at once; its loss is recorded as the listener records it.
    let (mut server, client) = tokio::io::duplex(1 << 20);
    drop(client);
    let admitted = match serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS).await {
        ConnectionEnd::AdmittedThenDisconnected(s) | ConnectionEnd::AdmittedThenClosed(s, _) => s,
        end => return Err(format!("served end: {end:?}").into()),
    };
    assert_eq!(
        owner
            .lose_control_for_cause(admitted, std::time::Duration::ZERO, None)
            .await,
        crate::gameplay_transport::connection::ControlLossResult::Recorded
    );
    let token = RecoverySource::token(7)?;
    // The held runtime stops the resume after its pre-PREPARE reads; the holder is taken
    // before the runtime is released, so PREPARE is the first pass that finds it busy.
    let runtime = owner.runtime.lock().await;
    let barrier = std::sync::Arc::new(crate::gameplay_transport::resume::PrepareTestBarrier {
        delay: delayed_reads,
        ..Default::default()
    });
    let mut resume = std::pin::pin!(
        crate::gameplay_transport::resume::PREPARE_TEST_BARRIER.scope(
            barrier.clone(),
            resume_same_session(&owner, admitted.game_session_id, &token, 10)
        )
    );
    if delayed_reads {
        wait_for_resume_barrier(resume.as_mut(), &barrier.entered).await?;
        // Keep a real root pass pending across the entire former 500 ms barrier.
        // This is the regression's injected delay, never the readiness condition.
        if let Ok(early) =
            tokio::time::timeout(std::time::Duration::from_millis(500), resume.as_mut()).await
        {
            return Err(format!("resume ended during delayed read: {early:?}").into());
        }
        barrier.release.notify_one();
    }
    wait_for_resume_barrier(resume.as_mut(), &barrier.ready).await?;
    let busy = hold_holder(&h.root, std::time::Duration::from_millis(300))?;
    drop(runtime);
    let resumed = resume.await;
    busy.await??;
    let resumed = resumed?;
    assert_eq!(resumed.game_session_id, admitted.game_session_id);
    assert_eq!(resumed.continuity.connection_generation, 2);
    if let Some(before) = before_conditions {
        assert_eq!(resumed.runtime_actor, Some(deps.actor));
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let after = states
            .get(&runtime, deps.actor, resumed.game_session_id)
            .ok_or("resumed condition state")?;
        assert_eq!(
            after.owned_conditions(),
            &before,
            "reconnect keeps definition, provenance, sequence, remaining damage and tick clock"
        );
    }
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(admitted.game_session_id)
        .await?;
    assert_eq!(
        current.session_state(),
        crate::foundation::GameSessionState::Active
    );
    if history_read_fails {
        let store = FreshAdmissionStore::from_root(h.root.clone());
        let held = h.root.try_issue_semantic_pass()?;
        assert!(matches!(
            store.resumed_history(resumed.game_session_id).await,
            Err(crate::durability::DurabilityError::RootUnavailable)
        ));
        held.run(|_, _| Box::pin(async { Ok(()) })).await?;
        assert!(
            store
                .resumed_history(resumed.game_session_id)
                .await?
                .is_some()
        );
        // Inject a recoverable read failure only at the retained-history boundary.
        // Current-session reads and relation fencing remain available.
        sqlx::query("REVOKE SELECT ON game_durability_admission_lifecycle_receipts FROM oteryn_game_runtime")
            .execute(&h.pool).await?;
        h.root
            .try_issue_semantic_pass()?
            .run(|holder, _| {
                Box::pin(async move {
                    sqlx::query("SET ROLE oteryn_game_runtime")
                        .execute(&mut **holder)
                        .await?;
                    Ok(())
                })
            })
            .await?;
        assert_eq!(
            store.current_session(resumed.game_session_id).await?,
            current
        );
        assert!(
            store
                .resumed_history(resumed.game_session_id)
                .await
                .is_err()
        );
        let loss = owner
            .lose_control_for_cause(resumed, std::time::Duration::ZERO, None)
            .await;
        sqlx::query(
            "GRANT SELECT ON game_durability_admission_lifecycle_receipts TO oteryn_game_runtime",
        )
        .execute(&h.pool)
        .await?;
        assert_eq!(
            loss,
            crate::gameplay_transport::connection::ControlLossResult::Unknown,
            "unavailable history must be retried, not treated as invalid history"
        );
        assert_eq!(
            store.current_session(resumed.game_session_id).await?,
            current
        );
    }
    assert_eq!(
        owner
            .lose_control_for_cause(resumed, std::time::Duration::ZERO, None)
            .await,
        crate::gameplay_transport::connection::ControlLossResult::Recorded,
        "a resumed connection must open its next reconnect window"
    );
    Ok(())
}
async fn install_reconnect_conditions(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
) -> TestResult<oteryn_gameplay_foundation::foundation::condition::ConditionStore<String>> {
    use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement, SpeedRange};
    use oteryn_gameplay_foundation::foundation::condition::{
        ApplicationFacts, ConditionDefinition,
    };
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    // Synthetic condition magnitudes isolate lifetime from combat/content balance. The
    // framed ClientResume below drives the real PostgreSQL reconnect owner unchanged.
    let definitions = [
        ConditionDefinition::new(
            "fixture:reconnect.poison",
            1,
            ConditionValues::DamageOverTime {
                element: DotElement::Poison,
                total_min: 100,
                total_max: 100,
                per_tick: 5,
                interval_ms: 5_000,
                delayed: true,
            },
        )
        .ok_or("poison fixture")?,
        ConditionDefinition::new(
            "fixture:reconnect.haste",
            1,
            ConditionValues::Speed {
                paralysis: false,
                range: SpeedRange {
                    a_min: 1000,
                    b_min: 100_000,
                    a_max: 1000,
                    b_max: 100_000,
                },
                duration_ms: 60_000,
            },
        )
        .ok_or("haste fixture")?,
    ];
    let runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    let state = states
        .get(&runtime, deps.actor, deps.admitted.game_session_id)
        .ok_or("condition fixture state")?;
    let root = GameplayDecisionRoot::from_bytes([7; 32]);
    let next = state
        .stage_owned_test_conditions(
            &definitions,
            &ApplicationFacts {
                now: owner.owner_now().get(),
                base_speed: 220,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &root,
                occurrence: DecisionOccurrenceId::from_bytes([7; 16]),
            },
        )
        .ok_or("condition fixture stage")?;
    assert_eq!(next.owned_conditions().instances().len(), 2);
    let snapshot = next.owned_conditions().clone();
    assert!(states.commit(&runtime, deps.actor, deps.admitted.game_session_id, next));
    Ok(snapshot)
}

async fn resume_same_session(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    source: crate::foundation::GameSessionId,
    token: &str,
    transport: u8,
) -> TestResult<AdmittedSession> {
    let mut body = Vec::new();
    bytes(&mut body, 1, source.as_bytes());
    bytes(&mut body, 2, token.as_bytes());
    body.extend([24, 0]);
    scalar(&mut body, 4, 1);
    scalar(&mut body, 5, 1);
    scalar(&mut body, 6, 1);
    scalar(&mut body, 7, 86);
    bytes(&mut body, 8, b"resume-busy-holder-test");
    let mut frame = vec![8, MessageType::ClientResume as u8];
    bytes(&mut frame, 4, &body);
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let admitted = crate::gameplay_transport::connection::admit_frame(
        &mut server,
        &frame,
        owner,
        &RecoveryIdentifiers(transport),
    )
    .await
    .map_err(|e| format!("same-session resume {e:?}"))?;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let n = u32::from_be_bytes(output.get(..4).ok_or("no reply")?.try_into()?) as usize;
    let envelope = decode_wire_envelope(&output[4..4 + n]).map_err(debug)?;
    assert_eq!(envelope.message_type(), MessageType::ServerResumeAccepted);
    Ok(admitted)
}
async fn case(h: &Harness) -> TestResult {
    let recovery_source = RecoverySource::start(h).await?;
    prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let base = Dependencies::new()?;
    let world = transfer_bundle(base.admitted.world_id, base.admitted.channel_id)?;
    let cells = world
        .movement_cells(base.room.movement_cells())
        .map_err(debug)?;
    let pin = ChannelContentPin::from_activation(
        base.admitted.world_id,
        1,
        cells.scope().generation_digest,
        [2; 32],
        [3; 32],
        [4; 32],
        (32309, 32805, 7),
    );
    let deps = Dependencies::with_content(pin, base.room, base.spells)?;
    let mut owner = deps.owner(h, &authority);
    owner.movement_cells = &cells;
    owner.evidence = &recovery_source.evidence;
    deps.initialize(&owner).await;
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 32309,
                    y: 32805,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
    }
    sqlx::query(include_str!(
        "../../../../../../apps/game-server/tests/support/owned_house_fixture.sql"
    ))
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(id(41).as_slice())
    .execute(&h.pool)
    .await?;
    sqlx::query("INSERT INTO game_control_house_scope_grants(control_role,world_id,house_key,operation) SELECT session_user,encode($1,'hex')::uuid,$2,1").bind(id(42).as_slice()).bind(HOUSE).execute(&h.pool).await?;
    assert!(matches!(
        h.root
            .assign_house_scope(HouseScopeAssignmentRequest {
                operation_key: OperationKey::from_bytes([82; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                actor_role: "oteryn_test_admin".into(),
                command: HouseScopeAssignmentCommand::Assign {
                    house: HouseId::new(deps.admitted.world_id, HOUSE).ok_or("house")?,
                    target: h.node.fact()
                }
            })
            .await
            .map_err(debug)?,
        HouseScopeAssignmentOutcome::Committed(_)
    ));
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(deps.admitted.game_session_id)
        .await?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[86]).ok_or("selection")?;
    let source = admitted;
    let mut unsupported = source;
    unsupported.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[]).ok_or("empty selection")?;
    let (_, status, payload) = drive(&owner, unsupported, false)
        .await
        .map_err(|error| format!("House unsupported selection: {error}"))?;
    assert_ne!(status, CommandStatus::Accepted);
    assert!(payload.is_empty());
    // Each stale value changes just one expected generation and leaves native facts live.
    for field in 0..3 {
        let mut fresh = deps.owner(h, &authority);
        fresh.movement_cells = &cells;
        fresh.evidence = &recovery_source.evidence;
        let mut stale = source;
        let fence = stale.item_fence.as_mut().ok_or("fence")?;
        match field {
            0 => {
                fence.connection_generation =
                    crate::foundation::ConnectionGeneration::new(2).map_err(debug)?
            }
            1 => fence.character_lease_generation += 1,
            _ => {
                fence.scope_ownership_generation =
                    crate::foundation::ScopeOwnershipGeneration::new(2).map_err(debug)?
            }
        }
        let (_, status, payload) = drive(&fresh, stale, false)
            .await
            .map_err(|error| format!("House stale fence field {field}: {error}"))?;
        assert_eq!(status, CommandStatus::Rejected);
        assert!(payload.is_empty());
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_house_transfer_receipts")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(count, 0);
    }
    // Use a new command directory for the valid source after the refused commands.
    drop(owner);
    let mut owner = deps.owner(h, &authority);
    owner.movement_cells = &cells;
    owner.evidence = &recovery_source.evidence;
    let (inside, status, ack) = drive(&owner, admitted, false)
        .await
        .map_err(|error| format!("House current source entry: {error}"))?;
    assert_eq!(status, CommandStatus::Accepted);
    assert!(!ack.is_empty());
    assert_ne!(inside.game_session_id, source.game_session_id);
    let state = owner
        .runtime
        .lock()
        .await
        .house_actor_state(deps.actor, inside.game_session_id)
        .map_err(debug)?;
    assert_eq!(state.house, HOUSE);
    assert_eq!(state.lease.generation(), 3);
    assert!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .is_err()
    );
    let (outside, status, ack) = drive(&owner, inside, true)
        .await
        .map_err(|error| format!("House current destination exit: {error}"))?;
    assert_eq!(status, CommandStatus::Accepted);
    assert!(!ack.is_empty());
    assert_eq!(
        outside
            .item_fence
            .ok_or("exit fence")?
            .character_lease_generation,
        4
    );
    assert_eq!(outside.channel_id, source.channel_id);
    let runtime = owner.runtime.lock().await;
    assert_eq!(
        runtime
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position(),
        MovementLocalPosition {
            x: 32309,
            y: 32805,
            floor: 7
        }
    );
    assert!(
        runtime
            .house_actor_state(deps.actor, inside.game_session_id)
            .is_err()
    );
    drop(runtime);
    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_house_transfer_receipts WHERE state=2")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(rows, 2);
    let pending = lose_ack(&owner, outside, false).await?;
    assert_ne!(pending.game_session_id, outside.game_session_id);
    assert_eq!(
        pending
            .item_fence
            .ok_or("lost ack fence")?
            .character_lease_generation,
        5
    );
    drop(owner);
    // Lose both the connection directory and the entire carrier. The current
    // same-node assignment remains authoritative; replaced assignments refuse.
    let pin = deps.runtime.lock().await.content_pin().clone();
    *deps.runtime.lock().await = ChannelRuntimeV1::from_committed_assignment(
        source.world_id,
        source.channel_id,
        h.node.fact().node_id(),
        h.node.fact().registration_revision(),
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        pin,
    )
    .map_err(debug)?;
    let mut restarted = deps.owner(h, &authority);
    restarted.movement_cells = &cells;
    restarted.evidence = &recovery_source.evidence;
    let token = RecoverySource::token(7)?;
    assert!(
        resume_transfer(&restarted, source.game_session_id, "invalid", 10)
            .await
            .is_err()
    );
    // NODEBOOT-1: a resume whose first read, its receipt, meets a busy holder retries it
    // instead of closing.
    let busy = hold_holder(restarted.root, std::time::Duration::from_millis(300))?;
    let recovered = resume_transfer(&restarted, source.game_session_id, &token, 10).await?;
    busy.await??;
    let replayed = resume_transfer(&restarted, source.game_session_id, &token, 10).await?;
    assert_eq!(replayed.game_session_id, recovered.game_session_id);
    assert_eq!(replayed.runtime_actor, recovered.runtime_actor);
    assert_eq!(replayed.continuity.connection_generation, 2);
    assert!(
        resume_transfer(&restarted, source.game_session_id, &token, 11)
            .await
            .is_err()
    );
    assert_eq!(recovered.game_session_id, pending.game_session_id);
    assert_eq!(recovered.continuity.connection_generation, 2);
    let recovered_actor = recovered.runtime_actor.ok_or("recovered actor")?;
    let native = restarted
        .runtime
        .lock()
        .await
        .house_actor_state(recovered_actor, recovered.game_session_id)
        .map_err(debug)?;
    assert_eq!(native.lease.generation(), 5);
    assert!(
        restarted
            .runtime
            .lock()
            .await
            .positioned_actor_census()
            .map_err(debug)?
            .is_empty()
    );
    let (used, status, _) = drive(&restarted, recovered, false).await?;
    assert_eq!(status, CommandStatus::Rejected);
    let later_token = RecoverySource::token(9)?;
    assert!(
        resume_transfer(&restarted, recovered.game_session_id, &later_token, 12)
            .await
            .is_err()
    );
    let pending_exit = lose_ack(&restarted, used, true).await?;
    assert_eq!(
        pending_exit
            .item_fence
            .ok_or("exit fence")?
            .character_lease_generation,
        6
    );
    drop(restarted);
    let pin = deps.runtime.lock().await.content_pin().clone();
    *deps.runtime.lock().await = ChannelRuntimeV1::from_committed_assignment(
        source.world_id,
        source.channel_id,
        h.node.fact().node_id(),
        h.node.fact().registration_revision(),
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        pin,
    )
    .map_err(debug)?;
    let mut restarted = deps.owner(h, &authority);
    restarted.movement_cells = &cells;
    restarted.evidence = &recovery_source.evidence;
    let token = RecoverySource::token(8)?;
    let after = resume_transfer(&restarted, source.game_session_id, &token, 11).await?;
    assert_eq!(after.game_session_id, pending_exit.game_session_id);
    assert_eq!(after.continuity.connection_generation, 2);
    assert_eq!(
        after
            .item_fence
            .ok_or("exit fence")?
            .character_lease_generation,
        6
    );
    let runtime = restarted.runtime.lock().await;
    assert_eq!(runtime.positioned_actor_census().map_err(debug)?.len(), 1);
    assert_eq!(
        runtime
            .read_actor_position(after.runtime_actor.ok_or("outside actor")?)
            .map_err(debug)?
            .position(),
        MovementLocalPosition {
            x: 32309,
            y: 32805,
            floor: 7
        }
    );
    drop(runtime);
    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_house_transfer_receipts WHERE state=2")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(rows, 4);
    let live: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_durability_reconnect_sessions WHERE session_state IN (1,2)",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(live, 1);
    let consumed: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_durability_recovery_grant_consumptions")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(consumed, 2);
    Ok(())
}
async fn drive(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    exit: bool,
) -> TestResult<(AdmittedSession, CommandStatus, Vec<u8>)> {
    let mut body = vec![8, if exit { 2 } else { 1 }, 18, HOUSE.len() as u8];
    body.extend_from_slice(HOUSE.as_bytes());
    let frame = encode_client_command(
        admitted.continuity.connection_generation,
        &ClientCommandValue {
            command_id: admitted.continuity.next_command_id,
            command_type: wire::COMMAND_TYPE_HOUSE_TRANSFER,
            payload: &body,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client
        .write_all(&(frame.len() as u32).to_be_bytes())
        .await?;
    client.write_all(&frame).await?;
    client.shutdown().await?;
    let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    let next = match end {
        ConnectionEnd::AdmittedThenDisconnected(s) | ConnectionEnd::AdmittedThenClosed(s, _) => s,
        _ => return Err(format!("transfer end: {end:?}").into()),
    };
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let mut at = 0;
    let mut result = None;
    let mut destination_snapshot = false;
    while at < output.len() {
        let n = u32::from_be_bytes(output[at..at + 4].try_into()?) as usize;
        let envelope = decode_wire_envelope(&output[at + 4..at + 4 + n]).map_err(debug)?;
        at += 4 + n;
        if envelope.message_type() == MessageType::CommandResult {
            let reply = decode_command_result(envelope.payload()).map_err(debug)?;
            assert!(result.is_none());
            result = Some((reply.status, reply.payload.to_vec()));
        } else if envelope.message_type() == MessageType::SnapshotBegin
            && result
                .as_ref()
                .is_some_and(|(status, _)| *status == CommandStatus::Accepted)
        {
            destination_snapshot = true;
            assert_eq!(
                envelope.connection_generation(),
                next.continuity.connection_generation
            );
        }
    }
    let (status, payload) = result.ok_or_else(|| format!("missing House result: exit={exit}"))?;
    if status == CommandStatus::Accepted {
        let receipt = owner
            .root
            .house_transfer_recovery(next.game_session_id)
            .await?
            .ok_or("missing receipt")?;
        assert_eq!(receipt.ack.world, *admitted.world_id.as_bytes());
        assert_eq!(receipt.ack.channel, *admitted.channel_id.as_bytes());
        assert_eq!(
            receipt.ack.house.as_deref(),
            if exit { None } else { Some(HOUSE) }
        );
        assert_eq!(
            receipt.ack.selection,
            admitted.continuity.selected_capabilities.as_slice()
        );
        assert_eq!(
            payload,
            wire::encode_house_transfer_ack(&receipt.ack).map_err(debug)?
        );
        assert!(
            destination_snapshot,
            "scope acknowledgement must precede the destination snapshot"
        );
    }
    Ok((next, status, payload))
}

// Initial authority fixture: the composition owns these two guards before the
// transfer. The runtime readiness still comes from the native assignment API.
// Negative cases above never derive their current facts from the transfer record.
pub(super) async fn prepare_owned_session(h: &Harness) -> TestResult {
    use crate::durability::admission_authority_guards::AdmissionGuardStore;
    use crate::foundation::admission_authority_publication::*;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let mut source = super::fresh_fixture::fresh::Source::new(now)?;
    for row in &mut source.rows {
        if matches!(
            row.key,
            AdmissionAuthorityGuardKeyV1::Account { .. }
                | AdmissionAuthorityGuardKeyV1::Character(_)
        ) {
            row.source.authority =
                crate::durability::fresh_admission_composition::COMPOSITION_SOURCE_AUTHORITY.into();
            if let AdmissionAuthorityGuardStateV1::Account { security, .. } = &mut row.state {
                security.provenance.source_authority = "platform-test".into();
            }
        }
    }
    let mut runtime = super::fresh_fixture::fresh::Source::new(now)?;
    runtime
        .rows
        .retain(|r| matches!(r.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    h.root
        .publish_runtime_readiness(
            &h.node,
            &AdmissionAuthorityPublicationV1::prepare(&runtime, now).map_err(debug)?,
        )
        .await
        .map_err(debug)?;
    let mut claims = super::fresh_fixture::fresh::Source::new(now)?;
    claims.rows = source
        .rows
        .iter()
        .filter(|r| !matches!(r.key, AdmissionAuthorityGuardKeyV1::Runtime(_)))
        .cloned()
        .collect();
    AdmissionGuardStore::from_root(h.root.clone())
        .publish(&AdmissionAuthorityPublicationV1::prepare(&claims, now).map_err(debug)?)
        .await
        .map_err(debug)?;
    let outcome = FreshAdmissionStore::from_root(h.root.clone())
        .commit(&source.request()?)
        .await
        .map_err(debug)?;
    assert!(matches!(
        outcome,
        crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1::Committed(_)
    ));
    Ok(())
}

pub(super) struct RecoverySource {
    pub(super) evidence: crate::gameplay_transport::fresh_evidence::FreshEvidenceSource,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for RecoverySource {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl RecoverySource {
    pub(super) async fn start(h: &Harness) -> TestResult<Self> {
        use crate::durability::native_admission_source::{
            DescriptorRegistration, FreshStoreProvenance,
        };
        use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
        use std::sync::Arc;
        let material = rcgen::generate_simple_self_signed(vec!["source.test".into()])?;
        let certificate = material.cert.der().clone();
        let key = PrivatePkcs8KeyDer::from(material.signing_key.serialize_der());
        let mut config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(vec![certificate.clone()], key.into())?;
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let descriptor=crate::native_admission_source::descriptor::ProducerDescriptor::new("platform-test".into(),("127.0.0.1".into(),port),"source.test".into(),"source.test".into(),vec![certificate],vec![CertificateDer::from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client.der").to_vec())],PrivateKeyDer::try_from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client-key.der").to_vec())?).map_err(debug)?;
        let registration = DescriptorRegistration {
            revision: 1,
            facts: vec![1],
            installed_at: 1,
        };
        let provenance = FreshStoreProvenance {
            namespace: "house-transfer-fixture".into(),
            authorization: "owner:test".into(),
            source_authority: "platform-test".into(),
            initialized_at: 1,
        };
        h.root
            .record_native_source_descriptor_issuance(
                "platform-test",
                registration.clone(),
                Some(provenance.clone()),
            )
            .await?;
        h.root
            .initialize_native_admission_source(&h.node, provenance, registration)
            .await?;
        let task = tokio::spawn(async move {
            let mut revision = 1_u64;
            loop {
                let Ok((socket, _)) = listener.accept().await else {
                    return;
                };
                let Ok(mut io) = acceptor.accept(socket).await else {
                    continue;
                };
                let mut request = Vec::new();
                loop {
                    let mut byte = [0];
                    if io.read_exact(&mut byte).await.is_err() {
                        return;
                    }
                    request.push(byte[0]);
                    if request.ends_with(b"\r\n\r\n") {
                        break;
                    }
                    assert!(request.len() < 4096);
                }
                let header = String::from_utf8_lossy(&request);
                let length = header
                    .lines()
                    .find_map(|l| l.strip_prefix("Content-Length: "))
                    .and_then(|n| n.parse::<usize>().ok())
                    .unwrap_or(0);
                assert!(length <= 1024);
                let mut body = vec![0; length];
                if io.read_exact(&mut body).await.is_err() {
                    return;
                }
                let Ok(input) = serde_json::from_slice::<serde_json::Value>(&body) else {
                    return;
                };
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|v| v.as_secs())
                    .unwrap_or(0);
                revision += 1;
                let recovery = input["operation"]
                    .as_str()
                    .is_some_and(|s| s.contains("Recovery"));
                let mut reply = serde_json::json!({"version":if recovery{2}else{1},"operation":input["operation"],"result":"observed","source_authority":"platform-test","source_revision":revision.to_string(),"decision_identity":revision.to_string(),"source_observed_at":now.to_string(),"clock_uncertainty_seconds":"0"});
                if input["operation"]
                    .as_str()
                    .is_some_and(|s| s.contains("AccountSecurity"))
                {
                    reply["account_id"] = input["account_id"].clone();
                    reply["purpose"] = "platform_security".into();
                    reply["scope"] = if recovery {
                        "existing_actor_recovery"
                    } else {
                        "fresh_admission"
                    }
                    .into();
                    reply["allowed"] = true.into();
                    reply["minimum_valid_generation"] = "1".into();
                } else {
                    use base64::Engine;
                    reply["issuer"] = if recovery { "urn:oteryn:platform:game-recovery" }
                        else { "urn:oteryn:platform:game-admission" }.into();
                    reply["profile"] = if recovery { "oteryn-reauth-recovery-v1" }
                        else { "oteryn-pre-admission-v1" }.into();
                    reply["key_purpose"] = if recovery { "existing_actor_recovery" }
                        else { "fresh_admission" }.into();
                    reply["key_id"] = if recovery { "house-recovery" } else { "house-fresh" }.into();
                    reply["trusted"] = true.into();
                    reply["public_key"] = base64::engine::general_purpose::URL_SAFE_NO_PAD
                        .encode(
                            ed25519_dalek::SigningKey::from_bytes(&[if recovery { 23 } else { 31 }; 32])
                                .verifying_key()
                                .to_bytes(),
                        )
                        .into();
                }
                let body = reply.to_string();
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                if io.write_all(response.as_bytes()).await.is_err() {
                    return;
                }
                let _ = io.shutdown().await;
            }
        });
        Ok(Self {
            evidence: crate::gameplay_transport::fresh_evidence::FreshEvidenceSource::new(
                descriptor,
            ),
            task,
        })
    }
    pub(super) fn token(nonce: u8) -> TestResult<String> {
        use base64::Engine;
        use ed25519_dalek::Signer;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let encode = |v: String| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(v);
        let header =
            serde_json::json!({"alg":"Ed25519","kid":"house-recovery","typ":"oteryn-recovery+jwt"});
        let payload = serde_json::json!({"iss":"urn:oteryn:platform:game-recovery","aud":"urn:oteryn:game:recovery","iat":now,"nbf":now,"exp":now+10,"jti":base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([nonce;32]),"profile":"oteryn-reauth-recovery-v1","purpose":"existing_actor_recovery","attempt_ref":"00000000-0000-7000-8000-000000000001","account_id":"28020304-0506-7008-800a-0b0c0d0e0f28","character_id":"29020304-0506-7008-800a-0b0c0d0e0f29","world_id":"2a020304-0506-7008-800a-0b0c0d0e0f2a","account_security_generation":"1","protocol_major":1,"transport_profile":1,"ruleset_revision":"ruleset-1","content_revision":"content-1","map_revision":"map-1","world_policy_revision":"policy-1"});
        let input = format!(
            "{}.{}",
            encode(header.to_string()),
            encode(payload.to_string())
        );
        let signature = ed25519_dalek::SigningKey::from_bytes(&[23; 32]).sign(input.as_bytes());
        Ok(format!(
            "{input}.{}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature.to_bytes())
        ))
    }

    /// A separately scoped fresh grant from this registered independent test
    /// producer. The native fresh-admission verifier/publication/commit remain
    /// unchanged; a recovery credential cannot be reused as a fresh grant.
    pub(super) fn fresh_token(nonce: u8) -> TestResult<String> {
        use base64::Engine;
        use ed25519_dalek::Signer;
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
        let encode = |value: String| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(value);
        let header = serde_json::json!({"alg":"Ed25519","kid":"house-fresh","typ":"oteryn-admission+jwt"});
        let payload = serde_json::json!({
            "iss":"urn:oteryn:platform:game-admission", "aud":"urn:oteryn:game:admission",
            "iat":now, "nbf":now, "exp":now+10,
            "jti":base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([nonce;32]),
            "profile":"oteryn-pre-admission-v1", "purpose":"fresh_entry",
            "attempt_ref":crate::gameplay_transport::canonical_uuid(&id(nonce)),
            "account_id":"28020304-0506-7008-800a-0b0c0d0e0f28",
            "character_id":"29020304-0506-7008-800a-0b0c0d0e0f29",
            "world_id":"2a020304-0506-7008-800a-0b0c0d0e0f2a",
            "channel_id":"2b020304-0506-7008-800a-0b0c0d0e0f2b",
            "account_security_generation":"1", "route_revision":"route-1",
            "runtime_observation_revision":"runtime-1", "scope_ownership_generation":"1",
            "protocol_major":1, "transport_profile":1, "ruleset_revision":"ruleset-1",
            "content_revision":"content-1", "map_revision":"map-1",
            "world_policy_revision":"policy-1", "offer_revision":"offer-1"
        });
        let input = format!("{}.{}", encode(header.to_string()), encode(payload.to_string()));
        let signature = ed25519_dalek::SigningKey::from_bytes(&[31;32]).sign(input.as_bytes());
        Ok(format!("{input}.{}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature.to_bytes())))
    }
}

struct DroppedAck {
    io: tokio::io::DuplexStream,
    dropped: bool,
}
impl tokio::io::AsyncRead for DroppedAck {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.io).poll_read(cx, buf)
    }
}
impl tokio::io::AsyncWrite for DroppedAck {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        if decode_wire_envelope(buf).is_ok_and(|e| {
            e.message_type() == MessageType::CommandResult
                && decode_command_result(e.payload())
                    .is_ok_and(|r| r.status == CommandStatus::Accepted && !r.payload.is_empty())
        }) {
            self.dropped = true;
            return std::task::Poll::Ready(Err(std::io::ErrorKind::BrokenPipe.into()));
        }
        std::pin::Pin::new(&mut self.io).poll_write(cx, buf)
    }
    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.io).poll_flush(cx)
    }
    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.io).poll_shutdown(cx)
    }
}
async fn lose_ack(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    exit: bool,
) -> TestResult<AdmittedSession> {
    let mut body = vec![8, if exit { 2 } else { 1 }, 18, HOUSE.len() as u8];
    body.extend_from_slice(HOUSE.as_bytes());
    let frame = encode_client_command(
        admitted.continuity.connection_generation,
        &ClientCommandValue {
            command_id: admitted.continuity.next_command_id,
            command_type: wire::COMMAND_TYPE_HOUSE_TRANSFER,
            payload: &body,
        },
    )
    .map_err(debug)?;
    let (server, mut client) = tokio::io::duplex(1 << 20);
    let mut server = DroppedAck {
        io: server,
        dropped: false,
    };
    client
        .write_all(&(frame.len() as u32).to_be_bytes())
        .await?;
    client.write_all(&frame).await?;
    client.shutdown().await?;
    let ended = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
    assert!(
        server.dropped,
        "the transfer must commit before its ack is lost"
    );
    match ended {
        ConnectionEnd::AdmittedThenDisconnected(current) => Ok(current),
        _ => Err(format!("lost ack end {ended:?}").into()),
    }
}
/// Takes `root`'s holder at once and keeps it for `hold` in a semantic pass.
fn hold_holder(
    root: &crate::durability::DurabilityRoot,
    hold: std::time::Duration,
) -> TestResult<tokio::task::JoinHandle<Result<(), crate::durability::DurabilityError>>> {
    let pass = root.try_issue_semantic_pass().map_err(debug)?;
    Ok(tokio::spawn(pass.run(move |_holder, _deadline| {
        Box::pin(async move {
            tokio::time::sleep(hold).await;
            Ok(())
        })
    })))
}
struct RecoveryIdentifiers(u8);
impl crate::gameplay_transport::connection::ConnectionIdentifiers for RecoveryIdentifiers {
    fn game_session_id(&self) -> Option<crate::foundation::GameSessionId> {
        None
    }
    fn transport_ref(&self) -> Option<crate::foundation::AuthenticatedTransportRefV1> {
        crate::foundation::AuthenticatedTransportRefV1::decode(&[self.0; 16]).ok()
    }
}
async fn resume_transfer(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    source: crate::foundation::GameSessionId,
    token: &str,
    transport: u8,
) -> TestResult<AdmittedSession> {
    let mut body = Vec::new();
    bytes(&mut body, 1, source.as_bytes());
    bytes(&mut body, 2, token.as_bytes());
    // last_applied_server_sequence is explicitly present, including zero.
    body.extend([24, 0]);
    scalar(&mut body, 4, 1);
    scalar(&mut body, 5, 1);
    scalar(&mut body, 6, 1);
    scalar(&mut body, 7, 86);
    bytes(&mut body, 8, b"house-transfer-test");
    let mut frame = vec![8, MessageType::ClientResume as u8];
    bytes(&mut frame, 4, &body);
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let admitted = crate::gameplay_transport::connection::admit_frame(
        &mut server,
        &frame,
        owner,
        &RecoveryIdentifiers(transport),
    )
    .await
    .map_err(|e| format!("House recovery {e:?}"))?;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let mut at = 0;
    let mut messages = Vec::new();
    while at < output.len() {
        let n = u32::from_be_bytes(output[at..at + 4].try_into()?) as usize;
        let envelope = decode_wire_envelope(&output[at + 4..at + 4 + n]).map_err(debug)?;
        at += 4 + n;
        messages.push(envelope.message_type());
        if envelope.message_type() == MessageType::CommandResult {
            let ack = decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(ack.status, CommandStatus::Accepted);
            assert!(!ack.payload.is_empty());
            let receipt = owner
                .root
                .house_transfer_recovery(admitted.game_session_id)
                .await?
                .ok_or("missing receipt")?;
            assert_eq!(receipt.ack.world, *admitted.world_id.as_bytes());
            assert_eq!(receipt.ack.channel, *admitted.channel_id.as_bytes());
            let mut expected = receipt.ack;
            expected.connection_generation = admitted.continuity.connection_generation;
            assert_eq!(
                ack.payload,
                wire::encode_house_transfer_ack(&expected).map_err(debug)?
            );
        }
    }
    assert_eq!(
        messages,
        vec![
            MessageType::ServerResumeAccepted,
            MessageType::CommandResult
        ]
    );
    Ok(admitted)
}

fn scalar(out: &mut Vec<u8>, tag: u8, mut value: u64) {
    out.push(tag << 3);
    while value >= 128 {
        out.push(value as u8 | 128);
        value >>= 7;
    }
    out.push(value as u8);
}
fn bytes(out: &mut Vec<u8>, tag: u8, value: &[u8]) {
    out.push(tag << 3 | 2);
    let mut n = value.len();
    while n >= 128 {
        out.push(n as u8 | 128);
        n >>= 7;
    }
    out.push(n as u8);
    out.extend_from_slice(value);
}

fn transfer_bundle(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    use crate::map::boot::{self, tests as fixture};
    use oteryn_world_bundle::bundle;
    let (bytes, mut pins) =
        fixture::bundle_with_items("oteryn:item.tibia.i1629", "oteryn:item.tibia.i1000");
    let mut data = bundle::read(&bytes).map_err(debug)?;
    let sector = data.sectors.first_mut().ok_or("sector")?;
    let mut entrance = sector
        .tiles
        .iter()
        .find(|t| t.x == 3)
        .ok_or("grass tile")?
        .clone();
    for tile in &mut sector.tiles {
        if tile.x == 6 {
            tile.house = 3437;
            tile.items.last_mut().ok_or("door")?.attrs.door = Some(42);
        }
        tile.x += 32303;
        tile.y += 32804;
    }
    entrance.x = 32309;
    entrance.y = 32805;
    sector.tiles.push(entrance);
    sector.sx = 1009;
    sector.sy = 1025;
    data.manifest.world.min_x = 32288;
    data.manifest.world.min_y = 32800;
    data.manifest.world.max_x = 32320;
    data.manifest.world.max_y = 32832;
    let bytes = oteryn_world_bundle_compiler::bundle::write_with_npcs(
        &data.manifest,
        &data.sectors,
        &data.spawns,
        &data.npcs,
    )
    .map_err(debug)?;
    pins.digest = bundle::read(&bytes).map_err(debug)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    boot::boot(
        &bytes,
        &boot::BootPins {
            bundle: pins,
            map_revision: crate::map::overlay::map_revision(&base),
            start: crate::map::overlay::TilePos {
                x: 32309,
                y: 32805,
                floor: -7,
            },
        },
        world,
        channel,
        fixture::door_items,
    )
    .map_err(debug)
}
