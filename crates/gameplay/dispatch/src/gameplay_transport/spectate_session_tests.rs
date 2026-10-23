#![allow(clippy::expect_used)]
use super::*;
use oteryn_gameplay_social::spectate::{
    WorldProjection,
    spectate_test_support::{facts, limits, observer},
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, encode_client_command,
    spectate::{Audience, EndReason},
};
use std::sync::Arc;

fn start(sessions: &SpectateSession) -> (u64, [u8; 16]) {
    let Reply::Grant { revision, code } =
        sessions.dispatch(SpectateDispatch::Owner(Box::new(Request::GrantSet {
            facts: facts(1, 1, 1),
            expected_revision: 0,
            audience: Audience::Code,
        })))
    else {
        unreachable!("grant");
    };
    assert_eq!(
        sessions.dispatch(SpectateDispatch::Owner(Box::new(Request::Admit {
            facts: facts(1, 1, 1),
            observer: observer(1),
            code: code.to_vec(),
            grant_revision: revision
        }))),
        Reply::Admitted
    );
    (revision, code)
}

#[test]
fn spectate_every_registered_and_future_gameplay_command_is_refused_before_handler() {
    let sessions = SpectateSession::from(limits(10, 40));
    start(&sessions);
    let registry: serde_json::Value =
        serde_json::from_str(oteryn_protocol_oteryn::PROTOCOL_OTERYN_V1_REGISTRY_JSON)
            .expect("registry");
    let command_ids = registry["command_types"]
        .as_array()
        .expect("commands")
        .iter()
        .map(|row| row["id"].as_u64().expect("id") as u32)
        .chain([u32::MAX]);
    let before = sessions.dispatch(SpectateDispatch::Owner(Box::new(Request::AuditRead)));
    for command_type in command_ids {
        // Even a gameplay payload that happens to encode OBSERVE_STOP cannot reach any handler.
        let frame = encode_client_command(
            1,
            &ClientCommandValue {
                command_id: 1,
                command_type,
                payload: &[8, 4],
            },
        )
        .expect("command");
        assert_eq!(
            sessions.dispatch(SpectateDispatch::ObserverFrame {
                observer: observer(1),
                frame: &frame
            }),
            Reply::NotAllowed
        );
    }
    assert_eq!(
        sessions.dispatch(SpectateDispatch::Owner(Box::new(Request::AuditRead))),
        before
    );
    assert_eq!(
        sessions.dispatch(SpectateDispatch::Owner(Box::new(Request::Refresh(facts(
            1, 1, 1
        ))))),
        Reply::State {
            active: true,
            count: 1
        }
    );
    let frame = Vec::<u8>::try_from(&ObserverEnvelope {
        generation: 1,
        message: SpectateMessage::Stop,
    })
    .expect("stop");
    assert_eq!(
        sessions.dispatch(SpectateDispatch::ObserverFrame {
            observer: observer(1),
            frame: &frame
        }),
        Reply::Applied
    );
    let mut output = Vec::new();
    assert_eq!(
        sessions.dispatch(SpectateDispatch::Flush {
            observer: observer(1),
            try_write: &mut |bytes| {
                output.extend_from_slice(bytes);
                Ok(bytes.len())
            }
        }),
        Reply::Applied
    );
    assert_eq!(
        ObserverEnvelope::try_from(&output[4..])
            .expect("end")
            .message,
        SpectateMessage::End(EndReason::ObserverStopped)
    );
}

#[test]
fn spectate_capability_stays_unoffered_and_player_commands_unselected() {
    use super::super::capabilities::{
        BUNDLE_WORLD_OFFERED_CAPABILITIES, PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities,
    };
    for offered in [
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
        PRODUCTION_OFFERED_CAPABILITIES,
    ] {
        assert!(!offered.iter().any(|capability| capability.id == 33));
        let selected =
            SelectedCapabilities::select(offered, &[33]).expect("valid additive support");
        assert!(!selected.contains(33));
        assert!(!selected.command_selected(34));
        assert!(!selected.command_selected(35));
        assert!(!selected.domain_selected(25));
    }
}

#[path = "spectate_concurrency_tests.rs"]
mod spectate_concurrency_tests;

#[path = "spectate_egress_tests.rs"]
mod spectate_egress_tests;
