#![allow(clippy::expect_used)]
use super::spectate_test_support::local;
use super::spectate_test_support::{facts, grant, limits, observer};
use super::*;
use oteryn_protocol_oteryn::{
    chat::{ChatLine, ChatRoom, encode_chat_line},
    spectate::SpectateMessage,
};

#[test]
fn spectate_projection_excludes_private_room_inventory_and_outcomes() {
    let mut owner = SpectateRuntime::from(limits(10, 40));
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
    for line in [
        ChatLine::Private {
            speaker_name: "secret".into(),
            text: "private".into(),
        },
        ChatLine::Room {
            room: ChatRoom::World,
            speaker_name: "room".into(),
            text: "private room".into(),
        },
        ChatLine::Dropped,
    ] {
        owner.apply(Request::Publish {
            facts: current,
            projection: WorldProjection::Chat(line),
        });
    }
    assert_eq!(
        owner.apply(Request::Drain(observer(1))),
        Reply::Frames(vec![])
    );
    let line = local();
    let expected = encode_chat_line(&line).expect("native chat");
    owner.apply(Request::Publish {
        facts: current,
        projection: WorldProjection::Chat(line),
    });
    assert_eq!(
        owner.apply(Request::Drain(observer(1))),
        Reply::Frames(vec![SpectateMessage::Frame {
            domain: 12,
            payload_type: 1,
            snapshot: false,
            payload: expected
        }])
    );
    // Inventory and CommandResult have no WorldProjection variant. Gameplay inputs have no handler.
    assert_eq!(
        owner.apply(Request::ObserverInput {
            observer: observer(1),
            command: ObserverCommand::Gameplay
        }),
        Reply::NotAllowed
    );
}
