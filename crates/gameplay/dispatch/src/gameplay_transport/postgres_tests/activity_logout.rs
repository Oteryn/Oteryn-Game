//! A real authenticated logout command returns its activity session before the
//! existing Channel logout owner releases it. Server expiry is a separate gate.
use super::TestResult;
use super::fixture::{self, Harness, debug};
use crate::durability::activity_handoff::{
    ActivityHandoffOwner, ActivityRefusal, ActivityReturnOutcome, ActivityReturnReason,
    ActivityReturnRequest,
};
use crate::durability::item_decay_retire::ActivityGroundRetirement;
use crate::durability::spell_owner_commit::SpellLane;
use crate::foundation::{AuthenticatedTransportRefV1, GameSessionId};

#[test]
fn activity_logout_postgres_real_command_and_server_authority() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("ACTIVITY_LOGOUT POSTGRES_CASE_SKIPPED: no admin URL");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "activity logout fixture panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        eprintln!("ACTIVITY_LOGOUT POSTGRES_CASE_SKIPPED: no configured admin");
        return Ok(());
    };
    let runtime = crate::bestiary_postgres_harness::runtime()?;
    let harness = runtime.block_on(Harness::create_with_admission_seed(
        admin,
        "activity_logout_owner",
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

async fn case(h: &Harness) -> TestResult {
    let entered = super::activity_runtime::enter_fixture(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let owner = ActivityHandoffOwner::new(h.root.clone());
    let placement = owner
        .read_return_placement(&authority, &h.node, entered.fence)
        .await?
        .ok_or("return placement")?;
    let request = ActivityReturnRequest {
        return_id: fixture::id(100),
        handoff_id: entered.receipt.handoff_id,
        command_id: Some(4101),
        source: entered.fence,
        destination_game_session_id: GameSessionId::decode(&fixture::id(101)).map_err(debug)?,
        destination_transport: Some(
            AuthenticatedTransportRefV1::decode(&[101; 16]).map_err(debug)?,
        ),
        destination_generation: placement.destination_generation,
        reason: ActivityReturnReason::Logout,
        current_vitals: entered.request.participants[0].source_position.vitals,
        current_rest: entered.request.participants[0].source_position.rest,
    };
    let mut forged_expiry = request.clone();
    forged_expiry.command_id = None;
    assert_eq!(
        owner
            .return_participant(&authority, &h.node, forged_expiry)
            .await?,
        ActivityReturnOutcome::Refused(ActivityRefusal::NotDue)
    );
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_activity_runtime_returns")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(receipts, 0);
    let returned = match owner
        .return_participant(&authority, &h.node, request.clone())
        .await?
    {
        ActivityReturnOutcome::Returned(receipt) => receipt,
        other => return Err(format!("actual logout command refused: {other:?}").into()),
    };
    assert_eq!(
        returned.destination_scope,
        entered.request.participants[0].source.runtime_scope
    );
    assert_eq!(
        owner
            .return_participant(&authority, &h.node, request)
            .await?,
        ActivityReturnOutcome::Replayed(returned)
    );
    let recorded: (i16, String) =
        sqlx::query_as("SELECT reason,command_id::text FROM game_activity_runtime_returns")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(recorded, (6, "4101".to_owned()));
    let proof = owner
        .retirement_ready(
            &authority,
            &h.node,
            entered.fence.runtime_scope,
            entered.fence.scope_ownership_generation,
        )
        .await?
        .ok_or("complete returned activity")?;
    let channel_lane = SpellLane::for_scope(placement.destination_scope);
    let channel_permit = channel_lane
        .acquire()
        .await
        .map_err(|_| "Channel lane unresolved")?;
    assert!(
        h.root
            .retire_activity_ground_step(&channel_permit, &authority, &h.node, &proof)
            .await
            .is_err()
    );
    let instance_lane = SpellLane::for_scope(entered.fence.runtime_scope);
    let instance_permit = instance_lane
        .acquire()
        .await
        .map_err(|_| "Instance lane unresolved")?;
    assert_eq!(
        h.root
            .retire_activity_ground_step(&instance_permit, &authority, &h.node, &proof)
            .await
            .map_err(debug)?,
        ActivityGroundRetirement::Complete
    );
    eprintln!("ACTIVITY_LOGOUT_POSTGRES_REAL_COMMAND_AND_SCOPE_FENCE POSTGRES_CASE_COMPLETED");
    Ok(())
}
