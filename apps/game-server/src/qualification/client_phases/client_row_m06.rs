//! M06 client replay capture: a real key USE-WITH produces the domain-2 overlay payload consumed
//! by `Session::apply_state_delta` in `crates/session/src/domain_dispatch.rs`; the resulting entry
//! is exposed by `Session::world_object_overlay` in `crates/session/src/session_access.rs` and by
//! `DevClientSession::world_object_overlay` in `tools/dev-client/src/lib.rs`.

use super::*;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

#[derive(Clone, Copy)]
struct Door {
    at: TilePos,
    ordinal: u8,
}

fn key_doors(bytes: &[u8], world: &BundleWorld) -> TestResult<Vec<Door>> {
    let mut found = Vec::new();
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
                if appearance == Some(1628) && world.facts().doors().at(at) {
                    found.push(Door {
                        at,
                        ordinal: u8::try_from(ordinal).map_err(|_| {
                            oteryn_world_bundle::Error::Limit("M06 client ordinal".into())
                        })?,
                    });
                }
            }
        }
        Ok(())
    })?;
    Ok(found)
}

fn target(world: &BundleWorld, door: Door) -> TestResult<WorldObjectTarget> {
    let key =
        crate::map::view::placement_key(door.at, door.ordinal).ok_or("M06 client placement key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
}

fn capture(path: &Path, name: &str, bytes: &[u8]) -> TestResult {
    std::fs::write(path.join(name), bytes)?;

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
    let mut client = connect_world(clients, 0xc6).await?;
    let row_result: TestResult<_> = async {
        let from = tile(client.world_spatial().actor_position)?;
        let (door, path) = key_doors(&bytes, world)?
            .into_iter()
            .find_map(|door| {
                route(world, from, |at| {
                    at.floor == door.at.floor
                        && at.x.abs_diff(door.at.x).max(at.y.abs_diff(door.at.y)) == 1
                })
                .map(|path| (door, path))
            })
            .ok_or("M06 client has no reachable authored key door")?;
        walk(&mut client, world, &path).await?;
        let carried = client
            .inventory()
            .and_then(|inventory| inventory.entries.first())
            .copied()
            .ok_or("M06 client has no carried key candidate")?;
        let intent = UseIntent {
            target: UseTarget::Item(carried.handle),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(target(world, door)?)),
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(|error| format!("{error:?}"))?;
        let result = client
            .session_mut()
            .gated_command(
                oteryn_protocol_oteryn::world_object::CAPABILITY_ITEM_USE_V1,
                oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT,
                &payload,
            )
            .await?;
        let disposition = oteryn_protocol_oteryn::world_object::decode_use_result(&result.payload)
            .map_err(|error| format!("{error:?}"))?;
        if result.status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
            return Err(format!(
                "M06 client key action observed={:?}/{disposition:?}; expected Accepted/Committed",
                result.status
            )
            .into());
        }
        let delta = client
            .session_mut()
            .read_gated_delta(
                oteryn_protocol_oteryn::world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                1,
            )
            .await?;
        let entry =
            oteryn_protocol_oteryn::world_object::decode_world_object_overlay_delta(&delta.payload)
                .map_err(|error| format!("{error:?}"))?;
        if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let root = Path::new(&root);
            capture(root, "m06-overlay.bin", &delta.payload)?;
            capture(root, "m06-placement.bin", &entry.placement)?;
            capture(
                root,
                "m06-expected.txt",
                format!(
                    "{} {} {} {}\n{}\n",
                    delta.server_sequence,
                    delta.base_revision,
                    delta.new_revision,
                    entry.revision,
                    String::from_utf8_lossy(&entry.state)
                )
                .as_bytes(),
            )?;
        }
        Ok(())
    }
    .await;
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    let result = journey(clients).await;
    evidence(&format!(
        "e2e_all scenario=M06.client status={}{}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        result
            .err()
            .map(|error| format!(" evidence={}", error.to_string().replace(' ', "-")))
            .unwrap_or_default()
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_bundle_cannot_supply_a_door() -> TestResult {
        let world = row_helper_world()?;
        assert!(key_doors(&[], &world).is_err());
        assert!(key_doors(&[255; 8], &world).is_err());
        Ok(())
    }
}
