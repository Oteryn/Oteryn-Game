//! CLIENT-I09 capture: rotate compiled, fixed chair 2358 through ordinary USE, then capture the
//! domain-17 map delta that reports chair 2359. The replay owner is `oteryn-session`:
//! `crates/session/src/map_transport.rs::apply` decodes and validates the delta and queues the
//! client-visible `SessionEvent::WorldMap`; `Session::take_events` exposes it to the native client.
use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::world_map::{self, MapDefinition, MapOrigin};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent,
};

const BEFORE: &str = "oteryn:item.tibia.i2358";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result =
        verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
            .await;
    match result {
        Ok(Ok(())) => evidence(
            "SEAM_EVIDENCE e2e_all scenario=I09.client status=PASS evidence=domain-17-furniture-rotation-replay-captured",
        ),
        Ok(Err(error)) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=I09.client status=FAIL evidence={}",
            error
                .to_string()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join("-")
        )),
        Err(_) => evidence(
            "SEAM_EVIDENCE e2e_all scenario=I09.client status=FAIL evidence=journey-timeout",
        ),
    }
}

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(Path::new(&directory).join(name))?;
        std::io::Write::write_all(&mut file, bytes)?;
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
    let at = TilePos {
        x: 32355,
        y: 32206,
        floor: -7,
    };
    if world.facts().house_id(at).is_some() {
        return Err("fixed chair 2358 belongs to a house".into());
    }
    let mut ordinal = None;
    oteryn_world_bundle::bundle::visit(&bytes, |manifest, sector| {
        for tile in sector.tiles {
            if (tile.x, tile.y, sector.floor) != (at.x, at.y, at.floor) {
                continue;
            }
            for (index, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                if manifest.palette[item.palette as usize].key == BEFORE {
                    if ordinal.is_some() {
                        return Err(oteryn_world_bundle::Error::Limit(
                            "ambiguous I09 chair".into(),
                        ));
                    }
                    ordinal = Some(u8::try_from(index).map_err(|_| {
                        oteryn_world_bundle::Error::Limit("I09 furniture ordinal".into())
                    })?);
                }
            }
        }
        Ok(())
    })?;
    let ordinal = ordinal.ok_or("compiled I09 tile omitted fixed chair 2358")?;
    let definitions = provisioning::item_index()?;
    let expected = provisioning::item_reference(&definitions, 2359)?;
    let mut client = connect_world(clients, 0xc9).await?;
    let adjacent = |cell: TilePos| {
        cell.floor == at.floor && cell.x.abs_diff(at.x).max(cell.y.abs_diff(at.y)) == 1
    };
    let path = route(
        world,
        tile(client.world_spatial().actor_position)?,
        adjacent,
    )
    .ok_or("no route to fixed chair 2358 at 32355,32206,7")?;
    walk(&mut client, world, &path).await?;
    if !adjacent(tile(client.world_spatial().actor_position)?) {
        return Err("arrived outside use distance of fixed chair 2358".into());
    }
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(
        &crate::map::view::placement_key(at, ordinal)
            .ok_or("chair placement")?
            .to_be_bytes(),
    );
    let intent = |revision| {
        encode_item_use_intent(&UseIntent {
            target: UseTarget::WorldObject(WorldObjectTarget {
                placement: placement.clone(),
                expected_revision: revision,
            }),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(|error| format!("I09 intent encode: {error:?}"))
    };
    let target = oteryn_protocol_oteryn::world_spatial::ActorPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    };
    let before = client
        .session_mut()
        .gated_snapshot(world_map::STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or("I09 predecessor map snapshot absent")?
        .clone();
    let before_map = world_map::decode_world_map_snapshot(&before.payload)
        .map_err(|error| format!("I09 predecessor decode: {error:?}"))?;
    let original = provisioning::item_reference(&definitions, 2358)?;
    if !before_map.tiles.iter().any(|tile| {
        tile.position == target
            && tile.items.iter().any(|item| {
                item.origin
                    == (MapOrigin::BaseOrdinal {
                        ordinal,
                        object_revision: 0,
                    })
                    && item.definition == MapDefinition::Item(original)
                    && item.appearance_id == 2358
            })
    }) {
        return Err("I09 predecessor omitted original chair at revision 0".into());
    }
    client.take_events();
    let reply = client.session_mut().world_context_use(&intent(0)?).await?;
    if reply.status != CommandStatus::Accepted
        || decode_use_result(&reply.payload)
            .map_err(|error| format!("I09 result decode: {error:?}"))?
            != UseDisposition::Committed
    {
        return Err(format!("rotation refused: {:?}", reply.status).into());
    }
    let rotated = MapOrigin::BaseOrdinal {
        ordinal,
        object_revision: 1,
    };
    let started = tokio::time::Instant::now();
    let delta = loop {
        let mut rotation = None;
        for event in client.take_events() {
            match event {
                SessionEvent::GatedSnapshotReplaced(update)
                    if update.value.domain_id == before.domain_id =>
                {
                    return Err("I09 map snapshot replaced before rotation delta".into());
                }
                SessionEvent::WorldMap(delta)
                    if delta.value.tiles.iter().any(|tile| tile.position == target) =>
                {
                    rotation = Some(delta);
                    break;
                }
                _ => {}
            }
        }
        if let Some(delta) = rotation {
            break delta;
        }
        // Match the full row's map observation budget. USE may publish on a later
        // visibility refresh, whose nominal cadence already exceeds 100 ms.
        if started.elapsed() >= Duration::from_secs(5) {
            return Err("rotation sent no map delta for the revised chair".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    };
    if delta.base_revision != before.revision {
        return Err("I09 predecessor revision does not match rotation delta".into());
    }
    let predecessor_sequence = delta
        .server_sequence
        .checked_sub(1)
        .ok_or("I09 rotation delta has no predecessor sequence")?;
    evidence(&format!(
        "I09 rotation_delta_ms={}",
        started.elapsed().as_millis()
    ));
    let tile = delta
        .value
        .tiles
        .iter()
        .find(|tile| tile.position == target)
        .ok_or("rotation delta omitted target tile")?;
    let item = tile
        .items
        .iter()
        .find(|item| item.origin == rotated)
        .ok_or("rotation delta omitted revised furniture")?;
    let MapDefinition::Item(definition) = item.definition else {
        return Err("rotated furniture was not an item".into());
    };
    if definition != expected || item.appearance_id != 2359 {
        return Err(format!(
            "rotation expected definition {expected} and appearance 2359, observed {item:?}"
        )
        .into());
    }
    let payload = oteryn_protocol_oteryn::world_map::encode_world_map_delta(
        &delta.value.header,
        &delta.value,
    )
    .map_err(|error| format!("I09 capture encode: {error:?}"))?;
    let stale = client.session_mut().world_context_use(&intent(0)?).await?;
    if decode_use_result(&stale.payload).map_err(|error| format!("I09 stale decode: {error:?}"))?
        != UseDisposition::StaleState
    {
        return Err("stale rotation was not refused".into());
    }
    assert_chair(&mut client, target, item)?;
    let session = client.join_snapshot().game_session_id;
    let position = client.world_spatial().actor_position;
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("I09 first logout refused: {logout:?}").into());
    }
    drop(client);
    let mut relogged = connect_world(clients, 0xca).await?;
    if relogged.join_snapshot().game_session_id == session
        || relogged.world_spatial().actor_position != position
    {
        return Err("I09 relog did not open a new session at the saved position".into());
    }
    assert_chair(&mut relogged, target, item)?;
    capture("i09-map-before.bin", &before.payload)?;
    capture(
        "i09-map-before.txt",
        format!(
            "{predecessor_sequence} {} {} {}\n",
            before.domain_id, before.snapshot_type, before.revision
        )
        .as_bytes(),
    )?;
    capture("i09-result.bin", &reply.payload)?;
    capture("i09-map-delta.bin", &payload)?;
    capture(
        "i09-expected.txt",
        format!(
            "{} {} {} {} {}\n",
            delta.base_revision,
            delta.new_revision,
            delta.server_sequence,
            definition,
            item.appearance_id
        )
        .as_bytes(),
    )?;
    Ok(())
}

fn assert_chair(
    client: &mut oteryn_dev_client::DevClientSession,
    target: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    expected: &world_map::MapItem,
) -> TestResult {
    let snapshot = client
        .session_mut()
        .world_map_snapshot()
        .ok_or("I09 verification map absent")?;
    if !snapshot
        .tiles
        .iter()
        .any(|tile| tile.position == target && tile.items.contains(expected))
    {
        return Err("I09 verification map omitted the exact rotated chair".into());
    }
    Ok(())
}
