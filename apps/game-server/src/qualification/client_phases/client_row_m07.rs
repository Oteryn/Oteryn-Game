//! Verifies the live M07 ascent, durable relog, and original-frame client replay.
use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::world_object::*;

fn connection_reset(mut error: &(dyn std::error::Error + 'static)) -> bool {
    loop {
        let io =
            error
                .downcast_ref::<std::io::Error>()
                .or_else(
                    || match error.downcast_ref::<oteryn_dev_client::SessionError>() {
                        Some(oteryn_dev_client::SessionError::Io(io)) => Some(io),
                        _ => None,
                    },
                );
        if let Some(io) = io {
            return io.kind() == std::io::ErrorKind::ConnectionReset;
        }
        match error.source() {
            Some(source) => error = source,
            None => return false,
        }
    }
}

#[test]
fn m07_reconnect_classifies_only_typed_connection_resets() {
    use oteryn_dev_client::{DevClientError, SessionError};
    use std::io::{Error, ErrorKind};

    for kind in [
        ErrorKind::ConnectionReset,
        ErrorKind::BrokenPipe,
        ErrorKind::TimedOut,
    ] {
        let error = DevClientError::Context {
            phase: "command",
            elapsed: Duration::ZERO,
            last_frame: String::new(),
            source: Box::new(DevClientError::Session(SessionError::Io(Error::from(kind)))),
        };
        assert_eq!(connection_reset(&error), kind == ErrorKind::ConnectionReset);
    }
    assert!(!connection_reset(&Error::other("Connection reset by peer")));
}

fn rope_units(
    client: &oteryn_dev_client::DevClientSession,
    reference: std::num::NonZeroU32,
) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("M07 inventory unavailable")?
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == reference)
        .map(|entry| u64::from(entry.count.get()))
        .sum())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::observe(verify_ascent(clients)).await;
    let (status, details) = match result {
        Ok(()) => ("PASS", "rope-ascent-spatial-relogin-and-raw-replay".into()),
        Err(error) => ("FAIL", error.to_string()),
    };
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=M07.client status={status} seconds={:.3} evidence={details}",
        started.elapsed().as_secs_f64(),
    ));
}

async fn verify_ascent(clients: &SeamClients<'_>) -> TestResult {
    let capture_directory = required("E2E_ALL_CAPTURE_DIR")?;
    let capture_path = std::path::Path::new(&capture_directory).join("m07-wire.bin");
    let bundle = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bundle.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut spots = Vec::new();
    let layout = oteryn_world_bundle::bundle::visit(&bundle, |manifest, sector| {
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
    let identity =
        character_prep::dedicated_character(clients, "NODE_BOOT_M07_CLIENT_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    let preparation = character_prep::Seeded {
        profile: "knight",
        reason: "M07 traversal survivability",
    };
    character_prep::declare_seeded(
        "M07.client",
        "q_m07 level-100 knight and equipment",
        preparation.reason,
    );
    let mut client = preparation.connect(clients, 0xa7, 0).await?;
    let (health, mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "knight",
        100,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("M07 knight stats unavailable")?;
    if full_e2ef7::player_level(&mut client).await? != 100
        || !client
            .actor_vitals()
            .is_some_and(|vitals| vitals.max_health == health && vitals.max_mana == mana)
    {
        return Err("M07 seeded level or derived vitals mismatch".into());
    }
    client_hotkey_equip::hotkey_equip(&mut client).await?;
    let equipment = client
        .inventory()
        .ok_or("M07 inventory unavailable")?
        .equipment
        .clone();
    if equipment.is_empty() {
        return Err("M07 fixture has no equipped item".into());
    }
    evidence(&format!(
        "M07_FIXTURE account=q_m07 level=100 max_health={health} max_mana={mana} equipment={equipment:?}"
    ));
    let unsafe_tiles: std::collections::BTreeSet<_> = layout
        .spawns
        .sources
        .iter()
        .flat_map(|source| {
            source.points.iter().map(|point| TilePos {
                x: point.x,
                y: point.y,
                floor: source.floor,
            })
        })
        .collect();
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
                let landing = TilePos { x: spot.x, y: spot.y.checked_add(1)?, floor: spot.floor.checked_add(1)? };
                if unsafe_tiles.contains(&spot) || unsafe_tiles.contains(&landing)
                    || !world.base().tile(landing.x, landing.y, landing.floor).is_some_and(|tile| tile.walkable())
                    || !world.enterable(landing)
                {
                    return None;
                }
                route_avoiding(world, from, |at| {
                    at.floor == spot.floor && at.x.abs_diff(spot.x).max(at.y.abs_diff(spot.y)) <= 1
                }, |at| !unsafe_tiles.contains(&at))
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
    let mut reconnected = false;
    while let Some(direction) = path.pop_front() {
        let from = tile(client.world_spatial().actor_position)?;
        let planned = destination(world, from, direction).ok_or("M07 route edge missing")?;
        let step = match client.step(direction).await {
            Ok(step) => step,
            Err(error) if !reconnected && connection_reset(&error) => {
                reconnected = true;
                let lost = client.join_snapshot().game_session_id;
                evidence(&format!(
                    "M07_RESET retry=1 phase=approach from={from:?} error={error}"
                ));
                drop(client);
                client = tokio::time::timeout(
                    Duration::from_secs(180),
                    connect_character(clients, 0xa7, 0),
                )
                .await??;
                if client.join_snapshot().game_session_id == lost
                    || tile(client.world_spatial().actor_position)? != from
                {
                    return Err("M07 reset did not read back the exact pre-step position on a fresh session".into());
                }
                path.push_front(direction);
                continue;
            }
            Err(error) => return Err(error.into()),
        };
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
    if !client.world_entities().is_some_and(|entities| {
        entities
            .others()
            .all(|entity| entity.kind != oteryn_dev_client::EntityKind::Creature)
    }) {
        return Err("M07 planned fixture approach is in view of a creature".into());
    }
    evidence(&format!(
        "M07_FIXTURE placement={before:?} spot={spot:?} visible_creatures=0"
    ));
    if before == expected {
        return Err("M07 already at the expected rope landing before use".into());
    }
    let index = provisioning::item_index()?;
    let reference = provisioning::item_reference(&index, 3003)?;
    let before_rope_units = rope_units(&client, reference)?;
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
    let checkpoint = client.session_mut().begin_spatial_capture()?;
    verdict::Harness::global().progress("M07 rope use");
    evidence(&format!(
        "M07_USE spot={spot:?} from={before:?} expected={expected:?}"
    ));
    let result = match client.session_mut().world_context_use(&payload).await {
        Ok(result) => result,
        Err(error) if !reconnected && connection_reset(&error) => {
            evidence(&format!(
                "M07_RESET retry=1 phase=rope-use from={before:?} error={error}"
            ));
            drop(client);
            let fresh = tokio::time::timeout(
                Duration::from_secs(180),
                connect_character(clients, 0xa7, 0),
            )
            .await??;
            return Err(format!(
                "M07 rope result lost after reset: fresh position={:?}; original command outcome and frame unverified",
                fresh.world_spatial().actor_position,
            )
            .into());
        }
        Err(error) => return Err(error.into()),
    };
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
    if rope_units(&client, reference)? != before_rope_units {
        return Err("M07 rope use changed carried rope quantity".into());
    }
    let recording = client
        .session_mut()
        .finish_spatial_capture()
        .ok_or("M07 raw capture unavailable or overflowed")?;
    let original_session = client.join_snapshot().game_session_id;
    evidence(&format!(
        "M07_WIRE session={} command={} result_sequence={} move_sequence={}",
        uuid_text(original_session.as_bytes()),
        result.command_id,
        result.server_sequence,
        delta.server_sequence,
    ));
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("M07 ascent logout refused".into());
    }
    drop(client);
    let mut relog = connect_character(clients, 0xa7, 0).await?;
    if relog.join_snapshot().game_session_id == original_session
        || relog.world_spatial().actor_position != expected
        || rope_units(&relog, reference)? != before_rope_units
    {
        return Err("M07 relog did not retain the landing and carried rope".into());
    }
    evidence(&format!(
        "M07_CHECK relog landing={expected:?} rope_count={}",
        before_rope_units
    ));
    if relog.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("M07 relog logout refused".into());
    }

    use std::io::Write as _;
    std::fs::File::create_new(&capture_path)?.write_all(&recording)?;
    checkpoint.verify_move(
        &std::fs::read(&capture_path)?,
        expected,
        result.command_id,
        result.server_sequence,
        delta.server_sequence,
    )?;
    evidence("M07_REPLAY status=PASS evidence=original-server-frames");
    Ok(())
}
