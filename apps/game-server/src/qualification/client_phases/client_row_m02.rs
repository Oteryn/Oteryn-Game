//! M02 client capture: a far door use makes the server autowalk and retry the use.

use super::*;
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

fn plain_doors(bytes: &[u8], world: &BundleWorld) -> TestResult<Vec<Door>> {
    let mut doors = Vec::new();
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        for tile in sector.tiles {
            let at = TilePos {
                x: tile.x,
                y: tile.y,
                floor: sector.floor,
            };
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                let appearance =
                    crate::map::palette_appearance(&manifest.palette[item.palette as usize].key);
                if appearance == Some(1638) && world.facts().doors().at(at) {
                    doors.push(Door {
                        at,
                        ordinal: u8::try_from(ordinal).map_err(|_| {
                            oteryn_world_bundle::Error::Limit("M02 door ordinal".into())
                        })?,
                    });
                }
            }
        }
        Ok(())
    })?;
    Ok(doors)
}

fn capture(payload: &[u8], expected: ActorPosition) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let result = (|| -> std::io::Result<()> {
        let root = Path::new(&root);
        std::fs::write(root.join("m02-world-spatial.bin"), payload)?;
        std::fs::write(
            root.join("m02-expected.txt"),
            format!("{} {} {}\n", expected.x, expected.y, expected.floor),
        )
    })();
    result?;

    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xa2).await?;
    let row_result: TestResult<_> = async {
        let from = tile(client.world_spatial().actor_position)?;
        let (door, path) = plain_doors(&bytes, world)?
            .into_iter()
            .filter_map(|door| {
                route(world, from, |at| distance(at, door.at) == 3).map(|path| (door, path))
            })
            .min_by_key(|(_, path)| path.len())
            .ok_or("M02 client capture has no reachable plain door")?;
        walk(&mut client, world, &path).await?;
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
        let payload =
            oteryn_protocol_oteryn::world_spatial::encode_world_spatial(client.world_spatial());
        capture(&payload, expected)?;
        let verdict =
            format!("SEAM_EVIDENCE e2e_all scenario=M02.client status=PASS position={expected:?}");
        Ok(verdict)
    }
    .await;
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if let Err(error) = journey(clients).await.inspect(|line| evidence(line)) {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=M02.client status=FAIL evidence={error}"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_handles_boundaries_and_floors() {
        let a = TilePos {
            x: 0,
            y: 0,
            floor: 7,
        };
        assert_eq!(distance(a, a), 0);
        assert_eq!(distance(a, TilePos { x: u16::MAX, ..a }), u16::MAX);
        assert_eq!(distance(a, TilePos { floor: 8, ..a }), u16::MAX);
    }
}
