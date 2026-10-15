#![allow(clippy::unwrap_used, clippy::panic)] // Test assertions only.

use super::*;
use crate::foundation::{CommandId, GameSessionId};
use oteryn_protocol_oteryn::house::encode_house_intent;

fn uuid(seed: u8) -> [u8; 16] {
    let mut b = [seed; 16];
    b[6] = 0x70;
    b[8] = 0x80;
    b
}
fn actor() -> CharacterId {
    CharacterId::decode(&uuid(1)).unwrap()
}
fn world(seed: u8) -> WorldId {
    WorldId::decode(&uuid(seed)).unwrap()
}
fn command(seed: u8) -> CommandRef {
    CommandRef::new(
        GameSessionId::decode(&uuid(seed)).unwrap(),
        CommandId::new(7).unwrap(),
    )
}
fn prepare(value: HouseIntent) -> PreparedHouseIntent {
    prepare_house_intent(
        &encode_house_intent(&value).unwrap(),
        command(2),
        actor(),
        world(3),
    )
    .unwrap()
}

#[test]
fn bid_resolves_exact_native_house_in_supplied_world_and_keeps_actor_command() {
    let request = prepare(HouseIntent::Bid {
        house_key: "oteryn:content.house.thais_1".into(),
        max_gold: 0,
    });
    assert_eq!(request.command(), command(2));
    assert_eq!(request.actor(), actor());
    assert_eq!(request.world(), world(3));
    let PreparedHouseAction::Bid { house, max_gold } = request.action() else {
        panic!("not bid")
    };
    assert_eq!(
        house,
        &HouseId::new(world(3), "oteryn:content.house.thais_1").unwrap()
    );
    assert_eq!(*max_gold, 0);
    // A second World resolves a different native identity, never a body-selected World.
    let body = encode_house_intent(&HouseIntent::Bid {
        house_key: house.house_key().into(),
        max_gold: 0,
    })
    .unwrap();
    let other = prepare_house_intent(&body, command(2), actor(), world(4)).unwrap();
    assert_ne!(request.action(), other.action());
}
#[test]
fn invalid_native_key_refuses_even_with_valid_wire_encoding() {
    for key in [
        "h",
        "oteryn:content.house.",
        "oteryn:content.house.THais",
        "oteryn:content.house.a-b",
        "oteryn:content.npc.thais",
    ] {
        let bytes = encode_house_intent(&HouseIntent::Bid {
            house_key: key.into(),
            max_gold: 1,
        })
        .unwrap();
        assert_eq!(
            prepare_house_intent(&bytes, command(2), actor(), world(3)),
            Err(HouseIntentPreparationError::InvalidHouseKey)
        );
    }
}
#[test]
fn native_max_key_is_accepted_and_codec_bound_is_preserved() {
    let key = format!(
        "oteryn:content.house.{}",
        "a".repeat(512 - "oteryn:content.house.".len())
    );
    assert!(matches!(
        prepare(HouseIntent::Bid {
            house_key: key,
            max_gold: u64::MAX
        })
        .action(),
        PreparedHouseAction::Bid {
            max_gold: u64::MAX,
            ..
        }
    ));
    assert_eq!(
        prepare_house_intent(&[26, 0, 58, 0], command(2), actor(), world(3)),
        Err(HouseIntentPreparationError::Wire(HouseWireError::Malformed))
    );
}
#[test]
fn own_house_requests_preserve_exact_inputs_without_resolving_or_claiming_owner() {
    assert_eq!(
        prepare(HouseIntent::MoveOut { date_unix_ms: 1 }).action(),
        &PreparedHouseAction::MoveOut { date_unix_ms: 1 }
    );
    assert_eq!(
        prepare(HouseIntent::CancelMoveOut).action(),
        &PreparedHouseAction::CancelMoveOut
    );
    let ids = vec![CharacterId::decode(&uuid(8)).unwrap(), actor()];
    assert_eq!(
        prepare(HouseIntent::AclSet {
            role: HouseAclRole::Guest,
            character_ids: ids.clone(),
            expected_revision: 0
        })
        .action(),
        &PreparedHouseAction::AclSet {
            role: HouseAclRole::Guest,
            character_ids: ids,
            expected_revision: 0
        }
    );
    assert_eq!(
        prepare(HouseIntent::Kick {
            character_id: actor()
        })
        .action(),
        &PreparedHouseAction::Kick {
            character_id: actor()
        }
    );
    assert_eq!(
        prepare(HouseIntent::Leave).action(),
        &PreparedHouseAction::Leave
    );
}
#[test]
fn door_request_keeps_revision_position_and_exact_session_correlation() {
    let pos = ActorPosition {
        x: 10,
        y: 24,
        floor: -7,
    };
    let bytes = encode_house_intent(&HouseIntent::DoorSet {
        position: pos,
        character_ids: vec![],
        expected_revision: u64::MAX,
    })
    .unwrap();
    let first = prepare_house_intent(&bytes, command(2), actor(), world(3)).unwrap();
    let successor = prepare_house_intent(&bytes, command(5), actor(), world(3)).unwrap();
    assert_eq!(
        first.action(),
        &PreparedHouseAction::DoorSet {
            position: pos,
            character_ids: vec![],
            expected_revision: u64::MAX
        }
    );
    assert_eq!(first.action(), successor.action());
    assert_ne!(first.command(), successor.command());
}

#[test]
fn prepared_door_coordinates_preserve_actual_native_tile_values() {
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
        let request = prepare(HouseIntent::DoorSet {
            position,
            character_ids: vec![],
            expected_revision: 0,
        });
        let PreparedHouseAction::DoorSet {
            position: prepared, ..
        } = request.action()
        else {
            panic!("not door")
        };
        assert_eq!(*prepared, position);
    }
    // Structurally valid signed-position bodies, independently encoded of the House validator.
    for body in [[42, 4, 10, 2, 24, 2], [42, 4, 10, 2, 8, 1]] {
        assert_eq!(
            prepare_house_intent(&body, command(2), actor(), world(3)),
            Err(HouseIntentPreparationError::Wire(HouseWireError::Malformed))
        );
    }
}
