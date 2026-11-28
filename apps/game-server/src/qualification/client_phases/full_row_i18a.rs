//! I18a: browse field, open a tile's browse container with walk-to.
//! Drops a backpack item, verifies the debit after relog, and walks away.
//! A different-floor request must be refused without changing game state.
//! One request for the actual tile must walk the actor back into reach and open the
//! browse view. The item is picked up and the original backpack quantity is verified
//! again after relog.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use std::collections::BTreeMap;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=I18a.full status={status} seconds={:.3} evidence={detail}",
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

use oteryn_dev_client::client_runtime::inventory::MoveRequest;
use oteryn_dev_client::{ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome};
use oteryn_protocol_oteryn::browse_field as browse;
use oteryn_protocol_oteryn::container_tree as tree;
use oteryn_protocol_oteryn::item_view::WorldTilePosition;

async fn browse_open(
    client: &mut DevClientSession,
    at: WorldTilePosition,
) -> TestResult<(tree::ContainerViewOutcome, Option<tree::ContainerView>)> {
    client.take_events();
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
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let events = client.take_events();
        let update = events.into_iter().find_map(|event| match event {
            SessionEvent::ContainerViews(update) => Some(update),
            _ => None,
        });
        if let Some(update) = update {
            if outcome != tree::ContainerViewOutcome::Opened
                || update.server_sequence <= reply.server_sequence
            {
                return Err("step=browse observed=unexpected-view-update".into());
            }
            return Ok((
                outcome,
                update
                    .value
                    .views
                    .into_iter()
                    .find(|view| view.browse.is_some()),
            ));
        }
        if outcome != tree::ContainerViewOutcome::Opened {
            return Ok((outcome, None));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=browse observed=opened-without-view-update".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
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
) -> TestResult<(std::num::NonZeroU32, std::num::NonZeroU32, u64)> {
    let item = client
        .inventory()
        .and_then(|inventory| inventory.entries.first())
        .copied()
        .ok_or("step=drop observed=no-backpack-item;needed:player-acquired-item")?;
    let at = ground(client);
    let before = quantity(client, item.item_definition_ref)?;
    let debited = before
        .checked_sub(u64::from(item.count.get()))
        .ok_or("step=drop observed=stack-larger-than-backpack-quantity")?;
    client.take_events();
    let moved = client
        .session_mut()
        .move_item_counted(MoveRequest {
            source: item.handle,
            destination: ItemMoveDestination::Ground(at),
            count: None,
        })
        .await
        .map_err(|error| format!("step=drop exchange={error}"))?;
    if moved.outcome != ItemMoveOutcome::Moved {
        return Err(format!("step=drop observed={:?};needed=Moved", moved.outcome).into());
    }
    wait_quantity(client, item.item_definition_ref, debited)
        .await
        .map_err(|error| format!("step=drop {error}"))?;
    Ok((item.item_definition_ref, item.count, before))
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
    let at = ground(&client);
    let (definition, count, before) = drop_first(&mut client).await?;
    let debited = before - u64::from(count.get());
    logout(&mut client).await?;
    let mut client = connect_world(clients, 0xf1).await?;
    let observed = quantity(&client, definition)?;
    if observed != debited {
        return Err(format!("step=drop-relog expected={debited} observed={observed}").into());
    }
    if tile(client.world_spatial().actor_position)? != t0 {
        return Err("step=drop-relog observed=actor-off-drop-tile".into());
    }
    let away = route(world, t0, |tile| {
        tile.floor == t0.floor && tile.x.abs_diff(t0.x).max(tile.y.abs_diff(t0.y)) >= 4
    })
    .ok_or("step=walk-away observed=no-route;needed:walkable-tiles-around-the-town-start")?;
    walk(&mut client, world, &away).await?;
    let distant = tile(client.world_spatial().actor_position)?;
    if distant.floor != t0.floor || distant.x.abs_diff(t0.x).max(distant.y.abs_diff(t0.y)) < 4 {
        return Err("step=walk-away observed=actor-not-distant".into());
    }
    use oteryn_protocol_oteryn::world_map::{BOTTOM_FLOOR, TOP_FLOOR};
    let other_floor = WorldTilePosition {
        floor: if at.floor == BOTTOM_FLOOR {
            TOP_FLOOR
        } else {
            BOTTOM_FLOOR
        },
        ..at
    };
    let inventory_before_refusal = client
        .inventory()
        .cloned()
        .ok_or("step=browse-precondition observed=no-inventory-snapshot")?;
    let views_before_refusal = client.session_mut().container_views().cloned();
    if views_before_refusal
        .as_ref()
        .is_some_and(|views| views.views.iter().any(|view| view.browse.is_some()))
    {
        return Err("step=browse-precondition observed=already-browsing".into());
    }
    let (refused, view) = browse_open(&mut client, other_floor).await?;
    if refused != tree::ContainerViewOutcome::TooFar
        || view.is_some()
        || tile(client.world_spatial().actor_position)? != distant
        || client.inventory() != Some(&inventory_before_refusal)
        || client.session_mut().container_views() != views_before_refusal.as_ref()
    {
        return Err(format!(
            "step=browse-other-floor observed={refused:?};needed=TooFar-with-unchanged-position-inventory-and-views"
        )
        .into());
    }
    let (opened, view) = browse_open(&mut client, at).await?;
    let view = view.ok_or_else(|| {
        format!("step=browse-walk-to observed={opened:?};needed=Opened-with-view")
    })?;
    let arrived = tile(client.world_spatial().actor_position)?;
    if arrived.floor != t0.floor || arrived.x.abs_diff(t0.x).max(arrived.y.abs_diff(t0.y)) > 1 {
        return Err("step=browse-walk-to observed=actor-not-adjacent".into());
    }
    let page = view
        .browse
        .ok_or("step=browse-near observed=no-browse-page")?;
    if opened != tree::ContainerViewOutcome::Opened || page.position != at {
        return Err(format!(
            "step=browse-near observed={opened:?}-{:?};needed=Opened-at-{at:?}",
            page.position
        )
        .into());
    }
    let entry =
        has(&view, definition).ok_or("step=browse-near observed=dropped-item-not-listed")?;
    if entry.count != count {
        return Err("step=browse-walk-to observed=dropped-stack-count-mismatch".into());
    }
    if quantity(&client, definition)? != debited {
        return Err("step=take observed=backpack-quantity-changed-before-pickup".into());
    }
    take_entry(&mut client, &entry).await?;
    if quantity(&client, definition)? != before {
        return Err("step=restore observed=backpack-count-not-restored".into());
    }
    logout(&mut client).await?;
    let mut relogged = connect_world(clients, 0xf1).await?;
    if quantity(&relogged, definition)? != before {
        return Err("step=relog observed=backpack-count-not-restored".into());
    }
    if tile(relogged.world_spatial().actor_position)? != arrived {
        return Err("step=relog observed=walk-to-position-not-restored".into());
    }
    logout(&mut relogged).await?;
    Ok(format!(
        "drop-relog={debited};wrong-floor-unchanged;walk-to-opened;pickup-relog={before};position-relog={},{},{};item-entries={}",
        arrived.x,
        arrived.y,
        arrived.floor,
        view.entries.len()
    ))
}
