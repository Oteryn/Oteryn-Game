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
    TalismanMint,
    CosmeticGrant,
}

impl RollbackFault {
    pub(super) fn tag(self) -> &'static str {
        match self {
            Self::TokenLedger => "task_board_rb_token",
            Self::StashAuditMissing => "task_board_rb_stash_missing",
            Self::StashAuditEvent => "task_board_rb_stash_event",
            Self::StashAuditEnvelope => "task_board_rb_stash_envelope",
            Self::TalismanMint => "task_board_rb_talisman",
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
    super::super::kill_reward_e2e::scenario_thread::run(name, scenario)
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
fn task_board_real_dispatch_talisman_mint_fee_rolls_back_and_disconnects() -> TestResult {
    run_fault("task-board-talisman-rollback", || {
        scenario_with_fault(Some(RollbackFault::TalismanMint))
    })
}

#[test]
fn task_board_real_dispatch_cosmetic_participant_omission_rolls_back_and_disconnects() -> TestResult {
    run_fault("task-board-cosmetic-rollback", || {
        scenario_with_fault(Some(RollbackFault::CosmeticGrant))
    })
}
