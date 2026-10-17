#![allow(clippy::unwrap_used, clippy::panic)] // Test assertions only.
use super::*;
use crate::foundation::{CommandId, GameSessionId, RuntimeScopeRefV1};
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

#[test]
fn query_preparation_preserves_command_actor_world_native_area_key_and_cursor() {
    use oteryn_protocol_oteryn::house::{HouseQuery, encode_house_query};
    let request = HouseQuery {
        world_id: world(3),
        catalogue_town: "oteryn:content.area.city.thais".into(),
        cursor: Some(vec![0, 255]),
    };
    let value = prepare_house_query(
        &encode_house_query(&request).unwrap(),
        command(2),
        actor(),
        world(3),
    )
    .unwrap();
    assert_eq!(value.command(), command(2));
    assert_eq!(value.actor(), actor());
    assert_eq!(value.world(), world(3));
    assert_eq!(
        value.catalogue_town(),
        &crate::content::ProductionKey::new(&request.catalogue_town).unwrap()
    );
    assert_eq!(value.cursor(), Some(&[0, 255][..]));
}
#[test]
fn query_preparation_refuses_cross_world_and_non_native_town_keys() {
    use oteryn_protocol_oteryn::house::{HouseQuery, encode_house_query};
    let request = HouseQuery {
        world_id: world(3),
        catalogue_town: "oteryn:content.area.city.thais".into(),
        cursor: None,
    };
    assert_eq!(
        prepare_house_query(
            &encode_house_query(&request).unwrap(),
            command(2),
            actor(),
            world(4)
        ),
        Err(HouseQueryPreparationError::WorldMismatch)
    );
    for key in [
        "oteryn:content.area.city.",
        "oteryn:content.house.thais",
        "oteryn:content.area.city.bad key",
        "oteryn:content.area.city.thais\0",
    ] {
        let bad = HouseQuery {
            catalogue_town: key.into(),
            ..request.clone()
        };
        assert_eq!(
            prepare_house_query(
                &encode_house_query(&bad).unwrap(),
                command(2),
                actor(),
                world(3)
            ),
            Err(HouseQueryPreparationError::InvalidCatalogueTown)
        );
    }
}

#[test]
fn query_empty_and_absent_cursor_normalize_only_to_first_page_without_changing_binding() {
    use oteryn_protocol_oteryn::house::{HouseQuery, encode_house_query};
    for cursor in [None, Some(vec![])] {
        let request = HouseQuery {
            world_id: world(3),
            catalogue_town: "oteryn:content.area.city.thais".into(),
            cursor,
        };
        let prepared = prepare_house_query(
            &encode_house_query(&request).unwrap(),
            command(2),
            actor(),
            world(3),
        )
        .unwrap();
        assert_eq!(prepared.cursor(), None);
        assert_eq!(prepared.command(), command(2));
        assert_eq!(prepared.actor(), actor());
        assert_eq!(prepared.world(), world(3));
        assert_eq!(prepared.catalogue_town().as_str(), request.catalogue_town);
    }
}

fn acl_fence() -> CurrentCharacterItemFence {
    use crate::foundation::{ChannelId, ConnectionGeneration, ScopeOwnershipGeneration};
    CurrentCharacterItemFence {
        character_id: crate::domain::CharacterId::from_bytes(*actor().as_bytes()).unwrap(),
        game_session_id: command(2).game_session_id(),
        connection_generation: ConnectionGeneration::new(3).unwrap(),
        character_lease_generation: 9,
        runtime_scope: RuntimeScopeRefV1::Channel {
            world_id: world(3),
            channel_id: ChannelId::decode(&uuid(4)).unwrap(),
        },
        scope_ownership_generation: ScopeOwnershipGeneration::new(11).unwrap(),
    }
}

#[test]
fn native_acl_adapter_preserves_remote_house_revision_subjects_and_command() {
    for (role, list) in [
        (HouseAclRole::Subowner, rules::List::Subowners),
        (HouseAclRole::Guest, rules::List::Guests),
    ] {
        for revision in [0, u64::MAX] {
            let ids = vec![CharacterId::decode(&uuid(8)).unwrap(), actor()];
            let prepared = prepare(
                &request(HouseAction::AclSet {
                    role,
                    character_ids: ids.clone(),
                    expected_revision: revision,
                }),
                world(3),
            )
            .unwrap();
            let native = native_acl_request(&prepared, &acl_fence()).unwrap();
            assert_eq!(&native.house, prepared.house());
            assert_eq!(native.command, command(2));
            assert_eq!(native.list, list);
            assert_eq!(native.expected_revision, revision);
            assert_eq!(
                native.entries,
                ids.iter()
                    .map(|id| {
                        rules::Entry::Character(
                            crate::domain::CharacterId::from_bytes(*id.as_bytes()).unwrap(),
                        )
                    })
                    .collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn native_acl_adapter_uses_native_door_coordinates_and_empty_list_clear() {
    for (x, y, floor) in [(0, 0, 0), (u16::MAX, u16::MAX, -15), (10, 24, -7)] {
        let prepared = prepare(
            &request(HouseAction::DoorSet {
                position: ActorPosition {
                    x: i32::from(x),
                    y: i32::from(y),
                    floor,
                },
                character_ids: vec![],
                expected_revision: 4,
            }),
            world(3),
        )
        .unwrap();
        let native = native_acl_request(&prepared, &acl_fence()).unwrap();
        assert_eq!(
            native.list,
            rules::List::Door(crate::map::overlay::TilePos {
                x,
                y,
                floor: i8::try_from(floor).unwrap(),
            })
        );
        assert!(native.entries.is_empty());
        assert_eq!(native.expected_revision, 4);
    }
}

#[test]
fn native_acl_adapter_refuses_each_mismatched_binding_before_writer_dispatch() {
    let prepared = prepare(
        &request(HouseAction::AclSet {
            role: HouseAclRole::Guest,
            character_ids: vec![],
            expected_revision: 1,
        }),
        world(3),
    )
    .unwrap();
    let mut different_actor = acl_fence();
    different_actor.character_id = crate::domain::CharacterId::from_bytes(uuid(8)).unwrap();
    let mut different_session = acl_fence();
    different_session.game_session_id = command(9).game_session_id();
    let mut different_world = acl_fence();
    if let RuntimeScopeRefV1::Channel { world_id, .. } = &mut different_world.runtime_scope {
        *world_id = world(9);
    }
    for (fence, expected) in [
        (different_actor, HouseAclRequestError::ActorMismatch),
        (different_session, HouseAclRequestError::SessionMismatch),
        (different_world, HouseAclRequestError::WorldMismatch),
    ] {
        assert!(matches!(native_acl_request(&prepared, &fence), Err(actual) if actual == expected));
    }
}

#[test]
fn native_acl_adapter_never_routes_another_house_action_to_acl_writer() {
    for action in [
        HouseAction::Bid { max_gold: 1 },
        HouseAction::MoveOut { date_unix_ms: 1 },
        HouseAction::CancelMoveOut,
        HouseAction::Kick {
            character_id: actor(),
        },
        HouseAction::Leave,
    ] {
        let prepared = prepare(&request(action), world(3)).unwrap();
        assert!(matches!(
            native_acl_request(&prepared, &acl_fence()),
            Err(HouseAclRequestError::NotAclAction)
        ));
    }
}

#[test]
fn native_acl_adapter_preserves_instance_scope_for_the_owning_writer() {
    let prepared = prepare(
        &request(HouseAction::AclSet {
            role: HouseAclRole::Guest,
            character_ids: vec![],
            expected_revision: 1,
        }),
        world(3),
    )
    .unwrap();
    let mut fence = acl_fence();
    fence.runtime_scope = RuntimeScopeRefV1::instance(world(3), uuid(7)).unwrap();
    let unchanged = fence;
    let native = native_acl_request(&prepared, &fence).unwrap();
    assert_eq!(fence, unchanged);
    assert_eq!(native.house.world_id(), fence.runtime_scope.world_id());
    // This is request conversion only: no receipt, scope admission or writer success.
    fence.runtime_scope = RuntimeScopeRefV1::instance(world(9), uuid(7)).unwrap();
    assert!(matches!(
        native_acl_request(&prepared, &fence),
        Err(HouseAclRequestError::WorldMismatch)
    ));
}
