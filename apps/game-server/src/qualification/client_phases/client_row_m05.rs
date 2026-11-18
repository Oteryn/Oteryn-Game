//! Captures each bundle door transition from the native world-map stream.

use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::world_map::{self, MapOrigin, MapTile, WorldMapViewDelta, WorldMapViewSnapshot};
use oteryn_protocol_oteryn::world_object::UseDisposition;

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
                let appearance = crate::map::palette_appearance(
                    &manifest.palette[item.palette as usize].key,
                );
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

fn tile_position(at: TilePos) -> ActorPosition {
    ActorPosition { x: i32::from(at.x), y: i32::from(at.y), floor: -i16::from(at.floor) }
}

pub(super) struct DoorMap {
    revision: u64,
    snapshot: WorldMapViewSnapshot,
}

impl DoorMap {
    pub(super) fn read(client: &mut DevClientSession) -> TestResult<Self> {
        let snapshot = client.session_mut().gated_snapshot(world_map::STATE_DOMAIN_WORLD_MAP_VIEW)
            .ok_or("door map snapshot absent")?;
        let map = world_map::decode_world_map_snapshot(&snapshot.payload)
            .map_err(|error| format!("door map snapshot: {error:?}"))?;
        Ok(Self { revision: snapshot.revision, snapshot: map })
    }

    pub(super) fn observe(&mut self, event: &SessionEvent) {
        match event {
            SessionEvent::WorldMap(delta) => {
                self.revision = delta.new_revision;
                self.snapshot.header = delta.value.header;
                self.snapshot.tiles.retain(|tile| !delta.value.cleared.contains(&tile.position)
                    && !delta.value.tiles.iter().any(|updated| updated.position == tile.position));
                self.snapshot.tiles.extend(delta.value.tiles.iter().cloned());
            }
            SessionEvent::WorldMapSnapshot(snapshot) => {
                self.revision = snapshot.new_revision;
                self.snapshot = snapshot.value.clone();
            }
            _ => {}
        }
    }

    pub(super) fn state(&self, at: TilePos, ordinal: u8) -> Option<(u64, u16)> {
        door_state(&self.snapshot.tiles, at, ordinal)
    }
}

fn door_state(tiles: &[MapTile], at: TilePos, ordinal: u8) -> Option<(u64, u16)> {
    tiles.iter().find(|tile| tile.position == tile_position(at))?.items.iter().find_map(|item| {
        match item.origin {
            MapOrigin::BaseOrdinal { ordinal: seen, object_revision } if seen == ordinal =>
                Some((object_revision, item.appearance_id)),
            _ => None,
        }
    })
}

pub(super) async fn capture_transition(
    client: &mut DevClientSession,
    name: &str,
    map: &mut DoorMap,
    cutoff: u64,
    at: TilePos,
    expected: (u8, u64, u16),
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let (delta, base, mut snapshot): (oteryn_dev_client::AppliedDelta<WorldMapViewDelta>, _, _) = 'wait: loop {
        let mut matched = None;
        for event in client.take_events() {
            if let SessionEvent::WorldMap(delta) = &event
                && matched.is_none()
                && delta.server_sequence > cutoff
                && delta.value.tiles.iter().any(|tile| tile.items.iter().any(|item| {
                    let MapOrigin::BaseOrdinal { ordinal, object_revision } = item.origin else { return false; };
                    door_transition(tile.position == tile_position(at), (ordinal, object_revision, item.appearance_id), expected)
                }))
            {
                matched = Some((delta.clone(), map.revision, map.snapshot.clone()));
            }
            map.observe(&event);
        }
        if let Some(observed) = matched {
            break 'wait observed;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("{name} missing door transition at {at:?}: {expected:?}").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    };
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = Path::new(&path);
        let (ordinal, revision, appearance) = expected;
        snapshot.tiles.sort_by_key(|tile| world_map::tile_order(&tile.position));
        if base != delta.base_revision {
            return Err(format!("{name} intervening map updates changed the replay baseline").into());
        }
        std::fs::write(path.join(format!("{name}-snapshot.bin")), world_map::encode_world_map_snapshot(&snapshot)
            .map_err(|err| format!("{name} encode snapshot: {err:?}"))?)?;
        std::fs::write(path.join(format!("{name}-delta.bin")), world_map::encode_world_map_delta(&snapshot.header, &delta.value)
            .map_err(|err| format!("{name} encode delta: {err:?}"))?)?;
        std::fs::write(path.join(format!("{name}-revisions.txt")), format!("{base} {} {} {}", delta.base_revision, delta.new_revision, delta.server_sequence))?;
        std::fs::write(path.join(format!("{name}-expected.txt")), format!("{} {} {} {ordinal} {appearance} {revision}\n", at.x, at.y, at.floor))?;
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
    let mut map = DoorMap::read(&mut client)?;
    for (appearance, family) in [(1638, "basic"), (1629, "key")] {
        let from = tile(client.world_spatial().actor_position)?;
        let (door, path) = first_reachable(
            doors.iter().copied().filter(|door| door.appearance == appearance),
            |door| route(world, from, |at| {
                at.floor == door.at.floor
                    && at.x.abs_diff(door.at.x).max(at.y.abs_diff(door.at.y)) == 1
            }),
        ).ok_or_else(|| format!("M05 client capture has no route to {family} door"))?;
        walk_observing(&mut client, world, &path, |_, events| {
            for event in events {
                map.observe(&event);
            }
            Ok(())
        }).await?;
        let placement = placement(world, door)?;
        let opened_appearance = match appearance {
            1638 => 1639,
            1629 => 1630,
            _ => return Err("M05 unknown door pair".into()),
        };
        for (action, before, after) in [("open", appearance, opened_appearance), ("closed", opened_appearance, appearance)] {
            for event in client.take_events() { map.observe(&event); }
            let (revision, observed) = map.state(door.at, door.ordinal)
                .ok_or("M05 door absent from current map")?;
            if observed != before { return Err(format!("M05 {family} unexpected prior door appearance {observed}").into()); }
            let next = revision.checked_add(1).ok_or("M05 door revision exhausted")?;
            let cutoff = client.last_server_sequence();
            let result = client.use_object(&placement, revision).await?;
            if result.disposition != UseDisposition::Committed {
                return Err(format!("M05 {family} {action} observed {:?}", result.disposition).into());
            }
            capture_transition(&mut client, &format!("m05-{family}-{action}"), &mut map, cutoff, door.at, (door.ordinal, next, after)).await?;
        }
    }
    client_row_c08::require_logout(&mut client, Duration::from_secs(15)).await?;
    Ok(())
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

fn door_transition(location_matches: bool, seen: (u8, u64, u16), expected: (u8, u64, u16)) -> bool {
    location_matches && seen == expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn door_delta_must_name_target_ordinal_revision_and_state() {
        let wanted = (1, 4, 1639);
        assert!(!door_transition(false, wanted, wanted));
        assert!(!door_transition(true, (2, 4, 1639), wanted));
        assert!(!door_transition(true, (1, 3, 1639), wanted));
        assert!(!door_transition(true, (1, 4, 1638), wanted));
        assert!(door_transition(true, wanted, wanted));
    }
}
