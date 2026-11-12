use super::*;
use crate::durability::{DurabilityError, SqlxFailureClass};

#[test]
fn player_trade_read_contention_is_unavailable() {
    for error in [
        DurabilityError::RootUnavailable,
        DurabilityError::RootPassDeadlineExceeded,
        DurabilityError::Database(SqlxFailureClass::RetryableStatement),
    ] {
        assert!(matches!(CheckError::from(error), CheckError::Unavailable));
    }
}

#[test]
fn player_trade_refusal_causes_are_preserved() {
    assert_eq!(
        CommandOutcome::from(CheckError::from(Refusal::Rejected)).cause,
        "refusal_rejected"
    );
    assert_eq!(
        CommandOutcome::from(CheckError::from(Refusal::UnknownWeight)).cause,
        "refusal_unknown_weight"
    );
    assert_eq!(
        CommandOutcome::budget_exhausted().cause,
        "retry_budget_exhausted"
    );
    let error = DurabilityError::Unavailable;
    assert_eq!(
        CommandOutcome::from(CheckError::from(error)).cause,
        "durability_durability_work_is_unavailable"
    );
}
