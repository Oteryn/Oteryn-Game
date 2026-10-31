//! More than one due page, real blocked returns and a real control-owner change.
//! Every source session and every room is issued through its production owner.
use super::TestResult;
use super::activity_runtime::{allocate_ready_instance, prepare_activity_fixture};
use super::fixture::{Harness, debug, id};
use crate::durability::activity_handoff::{
    ActivityEnterOutcome, ActivityHandoffOwner, ActivityRefusal, ActivityReturnOutcome,
    ActivityReturnReason, ActivityReturnRequest,
};
use crate::durability::admission_authority_guards::AdmissionGuardStore;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
    RuntimeScopeAssignmentWriter,
};
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1, AdmissionAuthorityPublicationV1,
    AdmissionPublicationPreconditionV1,
};
use crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
use crate::foundation::{
    AuthenticatedTransportRefV1, CharacterId, GameSessionId, RuntimeScopeRefV1,
};
use crate::gameplay_transport::item_fence_of;

const BOSS: &str = "fixture:activity_deadline_paging";

#[test]
fn activity_deadline_paging_postgres_blocked_and_foreign_first_page() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("ACTIVITY_DEADLINE_PAGING POSTGRES_CASE_SKIPPED: no admin URL");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "activity deadline pager panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    let runtime = crate::bestiary_postgres_harness::runtime()?;
    let h = runtime.block_on(Harness::create_with_admission_seed(
        admin,
        "activity_deadline_pager",
        true,
        false,
    ))?;
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runtime.block_on(case(&h))));
    let cleanup = runtime.block_on(h.cleanup());
    match result {
        Ok(result) => {
            result?;
            cleanup?;
        }
        Err(panic) => std::panic::resume_unwind(panic),
    }
    eprintln!("ACTIVITY_DEADLINE_PAGING POSTGRES_CASE_COMPLETED");
    Ok(())
}

async fn case(h: &Harness) -> TestResult {
    let (_evidence, mut template) = prepare_activity_fixture(h).await?;
    template.activity_key = BOSS.into();
    template.time_limit_ms = 1_000; // Explicit isolated fixture policy.
    let world = template.destination_scope.world_id();
    let pin = template.destination_content;
    let cell = |x, y| serde_json::json!({"x":x,"y":y,"floor":7});
    sqlx::query("INSERT INTO game_boss_room_catalogue(boss_key,content_revision,boss_creature_key,minimum_players,maximum_players,minimum_level,cooldown_seconds,time_limit_seconds,lever_positions,room_template_key,reward_key,enabled,template_revision,entry_positions,exit_position,boss_position,after_kill_seconds) VALUES($1,'content-1','fixture:pager_boss',1,1,0,20,1,$2,'fixture:pager_room','fixture:pager_reward',true,$3,$4,$5,$6,1)")
        .bind(BOSS).bind(serde_json::json!([cell(100,101)]))
        .bind(&template.template_revision).bind(serde_json::json!([cell(200,201)]))
        .bind(cell(102,103)).bind(cell(201,201)).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'activity-pager-fixture',$2,$3,$4,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),'2026-10')")
        .bind(world.as_bytes().as_slice()).bind(pin.server_artifact_digest.as_slice())
        .bind([0x44_u8;32].as_slice()).bind(pin.coordinate_frame_digest.as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let owner = ActivityHandoffOwner::new(h.root.clone());
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let mut requests = Vec::new();
    for member in 0_u8..17 {
        let mut request = template.clone();
        request.handoff_id = id(200 + member);
        if member != 0 {
            let session = admit_member(h, member).await?;
            request.participants[0].source =
                item_fence_of(store.current_session_at(session).await?.0)
                    .ok_or("real pager source fence")?;
            request.initiating_character_id =
                CharacterId::decode(request.participants[0].source.character_id.as_bytes())
                    .map_err(debug)?;
            let scope = allocate_ready_instance(h, 160 + member).await?;
            let assignment = h
                .root
                .read_runtime_scope_assignment(scope)
                .await
                .map_err(debug)?
                .ok_or("ready pager assignment")?;
            request.destination_scope = scope;
            request.destination_assignment_source_revision = assignment.source_revision;
            request.destination_generation =
                crate::foundation::ScopeOwnershipGeneration::new(assignment.ownership_generation)
                    .map_err(debug)?;
            request.participants[0].destination_game_session_id =
                GameSessionId::decode(&id(180 + member)).map_err(debug)?;
            request.participants[0].destination_transport =
                Some(AuthenticatedTransportRefV1::decode(&[180 + member; 16]).map_err(debug)?);
        }
        let outcome = owner
            .enter_boss_room(&authority, &h.node, request.clone())
            .await?;
        assert!(
            matches!(outcome, ActivityEnterOutcome::Entered(_)),
            "real pager room {member}: {outcome:?}"
        );
        requests.push(request);
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_boss_room_activities WHERE state=1"
        )
        .fetch_one(&h.pool)
        .await?,
        17
    );
    // Wait only the authored one-second fixture timer; never manufacture a DB deadline.
    tokio::time::sleep(std::time::Duration::from_millis(1_050)).await;

    // A genuinely revoked origin makes valid Timeout returns fail NoCapacity.
    // It must not prevent the due cursor from reaching later rooms.
    let origin = requests[0].participants[0].source.runtime_scope;
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = origin
    else {
        return Err("origin Channel".into());
    };
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,3)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).execute(&h.pool).await?;
    let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "sequencer-writer")
        .await
        .map_err(debug)?;
    let predecessor = h
        .root
        .read_runtime_scope_predecessor(origin)
        .await
        .map_err(debug)?
        .ok_or("origin predecessor")?;
    assert!(matches!(
        writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([229; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Revoke {
                    scope: origin,
                    predecessor
                },
            })
            .await
            .map_err(debug)?,
        AssignmentOutcome::Committed(_)
    ));
    let first = owner
        .due_boss_rooms_after(&authority, &h.node, world, None)
        .await?;
    assert_eq!(first.rooms.len(), 15);
    assert_eq!(first.next_after, Some(requests[14].handoff_id));
    for (index, request) in requests.iter().take(15).enumerate() {
        let member = &request.participants[0];
        let source = item_fence_of(
            store
                .current_session_at(member.destination_game_session_id)
                .await?
                .0,
        )
        .ok_or("current blocked room fence")?;
        let index = u8::try_from(index)?;
        let outcome = owner
            .return_participant(
                &authority,
                &h.node,
                ActivityReturnRequest {
                    return_id: id(220 + index),
                    handoff_id: request.handoff_id,
                    command_id: None,
                    source,
                    destination_game_session_id: GameSessionId::decode(&id(240 + index))
                        .map_err(debug)?,
                    destination_transport: Some(
                        AuthenticatedTransportRefV1::decode(&[220 + index; 16]).map_err(debug)?,
                    ),
                    destination_generation: member.source.scope_ownership_generation,
                    reason: ActivityReturnReason::Timeout,
                    current_vitals: member.source_position.vitals,
                    current_rest: member.source_position.rest,
                },
            )
            .await?;
        assert_eq!(
            outcome,
            ActivityReturnOutcome::Refused(ActivityRefusal::NoCapacity)
        );
    }
    let repeated = owner
        .due_boss_rooms_after(&authority, &h.node, world, None)
        .await?;
    assert_eq!(
        repeated, first,
        "blocked rows remain due instead of disappearing from fixture setup"
    );
    let second = owner
        .due_boss_rooms_after(&authority, &h.node, world, first.next_after)
        .await?;
    assert_eq!(
        second
            .rooms
            .iter()
            .map(|room| room.handoff_id)
            .collect::<Vec<_>>(),
        requests[15..]
            .iter()
            .map(|r| r.handoff_id)
            .collect::<Vec<_>>()
    );
    assert!(second.next_after.is_none());

    // Move the first fifteen scopes through the real control writer. Their
    // old sessions/room history remain, but those assignments now belong to a
    // different node/generation. The old global LIMIT would hide the final two.
    let other_node = super::fixture::registered_node(&h.root, 6).await?;
    for (index, request) in requests.iter().take(15).enumerate() {
        let RuntimeScopeRefV1::Instance {
            world_id,
            instance_id,
        } = request.destination_scope
        else {
            return Err("room Instance".into());
        };
        sqlx::query("INSERT INTO game_control_activity_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,2)")
            .bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice()).execute(&h.pool).await?;
        let predecessor = h
            .root
            .read_runtime_scope_predecessor(request.destination_scope)
            .await
            .map_err(debug)?
            .ok_or("room predecessor")?;
        let key = 230 + u8::try_from(index)?;
        let outcome = writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([key; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Replace {
                    scope: request.destination_scope,
                    predecessor,
                    target: other_node.fact(),
                },
            })
            .await
            .map_err(debug)?;
        assert!(
            matches!(outcome, AssignmentOutcome::Committed(_)),
            "foreign assignment: {outcome:?}"
        );
    }
    let own = owner
        .due_boss_rooms_after(&authority, &h.node, world, None)
        .await?;
    assert_eq!(
        own.rooms, second.rooms,
        "foreign first-page histories must not hide current owned rooms"
    );
    assert!(own.next_after.is_none());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_activity_runtime_returns")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    Ok(())
}

async fn admit_member(h: &Harness, member: u8) -> TestResult<GameSessionId> {
    let character = id(100 + member);
    let account = id(120 + member);
    // Only initial Character/progression input is fixture data, as in Harness.
    // Session, claim, ledger, transport and handoff rows are never copied.
    sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid)")
        .bind(account.as_slice())
        .execute(&h.pool)
        .await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name,sex) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,1,'profile-1','ruleset-1','content-1','starter-1',$4,NULL)")
        .bind(character.as_slice()).bind(account.as_slice()).bind(id(42).as_slice())
        .bind(format!("Fixture Member {}",char::from(b'A' + member))).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_progression_state VALUES(encode($1,'hex')::uuid,1,50,1000,'profile-1','ruleset-1','content-1','simulation-1','evidence-1','declaration-1','policy-1','reward-1')")
        .bind(character.as_slice()).execute(&h.pool).await?;
    let (account,character_text):(String,String)=sqlx::query_as("SELECT account_id::text,character_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_slice()).fetch_one(&h.pool).await?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let mut source = super::fresh_fixture::fresh::Source::appearance_observer(
        now,
        account,
        CharacterId::decode(&character).map_err(debug)?,
    )?;
    let guards = AdmissionGuardStore::from_root(h.root.clone());
    let keys = source
        .rows
        .iter()
        .filter(|row| {
            matches!(
                row.key,
                AdmissionAuthorityGuardKeyV1::Runtime(_)
                    | AdmissionAuthorityGuardKeyV1::SigningTrust { .. }
            )
        })
        .map(|row| row.key.clone())
        .collect::<Vec<_>>();
    for prior in guards.load(&keys).await.map_err(debug)? {
        let prior = prior.ok_or("current global fixture publication")?;
        let row = source
            .rows
            .iter_mut()
            .find(|row| row.key == prior.key)
            .ok_or("global fixture key")?;
        if matches!(row.key, AdmissionAuthorityGuardKeyV1::SigningTrust { .. }) {
            row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
                expected_publication_revision: prior.publication_revision,
            };
            row.publication_revision = prior
                .publication_revision
                .checked_add(1)
                .ok_or("publication revision")?;
            row.source.source_revision = prior
                .source
                .source_revision
                .checked_add(1)
                .ok_or("source revision")?;
            row.source.decision_identity =
                format!("platform-observation-{}", row.source.source_revision);
        } else {
            *row = prior;
        }
    }
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
    let rows = source.rows.clone();
    source
        .rows
        .retain(|row| !matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    guards
        .publish(&AdmissionAuthorityPublicationV1::prepare(&source, now).map_err(debug)?)
        .await
        .map_err(debug)?;
    source.rows = rows;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let session = GameSessionId::decode(&id(140 + member)).map_err(debug)?;
    let transport = AuthenticatedTransportRefV1::decode(&[150 + member; 16]).map_err(debug)?;
    let request = source.activity_room_request(
        session,
        transport,
        &character_text,
        [170 + member; 32],
        now,
    )?;
    let outcome = FreshAdmissionStore::from_root(h.root.clone())
        .commit(&request)
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, FreshAdmissionDurableOutcomeV1::Committed(_)),
        "real pager source admission: {outcome:?}"
    );
    Ok(session)
}
