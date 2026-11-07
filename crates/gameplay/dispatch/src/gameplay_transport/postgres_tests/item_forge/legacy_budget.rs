//! Existing DustLimit callers and command181 share one bounded retry allowance.
use super::super::fixture::debug;
use super::*;
use crate::durability::character_forge_dust::{
    ForgeDustLimitCandidate, ForgeDustLimitError, ForgeDustLimitRequest,
};
use crate::foundation::{CommandId, CommandRef};

#[test]
fn forge_postgres_legacy_retry_exhaustion_cannot_reset_through_command181() -> TestResult {
    super::super::kill_reward_e2e::scenario_thread::run("forge-legacy-budget", || {
        support::scenario(Case::LegacyBudget)
    })
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &super::super::Dependencies,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let before = balances(h).await?;
    let first = request(Operation::DustLimit, 201);
    let frozen = freeze(owner, *admitted, first).await?;
    stop_receipts(h).await?;
    for _ in 0..3 {
        fail_legacy(owner, *admitted, &frozen).await?;
    }
    resume_receipts(h).await?;
    assert_eq!(legacy_work(h, first).await?, 3);
    assert_eq!(
        exchange(owner, admitted, first).await?,
        (CommandStatus::Rejected, Value::Null)
    );
    let adopted: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_forge_reservations WHERE occurrence_id=encode($1,'hex')::uuid",
    )
    .bind(first.occurrence.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        adopted, 0,
        "an exhausted legacy reservation cannot receive a fresh allowance"
    );
    assert_eq!(legacy_work(h, first).await?, 3);
    assert_eq!(balances(h).await?, before);

    let second = request(Operation::DustLimit, 202);
    let frozen = freeze(owner, *admitted, second).await?;
    stop_receipts(h).await?;
    fail_legacy(owner, *admitted, &frozen).await?;
    assert_eq!(legacy_work(h, second).await?, 1);
    let wire_failure = exchange(owner, admitted, second).await;
    assert!(
        wire_failure.is_err(),
        "the guarded effects failure disconnects after its second reservation charge"
    );
    assert_eq!(both_work(h, second).await?, (2, 2));
    fail_legacy(owner, *admitted, &frozen).await?;
    assert_eq!(
        both_work(h, second).await?,
        (3, 3),
        "alternating APIs share the same third attempt"
    );
    resume_receipts(h).await?;
    // The interrupted command cannot reopen on its original session. A genuine
    // fresh login keeps the Character's exhausted native occurrence unchanged.
    super::recovery::fresh_login(h, owner, deps, admitted).await?;
    assert_eq!(
        exchange(owner, admitted, second).await?,
        (CommandStatus::Rejected, Value::Null)
    );
    assert_eq!(both_work(h, second).await?, (3, 3));
    let error = owner
        .root
        .commit_forge_dust_limit(
            owner.character,
            owner.holder,
            admitted.item_fence.ok_or("item fence")?,
            command(*admitted)?,
            &frozen,
        )
        .await
        .err()
        .ok_or("exhausted legacy API must refuse")?;
    assert!(
        matches!(error, ForgeDustLimitError::CapacityExceeded),
        "{error:?}"
    );
    assert_eq!(balances(h).await?, before);
    let effects: (i64, i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_forge_operations),(SELECT count(*) FROM game_forge_history),(SELECT count(*) FROM game_forge_dust_limit_receipts),(SELECT count(*) FROM game_item_audit_outbox)").fetch_one(&h.pool).await?;
    assert_eq!(
        effects,
        (0, 0, 0, 0),
        "failed retries leave no debit, terminal receipt, history or event"
    );
    Ok(())
}

fn command(admitted: AdmittedSession) -> TestResult<CommandRef> {
    Ok(CommandRef::new(
        admitted.game_session_id,
        CommandId::new(admitted.continuity.next_command_id).map_err(debug)?,
    ))
}

async fn freeze(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    action: Request,
) -> TestResult<ForgeDustLimitCandidate> {
    owner
        .root
        .freeze_forge_dust_limit(
            owner.character,
            owner.holder,
            admitted.item_fence.ok_or("item fence")?,
            command(admitted)?,
            ForgeDustLimitRequest {
                occurrence_id: action.occurrence,
                expected_upgrade_count: 0,
            },
        )
        .await
        .map_err(debug)
}

async fn fail_legacy(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    frozen: &ForgeDustLimitCandidate,
) -> TestResult {
    let error = owner
        .root
        .commit_forge_dust_limit(
            owner.character,
            owner.holder,
            admitted.item_fence.ok_or("item fence")?,
            command(admitted)?,
            frozen,
        )
        .await
        .err()
        .ok_or("injected native receipt constraint must fail")?;
    assert!(
        matches!(error, ForgeDustLimitError::Unavailable(_)),
        "{error:?}"
    );
    Ok(())
}

async fn stop_receipts(h: &Harness) -> TestResult {
    sqlx::query("ALTER TABLE game_forge_dust_limit_receipts ADD CONSTRAINT forge_test_stop_legacy_effects CHECK(false) NOT VALID").execute(&h.pool).await?;
    Ok(())
}

async fn resume_receipts(h: &Harness) -> TestResult {
    sqlx::query(
        "ALTER TABLE game_forge_dust_limit_receipts DROP CONSTRAINT forge_test_stop_legacy_effects",
    )
    .execute(&h.pool)
    .await?;
    Ok(())
}

async fn legacy_work(h: &Harness, action: Request) -> TestResult<i16> {
    Ok(sqlx::query_scalar("SELECT work_units_used FROM game_forge_dust_limit_reservations WHERE occurrence_id=encode($1,'hex')::uuid").bind(action.occurrence.as_slice()).fetch_one(&h.pool).await?)
}

async fn both_work(h: &Harness, action: Request) -> TestResult<(i16, i16)> {
    Ok(sqlx::query_as("SELECT p.work_units_used,l.work_units_used FROM game_forge_reservations p JOIN game_forge_dust_limit_reservations l USING(character_id,occurrence_id) WHERE p.occurrence_id=encode($1,'hex')::uuid").bind(action.occurrence.as_slice()).fetch_one(&h.pool).await?)
}
