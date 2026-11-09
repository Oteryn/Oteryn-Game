//! Source-qualified wire peer for the native Soulpit scope play test.
use super::*;
use oteryn_session::test_support::wire;
use tokio::io::{AsyncReadExt, DuplexStream};

fn source_map(stage: u8) -> Result<wire::world_map::WorldMapViewSnapshot, ScopeError> {
    use wire::world_map::*;
    let room: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/encounters/soulpit/source-room.json"
    )))?;
    let origin = match stage {
        1 => ActorPosition {
            x: 32375,
            y: 31158,
            floor: -8,
        },
        2 => ActorPosition {
            x: 32373,
            y: 31151,
            floor: -8,
        },
        _ => ActorPosition {
            x: 32373,
            y: 31158,
            floor: -8,
        },
    };
    let mut tiles = Vec::new();
    for tile in room["tiles"].as_array().ok_or("source Soulpit tiles")? {
        let position = tile["native_position"]
            .as_array()
            .ok_or("source position")?;
        let at = ActorPosition {
            x: i32::try_from(position[0].as_i64().ok_or("x")?)?,
            y: i32::try_from(position[1].as_i64().ok_or("y")?)?,
            floor: i16::try_from(position[2].as_i64().ok_or("z")?)?,
        };
        if !in_window(origin, at) {
            continue;
        }
        let mut items = Vec::new();
        for item in tile["items"].as_array().ok_or("source items")? {
            if item["depth"].as_u64() != Some(0) {
                continue;
            }
            let source = u32::try_from(item["server_item_id"].as_u64().ok_or("source item")?)?;
            let handle = match (at.x, at.y, source) {
                (32375, 31157, 47367) => Some(2_u64),
                (32375, 31153, 33017) => Some(3_u64),
                _ => None,
            };
            // The fixture's Item dictionary indexes source IDs; the client treats refs as opaque.
            items.push(MapItem {
                definition: MapDefinition::Item(source.try_into()?),
                count: 1,
                sub_type: 0,
                origin: match handle {
                    Some(value) => MapOrigin::Handle(value.try_into()?),
                    None => MapOrigin::DisplayOnly,
                },
                appearance_id: source.try_into()?,
            });
        }
        if items.is_empty() {
            continue;
        }
        tiles.push(MapTile {
            position: at,
            items,
            more: false,
            ground_speed: u16::try_from(tile["semantics"]["ground_speed"].as_u64().unwrap_or(0))?,
        });
    }
    tiles.sort_by_key(|tile| tile_order(&tile.position));
    Ok(WorldMapViewSnapshot {
        header: MapViewHeader {
            content_generation: [stage; 32],
            bundle_digest: [stage + 10; 32],
            reset_epoch: 1,
            origin,
            first_visible_floor: first_visible_floor_start(origin.floor),
        },
        tiles,
    })
}

type InitialDomainBody = (u32, u64, u32, Vec<u8>);

fn initial_body(stage: u8) -> Result<Vec<InitialDomainBody>, ScopeError> {
    let map = source_map(stage)?;
    let own = wire::world_spatial_entities::WorldSpatialEntity {
        display_name: None,
        kind: wire::world_spatial_entities::EntityKind::Player,
        entity: wire::world_spatial_entities::EntityRef {
            identity: id(50 + stage),
            generation: 1,
        },
        position: map.header.origin,
        detail: wire::world_spatial_entities::EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let entities = wire::world_spatial_entities::WorldSpatialEntitiesSnapshot {
        content_generation: map.header.content_generation,
        actor_position: map.header.origin,
        own_identity: own.entity.identity,
        entities: vec![own],
    };
    let item = |handle: u64, definition: u32| -> Result<oteryn_session::ItemEntry, ScopeError> {
        Ok(oteryn_session::ItemEntry {
            handle: handle.try_into()?,
            item_definition_ref: definition.try_into()?,
            count: 1.try_into()?,
            sub_type: 0,
        })
    };
    let inventory = oteryn_session::CharacterInventory {
        main_backpack: Some(item(5, 2854)?),
        entries: if stage == 1 {
            vec![item(1, 47381)?]
        } else {
            vec![]
        },
        ..Default::default()
    };
    Ok(vec![
        (
            1,
            1,
            2,
            wire::world_spatial_entities::encode_world_spatial_entities_snapshot(&entities)
                .map_err(|e| format!("{e:?}"))?,
        ),
        (
            9,
            1,
            1,
            wire::item_view::encode_character_inventory(&inventory)
                .map_err(|e| format!("{e:?}"))?,
        ),
        (
            17,
            1,
            1,
            wire::world_map::encode_world_map_snapshot(&map).map_err(|e| format!("{e:?}"))?,
        ),
    ])
}

async fn snapshot(peer: &mut DuplexStream, stage: u8) -> Result<(), ScopeError> {
    let mut body = initial_body(stage)?;
    if stage > 1 {
        body.insert(
            0,
            (
                99,
                1,
                1,
                wire::activity_scope::encode_activity_scope(&scope_value(stage))
                    .map_err(|e| format!("{e:?}"))?,
            ),
        );
    }
    let domains: Vec<_> = body
        .iter()
        .map(
            |(domain_id, revision, snapshot_type, payload)| wire::DomainSnapshot {
                domain_id: *domain_id,
                revision: *revision,
                snapshot_type: *snapshot_type,
                payload,
            },
        )
        .collect();
    for frame in wire::encode_single_chunk_snapshot(1, u64::from(stage), 1, &domains)? {
        oteryn_session::write_frame(peer, &frame).await?;
    }
    Ok(())
}

fn scope_value(stage: u8) -> wire::activity_scope::ActivityScopeSnapshot {
    use wire::activity_scope::*;
    ActivityScopeSnapshot {
        handoff: id(stage + 20),
        source_session: id(if stage == 2 { 1 } else { 10 }),
        session: id(if stage == 2 { 10 } else { 12 }),
        scope: if stage == 2 {
            ActivityScope::Instance {
                world: id(2),
                instance: id(60),
            }
        } else {
            ActivityScope::Channel {
                world: id(2),
                channel: id(3),
            }
        },
        scope_generation: 1,
        connection_generation: 1,
        lease_generation: u64::from(stage + 1),
        selection: CAPS.to_vec(),
        requires_ack: false,
    }
}

async fn barrier(peer: &mut DuplexStream, stage: u8) -> Result<(), ScopeError> {
    use wire::activity_scope::encode_activity_scope;
    let value = scope_value(stage);
    let payload = encode_activity_scope(&value).map_err(|e| format!("{e:?}"))?;
    let sequence = 3;
    for frame in wire::encode_single_chunk_snapshot(
        1,
        99,
        sequence,
        &[wire::DomainSnapshot {
            domain_id: 99,
            revision: 1,
            snapshot_type: 1,
            payload: &payload,
        }],
    )? {
        oteryn_session::write_frame(peer, &frame).await?;
    }
    snapshot(peer, stage).await
}

pub(super) async fn source_peer(mut peer: DuplexStream) -> Result<(), ScopeError> {
    oteryn_session::read_frame(&mut peer).await?;
    oteryn_session::write_frame(
        &mut peer,
        &wire::encode_server_accepted(&wire::ServerAcceptedValue {
            game_session_id: wire::GameSessionId::decode(&id(1))?,
            world_id: wire::WorldId::decode(&id(2))?,
            channel_id: wire::ChannelId::decode(&id(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 1,
            schema_revision: 1,
            selected_capabilities: CAPS,
        })?,
    )
    .await?;
    snapshot(&mut peer, 1).await?;
    for stage in [2, 3] {
        let frame = oteryn_session::read_frame(&mut peer).await?;
        let envelope = wire::decode_wire_envelope(&frame)?;
        let command = envelope.client_command(1)?;
        let expected_binding = id(10);
        assert_eq!(
            envelope.client_command_session_id()?,
            if stage == 2 {
                None
            } else {
                Some(expected_binding.as_slice())
            }
        );
        assert_eq!(
            (command.command_id, command.command_type),
            (1, wire::world_object::COMMAND_TYPE_USE_INTENT)
        );
        let intent = wire::world_object::decode_use_intent_target(CAPS, command.payload)
            .map_err(|e| format!("{e:?}"))?;
        if stage == 2 {
            assert_eq!(
                intent.target,
                wire::world_object::UseTarget::Item(1.try_into()?)
            );
            assert_eq!(
                intent.use_with_map_item,
                Some(wire::world_object::MapUseTarget::Item(2.try_into()?))
            );
        } else {
            assert_eq!(
                intent.target,
                wire::world_object::UseTarget::Item(3.try_into()?)
            );
        }
        let result =
            wire::world_object::encode_use_result(wire::world_object::UseDisposition::Committed)
                .map_err(|e| format!("{e:?}"))?;
        oteryn_session::write_frame(
            &mut peer,
            &wire::encode_command_result(1, 2, 1, wire::CommandStatus::Accepted, &result)?,
        )
        .await?;
        barrier(&mut peer, stage).await?;
        eprintln!("native peer accepted exact USE and published scope stage {stage}");
    }
    let mut rest = Vec::new();
    let _ = peer.read_to_end(&mut rest).await;
    Ok(())
}
