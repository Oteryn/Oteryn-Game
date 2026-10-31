//! The real activity writer, current-session reader and control allocator on PostgreSQL.
//! Test coordinates/digests are isolated authored fixture data, never gameplay defaults.
use super::fixture::{self, Harness, debug};
use super::{TestResult, reset};
use crate::durability::activity_handoff::{
    ActivityContentPin, ActivityEnterOutcome, ActivityEnterRequest, ActivityHandoffOwner,
    ActivityParticipant, ActivityRefusal, ActivityReturnOutcome, ActivityReturnReason,
    ActivityReturnRequest,
};
use crate::durability::character_last_position::{
    LastPositionCell, LastPositionRecord, LastVitals,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
    RuntimeScopeAssignmentWriter,
};
use crate::foundation::{
    AuthenticatedTransportRefV1, CharacterId, ConnectionGeneration, GameSessionId,
    GameSessionState, RuntimeScopeRefV1,
};
use crate::gameplay_transport::item_fence_of;

#[test]
fn activity_handoff_postgres_real_enter_return_replay_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("ACTIVITY_RUNTIME POSTGRES_CASE_SKIPPED: no admin URL");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "activity handoff fixture panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        eprintln!("ACTIVITY_RUNTIME POSTGRES_CASE_SKIPPED: no configured admin");
        return Ok(());
    };
    let runtime = crate::bestiary_postgres_harness::runtime()?;
    let harness = runtime.block_on(Harness::create_with_admission_seed(
        admin,
        "native_activity_owner",
        true,
        false,
    ))?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.block_on(case(&harness))
    }));
    let cleanup = runtime.block_on(harness.cleanup());
    match result {
        Ok(result) => {
            result?;
            cleanup
        }
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

pub(super) async fn allocate_ready_instance(h: &Harness, tag: u8) -> TestResult<RuntimeScopeRefV1> {
    use crate::durability::admission_authority_guards::GuardPublicationDisposition;
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1, AdmissionAuthorityPublicationV1,
    };
    let scope = RuntimeScopeRefV1::instance(
        fixture::fence(1)?.runtime_scope.world_id(),
        fixture::id(tag),
    )
    .map_err(debug)?;
    // Only the test control principal receives the exact existing control grant.
    // The production activity runtime cannot write grants, assignments or readiness.
    sqlx::query("INSERT INTO game_control_activity_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(scope.world_id().as_bytes().as_slice()).bind(fixture::id(tag).as_slice()).execute(&h.pool).await?;
    let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "sequencer-writer")
        .await
        .map_err(debug)?;
    let result = writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([tag; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Assign {
                scope,
                target: h.node.fact(),
            },
        })
        .await
        .map_err(debug)?;
    let AssignmentOutcome::Committed(receipt) = result else {
        return Err(format!("control allocation refused: {result:?}").into());
    };
    assert_eq!(receipt.assignment.scope, scope);
    assert_eq!(receipt.assignment.ownership_generation, 1);
    let now = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
        .fetch_one(&h.pool)
        .await?;
    let mut source = super::fresh_fixture::fresh::Source::new(now)?;
    source
        .rows
        .retain(|row| matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    assert_eq!(source.rows.len(), 1);
    source.rows[0].key = AdmissionAuthorityGuardKeyV1::Runtime(scope);
    let publication = AdmissionAuthorityPublicationV1::prepare(&source, now).map_err(debug)?;
    assert_eq!(
        h.root
            .publish_runtime_readiness(&h.node, &publication)
            .await
            .map_err(debug)?,
        GuardPublicationDisposition::Applied
    );
    Ok(scope)
}

pub(super) async fn issue_fixture_content(h: &Harness) -> TestResult<ActivityContentPin> {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fixture::fence(1)?.runtime_scope
    else {
        return Err("fixture must start in a Channel".into());
    };
    let pin = ActivityContentPin {
        coordinate_frame_digest: [0x11; 32],
        map_revision_digest: [0x22; 32],
        server_artifact_digest: [0x33; 32],
    };
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).execute(&h.pool).await?;
    // Invoke the existing control issuer, including its scope grant/CAS/immutable receipt.
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(pin.server_artifact_digest.as_slice()).bind([0x44_u8; 32].as_slice())
        .bind(pin.coordinate_frame_digest.as_slice()).execute(&h.pool).await?;
    Ok(pin)
}

fn session(tag: u8) -> TestResult<GameSessionId> {
    GameSessionId::decode(&fixture::id(tag)).map_err(debug)
}
fn transport(tag: u8) -> TestResult<AuthenticatedTransportRefV1> {
    AuthenticatedTransportRefV1::decode(&[tag; 16]).map_err(debug)
}

async fn retained_rows(h: &Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_activity_handoff_groups), \
        (SELECT count(*) FROM game_activity_runtime_returns), \
        (SELECT count(*) FROM game_durability_session_use_memberships), \
        (SELECT count(*) FROM game_character_last_positions)",
    )
    .fetch_one(&h.pool)
    .await?)
}

pub(super) async fn prepare_activity_fixture(
    h: &Harness,
) -> TestResult<(super::house_transfer::RecoverySource, ActivityEnterRequest)> {
    reset();
    let evidence = super::house_transfer::RecoverySource::start(h).await?;
    super::house_transfer::prepare_owned_session(h).await?;
    let account: String = sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fixture::id(41).as_slice()).fetch_one(&h.pool).await?;
    evidence
        .evidence
        .refresh_account(&h.root, &h.node, &account)
        .await
        .map_err(debug)?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let owner = ActivityHandoffOwner::new(h.root.clone());
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let source =
        item_fence_of(store.current_session_at(session(50)?).await?.0).ok_or("source fence")?;
    let instance = allocate_ready_instance(h, 88).await?;
    let pin = issue_fixture_content(h).await?;
    let allocation = owner
        .ready_allocations(&authority, &h.node, source.runtime_scope.world_id(), None)
        .await?
        .into_iter()
        .find(|row| row.scope == instance)
        .ok_or("ready control allocation missing")?;
    let position = LastPositionRecord {
        world_id: source.runtime_scope.world_id(),
        cell: LastPositionCell {
            x: 100,
            y: 101,
            floor: 7,
        },
        coordinate_frame_digest: pin.coordinate_frame_digest,
        map_revision_digest: pin.map_revision_digest,
        server_artifact_digest: pin.server_artifact_digest,
        vitals: LastVitals::new(83, 17),
        rest: Some([2, 300, 4, 5]),
    };
    let request = ActivityEnterRequest {
        handoff_id: fixture::id(89),
        command_id: 1,
        initiating_character_id: CharacterId::decode(source.character_id.as_bytes())
            .map_err(debug)?,
        activity_key: "oteryn:activity.owner_fixture".into(),
        template_revision: "fixture-template-1".into(),
        content_revision: "content-1".into(),
        destination_scope: instance,
        destination_generation: allocation.generation,
        destination_assignment_source_revision: allocation.source_revision,
        destination_content: pin,
        time_limit_ms: 60_000,
        participants: vec![ActivityParticipant {
            source,
            destination_game_session_id: session(90)?,
            destination_transport: Some(transport(90)?),
            selected_capabilities: vec![117],
            source_position: position,
            source_position_write_sequence: 1,
            destination_cell: LastPositionCell {
                x: 200,
                y: 201,
                floor: 7,
            },
            exit_position: LastPositionRecord {
                cell: LastPositionCell {
                    x: 102,
                    y: 103,
                    floor: 7,
                },
                ..position
            },
        }],
    };
    Ok((evidence, request))
}

pub(super) struct EnteredActivityFixture {
    pub(super) fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    pub(super) receipt: crate::durability::activity_handoff::ActivityEnterReceipt,
    pub(super) request: ActivityEnterRequest,
    pub(super) evidence: super::house_transfer::RecoverySource,
}

/// Shared production owner fixture for the real Instance death/Ground cases.
/// Callers retain this value so its authenticated evidence source stays alive.
pub(super) async fn enter_fixture(h: &Harness) -> TestResult<EnteredActivityFixture> {
    let (evidence, request) = prepare_activity_fixture(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let receipt = match ActivityHandoffOwner::new(h.root.clone())
        .enter(&authority, &h.node, request.clone())
        .await?
    {
        ActivityEnterOutcome::Entered(receipt) => receipt,
        other => return Err(format!("activity fixture enter refused: {other:?}").into()),
    };
    let current = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(request.participants[0].destination_game_session_id)
        .await?
        .0;
    let fence = item_fence_of(current).ok_or("activity fixture current Instance fence")?;
    Ok(EnteredActivityFixture {
        fence,
        receipt,
        request,
        evidence,
    })
}

async fn case(h: &Harness) -> TestResult {
    let (_evidence, request) = prepare_activity_fixture(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let owner = ActivityHandoffOwner::new(h.root.clone());
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let source = request.participants[0].source;
    let instance = request.destination_scope;
    let before = retained_rows(h).await?;
    let mut no_capacity = request.clone();
    no_capacity.destination_scope =
        RuntimeScopeRefV1::instance(source.runtime_scope.world_id(), fixture::id(87))
            .map_err(debug)?;
    assert_eq!(
        owner.enter(&authority, &h.node, no_capacity).await?,
        ActivityEnterOutcome::Refused(ActivityRefusal::NoCapacity)
    );
    for stale_lease in [false, true] {
        let mut stale = request.clone();
        if stale_lease {
            stale.participants[0].source.character_lease_generation += 1;
        } else {
            stale.participants[0].source.connection_generation =
                ConnectionGeneration::new(2).map_err(debug)?;
        }
        assert_eq!(
            owner.enter(&authority, &h.node, stale).await?,
            ActivityEnterOutcome::Refused(ActivityRefusal::AuthorityRejected)
        );
    }
    let mut reused_transport = request.clone();
    let retained_transport: Vec<u8> = sqlx::query_scalar("SELECT current_transport_ref FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(source.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    reused_transport.participants[0].destination_transport =
        Some(AuthenticatedTransportRefV1::decode(&retained_transport).map_err(debug)?);
    assert_eq!(
        owner.enter(&authority, &h.node, reused_transport).await?,
        ActivityEnterOutcome::Refused(ActivityRefusal::InvalidInput)
    );
    // A second invalid member is found after the first member's position write.
    // The entire semantic transaction must roll it back, including that position.
    let mut partial = request.clone();
    let mut missing = partial.participants[0].clone();
    missing.source.character_id =
        crate::domain::CharacterId::from_bytes(fixture::id(91)).map_err(debug)?;
    missing.source.game_session_id = session(92)?;
    missing.destination_game_session_id = session(93)?;
    missing.destination_transport = Some(transport(93)?);
    missing.destination_cell.x += 1;
    partial.participants.push(missing);
    assert_eq!(
        owner.enter(&authority, &h.node, partial).await?,
        ActivityEnterOutcome::Refused(ActivityRefusal::AuthorityRejected)
    );
    assert_eq!(retained_rows(h).await?, before);
    eprintln!("ACTIVITY_ATOMIC_REFUSALS POSTGRES_CASE_COMPLETED");

    let entered = match owner.enter(&authority, &h.node, request.clone()).await? {
        ActivityEnterOutcome::Entered(receipt) => receipt,
        other => return Err(format!("real enter refused: {other:?}").into()),
    };
    assert_eq!(entered.destination_scope, instance);
    assert_eq!(entered.participants.len(), 1);
    assert!(
        owner
            .retirement_ready(
                &authority,
                &h.node,
                instance,
                entered.destination_generation
            )
            .await?
            .is_none(),
        "one admitted participant prevents retirement"
    );
    assert_eq!(
        entered.participants[0].destination_transport,
        Some(transport(90)?)
    );
    let (current, _) = store.current_session_at(session(90)?).await?;
    assert_eq!(current.session_state(), GameSessionState::Active);
    let in_room = item_fence_of(current).ok_or("admitted Instance fence")?;
    assert_eq!(in_room.runtime_scope, instance);
    assert_eq!(
        in_room.character_lease_generation,
        source.character_lease_generation + 1
    );
    assert_eq!(in_room.connection_generation.get(), 1);
    let exact: (i16, Vec<u8>, bool) = sqlx::query_as("SELECT session_state,current_transport_ref,game_activity_session_origin_valid(game_session_id,character_id) \
        FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(session(90)?.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(exact, (2, vec![90; 16], true));
    let old_state: i16 = sqlx::query_scalar("SELECT session_state FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(source.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(old_state, 3);
    assert_eq!(
        owner.enter(&authority, &h.node, request.clone()).await?,
        ActivityEnterOutcome::Replayed(entered.clone())
    );
    let mut conflict = request.clone();
    conflict.participants[0].destination_cell.x += 1;
    assert_eq!(
        owner.enter(&authority, &h.node, conflict).await?,
        ActivityEnterOutcome::Refused(ActivityRefusal::ReplayConflict)
    );
    assert_eq!(retained_rows(h).await?, (1, 0, before.2 + 1, 1));
    assert!(
        owner
            .ready_allocations(&authority, &h.node, source.runtime_scope.world_id(), None)
            .await?
            .iter()
            .all(|row| row.scope != instance)
    );
    eprintln!("ACTIVITY_ENTER_CURRENT_IDENTITY POSTGRES_CASE_COMPLETED");

    let placement = owner
        .read_return_placement(&authority, &h.node, in_room)
        .await?
        .ok_or("retained return placement")?;
    assert_eq!(placement.handoff_id, entered.handoff_id);
    assert_eq!(placement.destination_scope, source.runtime_scope);
    assert_eq!(
        placement.exit_position.cell,
        request.participants[0].exit_position.cell
    );
    let returning = ActivityReturnRequest {
        return_id: fixture::id(94),
        handoff_id: entered.handoff_id,
        command_id: Some(2),
        source: in_room,
        destination_game_session_id: session(95)?,
        destination_transport: Some(transport(95)?),
        destination_generation: placement.destination_generation,
        reason: ActivityReturnReason::Exit,
        current_vitals: LastVitals::new(51, 9),
        current_rest: Some([2, 200, 8, 9]),
    };
    let mut premature = returning.clone();
    premature.reason = ActivityReturnReason::Timeout;
    assert_eq!(
        owner
            .return_participant(&authority, &h.node, premature)
            .await?,
        ActivityReturnOutcome::Refused(ActivityRefusal::NotDue)
    );
    let mut recovery = returning.clone();
    recovery.destination_transport = None;
    assert_eq!(
        owner.recover(&authority, &h.node, recovery).await?,
        ActivityReturnOutcome::Refused(ActivityRefusal::OwnerStillCurrent)
    );
    let returned = match owner
        .return_participant(&authority, &h.node, returning.clone())
        .await?
    {
        ActivityReturnOutcome::Returned(receipt) => receipt,
        other => return Err(format!("real return refused: {other:?}").into()),
    };
    assert_eq!(returned.destination_scope, source.runtime_scope);
    assert_eq!(
        returned.destination_lease_generation,
        source.character_lease_generation + 2
    );
    let back = item_fence_of(store.current_session_at(session(95)?).await?.0)
        .ok_or("returned Channel fence")?;
    assert_eq!(back.runtime_scope, source.runtime_scope);
    assert_eq!(
        back.character_lease_generation,
        source.character_lease_generation + 2
    );
    assert_eq!(
        owner
            .return_participant(&authority, &h.node, returning.clone())
            .await?,
        ActivityReturnOutcome::Replayed(returned)
    );
    let mut wrong_replay = returning.clone();
    wrong_replay.reason = ActivityReturnReason::Defeated;
    assert_eq!(
        owner
            .return_participant(&authority, &h.node, wrong_replay)
            .await?,
        ActivityReturnOutcome::Refused(ActivityRefusal::ReplayConflict)
    );
    let saved: (i32, i32, i16, Option<i32>, Option<i32>, String) = sqlx::query_as("SELECT tile_x,tile_y,tile_floor,health,mana,character_lease_generation::text FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid")
        .bind(source.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        saved,
        (
            102,
            103,
            7,
            Some(51),
            Some(9),
            (source.character_lease_generation + 2).to_string()
        )
    );
    assert_eq!(retained_rows(h).await?, (1, 1, before.2 + 2, 1));
    let retiring = owner
        .retirement_ready(
            &authority,
            &h.node,
            instance,
            entered.destination_generation,
        )
        .await?
        .ok_or("complete returns must qualify retirement")?;
    assert_eq!(retiring.scope(), instance);
    assert_eq!(retiring.generation(), entered.destination_generation);
    assert_eq!(retiring.handoff_id(), entered.handoff_id);
    // Retained enter receipts still replay after return, but do not make the old
    // Instance session current or permit that used allocation to be admitted again.
    assert_eq!(
        owner.enter(&authority, &h.node, request.clone()).await?,
        ActivityEnterOutcome::Replayed(entered)
    );
    let mut reused = request;
    reused.handoff_id = fixture::id(96);
    reused.command_id = 3;
    reused.participants[0].source = back;
    reused.participants[0].destination_game_session_id = session(97)?;
    reused.participants[0].destination_transport = Some(transport(97)?);
    reused.participants[0].source_position_write_sequence = 2;
    assert_eq!(
        owner.enter(&authority, &h.node, reused).await?,
        ActivityEnterOutcome::Refused(ActivityRefusal::NoCapacity)
    );
    eprintln!("ACTIVITY_RETURN_REPLAY_RETENTION POSTGRES_CASE_COMPLETED");

    // The actual runtime role has only the new append/state-column privileges.
    // It still cannot create capacity or erase provenance/history.
    for table in [
        "game_control_activity_scope_grants",
        "game_runtime_scope_assignments",
        "game_activity_runtime_entries",
        "game_activity_runtime_returns",
    ] {
        let prohibited: bool = sqlx::query_scalar(
            "SELECT has_table_privilege('oteryn_game_runtime',$1,'DELETE,TRUNCATE')",
        )
        .bind(table)
        .fetch_one(&h.pool)
        .await?;
        assert!(!prohibited, "runtime can erase {table}");
    }
    eprintln!("ACTIVITY_RUNTIME_PRIVILEGES POSTGRES_CASE_COMPLETED");
    Ok(())
}
