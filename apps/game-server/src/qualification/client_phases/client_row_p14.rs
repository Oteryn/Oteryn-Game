//! P14 client replay capture: the real OFFLINE_TRAINING replies for an unknown handle (refused)
//! and for a walked-to authored statue (disposition 3). The consumer is `apply` in
//! `apps/client/src/features/0054-training/protocol.rs`.
use super::client_row_p15::training_reply;
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::item_view::ItemHandle;
use oteryn_protocol_oteryn::loop_training::{
    CAPABILITY_TRAINING_V1, COMMAND_TYPE_OFFLINE_TRAINING, TrainingResult,
};
use oteryn_protocol_oteryn::world_map::{MapDefinition, MapOrigin};
use std::num::{NonZeroU32, NonZeroU64};

const STATUES: [&str; 6] = [
    "oteryn:item.tibia.i16198",
    "oteryn:item.tibia.i16199",
    "oteryn:item.tibia.i16200",
    "oteryn:item.tibia.i16201",
    "oteryn:item.tibia.i16202",
    "oteryn:item.tibia.i50296",
];

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

fn adjacent(a: TilePos, b: TilePos) -> bool {
    a.floor == b.floor && a.x.abs_diff(b.x).max(a.y.abs_diff(b.y)) <= 1
}

fn statue_positions(bytes: &[u8], references: &[NonZeroU32]) -> TestResult<Vec<TilePos>> {
    let mut positions = Vec::new();
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        for tile in sector.tiles {
            if tile.items.iter().any(|item| {
                references.iter().any(|reference| {
                    reference.get() == manifest.palette[item.palette as usize].id + 1
                })
            }) {
                positions.push(TilePos {
                    x: tile.x,
                    y: tile.y,
                    floor: sector.floor,
                });
            }
        }
        Ok(())
    })?;
    Ok(positions)
}

fn map_statue_handle(
    client: &mut DevClientSession,
    references: &[NonZeroU32],
) -> TestResult<ItemHandle> {
    let snapshot = client
        .session_mut()
        .world_map_snapshot()
        .ok_or("missing map snapshot")?;
    let player = tile(client.world_spatial().actor_position)?;
    snapshot
        .tiles
        .iter()
        .filter(|map_tile| tile(map_tile.position).is_ok_and(|at| adjacent(player, at)))
        .flat_map(|map_tile| &map_tile.items)
        .find_map(|item| match (item.definition, item.origin) {
            (MapDefinition::Item(reference), MapOrigin::Handle(handle))
                if references.contains(&reference) =>
            {
                Some(handle)
            }
            _ => None,
        })
        .ok_or_else(|| "authored statue without a live handle".into())
}

async fn offline(
    client: &mut DevClientSession,
    target: ItemHandle,
) -> TestResult<(Vec<u8>, TrainingResult)> {
    let mut payload = vec![8];
    let mut value = target.get();
    while value >= 128 {
        payload.push((value as u8 & 127) | 128);
        value >>= 7;
    }
    payload.push(value as u8);
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_TRAINING_V1,
            COMMAND_TYPE_OFFLINE_TRAINING,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("offline training status={:?}", reply.status).into());
    }
    let decoded = training_reply(&reply.payload).ok_or("malformed training reply")?;
    Ok((reply.payload, decoded))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let index = provisioning::item_index()?;
    let references: Vec<NonZeroU32> = STATUES
        .iter()
        .filter_map(|key| index.definition_ref("Item", key, "definition-r1"))
        .collect();
    if references.is_empty() {
        return Err("no statue definition in the item index".into());
    }
    let positions = statue_positions(&bytes, &references)?;
    let mut client = connect_world(clients, 0x40).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let (refused_bytes, refused) = offline(&mut client, NonZeroU64::MAX).await?;
    if refused != TrainingResult::REFUSED {
        return Err(format!("unknown handle reply={refused:?}").into());
    }
    capture("p14-refused.bin", &refused_bytes)?;

    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| {
        positions.iter().any(|statue| adjacent(at, *statue))
    })
    .ok_or("no reachable authored statue")?;
    walk(&mut client, world, &path).await?;
    let statue = map_statue_handle(&mut client, &references)?;
    let (selected_bytes, selected) = offline(&mut client, statue).await?;
    if selected.disposition != 3 || selected.revision == 0 || selected.pool_seconds == 0 {
        return Err(format!("statue reply={selected:?}").into());
    }
    capture("p14-selected.bin", &selected_bytes)?;
    let expected = format!(
        "refused_disposition={}\nselected_disposition={}\nselected_pool_seconds={}\nselected_charges={}\n",
        refused.disposition, selected.disposition, selected.pool_seconds, selected.charges
    );
    capture("p14-expected.txt", expected.as_bytes())?;
    Ok(expected.replace('\n', ";"))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P14.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P14.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P14.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacency_requires_same_floor_and_one_tile() {
        let a = TilePos {
            x: 100,
            y: 100,
            floor: 7,
        };
        assert!(adjacent(
            a,
            TilePos {
                x: 101,
                y: 101,
                ..a
            }
        ));
        assert!(!adjacent(a, TilePos { x: 102, ..a }));
        assert!(!adjacent(a, TilePos { floor: 8, ..a }));
    }
}
