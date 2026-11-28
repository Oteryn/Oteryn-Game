//! Live basic and unlocked key door transitions, followed by raw map frame replay.

use super::*;
use oteryn_dev_client::native_session::{GatedSnapshot, replay_world_map_frames};
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::world_map::{
    MapOrigin, MapTile, STATE_DOMAIN_WORLD_MAP_VIEW, WorldMapViewDelta,
};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::path::PathBuf;

#[derive(Clone, Copy)]
struct Door {
    at: TilePos,
    ordinal: u8,
    appearance: u16,
}

fn doors(bytes: &[u8], world: &BundleWorld) -> TestResult<Vec<Door>> {
    let mut found = Vec::new();
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        for tile in sector.tiles.into_iter().filter(|tile| tile.house == 0) {
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                let appearance =
                    crate::map::palette_appearance(&manifest.palette[item.palette as usize].key);
                let at = TilePos {
                    x: tile.x,
                    y: tile.y,
                    floor: sector.floor,
                };
                if let Some(appearance @ (2177 | 1629)) = appearance {
                    let ordinal = u8::try_from(ordinal).map_err(|_| {
                        oteryn_world_bundle::Error::Limit("M05 client door ordinal".into())
                    })?;
                    if crate::map::view::placement_key(at, ordinal)
                        .is_some_and(|key| world.facts().doors().contains(key))
                    {
                        found.push(Door {
                            at,
                            ordinal,
                            appearance,
                        });
                    }
                }
            }
        }
        Ok(())
    })?;
    Ok(found)
}

fn fixed_door(doors: &[Door], at: TilePos, appearance: u16) -> Option<Door> {
    let mut matches = doors
        .iter()
        .copied()
        .filter(|door| door.at == at && door.appearance == appearance);
    let door = matches.next()?;
    matches.next().is_none().then_some(door)
}

fn placement(world: &BundleWorld, door: Door) -> TestResult<Vec<u8>> {
    let key = crate::map::view::placement_key(door.at, door.ordinal)
        .ok_or("M05 client door placement")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(placement)
}

fn door_state(tiles: &[MapTile], door: Door) -> TestResult<(u16, u64)> {
    let position = door_position(door);
    tiles
        .iter()
        .find(|tile| tile.position == position)
        .and_then(|tile| {
            tile.items.iter().find_map(|item| match item.origin {
                MapOrigin::BaseOrdinal {
                    ordinal,
                    object_revision,
                } if ordinal == door.ordinal => Some((item.appearance_id, object_revision)),
                _ => None,
            })
        })
        .ok_or_else(|| format!("M05 missing exact door at {position:?}").into())
}

fn door_position(door: Door) -> ActorPosition {
    ActorPosition {
        x: i32::from(door.at.x),
        y: i32::from(door.at.y),
        floor: -i16::from(door.at.floor),
    }
}
fn door_update(delta: &WorldMapViewDelta, door: Door) -> TestResult<Option<(u16, u64)>> {
    let at = door_position(door);
    if delta.cleared.contains(&at)
        || !oteryn_protocol_oteryn::world_map::in_window(delta.header.origin, at)
    {
        return Err(format!("M05 map cleared exact door tile {at:?}").into());
    }
    if delta.tiles.iter().any(|tile| tile.position == at) {
        return door_state(&delta.tiles, door).map(Some);
    }
    Ok(None)
}

struct Capture {
    snapshot: GatedSnapshot,
    generation: u64,
    sequence: u64,
    observed_sequence: u64,
    before: (u16, u64),
    files: Vec<PathBuf>,
    door: Door,
    expected: (u16, u64),
}

async fn toggle(
    client: &mut DevClientSession,
    world: &BundleWorld,
    door: Door,
    before: (u16, u64),
    appearance: u16,
    directory: &Path,
    label: &str,
) -> TestResult<Capture> {
    let session = client.session_mut();
    let snapshot = session
        .gated_snapshot(STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or("M05 missing map baseline")?
        .clone();
    let map = session
        .world_map_snapshot()
        .ok_or("M05 invalid map baseline")?;
    if door_state(&map.tiles, door)? != before || before.0 == appearance {
        return Err(format!("M05 {label} has wrong pre-state, expected {before:?}").into());
    }
    let expected = (
        appearance,
        before
            .1
            .checked_add(1)
            .ok_or("M05 door revision overflow")?,
    );
    let sequence = client.last_server_sequence();
    client.take_events();
    let generation = client.session_mut().start_world_map_capture();
    let result = client
        .use_object(&placement(world, door)?, before.1)
        .await?;
    if result.disposition != UseDisposition::Committed {
        return Err(format!("M05 {label} observed {:?}", result.disposition).into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut changed = None;
    let observed_sequence = loop {
        for event in client.take_events() {
            match event {
                SessionEvent::WorldMap(delta) => {
                    if let Some(state) = door_update(&delta.value, door)? {
                        let may_change = delta.server_sequence > result.result_server_sequence;
                        let valid = if changed.is_some() {
                            state == expected
                        } else {
                            state == before || (may_change && state == expected)
                        };
                        if !valid {
                            return Err(format!(
                                "M05 {label} unexpected state {state:?} at sequence {}",
                                delta.server_sequence
                            )
                            .into());
                        }
                        if state == expected {
                            changed.get_or_insert(delta.server_sequence);
                        }
                    }
                }
                SessionEvent::WorldMapSnapshot(_) => {
                    return Err("M05 unexpected map replacement during toggle".into());
                }
                _ => {}
            }
        }
        if let Some(changed) = changed {
            break changed;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("M05 {label} timed out waiting for exact map change").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    };
    verdict::Harness::global().progress("m05-map-transition");
    let frames = client.session_mut().finish_world_map_capture();
    let mut files = Vec::new();
    for (index, frame) in frames.iter().enumerate() {
        use std::io::Write;
        let file = directory.join(format!("{label}-{index}.bin"));
        std::fs::File::create_new(&file)?.write_all(frame)?;
        files.push(file);
    }
    evidence(&format!(
        "M05_MAP step={label} sequence={observed_sequence} before={before:?} after={expected:?}"
    ));
    Ok(Capture {
        snapshot,
        generation,
        sequence,
        observed_sequence,
        before,
        files,
        door,
        expected,
    })
}

fn replay(captures: Vec<Capture>) -> TestResult {
    if captures.len() != 4 {
        return Err("M05 replay requires all four live transitions".into());
    }
    for capture in captures {
        let frames = capture
            .files
            .iter()
            .map(std::fs::read)
            .collect::<Result<Vec<_>, _>>()?;
        let mut witnessed = false;
        let result = replay_world_map_frames(
            capture.snapshot,
            capture.generation,
            capture.sequence,
            &frames,
            |sequence, delta| match door_update(delta, capture.door) {
                Ok(Some(state)) if sequence == capture.observed_sequence => {
                    witnessed = state == capture.expected;
                    witnessed
                }
                Ok(Some(state)) if sequence < capture.observed_sequence => state == capture.before,
                Ok(Some(state)) => state == capture.expected,
                Ok(None) => sequence != capture.observed_sequence,
                Err(_) => false,
            },
        )?;
        if !witnessed {
            return Err("M05 replay missed the live transition sequence".into());
        }
        let observed = door_state(&result.tiles, capture.door)?;
        if observed != capture.expected {
            return Err(format!(
                "M05 replay expected {:?}, observed {observed:?}",
                capture.expected
            )
            .into());
        }
    }
    verdict::Harness::global().progress("m05-replay");
    evidence("M05_REPLAY status=PASS transitions=4");
    Ok(())
}

fn require_empty_door(client: &DevClientSession, door: Door) -> TestResult {
    let entities = client.world_entities().ok_or("M05 entity view missing")?;
    if let Some(entity) = entities.iter().find(|entity| {
        entity.position == door_position(door)
            && matches!(
                entity.kind,
                oteryn_dev_client::EntityKind::Player
                    | oteryn_dev_client::EntityKind::Creature
                    | oteryn_dev_client::EntityKind::Npc
            )
    }) {
        return Err(format!(
            "M05 door collision cannot be isolated: occupant {:?} at {:?}",
            entity.entity, entity.position
        )
        .into());
    }
    Ok(())
}

fn check_step(
    from: TilePos,
    expected: TilePos,
    disposition: StepDisposition,
    actual: TilePos,
    observed: StepDisposition,
) -> TestResult {
    if observed != disposition || actual != expected {
        return Err(format!("M05 collision from={from:?} expected={disposition:?}/{expected:?} observed={observed:?}/{actual:?}").into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let doors = doors(&bytes, world)?;
    let mut client = connect_world(clients, 0xc5).await?;
    let mut captured = Vec::new();
    for (appearance, opened, family, target, approach) in [
        (
            2177,
            2178,
            "basic",
            TilePos {
                x: 32402,
                y: 32235,
                floor: -7,
            },
            TilePos {
                x: 32401,
                y: 32235,
                floor: -7,
            },
        ),
        (
            1629,
            1630,
            "key",
            TilePos {
                x: 32379,
                y: 32198,
                floor: -7,
            },
            TilePos {
                x: 32378,
                y: 32198,
                floor: -7,
            },
        ),
    ] {
        let enter = StepDirection::East;
        let from = tile(client.world_spatial().actor_position)?;
        let door = fixed_door(&doors, target, appearance)
            .ok_or_else(|| format!("M05 authored {family} door missing at {target:?}"))?;
        let path = route(world, from, |at| at == approach)
            .ok_or_else(|| format!("M05 no route to fixed {family} approach {approach:?}"))?;
        for direction in path {
            let before = tile(client.world_spatial().actor_position)?;
            let landing =
                destination(world, before, direction).ok_or("M05 missing route landing")?;
            let result = client.step(direction).await?;
            let after = tile(client.world_spatial().actor_position)?;
            if result.disposition != StepDisposition::Moved || after != landing || after == before {
                return Err(format!("M05 {family} STEP {direction:?} from={before:?} expected={landing:?} actual={after:?} disposition={:?}", result.disposition).into());
            }
            verdict::Harness::global().progress("m05-verified-walk");
            client.take_events();
        }
        let at = tile(client.world_spatial().actor_position)?;
        if at != approach {
            return Err("M05 route missed fixed approach".into());
        }
        require_empty_door(&client, door)?;
        let blocked_before = client.step(enter).await?;
        check_step(
            at,
            at,
            StepDisposition::Blocked,
            tile(client.world_spatial().actor_position)?,
            blocked_before.disposition,
        )?;
        let id = *client.join_snapshot().game_session_id.as_bytes();
        let directory = std::env::var_os("E2E_ALL_CAPTURE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!("m05-{}-{family}", uuid_text(&id)));
        std::fs::create_dir(&directory)?;
        let open = toggle(
            &mut client,
            world,
            door,
            (appearance, 0),
            opened,
            &directory,
            "open",
        )
        .await?;
        require_empty_door(&client, door)?;
        let crossing = client.step(enter).await?;
        check_step(
            at,
            target,
            StepDisposition::Moved,
            tile(client.world_spatial().actor_position)?,
            crossing.disposition,
        )?;
        let leaving = client.step(StepDirection::West).await?;
        check_step(
            target,
            at,
            StepDisposition::Moved,
            tile(client.world_spatial().actor_position)?,
            leaving.disposition,
        )?;
        client.service_liveness(Duration::from_millis(250)).await?;
        let close = toggle(
            &mut client,
            world,
            door,
            open.expected,
            appearance,
            &directory,
            "close",
        )
        .await?;
        require_empty_door(&client, door)?;
        let blocked_after = client.step(enter).await?;
        check_step(
            at,
            at,
            StepDisposition::Blocked,
            tile(client.world_spatial().actor_position)?,
            blocked_after.disposition,
        )?;
        evidence(&format!(
            "M05_COLLISION family={family} before=Blocked open=Moved exit=Moved closed=Blocked"
        ));
        let logout = client.session_mut().logout().await?;
        if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!("M05 {family} logout observed {:?}", logout.result).into());
        }
        drop(client);
        client = connect_world(clients, 0xc6).await?;
        let map = client
            .session_mut()
            .world_map_snapshot()
            .ok_or("M05 relog map absent")?;
        if client.join_snapshot().game_session_id.as_bytes() == &id
            || tile(client.world_spatial().actor_position)? != at
            || door_state(&map.tiles, door)? != close.expected
        {
            return Err(
                format!("M05 {family} relog did not retain position and door revision").into(),
            );
        }
        evidence(&format!(
            "M05_RELOG family={family} position={at:?} door={:?}",
            close.expected
        ));
        verdict::Harness::global().progress("m05-verified-relog");
        captured.extend([open, close]);
    }
    replay(captured)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::observe(journey(clients)).await;
    evidence(&format!(
        "e2e_all scenario=M05.client status={} seconds={:.3}{}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64(),
        result
            .err()
            .map(|error| format!(" evidence={}", error.to_string().replace(' ', "-")))
            .unwrap_or_default()
    ));
}

#[test]
fn m05_detects_missing_door_and_false_collision() {
    use oteryn_protocol_oteryn::world_map::MapViewHeader;
    let at = TilePos {
        x: 20,
        y: 20,
        floor: -7,
    };
    let door = Door {
        at,
        ordinal: 1,
        appearance: 2177,
    };
    let position = door_position(door);
    let mut delta = WorldMapViewDelta {
        header: MapViewHeader {
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 0,
            origin: position,
            first_visible_floor: 0,
        },
        tiles: Vec::new(),
        cleared: vec![position],
    };
    assert!(door_update(&delta, door).is_err());
    delta.cleared.clear();
    delta.tiles.push(MapTile {
        position,
        items: Vec::new(),
        more: false,
        ground_speed: 150,
    });
    assert!(door_update(&delta, door).is_err());
    let approach = TilePos { x: 19, ..at };
    assert!(fixed_door(&[door], approach, door.appearance).is_none());
    assert!(fixed_door(&[door, door], at, door.appearance).is_none());
    assert!(
        check_step(
            approach,
            approach,
            StepDisposition::Blocked,
            at,
            StepDisposition::Moved
        )
        .is_err()
    );
    assert!(
        check_step(
            approach,
            at,
            StepDisposition::Moved,
            approach,
            StepDisposition::Blocked
        )
        .is_err()
    );
}
