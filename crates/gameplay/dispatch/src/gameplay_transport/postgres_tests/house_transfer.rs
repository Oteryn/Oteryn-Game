//! HOUSE-TRANSFER-1 exercises the production composition through framed dispatch.
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use super::house_transfer_fixtures::{RecoveryIdentifiers, RecoverySource, TRANSFER_TEST_HOUSE as HOUSE, bytes, prepare_owned_session, prepare_transfer_scene, initialize_transfer_source, resume_same_session, scalar};

use super::*;
use crate::foundation::{MessageType, MovementLocalPosition, decode_wire_envelope};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    ConnectionEnd, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use oteryn_protocol_oteryn::house_transfer as wire;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};


#[test]
fn house_transfer_postgres_real_dispatch_entry_exit_recovery_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "House transfer thread panicked")??;
    eprintln!("HOUSE_TRANSFER POSTGRES_CASE_COMPLETED");
    Ok(())
}
#[test]
fn house_handoff_charges_worn_imbuements_without_a_timed_item_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "House imbuement transfer thread panicked")??;
    Ok(())
}

fn scenario(check_imbuements: bool) -> TestResult {
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
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rt.block_on(case(&h, check_imbuements))));
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
        .spawn(|| resume_scenario(false, false, false, false).map_err(|e| e.to_string()))?
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
        .spawn(|| resume_scenario(true, false, false, false).map_err(|e| e.to_string()))?
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
        .spawn(|| resume_scenario(false, true, false, false).map_err(|e| e.to_string()))?
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
        .spawn(|| resume_scenario(false, true, true, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "delayed condition reconnect thread panicked")??;
    Ok(())
}

#[test]
fn a_same_session_resume_keeps_its_item_fence_when_readback_holder_is_busy_postgres() -> TestResult
{
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| resume_scenario(false, false, false, true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "resume readback thread panicked")??;
    Ok(())
}

fn resume_scenario(
    history_read_fails: bool,
    conditions: bool,
    delayed_reads: bool,
    delayed_readback: bool,
) -> TestResult {
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
            delayed_readback,
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
    delayed_readback: bool,
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
    let supported = if delayed_readback {
        oteryn_protocol_oteryn::logout::CAPABILITY_LOGOUT_V1
    } else {
        wire::CAPABILITY_HOUSE_TRANSFER_V1
    };
    let mut supported_capabilities = vec![supported, 67];
    supported_capabilities.sort_unstable();
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::connection::fresh_capability_selection(
            &owner,
            &supported_capabilities,
        )
        .await
        .ok_or("selection")?;
    assert!(admitted.continuity.selected_capabilities.contains(67));
    assert!(
        !owner
            .required_capabilities
            .authenticate_support(&supported_capabilities)
            .map_err(debug)?
            .selected()
            .contains(67),
        "exercise a mechanic outside the immutable boot offer"
    );
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
        readback: delayed_readback,
        ..Default::default()
    });
    let mut resume = std::pin::pin!(
        crate::gameplay_transport::resume::PREPARE_TEST_BARRIER.scope(
            barrier.clone(),
            resume_same_session(
                &owner,
                admitted.game_session_id,
                &token,
                10,
                &supported_capabilities
            )
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
    let readback_busy = if delayed_readback {
        wait_for_resume_barrier(resume.as_mut(), &barrier.committed).await?;
        let busy = hold_holder(&h.root, std::time::Duration::from_millis(300))?;
        barrier.readback_release.notify_one();
        Some(busy)
    } else {
        None
    };
    let resumed = resume.await;
    busy.await??;
    if let Some(busy) = readback_busy {
        busy.await??;
    }
    let mut resumed = resumed?;
    assert_eq!(resumed.game_session_id, admitted.game_session_id);
    assert_eq!(resumed.continuity.connection_generation, 2);
    assert_eq!(
        resumed.continuity.selected_capabilities, admitted.continuity.selected_capabilities,
        "resume retains dynamically offered mechanics without widening or dropping them"
    );
    if delayed_readback {
        // Even an unregistered command must receive its ordered refusal. It traverses
        // the same retained-NPC reconciliation that precedes the node seam's step.
        let frame = encode_client_command(
            resumed.continuity.connection_generation,
            &ClientCommandValue {
                command_id: resumed.continuity.next_command_id,
                command_type: 999,
                payload: &[],
            },
        )
        .map_err(debug)?;
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        client
            .write_all(&(frame.len() as u32).to_be_bytes())
            .await?;
        client.write_all(&frame).await?;
        client.shutdown().await?;
        let before = resumed.continuity.server_sequence;
        resumed = match serve_admitted(&mut server, resumed, &owner, IDLE_LIVENESS).await {
            ConnectionEnd::AdmittedThenDisconnected(next) => next,
            end => return Err(format!("resumed command end: {end:?}").into()),
        };
        assert_eq!(
            resumed.continuity.server_sequence,
            before + 1,
            "first resumed command disconnected before its result"
        );
    }
    assert!(
        resumed.item_fence.is_some(),
        "committed resume lost its current item fence"
    );
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

async fn seed_house_transfer_imbuement(h: &Harness) -> TestResult {
    super::item_look_fixtures::seed_main_backpack(h, &[]).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_imbuements(item_instance_id,slot,category,imbuement_key,imbuement_revision,tier,remaining_ms,applying_occurrence_id,applying_transaction_id) VALUES(encode($1,'hex')::uuid,0,8,'oteryn:imbuement.featherweight','r1',1,72000000,game_character_uuid_v7(),game_character_uuid_v7())")
        .bind(id(70).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn house_transfer_imbuement_remaining(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT remaining_ms FROM game_item_imbuements WHERE item_instance_id=encode($1,'hex')::uuid AND slot=0")
        .bind(id(70).as_slice()).fetch_one(&h.pool).await?)
}

async fn case(h: &Harness, check_imbuements: bool) -> TestResult {
    let recovery_source = RecoverySource::start(h).await?;
    prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let (controller, cells, deps) = prepare_transfer_scene(h).await?;
    let generation = controller.active().ok_or("active native generation")?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    owner.evidence = &recovery_source.evidence;
    let admitted = initialize_transfer_source(h, &owner, &deps, cells.scope().generation_digest).await?;
    let source = admitted;
    let mut unsupported = source;
    unsupported.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[]).ok_or("empty selection")?;
    let (_, status, payload) = drive(&owner, unsupported, false)
        .await
        .map_err(|error| format!("House unsupported selection: {error}"))?;
    assert_eq!(status, CommandStatus::Rejected);
    assert_eq!(
        wire::decode_house_transfer_refusal(&payload),
        Some(wire::HouseTransferRefusal::CapabilityRequired)
    );
    // Each stale value changes just one expected generation and leaves native facts live.
    for field in 0..3 {
        let mut fresh = deps.owner(h, &authority);
        fresh.movement_cells = (&cells).into();
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
        assert_eq!(
            wire::decode_house_transfer_refusal(&payload),
            Some(wire::HouseTransferRefusal::Unavailable)
        );
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_house_transfer_receipts")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(count, 0);
    }
    // Use a new command directory for the valid source after the refused commands.
    drop(owner);
    let mut owner = deps.owner(h, &authority);
    owner.movement_cells = (&cells).into();
    owner.evidence = &recovery_source.evidence;
    if check_imbuements {
    owner.active_generation = Some(generation);
    seed_house_transfer_imbuement(h).await?;
    let source_fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "source gameplay fence")?.ok_or("source gameplay fence")?;
    let permit = owner.spell_lane_permit().await.ok_or("source spell lane")?;
    assert!(owner.checkpoint_imbuement_equipment(deps.actor, admitted.game_session_id, source_fence).await);
    drop(permit);
    owner.clock_origin -= std::time::Duration::from_secs(2);
    }
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
    let after_entry = if check_imbuements {
    let after_entry = house_transfer_imbuement_remaining(h).await?;
    assert!(after_entry < 72_000_000, "entry must charge a worn imbuement without an ordinary timed item");
    assert!(owner.states.timed_items.imbuements.lock().await.contains_key(&inside.game_session_id));
    owner.clock_origin -= std::time::Duration::from_secs(2);
        Some(after_entry)
    } else {
        None
    };
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
    if let Some(after_entry) = after_entry {
    assert!(house_transfer_imbuement_remaining(h).await? < after_entry, "exit must charge time spent in the House");
    assert!(owner.states.timed_items.imbuements.lock().await.contains_key(&outside.game_session_id));
        return Ok(());
    }
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
    // A fresh entry check cannot authorize a controller after a lock wait.
    let candidate = h.root.house_transfer_recovery(source.game_session_id).await.map_err(debug)?.ok_or("current house-exit recovery")?;
    assert!(candidate.ack.house.is_none());
    let credential = RecoverySource::token(250)?;
    let kid = crate::foundation::fnd04_verifier::fresh_grant_signing_key_id(&credential).ok_or("recovery key")?;
    recovery_source.evidence.refresh_recovery(&h.root,&h.node,&candidate.current.account_id,&kid).await.map_err(debug)?;
    let subject = crate::durability::recovery_evidence_composition::RecoveryEvidenceSubject::new(&candidate.current.account_id,&kid).map_err(debug)?;
    let facts=h.root.verify_registered_recovery(&h.node,&subject,&credential,&candidate.current).await.map_err(debug)?.map_err(debug)?;
    let before: i64=sqlx::query_scalar("SELECT count(*) FROM game_durability_recovery_grant_consumptions").fetch_one(&h.pool).await?;
    let clock=super::security_revocation_fixtures::RecoveryExpiryClock::install(h,facts.verified_at()).await?;
    let expired=h.root.recover_house_transfer(&authority,&h.node,candidate.with_security_revocation_freshness(true),facts,
        crate::foundation::AuthenticatedTransportRefV1::decode(&[250;16]).map_err(debug)?,clock.check.clone()).await;
    clock.finish(h).await?;
    assert!(matches!(expired,Err(crate::durability::DurabilityError::Unavailable)),"expired house recovery must not issue control");
    let after: i64=sqlx::query_scalar("SELECT count(*) FROM game_durability_recovery_grant_consumptions").fetch_one(&h.pool).await?;
    assert_eq!(after,before,"expired recovery must not consume its grant");
    let pending = lose_ack(&owner, outside, false).await?;
    assert_ne!(pending.game_session_id, outside.game_session_id);
    let generations: Vec<(Option<String>,bool)>=sqlx::query_as("SELECT admission_security_generation::text,security_revoked FROM game_durability_reconnect_sessions WHERE account_id=encode($1,'hex')::uuid ORDER BY character_lease_generation")
        .bind(super::fixture::id(40).as_slice()).fetch_all(&h.pool).await?;
    assert!(generations.len()>=4,"real house entry/exit and another entry must all have successors");
    assert!(generations.iter().all(|(generation,revoked)|generation.as_deref()==Some("1") && !revoked),"every house successor inherits the original admission generation");

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
    restarted.active_generation = Some(generation);
    restarted.movement_cells = (&cells).into();
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
    let recovered = resume_transfer(&restarted, source.game_session_id, &token, 10)
        .await
        .map_err(|e| format!("first House recovery: {e}"))?;
    busy.await??;
    let replayed = resume_transfer(&restarted, source.game_session_id, &token, 10)
        .await
        .map_err(|e| format!("House recovery replay: {e}"))?;
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
    restarted.active_generation = Some(generation);
    restarted.movement_cells = (&cells).into();
    restarted.evidence = &recovery_source.evidence;
    let token = RecoverySource::token(8)?;
    let after = resume_transfer(&restarted, source.game_session_id, &token, 11)
        .await
        .map_err(|e| format!("Channel exit recovery: {e}"))?;
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


#[test]
fn c08_conditions_postgres_logout_fresh_login_preserves_poison_haste_regeneration() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("C08_LOGOUT_LOGIN POSTGRES_CASE_STARTED");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| c08_logout_scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "C08 logout/login thread panicked");
    eprintln!(
        "C08_LOGOUT_LOGIN POSTGRES_CASE_FINISHED duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
    result??;
    eprintln!("C08_LOGOUT_LOGIN POSTGRES_CASE_COMPLETED");
    Ok(())
}

fn c08_logout_scenario() -> TestResult {
    let admin = crate::bestiary_postgres_harness::configured_admin().ok_or("PG route required")?;
    let rt = crate::bestiary_postgres_harness::runtime()?;
    let h = rt.block_on(Harness::create_with_admission_seed(
        admin,
        "c08_logout_login",
        true,
        false,
    ))?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        rt.block_on(c08_logout_case(&h))
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

async fn c08_logout_case(h: &Harness) -> TestResult {
    use oteryn_gameplay_domain::domain::condition::ConditionValues;
    use oteryn_gameplay_foundation::foundation::condition::{
        ApplicationFacts, ConditionDefinition, TickFacts,
    };
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    let recovery = RecoverySource::start(h).await?;
    prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let mut owner = deps.owner(h, &authority);
    owner.evidence = &recovery.evidence;
    owner.clock_origin = std::time::Instant::now() - std::time::Duration::from_secs(10);
    let character =
        crate::domain::CharacterId::from_bytes(super::fixture::id(41)).map_err(debug)?;
    let facts = match crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
        &h.root, &authority, character,
    )
    .await
    {
        crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready {
            facts, ..
        }
        | crate::gameplay_transport::spell_character_facts::CastFactsLoad::NoVocation {
            facts,
            ..
        } => facts,
        _ => return Err("fixture character facts".into()),
    };
    let monk = owner
        .load_monk_state(character)
        .await
        .ok_or("fixture monk state")?;
    {
        let runtime = owner.runtime.lock().await;
        assert!(
            owner
                .states
                .spell_states
                .lock()
                .await
                .initialize(
                    &runtime,
                    deps.actor,
                    deps.admitted.game_session_id,
                    facts,
                    monk,
                    owner.owner_now()
                )
                .is_some()
        );
    }
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
    install_reconnect_conditions(&owner, &deps).await?;
    let before_at = owner.owner_now().get();
    let before = {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let state = states
            .get_mut(&runtime, deps.actor, admitted.game_session_id)
            .ok_or("source state")?;
        let expected = state.owned_conditions().clone();
        let mut next = expected.clone();
        let root = GameplayDecisionRoot::from_bytes([8; 32]);
        let regeneration = ConditionDefinition::new(
            "fixture:c08.regeneration",
            1,
            ConditionValues::SpellRegeneration {
                duration_ms: 60_000,
                sub_id: 0,
                health_gain: 3,
                health_interval_ms: 2_000,
                mana_gain: 2,
                mana_interval_ms: 3_000,
            },
        )
        .ok_or("regeneration definition")?;
        next.apply(
            &regeneration,
            None,
            oteryn_gameplay_foundation::foundation::condition::ConditionSourceKind::SelfUse,
            &[],
            &ApplicationFacts {
                now: before_at,
                base_speed: 220,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &root,
                occurrence: DecisionOccurrenceId::from_bytes([8; 16]),
            },
        )
        .map_err(debug)?;
        // Advance through actual due ticks before saving; a reload must not restart a recipe.
        // Rebase the owner's clock by six seconds, without waiting on wall time.
        let ticks = next.take_due(before_at + 6_000_000, TickFacts::default());
        assert!(!ticks.is_empty());
        state
            .apply_batch_conditions(&expected, &next)
            .map_err(debug)?;
        next
    };
    owner.clock_origin -= std::time::Duration::from_secs(6);
    let logout_before = owner.owner_now().get();
    assert_eq!(
        owner.logout_session(admitted).await,
        crate::gameplay_transport::connection::LogoutReply::Result(
            oteryn_protocol_oteryn::logout::LogoutResult::Accepted
        )
    );
    assert!(
        owner
            .runtime
            .lock()
            .await
            .player_control_facts(deps.actor, admitted.game_session_id)
            .is_err(),
        "logout retires runtime authority"
    );
    let logout_after = owner.owner_now().get();
    drop(owner);
    // A new owner clock and an empty runtime: no same-session or in-memory copy can survive.
    let fresh_deps = Dependencies::new()?;
    let mut fresh_owner = fresh_deps.owner(h, &authority);
    fresh_owner.evidence = &recovery.evidence;
    let token = RecoverySource::fresh_token(91)?;
    let frame = oteryn_protocol_oteryn::encode_client_bootstrap(
        &oteryn_protocol_oteryn::ClientBootstrapValue {
            schema_revision: 1,
            character_id: crate::foundation::CharacterId::decode(
                admitted.item_fence.ok_or("fence")?.character_id.as_bytes(),
            )
            .map_err(debug)?,
            admission_material: token.as_bytes(),
            client_build_id: "c08-conditions",
            supported_capabilities: &[],
        },
    )
    .map_err(debug)?;
    struct FreshIds;
    impl crate::gameplay_transport::connection::ConnectionIdentifiers for FreshIds {
        fn game_session_id(&self) -> Option<crate::foundation::GameSessionId> {
            crate::foundation::GameSessionId::decode(&super::fixture::id(91)).ok()
        }
        fn transport_ref(&self) -> Option<crate::foundation::AuthenticatedTransportRefV1> {
            crate::foundation::AuthenticatedTransportRefV1::decode(&[91; 16]).ok()
        }
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let admission_before = fresh_owner.owner_now().get();
    let fresh = crate::gameplay_transport::connection::admit_frame(
        &mut server,
        &frame,
        &fresh_owner,
        &FreshIds,
    )
    .await
    .map_err(|e| format!("C08 fresh admission: {e:?}"))?;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    assert_ne!(fresh.game_session_id, admitted.game_session_id);
    assert!(
        matches!(
            fresh.first_entry,
            FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
        ),
        "C08 fresh placement: {:?}",
        fresh.first_entry
    );
    let runtime = fresh_owner.runtime.lock().await;
    let states = fresh_owner.states.spell_states.lock().await;
    let after = states
        .get(
            &runtime,
            fresh.runtime_actor.ok_or("fresh actor")?,
            fresh.game_session_id,
        )
        .ok_or("fresh state")?
        .owned_conditions();
    assert_eq!(
        after.instances().len(),
        3,
        "fresh login must restore poison, haste and regeneration"
    );
    let after_at = fresh_owner.owner_now().get();
    for old in before.instances() {
        let new = after
            .instances()
            .iter()
            .find(|i| i.definition() == old.definition())
            .ok_or("restored definition")?;
        assert_eq!(
            new.remaining_total(),
            old.remaining_total(),
            "remaining DOT damage"
        );
        assert_eq!(new.speed_delta(), old.speed_delta(), "applied speed draw");
        let upper = old
            .remaining_duration_ms(logout_before)
            .ok_or("upper duration")?;
        let lower = old
            .remaining_duration_ms(
                logout_after
                    .saturating_add(after_at - admission_before)
                    .saturating_add(1000),
            )
            .ok_or("lower duration")?;
        let restored = new.remaining_duration_ms(after_at).ok_or("new duration")?;
        if upper >= 0 {
            assert!(
                (lower..=upper).contains(&restored),
                "remaining duration {lower}..={upper} vs {restored}"
            );
        }
    }
    drop(states);
    drop(runtime);
    {
        let mut runtime = fresh_owner.runtime.lock().await;
        let mut states = fresh_owner.states.spell_states.lock().await;
        let actor = fresh.runtime_actor.ok_or("live fresh actor")?;
        let result = states.tick_current_creature_field_damage(
            &mut runtime,
            actor,
            fresh.game_session_id,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(after_at + 5_000_000),
            TickFacts::default(),
            None,
        );
        assert!(
            matches!(result, Some(Some(_))),
            "restored ticks must reach the current HP owner"
        );
        let poison = states
            .get(&runtime, actor, fresh.game_session_id)
            .ok_or("ticked player")?
            .owned_conditions()
            .get(
                oteryn_gameplay_domain::domain::condition::ConflictKey::Element(
                    oteryn_gameplay_domain::domain::condition::DotElement::Poison,
                ),
            )
            .ok_or("ticked poison")?;
        assert!(
            poison.remaining_total()
                < before
                    .instances()
                    .iter()
                    .find(|i| i.definition().key() == "fixture:reconnect.poison")
                    .ok_or("old poison")?
                    .remaining_total()
        );
    }
    let fence = fresh.item_fence.ok_or("fresh fence")?;
    let saved = h
        .root
        .read_character_conditions(&authority, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("saved conditions")?;
    assert!(saved.lease_generation < fence.character_lease_generation);
    for field in 0..3 {
        let mut stale = fence;
        match field {
            0 => stale.character_lease_generation += 1,
            1 => {
                stale.connection_generation =
                    crate::foundation::ConnectionGeneration::new(2).map_err(debug)?
            }
            _ => {
                stale.scope_ownership_generation =
                    crate::foundation::ScopeOwnershipGeneration::new(2).map_err(debug)?
            }
        }
        let mut candidate = saved.clone();
        candidate.lease_generation = stale.character_lease_generation;
        assert!(
            matches!(
                h.root
                    .save_character_conditions(
                        &authority,
                        &h.node,
                        stale,
                        *fresh.channel_id.as_bytes(),
                        candidate
                    )
                    .await,
                Err(crate::durability::item_use::ItemUseError::AuthorityRejected)
            ),
            "stale condition fence {field}"
        );
    }
    assert_eq!(
        h.root
            .read_character_conditions(&authority, fence.character_id)
            .await
            .map_err(debug)?
            .ok_or("unchanged snapshot")?
            .conditions,
        saved.conditions
    );
    Ok(())
}
