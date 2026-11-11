//! Captures M07's real WORLD_SPATIAL_ENTITIES payload for the session replay test.
use super::*;
use super::{
    row_connect_world as connect_world,
    row_finish as row_logout,
};
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::world_object::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    let result = capture(clients).await;
    if let Err(error) = result {
        evidence(&format!(
            "e2e_all scenario=M07.client status=FAIL evidence={error}"
        ));
    }
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    row_complete(async {
    let bundle = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bundle.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut spots = Vec::new();
    oteryn_world_bundle::bundle::visit(&bundle, |manifest, sector| {
        for map_tile in sector.tiles {
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
    let mut client = connect_world(clients, 0xa7).await?;
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let from = tile(client.world_spatial().actor_position)?;
    let (spot, ordinal, path) = spots
        .into_iter()
        .find_map(|(spot, ordinal)| {
            route(world, from, |at| {
                at.floor == spot.floor && at.x.abs_diff(spot.x).max(at.y.abs_diff(spot.y)) <= 1
            })
            .map(|path| (spot, ordinal, path))
        })
        .ok_or("M07 client capture has no reachable rope spot")?;
    walk(&mut client, world, &path).await?;
    let before = client.world_spatial().actor_position;
    let index = provisioning::item_index()?;
    let reference = provisioning::item_reference(&index, 3003)?;
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
    let result = client
        .session_mut()
        .gated_command(CAPABILITY_ITEM_USE_V1, COMMAND_TYPE_USE_INTENT, &payload)
        .await?;
    if result.status != CommandStatus::Accepted
        || decode_use_result(&result.payload)
            .map_err(|error| format!("M07 rope result: {error:?}"))?
            != UseDisposition::Committed
    {
        return Err("M07 rope ascent refused".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    let delta = loop {
        client.service_liveness(Duration::from_millis(25)).await?;
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
    };
    let bytes = oteryn_protocol_oteryn::world_spatial_entities::encode_world_spatial_entities_delta_with_item_handles(&delta.value).map_err(|error| format!("M07 spatial encode: {error:?}"))?;
    let expected = format!(
        "{} {} {}\n{} {} {}\n{} {} {} {}\n",
        before.x,
        before.y,
        before.floor,
        delta.value.actor_position.x,
        delta.value.actor_position.y,
        delta.value.actor_position.floor,
        delta.server_sequence,
        delta.base_revision,
        delta.new_revision,
        result.server_sequence
    );
    let path = std::path::Path::new(
        &std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("capture directory disappeared")?,
    )
    .to_path_buf();
    std::fs::write(path.join("m07-spatial.bin"), bytes)?;
    std::fs::write(path.join("m07-expected.txt"), expected)?;
    Ok(())

    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
    row_result

    }).await?
}
