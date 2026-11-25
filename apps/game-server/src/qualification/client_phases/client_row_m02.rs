//! M02 client capture: a far door use makes the server autowalk and retry the use.

use super::*;
use crate::map::door::AuthoredClosedDoor;
use oteryn_protocol_oteryn::world_object::UseDisposition;

#[derive(Clone, Copy)]
struct Door {
    at: TilePos,
    ordinal: u8,
}

fn distance(a: TilePos, b: TilePos) -> u16 {
    if a.floor == b.floor {
        a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
    } else {
        u16::MAX
    }
}

fn plain_doors(world: &BundleWorld) -> Vec<Door> {
    let doors = world.facts().doors();
    doors
        .keys()
        .filter(|&key| doors.authored_closed(key) == Some(AuthoredClosedDoor::Plain))
        .filter_map(|key| {
            let (at, ordinal) = crate::map::view::placement_of(key)?;
            Some(Door { at, ordinal })
        })
        .collect()
}

fn capture(payload: &[u8], expected: ActorPosition) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    let result = (|| -> std::io::Result<()> {
        let root = Path::new(&root);
        std::fs::write(root.join("m02-world-spatial.bin"), payload)?;
        std::fs::write(
            root.join("m02-expected.txt"),
            format!("{} {} {}\n", expected.x, expected.y, expected.floor),
        )
    })();
    if let Err(error) = result {
        evidence(&format!("M02 client capture write error: {error}"));
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xa2).await?;
    let from = tile(client.world_spatial().actor_position)?;
    let doors = plain_doors(world);
    let door_from = |at| doors.iter().find(|door| distance(at, door.at) == 3);
    let path = route(world, from, |at| door_from(at).is_some())
        .ok_or("M02 client capture has no reachable plain door")?;
    walk(&mut client, world, &path).await?;
    let stand = tile(client.world_spatial().actor_position)?;
    let door = door_from(stand).ok_or("M02 route missed its planned approach")?;
    client.take_events();
    let key = crate::map::view::placement_key(door.at, door.ordinal)
        .ok_or("M02 client capture placement")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let result = client.use_object(&placement, 0).await?;
    if result.disposition != UseDisposition::Committed {
        return Err(format!("M02 far use did not commit: {result:?}").into());
    }
    let expected = client.world_spatial().actor_position;
    if distance(tile(expected)?, door.at) > 1 {
        return Err("M02 far use did not walk into range".into());
    }
    let payload = oteryn_protocol_oteryn::world_spatial::encode_world_spatial(
        client.world_spatial(),
    );
    capture(&payload, expected);
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=M02.client status=PASS position={expected:?}"
    ));
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if let Err(error) = verdict::observe(journey(clients)).await {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=M02.client status=FAIL evidence={error}"
        ));
    }
}
