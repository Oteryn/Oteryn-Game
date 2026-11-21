use super::fixture::{self, Harness, debug};

use super::{TestResult, reset};

use crate::durability::activity_handoff::{
    ActivityContentPin, ActivityEnterOutcome, ActivityEnterRequest, ActivityHandoffOwner,
    ActivityParticipant,
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
    AuthenticatedTransportRefV1, CharacterId, GameSessionId, RuntimeScopeRefV1,
};

use crate::gameplay_transport::item_fence_of;

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

pub(super) fn session(tag: u8) -> TestResult<GameSessionId> {
    GameSessionId::decode(&fixture::id(tag)).map_err(debug)
}

pub(super) fn transport(tag: u8) -> TestResult<AuthenticatedTransportRefV1> {
    AuthenticatedTransportRefV1::decode(&[tag; 16]).map_err(debug)
}

pub(super) async fn prepare_activity_fixture(
    h: &Harness,
) -> TestResult<(super::house_transfer_fixtures::RecoverySource, ActivityEnterRequest)> {
    reset();
    let evidence = super::house_transfer_fixtures::RecoverySource::start(h).await?;
    super::house_transfer_fixtures::prepare_owned_session(h).await?;
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
    pub(super) evidence: super::house_transfer_fixtures::RecoverySource,
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
