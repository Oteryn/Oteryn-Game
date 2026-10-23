#![allow(clippy::expect_used)]
use super::spectate_test_support::local;
use super::spectate_test_support::{facts, grant, limits, observer};
use super::*;
use oteryn_protocol_oteryn::spectate::{EndReason, SpectateMessage};

#[test]
fn spectate_lifecycle_ends_before_the_next_frame_and_restart_is_empty() {
    use oteryn_gameplay_foundation::foundation::{
        ChannelId, GameSessionAuthoritySnapshot, RuntimeScopeRefV1,
    };
    let initial = facts(1, 1, 1);
    let old = initial.authority;
    let moved = TargetFacts {
        authority: old
            .with_current_runtime_scope(RuntimeScopeRefV1::channel(
                old.commit().world_id(),
                ChannelId::decode(observer(201).session.as_bytes()).expect("new channel"),
            ))
            .expect("scope"),
        ..initial
    };
    let replaced = TargetFacts {
        authority: GameSessionAuthoritySnapshot::from_persisted_current_facts(
            observer(202).session,
            old.commit(),
            old.session_state(),
            old.current_connection_generation(),
            old.current_transport(),
            old.current_character_lease(),
            old.current_character_world_eligibility(),
            old.current_runtime_scope(),
            old.current_scope_generation(),
        )
        .expect("replacement native facts"),
        ..initial
    };
    for (changed, reason) in [
        (moved, EndReason::TargetMoved),
        (replaced, EndReason::TargetGone),
        (facts(2, 1, 1), EndReason::TargetGone),
        (facts(1, 2, 1), EndReason::TargetGone),
        (facts(1, 1, 2), EndReason::TargetMoved),
        (
            TargetFacts {
                alive: false,
                ..facts(1, 1, 1)
            },
            EndReason::TargetGone,
        ),
        (
            TargetFacts {
                observable: false,
                ..facts(1, 1, 1)
            },
            EndReason::NotObservable,
        ),
        (
            TargetFacts {
                house: Some([1; 16]),
                ..facts(1, 1, 1)
            },
            EndReason::TargetMoved,
        ),
    ] {
        let mut owner = SpectateRuntime::from(limits(10, 40));
        let (revision, code) = grant(&mut owner, facts(1, 1, 1));
        assert_eq!(
            owner.apply(Request::Admit {
                facts: facts(1, 1, 1),
                observer: observer(1),
                code: code.to_vec(),
                grant_revision: revision
            }),
            Reply::Admitted
        );
        owner.apply(Request::Publish {
            facts: facts(1, 1, 1),
            projection: WorldProjection::Chat(local()),
        });
        assert_eq!(
            owner.apply(Request::Refresh(changed)),
            Reply::State {
                active: false,
                count: 0
            }
        );
        owner.apply(Request::Publish {
            facts: changed,
            projection: WorldProjection::Chat(local()),
        });
        assert_eq!(
            owner.apply(Request::Drain(observer(1))),
            Reply::Frames(vec![SpectateMessage::End(reason)])
        );
        let mut restarted = SpectateRuntime::from(limits(10, 40));
        assert_eq!(
            restarted.apply(Request::Admit {
                facts: changed,
                observer: observer(1),
                code: code.to_vec(),
                grant_revision: revision
            }),
            Reply::NotAvailable
        );
    }
}
