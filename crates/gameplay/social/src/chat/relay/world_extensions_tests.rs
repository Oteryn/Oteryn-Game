#![allow(clippy::expect_used)]
use super::*;

const CHANNEL: &str = "oteryn_chat_01890a5dac967ab280e70123456789ab";
const OTHER_WORLD: &str = "oteryn_chat_01890a5dac967ab280e70123456789ac";
const NOW: u64 = 1_000_000_000;

fn target() -> CharacterId {
    let mut bytes = [9; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    CharacterId::decode(&bytes).expect("UUIDv7 fixture")
}

fn line(destination: RelayDestination) -> RelayLine {
    RelayLine {
        message_id: [4; 16],
        sent_at_micros: NOW,
        destination,
        sender_name: CharacterName::parse("Sender").expect("name"),
        text: ChatText::parse("hello").expect("text"),
    }
}

#[test]
fn world_extensions_preserve_authentication_world_boundary_and_replay() {
    let ring = RelayKeyRing::new(vec![RelayKey::new(1, 0, [3; 32])]).expect("ring");
    for destination in [
        RelayDestination::Presence {
            target: target(),
            online: false,
        },
        RelayDestination::Presence {
            target: target(),
            online: true,
        },
        RelayDestination::PrivateChannel {
            channel_id: 100,
            epoch: 1,
        },
        RelayDestination::PrivateChannel {
            channel_id: 9_999,
            epoch: u64::MAX,
        },
    ] {
        let original = line(destination);
        let sealed = seal_with_nonce(&ring, CHANNEL, &original, [7; NONCE_BYTES]).expect("sealed");
        assert!(sealed.len() <= NOTIFY_MAX_BYTES);
        assert_eq!(open(&ring, CHANNEL, &sealed, NOW), Ok(original.clone()));
        assert_eq!(
            open(&ring, OTHER_WORLD, &sealed, NOW),
            Err(RelayError::Unauthentic)
        );
        let mut window = ReplayWindow::starting_at(NOW);
        assert_eq!(window.receive(&ring, CHANNEL, &sealed, NOW), Ok(original));
        assert_eq!(
            window.receive(&ring, CHANNEL, &sealed, NOW),
            Err(RelayError::Replayed)
        );
    }
    assert_eq!(
        SEALED_MAX_BYTES, 1_217,
        "new destinations fit the existing Guild width"
    );
}

#[test]
fn private_channel_relay_refuses_invalid_id_epoch_truncation_and_trailing_data() {
    for (channel_id, epoch) in [(0, 1), (99, 1), (10_000, 1), (u16::MAX, 1), (100, 0)] {
        assert_eq!(
            line(RelayDestination::PrivateChannel { channel_id, epoch }).encode(),
            Err(RelayError::BadPayload)
        );
    }
    let valid = line(RelayDestination::PrivateChannel {
        channel_id: 100,
        epoch: 1,
    })
    .encode()
    .expect("valid");
    for length in 0..valid.len() {
        assert!(RelayLine::decode(&valid[..length]).is_err());
    }
    let mut trailing = valid.clone();
    trailing.push(0);
    assert_eq!(RelayLine::decode(&trailing), Err(RelayError::BadPayload));
    let mut invalid_epoch = valid;
    invalid_epoch[27..35].fill(0);
    assert_eq!(
        RelayLine::decode(&invalid_epoch),
        Err(RelayError::BadPayload)
    );
}

#[test]
fn presence_refuses_non_boolean_and_non_uuid_and_redacts_routing() {
    let destination = RelayDestination::Presence {
        target: target(),
        online: true,
    };
    let valid = line(destination).encode().expect("presence");
    let mut bad_boolean = valid.clone();
    bad_boolean[41] = 2;
    assert_eq!(RelayLine::decode(&bad_boolean), Err(RelayError::BadPayload));
    let mut bad_id = valid;
    bad_id[25..41].fill(0);
    assert_eq!(RelayLine::decode(&bad_id), Err(RelayError::BadPayload));
    assert_eq!(format!("{destination:?}"), "Presence { .. }");
    assert_eq!(
        format!(
            "{:?}",
            RelayDestination::PrivateChannel {
                channel_id: 100,
                epoch: 9
            }
        ),
        "PrivateChannel { .. }"
    );
}
