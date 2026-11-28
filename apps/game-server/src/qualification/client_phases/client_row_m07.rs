//! Verifies the live M07 ascent, durable relog, and original-frame client replay.
use super::*;
use oteryn_dev_client::{EntityKind, SessionEvent};
use oteryn_protocol_oteryn::logout::LogoutResult;
use oteryn_protocol_oteryn::world_object::*;

// Authored fixture exercised by the independent live runs. Changing its landing
// cannot make the test select another spot.
const ROPE_SPOT: TilePos = TilePos {
    x: 32496,
    y: 32083,
    floor: -9,
};

fn capture_directory(parent: &std::path::Path) -> TestResult<std::path::PathBuf> {
    let output = std::process::Command::new("mktemp")
        .arg("-d")
        .arg("--")
        .arg(parent.join("m07.XXXXXXXX"))
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "M07 capture directory: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let path = String::from_utf8(output.stdout)?;
    Ok(path.trim_end_matches('\n').into())
}

fn require_accepted_logout(result: Option<LogoutResult>, phase: &str) -> TestResult {
    if result != Some(LogoutResult::Accepted) {
        return Err(format!("M07 {phase} logout refused: {result:?}").into());
    }
    Ok(())
}

fn unsafe_actor_distance(
    kind: EntityKind,
    distance: u16,
    clear_approach: bool,
    in_fight: bool,
) -> bool {
    (distance == 0
        && matches!(
            kind,
            EntityKind::Creature | EntityKind::Npc | EntityKind::Player
        ))
        || (kind == EntityKind::Creature && (clear_approach || in_fight) && distance <= 4)
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

async fn require_safe_position(
    client: &mut oteryn_dev_client::DevClientSession,
    clear_approach: bool,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        let in_fight = client
            .session_mut()
            .combat_state()
            .ok_or("M07 combat status unavailable")?
            .in_fight;
        let at = tile(client.world_spatial().actor_position)?;
        let entities = client
            .world_entities()
            .ok_or("M07 occupant observations unavailable")?;
        if let Some(occupant) = entities.others().find(|entity| {
            tile(entity.position).is_ok_and(|position| {
                position.floor == at.floor
                    && unsafe_actor_distance(
                        entity.kind,
                        position.x.abs_diff(at.x).max(position.y.abs_diff(at.y)),
                        clear_approach,
                        in_fight,
                    )
            })
        }) {
            return Err(format!(
                "M07 fixture unsafe at {at:?}: {:?} {:?} at {:?}",
                occupant.kind, occupant.entity, occupant.position
            )
            .into());
        }
        if !in_fight {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("M07 combat lock did not expire at the fixed position".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
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
    let capture_parent = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        .map_or_else(std::env::temp_dir, std::path::PathBuf::from);
    let capture_directory = capture_directory(&capture_parent)?;
    let capture_path = capture_directory.join("m07-wire.bin");
    evidence(&format!("M07_CAPTURE directory={capture_directory:?}"));
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
            if (map_tile.x, map_tile.y, sector.floor) != (ROPE_SPOT.x, ROPE_SPOT.y, ROPE_SPOT.floor)
            {
                continue;
            }
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
    character_prep::declare_seeded(
        "M07.client",
        "q_m07 level-100 knight and equipment",
        "M07 traversal survivability",
    );
    let mut preparation = connect_character(clients, 0xa7, 0).await?;
    let preparation_session = preparation.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(preparation_session.as_bytes()),
        "--profile",
        "knight".into(),
    )
    .await?;
    require_accepted_logout(
        preparation.session_mut().logout().await?.result,
        "preparation",
    )?;
    drop(preparation);
    let mut client = connect_character(clients, 0xa7, 0).await?;
    if client.join_snapshot().game_session_id == preparation_session {
        return Err("M07 preparation reused the original session".into());
    }
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
    let [(spot, ordinal)] = spots.as_slice() else {
        return Err("M07 fixed authored rope spot missing or ambiguous".into());
    };
    let (spot, ordinal) = (*spot, *ordinal);
    let path = route_work(&mut client, move || {
        route_avoiding(
            world,
            from,
            |at| at.floor == spot.floor && at.x.abs_diff(spot.x).max(at.y.abs_diff(spot.y)) <= 1,
            |at| !unsafe_tiles.contains(&at),
        )
        .ok_or_else(|| "M07 fixed authored rope spot is unreachable".to_owned())
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
    require_safe_position(&mut client, true).await?;
    let before = client.world_spatial().actor_position;
    evidence(&format!(
        "M07_FIXTURE placement={before:?} spot={spot:?} nearby_creatures=0 in_fight=false"
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
    require_safe_position(&mut client, false).await?;
    require_accepted_logout(client.session_mut().logout().await?.result, "ascent")?;
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
    require_safe_position(&mut relog, false).await?;
    require_accepted_logout(relog.session_mut().logout().await?.result, "relog")?;

    Ok(())
}

#[test]
fn m07_audit_capture_directory_is_private_and_fresh() -> TestResult {
    use std::os::unix::fs::PermissionsExt as _;
    let first = capture_directory(&std::env::temp_dir())?;
    let second = capture_directory(&std::env::temp_dir())?;
    let name = "m07-wire.bin";
    std::fs::write(first.join(name), b"stale")?;
    assert_eq!(
        std::fs::metadata(&second)?.permissions().mode() & 0o777,
        0o700
    );
    assert_ne!(first, second);
    assert!(!second.join(name).exists());
    std::fs::remove_dir_all(first)?;
    std::fs::remove_dir(second)?;
    Ok(())
}

#[test]
fn m07_audit_busy_and_occupied_landing_fail() {
    assert!(require_accepted_logout(Some(LogoutResult::Busy), "preparation").is_err());
    assert!(require_accepted_logout(Some(LogoutResult::Accepted), "preparation").is_ok());
    for kind in [EntityKind::Creature, EntityKind::Npc, EntityKind::Player] {
        assert!(unsafe_actor_distance(kind, 0, false, false));
    }
    assert!(!unsafe_actor_distance(
        EntityKind::Creature,
        3,
        false,
        false
    ));
    assert!(unsafe_actor_distance(EntityKind::Creature, 3, true, false));
}

#[test]
fn m07_audit_journey_keeps_first_attempt_and_fixed_spot() -> TestResult {
    let source = include_str!("client_row_m07.rs");
    let (_, safety) = source
        .split_once("async fn require_safe_position(")
        .ok_or("missing safety check")?;
    let (safety, _) = safety
        .split_once("pub(super) async fn run_packet(")
        .ok_or("missing safety check end")?;
    assert!(
        safety
            .find("unsafe_actor_distance(")
            .ok_or("missing occupancy check")?
            < safety
                .find("return Ok(());")
                .ok_or("missing safety success")?
    );
    let (_, journey) = source
        .split_once("async fn verify_ascent(")
        .ok_or("missing journey")?;
    let (journey, _) = journey
        .split_once("\n#[test]")
        .ok_or("missing journey end")?;
    assert!(journey.contains("!= (ROPE_SPOT.x, ROPE_SPOT.y, ROPE_SPOT.floor)"));
    assert!(journey.contains("let [(spot, ordinal)] = spots.as_slice()"));
    assert!(!journey.contains("world.enterable(landing)"));
    assert!(journey.contains("let step = client.step(direction).await?;"));
    assert!(!journey.contains("connection_reset"));
    assert!(!journey.contains("logout_retrying_busy"));
    assert!(!journey.contains("preparation.connect("));
    assert!(journey.contains(".map_or_else(std::env::temp_dir, std::path::PathBuf::from)"));
    assert!(journey.contains("require_safe_position(&mut client, false).await?;"));
    assert!(journey.contains("require_safe_position(&mut relog, false).await?;"));
    assert_eq!(
        journey
            .matches("preparation.session_mut().logout().await?")
            .count(),
        1
    );
    Ok(())
}
