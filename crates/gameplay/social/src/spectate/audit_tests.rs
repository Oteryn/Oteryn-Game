#![allow(clippy::expect_used)]
use super::spectate_test_support::{facts, grant, limits, observer};
use super::*;
use oteryn_protocol_oteryn::spectate::SpectateObservation;

#[test]
fn spectate_audit_reserves_all_terminal_facts_and_acks_only_committed_prefix() {
    let mut owner = SpectateRuntime::from(limits(10, 4));
    let current = facts(1, 1, 1);
    let (revision, code) = grant(&mut owner, current);
    assert_eq!(
        owner.apply(Request::Admit {
            facts: current,
            observer: observer(1),
            code: code.to_vec(),
            grant_revision: revision
        }),
        Reply::Admitted
    );
    assert_eq!(
        owner.apply(Request::Admit {
            facts: current,
            observer: observer(2),
            code: code.to_vec(),
            grant_revision: revision
        }),
        Reply::NotAvailable
    );
    assert_eq!(
        owner.apply(Request::GrantRevoke {
            facts: current,
            expected_revision: revision
        }),
        Reply::State {
            active: false,
            count: 0
        }
    );
    let Reply::Audit(first) = owner.apply(Request::AuditRead) else {
        unreachable!("audit");
    };
    assert_eq!(first.len(), 4);
    for (index, fact) in first.iter().enumerate() {
        assert_eq!(fact.ordinal, (index + 1) as u64);
        assert_eq!(
            fact.target_session,
            current.authority.current_game_session_id()
        );
        assert!(SpectateObservation::try_from(fact.payload.as_slice()).is_ok());
        assert!(!fact.payload.windows(16).any(|bytes| bytes == code));
    }
    assert_eq!(owner.apply(Request::AuditRead), Reply::Audit(first.clone()));
    assert_eq!(
        owner.apply(Request::AuditCommitted {
            incarnation: [0; 16],
            through: 4
        }),
        Reply::StaleRevision
    );
    assert_eq!(
        owner.apply(Request::AuditCommitted {
            incarnation: first[0].incarnation,
            through: 5
        }),
        Reply::StaleRevision
    );
    assert_eq!(
        owner.apply(Request::AuditCommitted {
            incarnation: first[0].incarnation,
            through: 2
        }),
        Reply::Applied
    );
    assert_eq!(
        owner.apply(Request::AuditRead),
        Reply::Audit(first[2..].to_vec())
    );
    let mut recreated = SpectateRuntime::from(limits(10, 4));
    grant(&mut recreated, current);
    let Reply::Audit(other) = recreated.apply(Request::AuditRead) else {
        unreachable!("audit");
    };
    assert_ne!(first[0].incarnation, other[0].incarnation);
}
