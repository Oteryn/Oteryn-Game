//! Captures M05's real `WORLD_OBJECT_OVERLAY` payloads for the session replay test.

use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectOverlayEntry, encode_world_object_overlay_delta,
};

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

async fn next_overlay(client: &mut DevClientSession) -> TestResult<WorldObjectOverlayEntry> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        if let Some(entry) = client.take_events().into_iter().find_map(|event| match event {
            SessionEvent::WorldObjectOverlay(delta) => Some(delta.value),
            _ => None,
        }) {
            return Ok(entry);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("M05 client capture timed out waiting for overlay".into());
        }
    }
}

fn capture(entries: &[(&str, &WorldObjectOverlayEntry)]) {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    let result: TestResult = (|| {
        let path = Path::new(&path);
        for (name, entry) in entries {
            std::fs::write(
                path.join(format!("m05-{name}.bin")),
                encode_world_object_overlay_delta(entry)
                    .map_err(|error| format!("M05 {name} encode: {error:?}"))?,
            )?;
        }
        std::fs::write(
            path.join("m05-expected.txt"),
            entries
                .iter()
                .map(|(_, entry)| format!("{}\n", String::from_utf8_lossy(&entry.state)))
                .collect::<String>(),
        )?;
        Ok(())
    })();
    if let Err(error) = result {
        evidence(&format!("M05_CLIENT_CAPTURE_WRITE_ERROR {error}"));
    }
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
    for (appearance, family) in [(1638, "basic"), (1629, "key")] {
        let door = doors
            .iter()
            .copied()
            .find(|door| door.appearance == appearance)
            .ok_or_else(|| format!("M05 client capture found no {family} door"))?;
        let from = tile(client.world_spatial().actor_position)?;
        let path = route(world, from, |at| {
            at.floor == door.at.floor
                && at.x.abs_diff(door.at.x).max(at.y.abs_diff(door.at.y)) == 1
        })
        .ok_or_else(|| format!("M05 client capture has no route to {family} door"))?;
        walk(&mut client, world, &path).await?;
        let placement = placement(world, door)?;
        let opened = client.use_object(&placement, 0).await?;
        if opened.disposition != UseDisposition::Committed {
            return Err(format!("M05 {family} open observed {:?}", opened.disposition).into());
        }
        let open = next_overlay(&mut client).await?;
        let closed = client.use_object(&placement, open.revision).await?;
        if closed.disposition != UseDisposition::Committed {
            return Err(format!("M05 {family} close observed {:?}", closed.disposition).into());
        }
        let closed = next_overlay(&mut client).await?;
        captured.push((format!("{family}-open"), open));
        captured.push((format!("{family}-closed"), closed));
    }
    let entries = captured
        .iter()
        .map(|(name, entry)| (name.as_str(), entry))
        .collect::<Vec<_>>();
    capture(&entries);
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
