//! Live basic and unlocked key door transitions, followed by raw map frame replay.

use super::*;
use oteryn_dev_client::native_session::{GatedSnapshot, replay_world_map_frames};
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::world_map::{MapOrigin, MapTile, STATE_DOMAIN_WORLD_MAP_VIEW};
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
        for tile in sector.tiles {
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                let appearance =
                    crate::map::palette_appearance(&manifest.palette[item.palette as usize].key);
                let at = TilePos {
                    x: tile.x,
                    y: tile.y,
                    floor: sector.floor,
                };
                if appearance.is_some_and(|appearance| matches!(appearance, 1638 | 1629))
                    && world.facts().doors().at(at)
                {
                    found.push(Door {
                        at,
                        ordinal: u8::try_from(ordinal).map_err(|_| {
                            oteryn_world_bundle::Error::Limit("M05 client door ordinal".into())
                        })?,
                        appearance: appearance.unwrap_or_default(),
                    });
                }
            }
        }
        Ok(())
    })?;
    Ok(found)
}

fn placement(world: &BundleWorld, door: Door) -> TestResult<Vec<u8>> {
    let key = crate::map::view::placement_key(door.at, door.ordinal)
        .ok_or("M05 client door placement")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(placement)
}

fn door_state(tiles: &[MapTile], door: Door) -> TestResult<(u16, u64)> {
    let position = ActorPosition {
        x: i32::from(door.at.x),
        y: i32::from(door.at.y),
        floor: -i16::from(door.at.floor),
    };
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

struct Capture {
    snapshot: GatedSnapshot,
    generation: u64,
    sequence: u64,
    observed_sequence: u64,
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
    let observed_sequence = loop {
        let mut changed = None;
        for event in client.take_events() {
            if let SessionEvent::WorldMap(delta) = event
                && delta.server_sequence > result.result_server_sequence
                && let Ok(observed) = door_state(&delta.value.tiles, door)
                && observed != before
            {
                if observed != expected {
                    return Err(
                        format!("M05 {label} observed {observed:?}, want {expected:?}").into(),
                    );
                }
                changed = Some(delta.server_sequence);
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
        if !frames.iter().any(|frame| {
            oteryn_protocol_oteryn::decode_wire_envelope(frame)
                .is_ok_and(|envelope| envelope.server_sequence() == capture.observed_sequence)
        }) {
            return Err("M05 replay missed the live transition sequence".into());
        }
        let result = replay_world_map_frames(
            capture.snapshot,
            capture.generation,
            capture.sequence,
            &frames,
        )?;
        let observed = door_state(&result.tiles, capture.door)?;
        if observed != capture.expected {
            return Err(format!(
                "M05 replay expected {:?}, observed {observed:?}",
                capture.expected
            )
            .into());
        }
    }
    evidence("M05_REPLAY status=PASS transitions=4");
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
    for (appearance, opened, family) in [(1638, 1639, "basic"), (1629, 1630, "key")] {
        let from = tile(client.world_spatial().actor_position)?;
        let candidates = doors
            .iter()
            .copied()
            .filter(|door| door.appearance == appearance);
        let approach = |at: TilePos, door: Door| {
            at.floor == door.at.floor
                && u32::from(at.x.abs_diff(door.at.x)) + u32::from(at.y.abs_diff(door.at.y)) == 1
        };
        let path = route(world, from, |at| {
            candidates.clone().any(|door| approach(at, door))
        })
        .ok_or_else(|| format!("M05 client capture has no route to {family} door"))?;
        for direction in path {
            let before = tile(client.world_spatial().actor_position)?;
            let landing =
                destination(world, before, direction).ok_or("M05 missing route landing")?;
            let result = client.step(direction).await?;
            let after = tile(client.world_spatial().actor_position)?;
            if result.disposition != StepDisposition::Moved || after != landing || after == before {
                return Err(format!("M05 {family} STEP {direction:?} from={before:?} expected={landing:?} actual={after:?} disposition={:?}", result.disposition).into());
            }
            client.take_events();
        }
        let at = tile(client.world_spatial().actor_position)?;
        let door = candidates
            .clone()
            .find(|door| approach(at, *door))
            .ok_or("M05 route missed door")?;
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
