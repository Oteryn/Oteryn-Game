//! Each injected commit failure owns its admitted session and isolated database.
//! An unavailable command remains outstanding: no fixture resets ingress or resumes
//! that stream. The scenario checks rollback through the durable readers, then ends.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RollbackFault {
    TokenLedger,
    StashAuditMissing,
    StashAuditEvent,
    StashAuditEnvelope,
    TalismanUpgrade,
    CosmeticGrant,
}

impl RollbackFault {
    pub(super) fn tag(self) -> &'static str {
        match self {
            Self::TokenLedger => "task_board_rb_token",
            Self::StashAuditMissing => "task_board_rb_stash_missing",
            Self::StashAuditEvent => "task_board_rb_stash_event",
            Self::StashAuditEnvelope => "task_board_rb_stash_envelope",
            Self::TalismanUpgrade => "task_board_rb_talisman",
            Self::CosmeticGrant => "task_board_rb_cosmetic",
        }
    }

    pub(super) fn is_stash(self) -> bool {
        matches!(
            self,
            Self::StashAuditMissing | Self::StashAuditEvent | Self::StashAuditEnvelope
        )
    }
}

/// The production route maps TaskBoardReply::Unavailable to a disconnect without a
/// terminal result. The advanced command watermark proves the command was reserved,
/// rather than rejected before dispatch. This session must never be reused afterward.
pub(super) fn assert_unavailable(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    end: ConnectionEnd,
    replies: &[(u64, CommandStatus, Vec<u8>)],
    admitted: &AdmittedSession,
    command: u64,
) {
    assert_eq!(end, ConnectionEnd::AdmittedThenDisconnected(*admitted));
    assert!(replies.is_empty(), "{end:?}; {replies:?}");
    assert_eq!(admitted.continuity.next_command_id, command + 1);
    assert!(!owner.states.command_ingress.resumable(admitted));
}

/// Sequences survive rollback. This isolated fixture counter proves the injected
/// trigger was reached; an unrelated earlier refusal cannot pass an atomicity case.
pub(super) async fn start_fault_witness(harness: &Harness) -> TestResult {
    sqlx::raw_sql("CREATE SEQUENCE task_board_test_fault_hits; GRANT USAGE ON SEQUENCE task_board_test_fault_hits TO oteryn_game_runtime;")
        .execute(&harness.pool).await?;
    Ok(())
}

pub(super) async fn finish_fault_witness(harness: &Harness, minimum: i64) -> TestResult {
    let (calls, called): (i64, bool) =
        sqlx::query_as("SELECT last_value,is_called FROM task_board_test_fault_hits")
            .fetch_one(&harness.pool)
            .await?;
    sqlx::query("DROP SEQUENCE task_board_test_fault_hits")
        .execute(&harness.pool)
        .await?;
    assert!(called && calls >= minimum, "injected trigger was not reached: {called}/{calls}");
    Ok(())
}

fn run_fault(name: &str, scenario: fn() -> TestResult) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("<TASK_BOARD> POSTGRES_CASE_SKIPPED: {name}: OTERYN_TEST_POSTGRES_ADMIN_URL is not set");
        return Ok(());
    }
    super::super::kill_reward_e2e_fixtures::scenario_thread::run(name, scenario)
}

#[test]
fn task_board_real_dispatch_token_ledger_omission_rolls_back_and_disconnects() -> TestResult {
    run_fault("task-board-token-rollback", || {
        scenario_with_fault(Some(RollbackFault::TokenLedger))
    })
}

#[test]
fn task_board_real_dispatch_stash_audit_omission_rolls_back_and_disconnects() -> TestResult {
    run_fault("task-board-stash-missing-rollback", || {
        scenario_with_fault(Some(RollbackFault::StashAuditMissing))
    })
}

#[test]
fn task_board_real_dispatch_stash_audit_event_mismatch_rolls_back_and_disconnects() -> TestResult {
    run_fault("task-board-stash-event-rollback", || {
        scenario_with_fault(Some(RollbackFault::StashAuditEvent))
    })
}

#[test]
fn task_board_real_dispatch_stash_audit_envelope_mismatch_rolls_back_and_disconnects() -> TestResult {
    run_fault("task-board-stash-envelope-rollback", || {
        scenario_with_fault(Some(RollbackFault::StashAuditEnvelope))
    })
}

#[test]
fn task_board_real_dispatch_cosmetic_participant_omission_rolls_back_and_disconnects() -> TestResult {
    run_fault("task-board-cosmetic-rollback", || {
        scenario_with_fault(Some(RollbackFault::CosmeticGrant))
    })
}

#[test]
fn task_board_real_dispatch_talisman_upgrade_rolls_back_and_disconnects() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::super::kill_reward_e2e_fixtures::scenario_thread::run(
        "task-board-talisman-rollback",
        || scenario_with_fault(Some(RollbackFault::TalismanUpgrade)),
    )
}

pub(super) async fn talisman_upgrade_rollback(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &mut AdmittedSession,
) -> TestResult {
    fixture_track(harness, task_board::BOUNTY_POINTS, 5).await?;
    let before = full_board(owner, admitted, 4, 0, "").await?;
    assert_eq!(before.bounty_points, 5);
    assert_eq!(before.talisman[0].level, 0);
    assert_eq!(before.talisman[0].next_cost, 5);
    start_fault_witness(harness).await?;
    sqlx::raw_sql("CREATE FUNCTION task_board_test_fail_upgrade() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('task_board_test_fault_hits'); RAISE EXCEPTION 'task board upgrade rollback fault' USING ERRCODE='23514'; END $$; CREATE CONSTRAINT TRIGGER task_board_test_fail_upgrade AFTER INSERT ON game_task_board_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN (NEW.action ? 'UpgradeTalisman' AND NEW.refusal = 'null'::jsonb) EXECUTE FUNCTION task_board_test_fail_upgrade();")
        .execute(&harness.pool).await?;
    let command = admitted.continuity.next_command_id;
    let (end, replies) =
        serve_attempt(owner, admitted, command, &[full_request(18, 0, "")]).await?;
    assert_unavailable(owner, end, &replies, admitted, command);
    finish_fault_witness(harness, 1).await?;
    let points: i64 = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2",
    )
    .bind(id(41).as_slice()).bind(task_board::BOUNTY_POINTS).fetch_one(&harness.pool).await?;
    assert_eq!(points, 5);
    let upgrades: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_task_board_receipts WHERE character_id=encode($1,'hex')::uuid AND action ? 'UpgradeTalisman'",
    ).bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
    assert_eq!(upgrades, 0);
    let levels: i64 = sqlx::query_scalar(
        "SELECT COALESCE(sum(value),0)::bigint FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2",
    )
    .bind(id(41).as_slice()).bind(task_board::TALISMAN_DAMAGE).fetch_one(&harness.pool).await?;
    assert_eq!(levels, 0);
    Ok(())
}
