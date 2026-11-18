use super::test_support::Stack;
use oteryn_renderer::TileCoord;
use oteryn_session::test_support::wire::{
    self, DomainSnapshot, world_map, world_spatial_entities as entities,
};

fn id(value: u8) -> [u8; 16] {
    let mut bytes = [value; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}

fn frames() -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
    use entities::*;
    let caps = [4, 6, 18, 286];
    let own = WorldSpatialEntity {
        motion: None,
        display_name: Some(
            wire::entity_names::EntityDisplayName::new("Ari")
                .map_err(|error| format!("{error:?}"))?,
        ),
        entity: EntityRef {
            identity: id(1),
            generation: 1,
        },
        kind: EntityKind::Player,
        position: wire::world_spatial::ActorPosition {
            x: 100,
            y: 200,
            floor: 7,
        },
        detail: EntityDetail::Actor {
            direction: wire::world_spatial::StepDirection::South,
            appearance_ref: 0,
            health_percent: 100,
        },
    };
    let neighbour = WorldSpatialEntity {
        kind: EntityKind::Creature,
        entity: EntityRef {
            identity: id(2),
            generation: 1,
        },
        position: wire::world_spatial::ActorPosition {
            x: 101,
            ..own.position
        },
        display_name: Some(
            wire::entity_names::EntityDisplayName::new("Rat")
                .map_err(|error| format!("{error:?}"))?,
        ),
        ..own
    };
    let spatial = encode_world_spatial_entities_snapshot_with_fields(
        &WorldSpatialEntitiesSnapshot {
            content_generation: [17; 32],
            actor_position: own.position,
            own_identity: own.entity.identity,
            entities: vec![own, neighbour],
        },
        EntityFields::selected(&caps),
        &EntityLights::new(),
    )
    .map_err(|error| format!("{error:?}"))?;
    let map = world_map::encode_world_map_snapshot(&world_map::WorldMapViewSnapshot {
        header: world_map::MapViewHeader {
            origin: wire::world_spatial::ActorPosition {
                floor: -7,
                ..own.position
            },
            first_visible_floor: -7,
            content_generation: [17; 32],
            bundle_digest: [3; 32],
            reset_epoch: 1,
        },
        tiles: Vec::new(),
    })
    .map_err(|error| format!("{error:?}"))?;
    let inventory = wire::item_view::encode_character_inventory(&Default::default())
        .map_err(|error| format!("{error:?}"))?;
    let mut frames = vec![wire::encode_server_accepted(&wire::ServerAcceptedValue {
        game_session_id: wire::GameSessionId::decode(&id(3))?,
        world_id: wire::WorldId::decode(&id(4))?,
        channel_id: wire::ChannelId::decode(&id(5))?,
        connection_generation: 1,
        current_server_sequence: 0,
        next_command_id: 1,
        schema_revision: 1,
        selected_capabilities: &caps,
    })?];
    frames.extend(wire::encode_single_chunk_snapshot(
        1,
        1,
        0,
        &[
            DomainSnapshot {
                domain_id: 1,
                revision: 1,
                snapshot_type: 2,
                payload: &spatial,
            },
            DomainSnapshot {
                domain_id: 9,
                revision: 1,
                snapshot_type: 1,
                payload: &inventory,
            },
            DomainSnapshot {
                domain_id: 17,
                revision: 1,
                snapshot_type: 1,
                payload: &map,
            },
        ],
    )?);
    Ok(frames)
}

#[test]
fn review_admitted_first_scene_projects_wire_entities_and_presentations()
-> Result<(), Box<dyn std::error::Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let run = || -> Result<(), Box<dyn std::error::Error>> {
                let stack = Stack::start_with_frames(Vec::new(), frames()?)?;
                let client = crate::ClientBootstrap::new()?.with_native_login(stack.config());
                let admission = client.request_gameplay_entry()?;
                let view = admission.play_view()?;
                assert_eq!(view.floor(), -7);
                let target = view.scene().target_at(TileCoord::new(101, 200));
                assert_eq!((target.is_some(), view.scene().name_count()), (true, 2));
                drop(admission);
                client.shutdown();
                stack.finish();
                Ok(())
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "admission scene test thread failed")?
        .map_err(Into::into)
}
