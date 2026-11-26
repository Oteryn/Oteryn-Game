//! CLIENT-I09 capture: rotate compiled, fixed chair 2358 through ordinary USE, then capture the
//! domain-17 map delta that reports chair 2359. The replay owner is `oteryn-session`:
//! `crates/session/src/map_transport.rs::apply` decodes and validates the delta and queues the
//! client-visible `SessionEvent::WorldMap`; `Session::take_events` exposes it to the native client.
use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::world_map::{MapDefinition, MapOrigin};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget,
    WorldObjectTarget, decode_use_result, encode_item_use_intent,
};

const BEFORE: &str = "oteryn:item.tibia.i2358";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await;
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

fn capture(name: &str, bytes: &[u8]) {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&directory).join(name), bytes) {
        evidence(&format!(
            "I09_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
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
    let mut approaches = BTreeMap::new();
    oteryn_world_bundle::bundle::visit(&bytes, |manifest, sector| {
        for tile in sector.tiles {
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                if manifest.palette[item.palette as usize].key == BEFORE {
                    let chair = TilePos {
                        x: tile.x,
                        y: tile.y,
                        floor: sector.floor,
                    };
                    if world.facts().house_id(chair).is_some() {
                        continue;
                    }
                    let ordinal = u8::try_from(ordinal).map_err(|_| {
                        oteryn_world_bundle::Error::Limit("I09 furniture ordinal".into())
                    })?;
                    for x in chair.x.saturating_sub(1)..=chair.x.saturating_add(1) {
                        for y in chair.y.saturating_sub(1)..=chair.y.saturating_add(1) {
                            approaches
                                .entry(TilePos {
                                    x,
                                    y,
                                    floor: chair.floor,
                                })
                                .or_insert((chair, ordinal));
                        }
                    }
                }
            }
        }
        Ok(())
    })?;
    if approaches.is_empty() {
        return Err("no compiled non-house fixed chair 2358".into());
    }
    let mut client = connect_world(clients, 0xc9).await?;
    let path = route(
        world,
        tile(client.world_spatial().actor_position)?,
        |cell| approaches.contains_key(&cell),
    )
    .ok_or("no route to fixed chair 2358")?;
    walk(&mut client, world, &path).await?;
    let arrived = tile(client.world_spatial().actor_position)?;
    let &(at, ordinal) = approaches
        .get(&arrived)
        .ok_or("arrived outside use distance of a fixed chair 2358")?;
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
    client.take_events();
    let reply = client
        .session_mut()
        .world_context_use(&intent(0)?)
        .await?;
    if reply.status != CommandStatus::Accepted
        || decode_use_result(&reply.payload)
            .map_err(|error| format!("I09 result decode: {error:?}"))?
            != UseDisposition::Committed
    {
        return Err(format!("rotation refused: {:?}", reply.status).into());
    }
    capture("i09-result.bin", &reply.payload);
    let target = oteryn_protocol_oteryn::world_spatial::ActorPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    };
    let rotated = MapOrigin::BaseOrdinal {
        ordinal,
        object_revision: 1,
    };
    let started = tokio::time::Instant::now();
    let delta = loop {
        if let Some(delta) = client.take_events().into_iter().find_map(|event| match event {
            SessionEvent::WorldMap(delta)
                if delta.value.tiles.iter().any(|tile| {
                    tile.position == target && tile.items.iter().any(|item| item.origin == rotated)
                }) =>
            {
                Some(delta)
            }
            _ => None,
        }) {
            break delta;
        }
        // Match the full row's map observation budget. USE may publish on a later
        // visibility refresh, whose nominal cadence already exceeds 100 ms.
        if started.elapsed() >= Duration::from_secs(5) {
            return Err("rotation sent no map delta for the revised chair".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    };
    evidence(&format!("I09 rotation_delta_ms={}", started.elapsed().as_millis()));
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
    let payload = oteryn_protocol_oteryn::world_map::encode_world_map_delta(
        &delta.value.header,
        &delta.value,
    )
    .map_err(|error| format!("I09 capture encode: {error:?}"))?;
    capture("i09-map-delta.bin", &payload);
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
    );
    let stale = client
        .session_mut()
        .world_context_use(&intent(0)?)
        .await?;
    if decode_use_result(&stale.payload).map_err(|error| format!("I09 stale decode: {error:?}"))?
        != UseDisposition::StaleState
    {
        return Err("stale rotation was not refused".into());
    }
    Ok(())
}
