//! Verifies M07 through the live session and optionally exports its spatial delta for replay.
use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::world_object::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::observe(verify_ascent(clients)).await;
    let (status, details) = match result {
        Ok(()) => ("PASS", "rope-ascent-spatial-and-relogin".into()),
        Err(error) => ("FAIL", error.to_string()),
    };
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=M07.client status={status} seconds={:.3} evidence={details}",
        started.elapsed().as_secs_f64(),
    ));
}

async fn verify_ascent(clients: &SeamClients<'_>) -> TestResult {
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
    let (spot, ordinal, path) = route_work(&mut client, move || {
        spots
            .into_iter()
            // A route stays within 160 tiles and two floors of its origin.
            // The spot may be one tile beyond its reachable approach.
            .filter(|(spot, _)| {
                spot.x.abs_diff(from.x) <= 161
                    && spot.y.abs_diff(from.y) <= 161
                    && spot.floor.abs_diff(from.floor) <= 2
            })
            .find_map(|(spot, ordinal)| {
                route(world, from, |at| {
                    at.floor == spot.floor && at.x.abs_diff(spot.x).max(at.y.abs_diff(spot.y)) <= 1
                })
                .map(|path| (spot, ordinal, path))
            })
            .ok_or_else(|| "M07 client capture has no reachable rope spot".to_owned())
    })
    .await?;
    // The donor's first landing is directly south on the floor above the spot.
    let expected = ActorPosition {
        x: i32::from(spot.x),
        y: i32::from(spot.y) + 1,
        floor: -i16::from(spot.floor) - 1,
    };
    client.take_events();
    let mut path = VecDeque::from(path);
    let mut detours = 0;
    while let Some(direction) = path.pop_front() {
        let from = tile(client.world_spatial().actor_position)?;
        let planned = destination(world, from, direction).ok_or("M07 route edge missing")?;
        let step = client.step(direction).await?;
        if step.disposition == StepDisposition::Blocked
            && step.world_spatial_delta.is_none()
            && tile(client.world_spatial().actor_position)? == from
        {
            let identity = client
                .world_entities()
                .and_then(|entities| {
                    entities.iter().find(|entity| {
                        entity.kind == oteryn_dev_client::EntityKind::Creature
                            && tile(entity.position).ok() == Some(planned)
                    })
                })
                .map(|entity| entity.entity)
                .ok_or_else(|| format!("M07 blocked without a visible creature: from={from:?} target={planned:?} step={step:?}"))?;
            if detours == 8 {
                return Err("M07 creature detour limit reached".into());
            }
            detours += 1;
            client.service_liveness(Duration::from_millis(25)).await?;
            let current = client
                .world_entities()
                .and_then(|entities| entities.get(&identity))
                .ok_or("M07 lost sight of the creature blocking the approach")?;
            let current = tile(current.position)?;
            evidence(&format!(
                "M07_BLOCKED creature={identity:?} target={planned:?} current={current:?} step={step:?}"
            ));
            if current == planned {
                path = route_work(&mut client, move || {
                    route_around_occupant(
                        from,
                        planned,
                        &path,
                        |_| true,
                        |at, heading| destination(world, at, heading),
                    )
                    .ok_or_else(|| {
                        "M07 no route to the original approach around creature".to_owned()
                    })
                })
                .await?
                .into();
            } else {
                path.push_front(direction);
            }
            client.take_events();
            continue;
        }
        if step.disposition != StepDisposition::Moved
            || step.world_spatial_delta.is_none()
            || tile(client.world_spatial().actor_position)? != planned
        {
            return Err(format!(
                "M07 route step {direction:?} from {from:?} to {planned:?}: {step:?}"
            )
            .into());
        }
        verdict::Harness::global().progress("M07 approach");
        client.take_events();
    }
    let before = client.world_spatial().actor_position;
    if before == expected {
        return Err("M07 already at the expected rope landing before use".into());
    }
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
    let result = client.session_mut().world_context_use(&payload).await?;
    let disposition = decode_use_result(&result.payload)
        .map_err(|error| format!("M07 rope result: {error:?}"))?;
    if result.status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(format!(
            "M07 rope ascent refused: spot={spot:?} status={:?} disposition={disposition:?}",
            result.status
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    let delta = loop {
        if let Some(delta) = client
            .take_events()
            .into_iter()
            .find_map(|event| match event {
                SessionEvent::WorldSpatialEntities(delta)
                    if delta.value.actor_position != before =>
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
        client.service_liveness(Duration::from_millis(25)).await?;
    };
    if delta.value.actor_position != expected || client.world_spatial().actor_position != expected {
        return Err(format!(
            "M07 wrong landing: spot={spot:?} expected={expected:?} delta={:?} session={:?}",
            delta.value.actor_position,
            client.world_spatial().actor_position,
        )
        .into());
    }
    let original_session = client.join_snapshot().game_session_id;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("M07 ascent logout refused".into());
    }
    drop(client);
    let mut relog = connect_world(clients, 0xa7).await?;
    if relog.join_snapshot().game_session_id == original_session
        || relog.world_spatial().actor_position != expected
        || !relog.inventory().is_some_and(|inventory| {
            inventory
                .entries
                .iter()
                .any(|entry| entry.item_definition_ref == reference && entry.count == rope.count)
        })
    {
        return Err("M07 relog did not retain the landing and carried rope".into());
    }
    evidence(&format!(
        "M07_CHECK relog landing={expected:?} rope_count={}",
        rope.count
    ));
    if relog.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("M07 relog logout refused".into());
    }

    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
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
    let path = std::path::Path::new(&directory);
    use std::io::Write as _;
    std::fs::File::create_new(path.join("m07-spatial.bin"))?.write_all(&bytes)?;
    std::fs::File::create_new(path.join("m07-expected.txt"))?.write_all(expected.as_bytes())?;
    Ok(())
}
