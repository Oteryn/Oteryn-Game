#![allow(clippy::unwrap_used, clippy::panic)] // Test assertions only.
use super::*;
use crate::foundation::{CommandId, GameSessionId};
use oteryn_protocol_oteryn::house::{HouseIntent, HouseTarget, encode_house_intent};
fn uuid(seed: u8) -> [u8; 16] {
    let mut bytes = [seed; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}
fn world(seed: u8) -> WorldId {
    WorldId::decode(&uuid(seed)).unwrap()
}
fn actor() -> CharacterId {
    CharacterId::decode(&uuid(1)).unwrap()
}
fn command(seed: u8) -> CommandRef {
    CommandRef::new(
        GameSessionId::decode(&uuid(seed)).unwrap(),
        CommandId::new(7).unwrap(),
    )
}
fn request(action: HouseAction) -> HouseIntent {
    HouseIntent {
        house: HouseTarget {
            world_id: world(3),
            house_key: "oteryn:content.house.thais_1".into(),
        },
        action,
    }
}
fn prepare(
    value: &HouseIntent,
    current_world: WorldId,
) -> Result<PreparedHouseIntent, HouseIntentPreparationError> {
    prepare_house_intent(
        &encode_house_intent(value).unwrap(),
        command(2),
        actor(),
        current_world,
    )
}
#[test]
fn every_action_binds_explicit_native_house_and_exact_actor_command() {
    for action in [
        HouseAction::Bid { max_gold: u64::MAX },
        HouseAction::MoveOut { date_unix_ms: 1 },
        HouseAction::CancelMoveOut,
        HouseAction::AclSet {
            role: HouseAclRole::Guest,
            character_ids: vec![actor()],
            expected_revision: 7,
        },
        HouseAction::DoorSet {
            position: ActorPosition {
                x: 10,
                y: 24,
                floor: -7,
            },
            character_ids: vec![],
            expected_revision: 8,
        },
        HouseAction::Kick {
            character_id: actor(),
        },
        HouseAction::Leave,
    ] {
        let value = request(action);
        let prepared = prepare(&value, world(3)).unwrap();
        assert_eq!(
            prepared.house(),
            &HouseId::new(world(3), "oteryn:content.house.thais_1").unwrap()
        );
        assert_eq!(prepared.world(), world(3));
        assert_eq!(prepared.actor(), actor());
        assert_eq!(prepared.command(), command(2));
        assert_eq!(
            prepare(&value, world(4)),
            Err(HouseIntentPreparationError::WorldMismatch)
        );
        let mut other = value.clone();
        other.house.house_key = "oteryn:content.house.thais_2".into();
        let other = prepare(&other, world(3)).unwrap();
        assert_ne!(prepared.house(), other.house());
        assert_eq!(prepared.action(), other.action());
    }
}
#[test]
fn invalid_native_key_refuses_for_every_action_including_leave() {
    for action in [
        HouseAction::Bid { max_gold: 1 },
        HouseAction::Leave,
        HouseAction::CancelMoveOut,
    ] {
        for key in [
            "h",
            "oteryn:content.house.",
            "oteryn:content.house.THais",
            "oteryn:content.house.a-b",
            "oteryn:content.npc.thais",
        ] {
            let mut value = request(action.clone());
            value.house.house_key = key.into();
            assert_eq!(
                prepare(&value, world(3)),
                Err(HouseIntentPreparationError::InvalidHouseKey)
            );
        }
    }
}
#[test]
fn native_max_key_is_preserved_for_non_bid_requests() {
    let mut value = request(HouseAction::Leave);
    value.house.house_key = format!(
        "oteryn:content.house.{}",
        "a".repeat(512 - "oteryn:content.house.".len())
    );
    assert_eq!(
        prepare(&value, world(3)).unwrap().house().house_key(),
        value.house.house_key
    );
}
#[test]
fn request_data_preserves_revision_order_and_milliseconds_without_authority_claim() {
    let value = request(HouseAction::AclSet {
        role: HouseAclRole::Guest,
        character_ids: vec![CharacterId::decode(&uuid(8)).unwrap(), actor()],
        expected_revision: 0,
    });
    assert_eq!(
        prepare(&value, world(3)).unwrap().action(),
        &PreparedHouseAction::AclSet {
            role: HouseAclRole::Guest,
            character_ids: vec![CharacterId::decode(&uuid(8)).unwrap(), actor()],
            expected_revision: 0
        }
    );
    assert_eq!(
        prepare(&request(HouseAction::MoveOut { date_unix_ms: 1 }), world(3))
            .unwrap()
            .action(),
        &PreparedHouseAction::MoveOut { date_unix_ms: 1 }
    );
}
#[test]
fn prepared_door_coordinates_and_session_correlation_keep_native_values() {
    use crate::map::overlay::TilePos;
    for tile in [
        TilePos {
            x: 0,
            y: 0,
            floor: 0,
        },
        TilePos {
            x: u16::MAX,
            y: u16::MAX,
            floor: -15,
        },
        TilePos {
            x: 10,
            y: 24,
            floor: -7,
        },
    ] {
        let position = ActorPosition {
            x: i32::from(tile.x),
            y: i32::from(tile.y),
            floor: i16::from(tile.floor),
        };
        let value = request(HouseAction::DoorSet {
            position,
            character_ids: vec![],
            expected_revision: u64::MAX,
        });
        let bytes = encode_house_intent(&value).unwrap();
        let first = prepare_house_intent(&bytes, command(2), actor(), world(3)).unwrap();
        let second = prepare_house_intent(&bytes, command(5), actor(), world(3)).unwrap();
        assert_eq!(
            first.action(),
            &PreparedHouseAction::DoorSet {
                position,
                character_ids: vec![],
                expected_revision: u64::MAX
            }
        );
        assert_eq!(first.house(), second.house());
        assert_eq!(first.action(), second.action());
        assert_ne!(first.command(), second.command());
    }
}
#[test]
fn historical_unqualified_or_malformed_bodies_never_prepare() {
    for raw in [
        &[58, 0][..],
        &[26, 0, 58, 0][..],
        &[42, 4, 10, 2, 24, 2][..],
        &[42, 4, 10, 2, 8, 1][..],
    ] {
        assert_eq!(
            prepare_house_intent(raw, command(2), actor(), world(3)),
            Err(HouseIntentPreparationError::Wire(HouseWireError::Malformed))
        );
    }
}
