//! HOUSE-RUNTIME-1 blocked boundary: management authority cannot substitute for
//! an admitted House runtime or bypass the native handoff's generation fences.
use super::*;
use crate::durability::house_scope_handoff::{
    HouseAbortOutcome, HouseEntryRequest, HouseHandoffDirection, HouseHandoffError, HouseId,
    HousePrepareOutcome, HouseScopeAssignmentCommand, HouseScopeAssignmentOutcome,
    HouseScopeAssignmentRequest,
};
use crate::durability::runtime_scope_assignment::{ControlActor, OperationKey};
use crate::foundation::RuntimeScopeRefV1;

#[test]
fn house_runtime_gate_postgres_dispatch_preserves_channel_and_handoff_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "House runtime gate PG thread panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    reset();
    let rt = crate::bestiary_postgres_harness::runtime()?;
    let h = rt.block_on(Harness::create_with_admission_seed(
        admin,
        "house_runtime_gate",
        true,
        false,
    ))?;
    // Keep cleanup outside the fallible case, including a failed fence assertion.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rt.block_on(case(&h))));
    let cleanup = rt.block_on(h.cleanup());
    match result {
        Ok(case) => {
            case?;
            cleanup
        }
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

async fn case(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let owner = deps.owner(h, &authority);
    deps.initialize(&owner).await;
    let position = {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .initialize_first_entry_position(deps.actor)
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
        runtime.read_actor_position(deps.actor).map_err(debug)?
    };
    let (current, _) =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(deps.admitted.game_session_id)
            .await?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = item_fence_of(current);
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        owner.offered_capabilities(),
        &[6, wire::CAPABILITY_HOUSE_V1],
    )
    .ok_or("House selection")?;
    let fence = admitted.item_fence.ok_or("live Channel fence")?;
    let house = HouseId::new(admitted.world_id, HOUSE).ok_or("House identity")?;
    sqlx::query(include_str!(
        "../../../../../../../apps/game-server/tests/support/owned_house_fixture.sql"
    ))
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(id(41).as_slice())
    .execute(&h.pool)
    .await?;
    // A real assigned House and an owned property eliminate missing authority as
    // a reason for the source-generation refusals below. No House actor is forged.
    sqlx::query("INSERT INTO game_control_house_scope_grants(control_role,world_id,house_key,operation) SELECT session_user,encode($1,'hex')::uuid,$2,1")
        .bind(id(42).as_slice()).bind(HOUSE).execute(&h.pool).await?;
    assert!(matches!(
        h.root
            .assign_house_scope(HouseScopeAssignmentRequest {
                operation_key: OperationKey::from_bytes([80; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                actor_role: "oteryn_test_admin".into(),
                command: HouseScopeAssignmentCommand::Assign {
                    house: house.clone(),
                    target: h.node.fact(),
                },
            })
            .await
            .map_err(debug)?,
        HouseScopeAssignmentOutcome::Committed(_)
    ));
    let before = session_binding(h, admitted.game_session_id).await?;
    let mut kick = Vec::new();
    bytes(&mut kick, 1, &id(41));
    // These are valid, negotiated command bytes through serve_admitted and the
    // production ComposedFreshAdmission, not a mock FreshAdmissionAuthority.
    for body in [intent(7, &[]), intent(6, &kick)] {
        assert_eq!(
            send(
                &owner,
                &mut admitted,
                wire::COMMAND_TYPE_HOUSE_INTENT,
                &body
            )
            .await?,
            (
                CommandStatus::Accepted,
                wire::encode_house_result(wire::HouseDisposition::Rejected, 0)
            )
        );
        assert_eq!(session_binding(h, admitted.game_session_id).await?, before);
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime.read_actor_position(deps.actor).map_err(debug)?,
            position
        );
        assert_eq!(
            runtime
                .positioned_player_for_session(admitted.game_session_id)
                .map_err(debug)?,
            Some((deps.actor, position))
        );
    }
    let request = HouseEntryRequest {
        direction: HouseHandoffDirection::Entry,
        handoff_id: id(81),
        character_id: crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
            .map_err(debug)?,
        source_game_session_id: admitted.game_session_id,
        source_connection_generation: fence.connection_generation.get(),
        source_character_lease_generation: fence.character_lease_generation,
        source_scope: fence.runtime_scope,
        source_scope_ownership_generation: fence.scope_ownership_generation.get(),
        house,
        house_scope_ownership_generation: 1,
        acl_revision: 0,
        guild_revisions: None,
        reserved_tile: vec![1, 2, 3],
        prepared_at: 100,
    };
    // Vary exactly one live generation at a time. The valid request below must
    // prepare, so these failures cannot be explained by unrelated bad facts.
    for field in 0..3 {
        let mut stale = request.clone();
        match field {
            0 => stale.source_connection_generation += 1,
            1 => stale.source_character_lease_generation += 1,
            _ => stale.source_scope_ownership_generation += 1,
        }
        assert!(matches!(
            h.root.prepare_house_entry(&authority, &h.node, stale).await,
            Err(HouseHandoffError::AuthorityRejected)
        ));
        assert_eq!(session_binding(h, admitted.game_session_id).await?, before);
        assert_eq!(handoffs(h, admitted.game_session_id).await?, 0);
    }
    let mut exit = request.clone();
    exit.direction = HouseHandoffDirection::Exit;
    assert!(matches!(
        h.root.prepare_house_entry(&authority, &h.node, exit).await,
        Err(HouseHandoffError::ExitNotAdmitted)
    ));
    assert!(matches!(
        crate::house_scope::exit_into_channel(),
        Err(HouseHandoffError::ExitNotAdmitted)
    ));
    assert!(matches!(
        h.root
            .prepare_house_entry(&authority, &h.node, request.clone())
            .await
            .map_err(debug)?,
        HousePrepareOutcome::Prepared(_)
    ));
    assert_eq!(handoffs(h, admitted.game_session_id).await?, 1);
    assert_eq!(session_binding(h, admitted.game_session_id).await?, before);
    assert_eq!(
        h.root
            .abort_house_entry(&authority, &h.node, request.handoff_id)
            .await
            .map_err(debug)?,
        HouseAbortOutcome::Aborted
    );
    assert_eq!(handoffs(h, admitted.game_session_id).await?, 0);
    assert_eq!(session_binding(h, admitted.game_session_id).await?, before);
    assert_eq!(
        fence.runtime_scope,
        RuntimeScopeRefV1::channel(admitted.world_id, admitted.channel_id)
    );
    Ok(())
}

async fn handoffs(h: &Harness, session: crate::foundation::GameSessionId) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_house_scope_handoffs WHERE source_game_session_id=encode($1,'hex')::uuid")
        .bind(session.as_bytes().as_slice()).fetch_one(&h.pool).await?)
}

async fn session_binding(
    h: &Harness,
    session: crate::foundation::GameSessionId,
) -> TestResult<serde_json::Value> {
    // Exactly one primary-key row: compare all fences, transport, origin and scope,
    // including the state that a rejected handoff must leave live.
    let json: String = sqlx::query_scalar("SELECT row_to_json(s)::text FROM game_durability_reconnect_sessions s WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(session.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    Ok(serde_json::from_str(&json)?)
}
