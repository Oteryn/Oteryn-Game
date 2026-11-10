//! I18a: browse field, open a tile's browse container with walk-to.
//! Drops one of the character's own backpack items on its tile (item move to ground, Moved),
//! walks at least four tiles away and asks to browse the dropped item's tile: the server must
//! refuse `TooFar` and open no view. The character then walks back next to the tile and the same
//! request must answer `Opened` with a browse view whose page is that tile and that lists the
//! dropped item. The item is picked up again and the restored backpack count is re-read.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use std::collections::BTreeMap;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(240), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=I18A.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

fn quantities(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let mut values = BTreeMap::new();
    for item in &client
        .inventory()
        .ok_or("inventory snapshot absent")?
        .entries
    {
        *values.entry(item.item_definition_ref.get()).or_default() += u64::from(item.count.get());
    }
    Ok(values)
}

fn quantity(client: &DevClientSession, reference: std::num::NonZeroU32) -> TestResult<u64> {
    Ok(*quantities(client)?.get(&reference.get()).unwrap_or(&0))
}

async fn wait_quantity(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
    expected: u64,
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::Inventory(delta) = event
                && quantity(client, reference)? == expected
            {
                return Ok(delta.server_sequence);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "inventory delta timeout definition={reference} expected={expected} observed={}",
                quantity(client, reference)?
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("logout refused".into());
    }
    Ok(())
}

async fn open_world(clients: &SeamClients<'_>) -> TestResult<&'static BundleWorld> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )
}

use oteryn_dev_client::{ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome};
use oteryn_protocol_oteryn::browse_field as browse;
use oteryn_protocol_oteryn::container_tree as tree;
use oteryn_protocol_oteryn::item_view::WorldTilePosition;

fn view_of(payload: &[u8]) -> TestResult<tree::ContainerViews> {
    tree::decode_container_views_for(payload, true)
        .map_err(|error| format!("step=views-decode observed={error:?}").into())
}

async fn browse_open(
    client: &mut DevClientSession,
    at: WorldTilePosition,
) -> TestResult<(tree::ContainerViewOutcome, Option<tree::ContainerView>)> {
    let payload =
        browse::encode_browse_field_intent(&browse::BrowseFieldIntent::Open { position: at })
            .map_err(|error| format!("step=browse-encode observed={error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            browse::CAPABILITY_BROWSE_FIELD_V1,
            browse::COMMAND_TYPE_BROWSE_FIELD_INTENT,
            &payload,
        )
        .await?;
    let outcome = tree::decode_container_view_result(&reply.payload)
        .map_err(|error| format!("step=browse-reply observed={error:?}"))?;
    if outcome != tree::ContainerViewOutcome::Opened {
        return Ok((outcome, None));
    }
    let delta = client
        .session_mut()
        .read_gated_delta(
            tree::STATE_DOMAIN_CONTAINER_VIEWS,
            tree::DELTA_TYPE_CONTAINER_VIEWS_V1,
        )
        .await?;
    let view = view_of(&delta.payload)?
        .views
        .into_iter()
        .find(|view| view.browse.is_some());
    Ok((outcome, view))
}

fn here(client: &DevClientSession) -> WorldTilePosition {
    let at = client.world_spatial().actor_position;
    WorldTilePosition {
        x: at.x,
        y: at.y,
        floor: -at.floor,
    }
}

fn ground(client: &DevClientSession) -> WorldTilePosition {
    let at = client.world_spatial().actor_position;
    WorldTilePosition {
        x: at.x,
        y: at.y,
        floor: at.floor,
    }
}

/// Drops the first backpack entry on the actor's own tile and returns its definition and count.
async fn drop_first(
    client: &mut DevClientSession,
    prefer: Option<std::num::NonZeroU32>,
) -> TestResult<(std::num::NonZeroU32, std::num::NonZeroU32)> {
    let item = client
        .inventory()
        .and_then(|inventory| {
            prefer
                .and_then(|want| {
                    inventory
                        .entries
                        .iter()
                        .find(|item| item.item_definition_ref == want)
                })
                .or(inventory.entries.first())
        })
        .copied()
        .ok_or("step=drop observed=no-backpack-item;needed:player-acquired-item")?;
    let at = ground(client);
    let before = quantity(client, item.item_definition_ref)?;
    client.take_events();
    let moved = client
        .move_item(&ItemMoveIntent {
            source: item.handle,
            destination: ItemMoveDestination::Ground(at),
        })
        .await?;
    if moved.outcome != ItemMoveOutcome::Moved {
        return Err(format!("step=drop observed={:?};needed=Moved", moved.outcome).into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while quantity(client, item.item_definition_ref)? >= before {
        client.take_events();
        if tokio::time::Instant::now() >= deadline {
            return Err("step=drop observed=no-inventory-debit".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    Ok((item.item_definition_ref, item.count))
}

/// Moves a browse entry into the main backpack and waits for the inventory credit.
async fn take_entry(
    client: &mut DevClientSession,
    entry: &oteryn_protocol_oteryn::item_view::ItemEntry,
) -> TestResult<()> {
    let before = quantity(client, entry.item_definition_ref)?;
    client.take_events();
    let moved = client
        .move_item(&ItemMoveIntent {
            source: entry.handle,
            destination: ItemMoveDestination::MainBackpack,
        })
        .await?;
    if moved.outcome != ItemMoveOutcome::Moved {
        return Err(format!("step=take observed={:?};needed=Moved", moved.outcome).into());
    }
    wait_quantity(
        client,
        entry.item_definition_ref,
        before + u64::from(entry.count.get()),
    )
    .await?;
    Ok(())
}

fn has(
    view: &tree::ContainerView,
    definition: std::num::NonZeroU32,
) -> Option<oteryn_protocol_oteryn::item_view::ItemEntry> {
    view.entries
        .iter()
        .find(|entry| entry.item_definition_ref == definition)
        .copied()
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let world = open_world(clients).await?;
    let mut client = connect_world(clients, 0xf1).await?;
    let t0 = tile(client.world_spatial().actor_position)?;
    let at = here(&client);
    let (definition, count) = drop_first(&mut client, None).await?;
    let away = route(world, t0, |tile| {
        tile.floor == t0.floor && tile.x.abs_diff(t0.x).max(tile.y.abs_diff(t0.y)) >= 4
    })
    .ok_or("step=walk-away observed=no-route;needed:walkable-tiles-around-the-town-start")?;
    walk(&mut client, world, &away).await?;
    let (far, view) = browse_open(&mut client, at).await?;
    if far != tree::ContainerViewOutcome::TooFar || view.is_some() {
        return Err(format!("step=browse-far observed={far:?};needed=TooFar").into());
    }
    let now = tile(client.world_spatial().actor_position)?;
    let back = route(world, now, |tile| {
        tile.floor == t0.floor && tile.x.abs_diff(t0.x).max(tile.y.abs_diff(t0.y)) <= 1
    })
    .ok_or("step=walk-back observed=no-route")?;
    walk(&mut client, world, &back).await?;
    let (near, view) = browse_open(&mut client, at).await?;
    let view =
        view.ok_or_else(|| format!("step=browse-near observed={near:?};needed=Opened-with-view"))?;
    let page = view
        .browse
        .ok_or("step=browse-near observed=no-browse-page")?;
    if near != tree::ContainerViewOutcome::Opened || page.position != at {
        return Err(format!(
            "step=browse-near observed={near:?}-{:?};needed=Opened-at-{at:?}",
            page.position
        )
        .into());
    }
    let entry =
        has(&view, definition).ok_or("step=browse-near observed=dropped-item-not-listed")?;
    take_entry(&mut client, &entry).await?;
    if quantity(&client, definition)? < u64::from(count.get()) {
        return Err("step=restore observed=backpack-count-not-restored".into());
    }
    logout(&mut client).await?;
    Ok(format!(
        "too-far-then-opened-with-item-entries={}",
        view.entries.len()
    ))
}
