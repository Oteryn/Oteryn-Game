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

async fn browse_command(
    client: &mut DevClientSession,
    intent: browse::BrowseFieldIntent,
) -> TestResult<(tree::ContainerViewOutcome, u64)> {
    let payload = browse::encode_browse_field_intent(&intent)
        .map_err(|error| format!("step=browse-encode observed={error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            browse::CAPABILITY_BROWSE_FIELD_V1,
            browse::COMMAND_TYPE_BROWSE_FIELD_INTENT,
            &payload,
        )
        .await?;
    if reply.status != oteryn_protocol_oteryn::CommandStatus::Accepted {
        return Err(format!("step=browse observed=command-status-{:?}", reply.status).into());
    }
    let outcome = tree::decode_container_view_result(&reply.payload)
        .map_err(|error| format!("step=browse-reply observed={error:?}"))?;
    Ok((outcome, reply.server_sequence))
}

fn verify_walk(
    mut current: TilePos,
    target: TilePos,
    landings: &[(u64, TilePos)],
    opened_sequence: u64,
) -> TestResult<TilePos> {
    let mut previous_sequence = 0;
    for &(sequence, next) in landings {
        if sequence <= previous_sequence
            || sequence >= opened_sequence
            || current == next
            || current.x.abs_diff(next.x).max(current.y.abs_diff(next.y)) != 1
            || next.floor != target.floor
        {
            return Err(format!("step=browse-walk-to invalid-step={current:?}->{next:?} sequence={sequence} opened={opened_sequence}").into());
        }
        current = next;
        previous_sequence = sequence;
    }
    if landings.is_empty()
        || current
            .x
            .abs_diff(target.x)
            .max(current.y.abs_diff(target.y))
            > 1
    {
        return Err("step=browse-walk-to observed=no-walk-into-reach-before-opened".into());
    }
    Ok(current)
}

async fn browse_open(
    client: &mut DevClientSession,
    at: WorldTilePosition,
    target: TilePos,
    expect_walk: bool,
) -> TestResult<tree::ContainerView> {
    client.take_events();
    let start = tile(client.world_spatial().actor_position)?;
    let (outcome, reply_sequence) =
        browse_command(client, browse::BrowseFieldIntent::Open { position: at }).await?;
    if outcome != tree::ContainerViewOutcome::Opened {
        return Err(format!("step=browse-open observed={outcome:?};needed=Opened").into());
    }
    let mut landings = Vec::new();
    for event in client.take_events() {
        match event {
            SessionEvent::WorldSpatial(update) => {
                landings.push((update.server_sequence, tile(update.value.actor_position)?));
            }
            SessionEvent::WorldSpatialEntities(update) => {
                let position = tile(update.value.actor_position)?;
                if position != landings.last().map_or(start, |&(_, at)| at) {
                    landings.push((update.server_sequence, position));
                }
            }
            SessionEvent::GatedSnapshotReplaced(_) | SessionEvent::ContainerViews(_) => {
                return Err("step=browse-open observed=replacement-or-premature-view".into());
            }
            _ => {}
        }
    }
    let expected = if expect_walk {
        verify_walk(start, target, &landings, reply_sequence)?
    } else if landings.is_empty()
        && start.floor == target.floor
        && start.x.abs_diff(target.x).max(start.y.abs_diff(target.y)) <= 1
    {
        start
    } else {
        return Err("step=browse-open observed=movement-on-adjacent-open".into());
    };
    let delta = client
        .session_mut()
        .read_gated_delta(
            tree::STATE_DOMAIN_CONTAINER_VIEWS,
            tree::DELTA_TYPE_CONTAINER_VIEWS_V1,
        )
        .await?;
    if delta.server_sequence <= reply_sequence
        || tile(client.world_spatial().actor_position)? != expected
        || client.take_events().iter().any(|event| {
            matches!(
                event,
                SessionEvent::GatedSnapshotReplaced(_)
                    | SessionEvent::WorldSpatial(_)
                    | SessionEvent::ContainerViews(_)
            )
        })
    {
        return Err("step=browse-open observed=invalid-view-delta-order".into());
    }
    let view = selected_container_views(client.selected_capabilities(), &delta.payload)
        .map_err(|error| format!("step=browse-open observed=invalid-view-{error:?}"))?
        .views
        .into_iter()
        .find(|view| {
            view.browse
                .is_some_and(|page| page.position == at && page.index == 0)
        })
        .ok_or("step=browse-open observed=missing-exact-field")?;
    Ok(view)
}

fn assert_empty_field(view: &tree::ContainerView) -> TestResult {
    if !view.entries.is_empty() || view.browse.is_none_or(|page| page.total != 0) {
        return Err("step=browse-custody observed=field-not-empty".into());
    }
    Ok(())
}

async fn walk_once(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let expected =
            destination(world, before, direction).ok_or("step=setup-walk missing-landing")?;
        let receipt = client.step(direction).await?;
        if receipt.disposition != StepDisposition::Moved
            || receipt
                .world_spatial_delta
                .as_ref()
                .map(|delta| tile(delta.value.actor_position))
                .transpose()?
                != Some(expected)
            || tile(client.world_spatial().actor_position)? != expected
        {
            return Err(format!(
                "step=setup-walk direction={direction:?} first-receipt={receipt:?}"
            )
            .into());
        }
    }
    Ok(())
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

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let world = open_world(clients).await?;
    let mut client = connect_world(clients, 0xf1).await?;
    let t0 = tile(client.world_spatial().actor_position)?;
    let at = ground(&client);
    assert_empty_field(&browse_open(&mut client, at, t0, false).await?)?;
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
    walk_once(&mut client, world, &away).await?;
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
    client.take_events();
    let (refused, _) = browse_command(
        &mut client,
        browse::BrowseFieldIntent::Open {
            position: other_floor,
        },
    )
    .await?;
    // Seek an unopened view as an ordered, read-only barrier. No state-changing request
    // is sent while draining the refusal's frames.
    let (barrier, _) = browse_command(
        &mut client,
        browse::BrowseFieldIntent::Seek {
            view_id: 0,
            index: 0,
        },
    )
    .await?;
    client.service_liveness(Duration::from_secs(1)).await?;
    if refused != tree::ContainerViewOutcome::TooFar
        || barrier != tree::ContainerViewOutcome::Stale
        || client.take_events().iter().any(|event| match event {
            SessionEvent::WorldSpatialEntities(update) => {
                tile(update.value.actor_position).ok() != Some(distant)
            }
            SessionEvent::WorldSpatial(_)
            | SessionEvent::Inventory(_)
            | SessionEvent::ContainerViews(_)
            | SessionEvent::GatedSnapshotReplaced(_) => true,
            _ => false,
        })
        || tile(client.world_spatial().actor_position)? != distant
        || client.inventory() != Some(&inventory_before_refusal)
        || client.session_mut().container_views() != views_before_refusal.as_ref()
    {
        return Err(format!("step=browse-other-floor observed={refused:?} barrier={barrier:?};needed=TooFar-with-unchanged-position-inventory-and-views").into());
    }
    let view = browse_open(&mut client, at, t0, true).await?;
    let arrived = tile(client.world_spatial().actor_position)?;
    let [entry] = view.entries.as_slice() else {
        return Err("step=browse-custody observed=field-does-not-contain-exactly-one-stack".into());
    };
    if entry.item_definition_ref != definition
        || entry.count != count
        || view.browse.is_none_or(|page| page.total != 1)
    {
        return Err("step=browse-custody observed=dropped-stack-mismatch".into());
    }
    if quantity(&client, definition)? != debited {
        return Err("step=take observed=backpack-quantity-changed-before-pickup".into());
    }
    take_entry(&mut client, entry).await?;
    if quantity(&client, definition)? != before {
        return Err("step=restore observed=backpack-count-not-restored".into());
    }
    assert_empty_field(&browse_open(&mut client, at, t0, false).await?)?;
    logout(&mut client).await?;
    let mut relogged = connect_world(clients, 0xf1).await?;
    if quantity(&relogged, definition)? != before {
        return Err("step=relog observed=backpack-count-not-restored".into());
    }
    if tile(relogged.world_spatial().actor_position)? != arrived {
        return Err("step=relog observed=walk-to-position-not-restored".into());
    }
    assert_empty_field(&browse_open(&mut relogged, at, t0, false).await?)?;
    logout(&mut relogged).await?;
    Ok(format!(
        "drop-relog={debited};wrong-floor-unchanged;walk-frames-before-open;field-empty-after-pickup-and-relog;pickup-relog={before};position-relog={},{},{};item-entries={}",
        arrived.x,
        arrived.y,
        arrived.floor,
        view.entries.len()
    ))
}
