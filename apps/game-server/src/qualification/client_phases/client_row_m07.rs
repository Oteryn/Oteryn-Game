//! Captures M07's real WORLD_SPATIAL_ENTITIES payload for the session replay test.
use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::world_object::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::observe(capture(clients)).await {
        Ok(()) => evidence("e2e_all scenario=M07.client status=PASS evidence=rope-upstairs-landing"),
        Err(error) => evidence(&format!("e2e_all scenario=M07.client status=FAIL evidence={error}")),
    }
}

fn rope_landing(spot: TilePos, before: ActorPosition, observed: ActorPosition) -> bool {
    before.floor.checked_sub(1) == Some(observed.floor)
        && observed == (ActorPosition { x: i32::from(spot.x), y: i32::from(spot.y), floor: -i16::from(spot.floor) })
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let bundle = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bundle.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut spots = Vec::new();
    oteryn_world_bundle::bundle::visit(&bundle, |manifest, sector| {
        for map_tile in sector.tiles {
            for (ordinal, item) in map_tile
                .items
                .iter()
                .filter(|item| item.depth == 0)
                .enumerate()
            {
                let Some(appearance) =
                    crate::map::palette_appearance(&manifest.palette[item.palette as usize].key)
                else {
                    continue;
                };
                if [
                    386, 421, 7762, 12202, 12936, 14238, 17238, 21965, 21966, 21967, 21968, 23363,
                    12935,
                ]
                .contains(&appearance)
                {
                    spots.push((
                        TilePos {
                            x: map_tile.x,
                            y: map_tile.y,
                            floor: sector.floor,
                        },
                        u8::try_from(ordinal).map_err(|_| {
                            oteryn_world_bundle::Error::Limit("M07 placement ordinal".into())
                        })?,
                    ));
                }
            }
        }
        Ok(())
    })?;
    let mut client = connect_world(clients, 0xa7).await?;
    let from = tile(client.world_spatial().actor_position)?;
    let (spot, ordinal, path) = spots
        .into_iter()
        .find_map(|(spot, ordinal)| {
            route(world, from, |at| {
                at.floor == spot.floor && at.x.abs_diff(spot.x).max(at.y.abs_diff(spot.y)) <= 1
            })
            .map(|path| (spot, ordinal, path))
        })
        .ok_or("M07 client capture has no reachable rope spot")?;
    walk(&mut client, world, &path).await?;
    let before = client.world_spatial().actor_position;
    let upstairs = spot.floor.checked_add(1).ok_or("M07 upper floor overflow")?;
    let landing = [(0, 1), (0, -1), (1, 0), (-1, 0), (-1, 1), (1, 1), (-1, -1), (1, -1)]
        .into_iter().filter_map(|(dx, dy)| Some(TilePos {
            x: spot.x.checked_add_signed(dx)?,
            y: spot.y.checked_add_signed(dy)?,
            floor: upstairs,
        }))
        .find(|at| world.enterable(*at) && world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| tile.walkable())
            && client.world_entities().is_some_and(|entities| !entities.others().any(|entity| {
                matches!(entity.kind, oteryn_dev_client::EntityKind::Creature | oteryn_dev_client::EntityKind::Player | oteryn_dev_client::EntityKind::Npc)
                    && tile(entity.position).ok() == Some(*at)
            })))
        .ok_or("M07 no free upstairs landing")?;
    let index = provisioning::item_index()?;
    let reference = provisioning::item_reference(&index, 3003)?;
    let rope = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == reference)
        })
        .copied()
        .ok_or("M07 client capture character has no rope")?;
    let key = crate::map::view::placement_key(spot, ordinal).ok_or("M07 placement key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(rope.handle),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: 0,
        })),
        use_with_position: None,
    })
    .map_err(|error| format!("M07 rope intent: {error:?}"))?;
    client.take_events();
    let result = client
        .session_mut()
        .world_context_use(&payload)
        .await?;
    if result.status != CommandStatus::Accepted
        || decode_use_result(&result.payload)
            .map_err(|error| format!("M07 rope result: {error:?}"))?
            != UseDisposition::Committed
    {
        return Err("M07 rope ascent refused".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    let delta = loop {
        client.service_liveness(Duration::from_millis(25)).await?;
        if let Some(delta) = client
            .take_events()
            .into_iter()
            .find_map(|event| match event {
                SessionEvent::WorldSpatialEntities(delta)
                    if delta.server_sequence > result.server_sequence && rope_landing(landing, before, delta.value.actor_position) =>
                {
                    Some(delta)
                }
                _ => None,
            })
        {
            break delta;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("M07 spatial delta deadline".into());
        }
    };
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else { return Ok(()); };
    let path = Path::new(&path);
    let bytes = oteryn_protocol_oteryn::world_spatial_entities::encode_world_spatial_entities_delta_with_item_handles(&delta.value).map_err(|error| format!("M07 spatial encode: {error:?}"))?;
    let expected = format!(
        "{} {} {}\n{} {} {}\n{} {} {} {}\n",
        before.x,
        before.y,
        before.floor,
        delta.value.actor_position.x,
        delta.value.actor_position.y,
        delta.value.actor_position.floor,
        delta.server_sequence,
        delta.base_revision,
        delta.new_revision,
        result.server_sequence
    );
    std::fs::write(path.join("m07-spatial.bin"), bytes)?;
    std::fs::write(path.join("m07-expected.txt"), expected)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rope_landing_rejects_same_floor_and_remote_tiles() {
        let spot = TilePos { x: 30, y: 41, floor: -6 };
        let before = ActorPosition { x: 30, y: 39, floor: 7 };
        let landing = ActorPosition { x: 30, y: 41, floor: 6 };
        assert!(!rope_landing(spot, before, ActorPosition { floor: 7, ..landing }));
        assert!(!rope_landing(spot, before, ActorPosition { y: 43, ..landing }));
        assert!(!rope_landing(spot, before, ActorPosition { y: 39, ..landing }));
        assert!(rope_landing(spot, before, landing));
    }
}
