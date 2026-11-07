#![allow(clippy::expect_used)]
use super::spectate_test_support::local;
use super::spectate_test_support::{facts, grant, limits, observer};
use super::*;
use oteryn_protocol_oteryn::spectate::{EndReason, SpectateMessage};

#[test]
fn spectate_caps_and_overload_never_mutate_the_target() {
    let current = facts(1, 1, 1);
    let mut owner = SpectateRuntime::from(limits(60, 400));
    let (revision, code) = grant(&mut owner, current);
    for marker in 1..=50 {
        assert_eq!(
            owner.apply(Request::Admit {
                facts: current,
                observer: observer(marker),
                code: code.to_vec(),
                grant_revision: revision
            }),
            Reply::Admitted
        );
    }
    assert_eq!(
        owner.apply(Request::Admit {
            facts: current,
            observer: observer(51),
            code: code.to_vec(),
            grant_revision: revision
        }),
        Reply::Full
    );
    for _ in 0..65 {
        owner.apply(Request::Publish {
            facts: current,
            projection: WorldProjection::Chat(local()),
        });
    }
    assert_eq!(
        owner.apply(Request::Refresh(current)),
        Reply::State {
            active: true,
            count: 0
        }
    );
    assert_eq!(
        owner.apply(Request::Drain(observer(1))),
        Reply::Frames(vec![SpectateMessage::End(EndReason::Overload)])
    );
    let mut owner = SpectateRuntime::from(limits(10, 40));
    let (revision, code) = grant(&mut owner, current);
    for marker in 1..=3 {
        let same_account = ObserverIdentity {
            account: observer(1).account,
            ..observer(marker)
        };
        assert_eq!(
            owner.apply(Request::Admit {
                facts: current,
                observer: same_account,
                code: code.to_vec(),
                grant_revision: revision
            }),
            if marker <= 2 {
                Reply::Admitted
            } else {
                Reply::NotAvailable
            }
        );
    }
}

#[test]
fn spectate_node_capacity_and_rolling_attempts_fail_uniformly() {
    let current = facts(1, 1, 1);
    let mut owner = SpectateRuntime::from(limits(1, 40));
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
        owner.apply(Request::Refresh(current)),
        Reply::State {
            active: true,
            count: 1
        }
    );
    let mut owner = SpectateRuntime::from(limits(10, 40));
    let (revision, code) = grant(&mut owner, current);
    for _ in 0..5 {
        assert_eq!(
            owner.apply(Request::Admit {
                facts: current,
                observer: observer(1),
                code: vec![0; 16],
                grant_revision: revision
            }),
            Reply::NotAvailable
        );
    }
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
            observer: observer(2),
            code: code.to_vec(),
            grant_revision: revision
        }),
        Reply::Admitted
    );
}
