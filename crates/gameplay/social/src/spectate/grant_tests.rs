#![allow(clippy::expect_used)]
use super::spectate_test_support::{facts, grant, limits, observer};
use super::*;
use oteryn_protocol_oteryn::spectate::{Audience, EndReason, SpectateMessage};

#[test]
fn spectate_code_consent_rotation_cas_and_contacts_unavailable() {
    let current = facts(1, 1, 1);
    let mut dormant = SpectateRuntime::default();
    assert_eq!(
        dormant.apply(Request::GrantSet {
            facts: current,
            expected_revision: 0,
            audience: Audience::Code
        }),
        Reply::NotAvailable
    );
    let mut owner = SpectateRuntime::from(limits(60, 300));
    assert_eq!(
        owner.apply(Request::GrantSet {
            facts: current,
            expected_revision: 0,
            audience: Audience::Contacts
        }),
        Reply::NotAvailable
    );
    let (revision, code) = grant(&mut owner, current);
    assert_eq!(
        owner.apply(Request::Admit {
            facts: current,
            observer: observer(1),
            code: vec![0; 16],
            grant_revision: revision
        }),
        Reply::NotAvailable
    );
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
        owner.apply(Request::GrantSet {
            facts: current,
            expected_revision: 0,
            audience: Audience::Code
        }),
        Reply::StaleRevision
    );
    let Reply::Grant {
        revision: next,
        code: next_code,
    } = owner.apply(Request::GrantSet {
        facts: current,
        expected_revision: revision,
        audience: Audience::Code,
    })
    else {
        unreachable!("rotation");
    };
    assert!(next > revision);
    assert_ne!(code, next_code);
    assert_eq!(
        owner.apply(Request::Drain(observer(1))),
        Reply::Frames(vec![SpectateMessage::End(EndReason::Revoked)])
    );
    assert_eq!(
        owner.apply(Request::Admit {
            facts: current,
            observer: observer(1),
            code: code.to_vec(),
            grant_revision: revision
        }),
        Reply::NotAvailable
    );
    assert_eq!(
        owner.apply(Request::Admit {
            facts: current,
            observer: observer(1),
            code: next_code.to_vec(),
            grant_revision: next
        }),
        Reply::Admitted
    );
}
