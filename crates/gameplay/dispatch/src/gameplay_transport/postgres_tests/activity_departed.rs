//! Production origin lifecycle without the departed process's native directory.
//! The successor is offline; only a separately authenticated fresh admission
//! obtains new control. Fixture content and initial inventory remain explicit.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult};
use crate::durability::activity_handoff::ActivityHandoffOwner;
use crate::durability::admission_authority_guards::AdmissionGuardStore;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::item_loop_move::{
    ItemLoopContent, LoopDestination, LoopGround, LoopMoveRequest,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{
    ItemDefinitionFacts, ItemStackClass, ItemWeightLookup, ItemWeightRefused,
};
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, AssignmentState, ControlActor,
    NodeIncarnationProof, OperationKey, RuntimeScopeAssignmentWriter,
};
use crate::durability::spell_owner_commit::SpellLane;
use crate::foundation::admission_authority_publication::AdmissionAuthorityGuardStateV1;
use crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
use crate::foundation::{
    AuthenticatedTransportRefV1, CharacterId, CommandId, CommandRef, GameSessionId,
    GameSessionState, RuntimeScopeRefV1, ScopeOwnershipGeneration,
};
use crate::gameplay_transport::item_fence_of;

#[test]
fn activity_departed_postgres_origin_lifecycle_releases_for_fresh_admission_and_retires_ground()
-> TestResult {
    run(false)
}

#[test]
fn activity_departed_postgres_committed_return_survives_cancel_and_empty_registry() -> TestResult {
    run(true)
}

#[test]
fn activity_departed_postgres_ended_registration_retires_active_assignment_ground() -> TestResult {
    run_with_departure(false, true)
}

fn run(cancel_commit: bool) -> TestResult {
    run_with_departure(cancel_commit, false)
}

fn run_with_departure(cancel_commit: bool, registration_ended: bool) -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        eprintln!("ACTIVITY_DEPARTED POSTGRES_CASE_SKIPPED: no configured admin");
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            let runtime =
                crate::bestiary_postgres_harness::runtime().map_err(|error| error.to_string())?;
            let h = runtime
                .block_on(Harness::create_with_admission_seed(
                    admin,
                    "activity_departed",
                    true,
                    false,
                ))
                .map_err(|error| error.to_string())?;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                runtime.block_on(scenario(&h, cancel_commit, registration_ended))
            }));
            let cleanup = runtime
                .block_on(h.cleanup())
                .map_err(|error| error.to_string());
            match result {
                Ok(result) => {
                    result.map_err(|error| error.to_string())?;
                    cleanup
                }
                Err(panic) => std::panic::resume_unwind(panic),
            }
        })?
        .join()
        .map_err(|_| "departed lifecycle fixture panicked")??;
    Ok(())
}

async fn scenario(h: &Harness, cancel_commit: bool, registration_ended: bool) -> TestResult {
    let entered = super::activity_runtime::enter_fixture(h).await?;
    let source = entered.fence;
    let origin = entered.request.participants[0].source.runtime_scope;
    let origin_generation = entered.request.participants[0]
        .source
        .scope_ownership_generation;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let durable = ActivityHandoffOwner::new(h.root.clone());
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let deps = Dependencies::new()?;
    // The helper creates an initial Channel slot. Its session really became
    // terminal on entry; remove that exact slot to model a new empty runtime.
    assert_eq!(
        store
            .current_session_at(deps.admitted.game_session_id)
            .await?
            .0
            .session_state(),
        GameSessionState::Terminal
    );
    deps.runtime
        .lock()
        .await
        .remove_terminal_session(deps.admitted.game_session_id, deps.actor)
        .map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.evidence = &entered.evidence.evidence;
    assert!(
        owner
            .activity_runtimes
            .activity_owners()
            .map_err(|_| "directory")?
            .is_empty()
    );
    assert!(
        owner
            .runtime
            .lock()
            .await
            .activity_participant_sessions()
            .map_err(debug)?
            .is_empty()
    );
    seed_private_ground(h, &authority, &entered).await?;
    let before = census(h).await?;
    assert!(
        durable
            .departed_activities_after(&authority, &h.node, origin, origin_generation, None)
            .await?
            .activities
            .is_empty()
    );
    owner
        .activity_lifecycle_pass()
        .await
        .map_err(|_| "current-owner pass")?;
    assert_eq!(
        census(h).await?,
        before,
        "absence from the directory never proves the live assignment departed"
    );
    assert!(
        durable
            .departed_activities_after(
                &authority,
                &h.node,
                origin,
                ScopeOwnershipGeneration::new(origin_generation.get() + 1).map_err(debug)?,
                None
            )
            .await
            .is_err()
    );
    assert_eq!(
        census(h).await?,
        before,
        "wrong current origin generation cannot release or retire anything"
    );
    // Preserve both existing revoke scenarios. The registration-ended case
    // leaves the exact Instance assignment untouched and replaces only origin.
    drop(owner);
    let source_assignment = h
        .root
        .read_runtime_scope_assignment(source.runtime_scope)
        .await
        .map_err(debug)?
        .ok_or("retained source assignment")?;
    assert_eq!(source_assignment.state, AssignmentState::Assigned);
    assert_eq!(source_assignment.holder, Some(h.node.fact()));
    let (node, origin_generation) = if registration_ended {
        let (node, generation) = restart_origin(h, &deps, origin).await?;
        assert_eq!(
            h.root
                .read_runtime_scope_assignment(source.runtime_scope)
                .await
                .map_err(debug)?,
            Some(source_assignment.clone()),
            "ending the exact holder registration must not revoke or replace Instance"
        );
        assert!(
            durable
                .departed_activities_after(&authority, &h.node, origin, generation, None)
                .await
                .is_err(),
            "the ended registration cannot recover through the replacement origin"
        );
        assert!(
            durable
                .departed_activities_after(&authority, &node, origin, origin_generation, None)
                .await
                .is_err(),
            "the replacement node cannot use the old origin generation"
        );
        assert_eq!(census(h).await?, before);
        (node, generation)
    } else {
        revoke(h, source.runtime_scope).await?;
        (h.node.clone(), origin_generation)
    };
    let mut owner = deps.owner(h, &authority);
    owner.holder = &node;
    owner.evidence = &entered.evidence.evidence;
    bind_origin_capabilities(&mut owner, origin, origin_generation)?;
    let page = durable
        .departed_activities_after(&authority, &node, origin, origin_generation, None)
        .await?;
    assert_eq!(page.activities.len(), 1);
    assert_eq!(page.activities[0].source(), source);
    assert!(page.activities[0].returned().is_none());
    if cancel_commit {
        cancel_after_committed_return(&owner).await?;
        assert_eq!(
            owner
                .activity_runtimes
                .departed_recoveries
                .lock()
                .map_err(|_| "pending proposals")?
                .len(),
            1
        );
    } else {
        owner
            .activity_lifecycle_pass()
            .await
            .map_err(|_| "departed return pass")?;
    }
    let returned = durable
        .departed_activities_after(&authority, &node, origin, origin_generation, None)
        .await?;
    assert_eq!(returned.activities.len(), 1);
    let proof = returned.activities[0].clone();
    let receipt = proof.returned().ok_or("committed offline return")?.clone();
    assert_eq!(receipt.source_game_session_id, source.game_session_id);
    assert_eq!(receipt.destination_scope, origin);
    assert_eq!(receipt.destination_transport, None);
    assert_eq!(
        receipt.destination_lease_generation,
        source.character_lease_generation + 1
    );
    let offline = store
        .current_session_at(receipt.destination_game_session_id)
        .await?
        .0;
    assert_eq!(offline.session_state(), GameSessionState::Reconnectable);
    assert!(offline.current_transport().is_none());
    assert_eq!(offline.current_connection_generation().get(), 1);
    let loss_key = [
        b"owning-loss-v1".as_slice(),
        receipt.destination_game_session_id.as_bytes().as_slice(),
        1_u64.to_be_bytes().as_slice(),
    ]
    .concat();
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_durability_admission_lifecycle_receipts WHERE operation_key=$1")
        .bind(loss_key).fetch_one(&h.pool).await?, 0,
        "offline recovery must not manufacture an owning loss");
    // Drop all process-local proposals after a real return COMMIT. The next
    // owner must discover its receipt, not reconstruct a native actor/token.
    drop(owner);
    let mut restarted = deps.owner(h, &authority);
    restarted.holder = &node;
    restarted.evidence = &entered.evidence.evidence;
    bind_origin_capabilities(&mut restarted, origin, origin_generation)?;
    assert!(
        restarted
            .activity_runtimes
            .departed_recoveries
            .lock()
            .map_err(|_| "new directory")?
            .is_empty()
    );
    restarted
        .activity_lifecycle_pass()
        .await
        .map_err(|_| "offline successor release")?;
    assert_eq!(
        store
            .current_session_at(receipt.destination_game_session_id)
            .await?
            .0
            .session_state(),
        GameSessionState::Terminal
    );
    assert!(
        restarted
            .runtime
            .lock()
            .await
            .activity_participant_sessions()
            .map_err(debug)?
            .is_empty()
    );
    assert!(
        h.root
            .read_item_loop_ground_for_scope(source.runtime_scope)
            .await?
            .is_empty()
    );
    assert!(
        h.root
            .read_item_loop_ground_for_scope(origin)
            .await?
            .is_empty()
    );
    assert_eq!(sqlx::query_scalar::<_, i16>("SELECT lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?, 2);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_activity_runtime_returns")
            .fetch_one(&h.pool)
            .await?,
        1
    );
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_container_slots WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(70).as_slice()).fetch_one(&h.pool).await?, 1);
    let fresh = fresh_after_release(h, source.character_id, origin).await?;
    assert_eq!(fresh.current_runtime_scope(), origin);
    assert_eq!(fresh.current_scope_generation(), origin_generation);
    assert_eq!(
        fresh.current_character_lease().generation(),
        source.character_lease_generation + 2
    );
    assert_eq!(fresh.session_state(), GameSessionState::Active);
    assert_ne!(
        fresh.current_game_session_id(),
        receipt.destination_game_session_id
    );
    let retained = census(h).await?;
    restarted
        .activity_lifecycle_pass()
        .await
        .map_err(|_| "terminal replay pass")?;
    assert_eq!(
        census(h).await?,
        retained,
        "later lifecycle cannot release newly authenticated control"
    );
    assert_eq!(
        store
            .current_session_at(fresh.current_game_session_id())
            .await?
            .0,
        fresh
    );
    assert!(
        durable
            .departed_activities_after(&authority, &node, origin, origin_generation, None)
            .await?
            .activities
            .is_empty()
    );
    if registration_ended {
        assert_eq!(
            h.root
                .read_runtime_scope_assignment(source.runtime_scope)
                .await
                .map_err(debug)?,
            Some(source_assignment),
            "return, retirement, terminal release and replay leave Instance assigned to its ended holder"
        );
        assert!(
            h.root
                .require_current_node_registration(&h.node)
                .await
                .is_err()
        );
        h.root
            .require_current_node_registration(&node)
            .await
            .map_err(debug)?;
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(71).as_slice()).fetch_one(&h.pool).await?,
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_item_decay_retire_receipts r \
                 JOIN game_item_audit_outbox a USING(event_id) \
                 WHERE r.item_instance_id=encode($1,'hex')::uuid \
                   AND r.cause_kind='ACTIVITY_RETIREMENT' \
                   AND r.activity_handoff_id=encode($2,'hex')::uuid"
            )
            .bind(id(71).as_slice())
            .bind(entered.request.handoff_id.as_slice())
            .fetch_one(&h.pool)
            .await?,
            1,
            "exactly one retained one-item retirement and matching audit"
        );
    }
    eprintln!(
        "ACTIVITY_DEPARTED POSTGRES_CASE_COMPLETED cancel_commit={cancel_commit} registration_ended={registration_ended}"
    );
    Ok(())
}

async fn cancel_after_committed_return(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult {
    use crate::gameplay_transport::activity_departed::{
        DEPARTED_RETURN_COMMITTED_TEST_BARRIER, DepartedReturnCommittedTestBarrier,
    };
    use std::future::{Future, poll_fn};
    use std::task::Poll;
    let barrier = std::sync::Arc::new(DepartedReturnCommittedTestBarrier::default());
    let mut pass = std::pin::pin!(
        DEPARTED_RETURN_COMMITTED_TEST_BARRIER
            .scope(barrier.clone(), owner.activity_lifecycle_pass())
    );
    let mut entered = std::pin::pin!(barrier.entered.notified());
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        poll_fn(|context| {
            if entered.as_mut().poll(context).is_ready() {
                return Poll::Ready(Ok(()));
            }
            if let Poll::Ready(result) = pass.as_mut().poll(context) {
                return Poll::Ready(Err(format!(
                    "lifecycle ended before committed barrier: {result:?}"
                )));
            }
            Poll::Pending
        }),
    )
    .await
    .map_err(|_| "departed return commit barrier timeout")??;
    // Dropping this pending future models the owner ending after COMMIT. Its
    // retained proposal is observable above; no receipt or transport is edited.
    Ok(())
}

pub(super) async fn fresh_after_release(
    h: &Harness,
    character: crate::domain::CharacterId,
    origin: RuntimeScopeRefV1,
) -> TestResult<crate::foundation::GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>> {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let mut source = super::fresh_fixture::fresh::Source::new(now)?;
    let keys = source
        .rows
        .iter()
        .map(|row| row.key.clone())
        .collect::<Vec<_>>();
    source.rows = AdmissionGuardStore::from_root(h.root.clone())
        .load(&keys)
        .await?
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("current fresh guards")?;
    for row in &source.rows {
        match &row.state {
            AdmissionAuthorityGuardStateV1::Account { presence, .. } => assert!(presence.is_none()),
            AdmissionAuthorityGuardStateV1::Character {
                holder,
                lease_generation,
                ..
            } => {
                assert!(holder.is_none());
                source.current.character_lease_generation = *lease_generation;
            }
            AdmissionAuthorityGuardStateV1::Runtime {
                ownership_generation,
                ready,
                ..
            } => {
                assert!(*ready);
                source.current.scope_ownership_generation = *ownership_generation;
            }
            _ => {}
        }
    }
    assert_eq!(
        source.current.character_id,
        CharacterId::decode(character.as_bytes()).map_err(debug)?
    );
    assert_eq!(
        RuntimeScopeRefV1::channel(source.current.world_id, source.current.channel_id),
        origin
    );
    let character_text: String = sqlx::query_scalar("SELECT character_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let session = GameSessionId::decode(&id(197)).map_err(debug)?;
    let transport = AuthenticatedTransportRefV1::decode(&[197; 16]).map_err(debug)?;
    let request =
        source.activity_room_request(session, transport, &character_text, [197; 32], now)?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    assert!(matches!(
        store.commit(&request).await?,
        FreshAdmissionDurableOutcomeV1::Committed(_)
    ));
    let current = store.current_session_at(session).await?.0;
    assert_eq!(current.current_transport(), Some(transport));
    assert_eq!(
        item_fence_of(current)
            .ok_or("fresh current fence")?
            .character_id,
        character
    );
    Ok(current)
}

fn bind_origin_capabilities(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    origin: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
) -> TestResult {
    use crate::gameplay_transport::world_required_capabilities::{
        BoundRequiredCapabilities, offered_for_scope,
    };
    owner.required_capabilities = BoundRequiredCapabilities::for_channel_boot(
        &[],
        origin,
        generation.get(),
        "policy-1",
        offered_for_scope(false, None),
    )
    .map_err(debug)?;
    Ok(())
}

async fn restart_origin(
    h: &Harness,
    deps: &Dependencies,
    origin: RuntimeScopeRefV1,
) -> TestResult<(NodeIncarnationProof, ScopeOwnershipGeneration)> {
    use crate::durability::admission_authority_guards::GuardPublicationDisposition;
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1, AdmissionAuthorityPublicationV1,
        AdmissionPublicationPreconditionV1,
    };
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = origin
    else {
        return Err("Channel origin required".into());
    };
    // Real registration ending and bootstrap issuance. The existing control
    // writer owns assignment generation, its receipt and the readiness fence.
    h.root
        .revoke_node_registration(h.node.fact())
        .await
        .map_err(debug)?;
    assert!(
        h.root
            .require_current_node_registration(&h.node)
            .await
            .is_err()
    );
    let node = super::fixture::registered_node(&h.root, 198).await?;
    // The normal replacement process claims the existing S2 custody only
    // after its exact prior registration ended; source observations persist.
    h.root
        .claim_native_admission_source_custody(&node)
        .await
        .map_err(debug)?;
    h.root
        .require_current_node_registration(&node)
        .await
        .map_err(debug)?;
    assert_ne!(node.fact().node_id(), h.node.fact().node_id());

    // Only the explicit fixture control principal receives Replace on origin.
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,2)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .execute(&h.pool).await?;
    let predecessor = h
        .root
        .read_runtime_scope_predecessor(origin)
        .await
        .map_err(debug)?
        .ok_or("origin control predecessor")?;
    let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "sequencer-writer")
        .await
        .map_err(debug)?;
    let outcome = writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([198; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Replace {
                scope: origin,
                predecessor,
                target: node.fact(),
            },
        })
        .await
        .map_err(debug)?;
    let AssignmentOutcome::Committed(receipt) = outcome else {
        return Err(format!("replacement origin: {outcome:?}").into());
    };
    assert_eq!(receipt.assignment.scope, origin);
    assert_eq!(receipt.assignment.state, AssignmentState::Assigned);
    assert_eq!(receipt.assignment.holder, Some(node.fact()));
    assert_eq!(
        receipt.assignment.ownership_generation,
        predecessor.ownership_generation + 1
    );
    let generation =
        ScopeOwnershipGeneration::new(receipt.assignment.ownership_generation).map_err(debug)?;

    // Republish from the actual post-Replace guard. Keep its exact authority
    // chain, source revision and CAS; Replace itself deliberately fenced ready.
    let key = AdmissionAuthorityGuardKeyV1::Runtime(origin);
    let mut next = AdmissionGuardStore::from_root(h.root.clone())
        .load(&[key])
        .await?
        .into_iter()
        .next()
        .flatten()
        .ok_or("replacement runtime guard")?;
    assert_eq!(
        Some(next.publication_revision),
        receipt.fenced_publication_revision
    );
    let AdmissionAuthorityGuardStateV1::Runtime {
        ownership_generation,
        ready,
        ..
    } = &mut next.state
    else {
        return Err("runtime guard required".into());
    };
    assert_eq!(*ownership_generation, generation.get());
    assert!(!*ready);
    *ready = true;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    next.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
        expected_publication_revision: next.publication_revision,
    };
    next.publication_revision = next
        .publication_revision
        .checked_add(1)
        .ok_or("guard revision")?;
    next.source.source_revision = next
        .source
        .source_revision
        .checked_add(1)
        .ok_or("source revision")?;
    next.source.decision_identity = "departed-origin-restarted-ready".into();
    next.source.source_observed_at = now.max(next.source.source_observed_at);
    next.source.clock_uncertainty_seconds = 0;
    let mut publisher = super::fresh_fixture::fresh::Source::new(now)?;
    publisher.rows = vec![next];
    let publication = AdmissionAuthorityPublicationV1::prepare(&publisher, now).map_err(debug)?;
    assert_eq!(
        h.root
            .publish_runtime_readiness(&node, &publication)
            .await
            .map_err(debug)?,
        GuardPublicationDisposition::Applied
    );

    // Recreate the empty native Channel from the real committed assignment.
    // Bind its door to the same generation; no old actor is retained.
    let mut runtime = deps.runtime.lock().await;
    assert!(
        runtime
            .activity_participant_sessions()
            .map_err(debug)?
            .is_empty()
    );
    let pin = runtime.content_pin().clone();
    *runtime = crate::foundation::ChannelRuntimeV1::from_committed_assignment(
        world_id,
        channel_id,
        node.fact().node_id(),
        node.fact().registration_revision(),
        receipt.assignment.ownership_generation,
        receipt.assignment.source_revision,
        &receipt.assignment.decision_identity,
        2,
        pin,
    )
    .map_err(debug)?;
    assert_eq!(runtime.binding().scope_generation(), generation);
    let content = crate::world_runtime::ReferenceContentGeneration::from_content(deps.room.door())
        .map_err(debug)?;
    let door_fence =
        crate::world_runtime::ScopeContentGenerationFence::for_test(origin, generation, content);
    *deps.door.lock().await = crate::world_runtime::bind_native_entry_door(
        deps.room.door(),
        &door_fence,
        origin,
        generation,
    )
    .map_err(debug)?;
    Ok((node, generation))
}

async fn revoke(h: &Harness, scope: RuntimeScopeRefV1) -> TestResult {
    let RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = scope
    else {
        return Err("Instance control only".into());
    };
    sqlx::query("INSERT INTO game_control_activity_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,3)")
        .bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice()).execute(&h.pool).await?;
    let predecessor = h
        .root
        .read_runtime_scope_predecessor(scope)
        .await
        .map_err(debug)?
        .ok_or("control predecessor")?;
    let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "sequencer-writer")
        .await
        .map_err(debug)?;
    let outcome = writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([196; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Revoke { scope, predecessor },
        })
        .await
        .map_err(debug)?;
    let AssignmentOutcome::Committed(receipt) = outcome else {
        return Err(format!("control revoke: {outcome:?}").into());
    };
    assert_eq!(receipt.assignment.scope, scope);
    assert_eq!(receipt.assignment.state, AssignmentState::Revoked);
    Ok(())
}

async fn census(h: &Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_activity_runtime_returns),(SELECT count(*) FROM game_durability_reconnect_sessions),(SELECT count(*) FROM game_item_audit_outbox),(SELECT count(*) FROM game_item_ground_locations)").fetch_one(&h.pool).await?)
}

async fn seed_private_ground(
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    entered: &super::activity_runtime::EnteredActivityFixture,
) -> TestResult {
    super::item_look::seed_main_backpack(h, &[("oteryn:item.tibia.i3031", 5)]).await?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(entered.fence.character_id.as_bytes().as_slice()).execute(&h.pool).await?;
    let cell = entered.request.participants[0].destination_cell;
    let pin = entered.request.destination_content;
    let lane = SpellLane::for_scope(entered.fence.runtime_scope);
    let permit = lane.acquire().await.map_err(|_| "drop lane")?;
    h.root
        .commit_item_loop_move(
            &permit,
            authority,
            &h.node,
            &entered.fence,
            CommandRef::new(
                entered.fence.game_session_id,
                CommandId::new(11).map_err(debug)?,
            ),
            &Content(pin.server_artifact_digest),
            LoopMoveRequest {
                item: id(71),
                count: None,
                pickup_cell: None,
                destination: LoopDestination::Ground(LoopGround {
                    spatial_position: [
                        cell.x.to_be_bytes().as_slice(),
                        cell.y.to_be_bytes().as_slice(),
                        cell.floor.to_be_bytes().as_slice(),
                    ]
                    .concat(),
                    map_revision: pin
                        .map_revision_digest
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect(),
                    content_revision: entered.request.content_revision.clone(),
                    placement_context: pin.coordinate_frame_digest.to_vec(),
                }),
            },
        )
        .await?
        .map_err(debug)?;
    assert_eq!(
        h.root
            .read_item_loop_ground_for_scope(entered.fence.runtime_scope)
            .await?
            .len(),
        1
    );
    Ok(())
}

// Same two explicit fixture definitions as activity_ground_loop; no production
// Item or encounter defaults are introduced by this isolated fixture extension.
struct Content([u8; 32]);
impl ItemWeightLookup for Content {
    fn generation_digest(&self) -> [u8; 32] {
        self.0
    }
    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        self.loop_facts(definition)
            .and_then(|facts| facts.weight)
            .ok_or(ItemWeightRefused::Unknown)
    }
}
impl ItemLoopContent for Content {
    fn loop_content_digest(&self) -> [u8; 32] {
        self.0
    }
    fn loop_facts(&self, definition: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        if definition.family != "Item" || definition.revision_ref != "definition-r1" {
            return None;
        }
        let (stack, container_capacity, container_slot_equip_pattern) =
            match definition.production_key.as_str() {
                "oteryn:item.tibia.i2854" => (ItemStackClass::NonStackable, Some(20), true),
                "oteryn:item.tibia.i3031" => (
                    ItemStackClass::Stackable {
                        proven_maximum: Some(100),
                    },
                    None,
                    false,
                ),
                _ => return None,
            };
        Some(ItemDefinitionFacts {
            definition: definition.clone(),
            weight: Some(1),
            stack,
            container_capacity,
            container_slot_equip_pattern,
        })
    }
}
