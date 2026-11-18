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
    let result = verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients))).await;
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

use oteryn_dev_client::{ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome};
use oteryn_protocol_oteryn::browse_field as browse;
use oteryn_protocol_oteryn::container_tree as tree;
use oteryn_protocol_oteryn::item_view::{ItemEntry, ItemHandle, WorldTilePosition};

pub(super) async fn browse_open(
    client: &mut DevClientSession,
    at: WorldTilePosition,
) -> TestResult<(tree::ContainerViewOutcome, Option<tree::ContainerView>)> {
    let payload =
        browse::encode_browse_field_intent(&browse::BrowseFieldIntent::Open { position: at })
            .map_err(|error| format!("step=browse-encode observed={error:?}"))?;
    client.take_events();
    let before = client.last_server_sequence();
    let reply = client
        .session_mut()
        .gated_command(
            browse::CAPABILITY_BROWSE_FIELD_V1,
            browse::COMMAND_TYPE_BROWSE_FIELD_INTENT,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=browse observed={:?}", reply.status).into());
    }
    let outcome = tree::decode_container_view_result(&reply.payload)
        .map_err(|error| format!("step=browse-reply observed={error:?}"))?;
    if outcome != tree::ContainerViewOutcome::Opened {
        return Ok((outcome, None));
    }
    let view = wait_container_views(client, before).await?
        .views
        .into_iter()
        .find(|view| view.browse.is_some_and(|page| page.position == at));
    Ok((outcome, view))
}

pub(super) fn here(client: &DevClientSession) -> WorldTilePosition {
    let at = client.world_spatial().actor_position;
    WorldTilePosition {
        x: at.x,
        y: at.y,
        floor: -at.floor.abs(),
    }
}

/// Drop a carried stack and identify its new ground handle in the pushed tile delta.
pub(super) async fn drop_first(
    client: &mut DevClientSession,
    prefer: Option<std::num::NonZeroU32>,
) -> TestResult<ItemEntry> {
    use oteryn_protocol_oteryn::world_map::{MapDefinition, MapOrigin};
    let at = here(client);
    let (outcome, view) = browse_open(client, at).await?;
    let baseline = require_browse(at, outcome, view)?;
    if baseline
        .browse
        .is_none_or(|page| usize::from(page.total) != baseline.entries.len())
    {
        return Err("step=drop observed=incomplete-ground-baseline".into());
    }
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
                .or_else(|| {
                    prefer
                        .is_none()
                        .then(|| inventory.entries.first())
                        .flatten()
                })
        })
        .copied()
        .ok_or("step=drop observed=no-backpack-item;needed:player-acquired-item")?;
    let before = quantity(client, item.item_definition_ref)?;
    client.take_events();
    let moved = client
        .move_item(&ItemMoveIntent {
            source: item.handle,
            destination: ItemMoveDestination::Ground(at),
        })
        .await?;
    if moved.status != CommandStatus::Accepted || moved.outcome != ItemMoveOutcome::Moved {
        return Err(format!("step=drop observed={:?};needed=Moved", moved.outcome).into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let expected = before
        .checked_sub(u64::from(item.count.get()))
        .ok_or("drop custody underflow")?;
    let mut dropped = None;
    loop {
        for event in client.take_events() {
            if let SessionEvent::WorldMap(delta) = event
                && delta.server_sequence > moved.result_server_sequence
                && let Some(tile) = delta.value.tiles.iter().find(|tile| {
                    (tile.position.x, tile.position.y, tile.position.floor)
                        == (at.x, at.y, at.floor)
                })
            {
                let mut candidates = tile.items.iter().filter_map(|ground| {
                    let MapOrigin::Handle(handle) = ground.origin else {
                        return None;
                    };
                    (ground.definition == MapDefinition::Item(item.item_definition_ref)
                        && ground.count == item.count.get()
                        && ground.sub_type == item.sub_type
                        && !baseline.entries.iter().any(|entry| entry.handle == handle))
                    .then_some(handle)
                });
                if let (Some(handle), None) = (candidates.next(), candidates.next()) {
                    dropped = Some(ItemEntry { handle, ..item });
                }
            }
        }
        if let Some(dropped) = dropped
            && quantity(client, item.item_definition_ref)? == expected
        {
            close_browse(client, baseline.view_id).await?;
            return Ok(dropped);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                "step=drop observed=no-exact-inventory-debit-and-unique-ground-handle".into(),
            );
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
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

pub(super) fn require_browse(
    at: WorldTilePosition,
    outcome: tree::ContainerViewOutcome,
    view: Option<tree::ContainerView>,
) -> TestResult<tree::ContainerView> {
    match view {
        Some(view)
            if outcome == tree::ContainerViewOutcome::Opened
                && view.browse.is_some_and(|page| page.position == at) =>
        {
            Ok(view)
        }
        _ => Err(
            format!("step=browse observed={outcome:?};needed=Opened-with-view-at-{at:?}").into(),
        ),
    }
}

pub(super) async fn close_browse(client: &mut DevClientSession, view_id: u8) -> TestResult {
    let payload = tree::encode_container_view_intent(&tree::ContainerViewIntent::Close { view_id })
        .map_err(|error| format!("step=close-browse encode={error:?}"))?;
    client.take_events();
    let before = client.last_server_sequence();
    let reply = client
        .session_mut()
        .gated_command(
            tree::CAPABILITY_CONTAINER_TREE_V1,
            tree::COMMAND_TYPE_CONTAINER_VIEW_INTENT,
            &payload,
        )
        .await?;
    let outcome = tree::decode_container_view_result(&reply.payload)
        .map_err(|error| format!("step=close-browse decode={error:?}"))?;
    if reply.status != CommandStatus::Accepted || outcome != tree::ContainerViewOutcome::Closed {
        return Err(format!("step=close-browse observed={:?}/{outcome:?}", reply.status).into());
    }
    let still_open = wait_container_views(client, before).await?
        .views
        .iter()
        .any(|view| view.view_id == view_id);
    super::full_row_i18b::verify_browse_close(outcome, still_open)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let world = open_world(clients).await?;
    let mut client = connect_world(clients, 0xf1).await?;
    let t0 = tile(client.world_spatial().actor_position)?;
    let at = here(&client);
    let dropped = drop_first(&mut client, None).await?;
    let away = route_with(world, t0, |tile| {
        tile.floor == t0.floor && tile.x.abs_diff(t0.x).max(tile.y.abs_diff(t0.y)) >= 4
    }, |at| keeps_drop_projected(at, t0))
    .ok_or("step=walk-away observed=no-route;needed:walkable-tiles-around-the-town-start")?;
    walk(&mut client, world, &away).await?;
    let (far, view) = browse_open(&mut client, at).await?;
    if far != tree::ContainerViewOutcome::TooFar || view.is_some() {
        return Err(format!("step=browse-far observed={far:?};needed=TooFar").into());
    }
    let now = tile(client.world_spatial().actor_position)?;
    let back = route_with(world, now, |tile| {
        tile.floor == t0.floor && tile.x.abs_diff(t0.x).max(tile.y.abs_diff(t0.y)) <= 1
    }, |at| keeps_drop_projected(at, t0))
    .ok_or("step=walk-back observed=no-route")?;
    walk(&mut client, world, &back).await?;
    let (near, view) = browse_open(&mut client, at).await?;
    let view = require_browse(at, near, view)?;
    let entry = browse_entry(&view.entries, dropped.item_definition_ref, dropped.handle)
        .ok_or("step=browse-near observed=dropped-item-not-listed")?;
    take_entry(&mut client, &entry).await?;
    if quantity(&client, dropped.item_definition_ref)? < u64::from(dropped.count.get()) {
        return Err("step=restore observed=backpack-count-not-restored".into());
    }
    logout(&mut client).await?;
    Ok(format!(
        "too-far-then-opened-with-item-entries={}",
        view.entries.len()
    ))
}

pub(super) fn browse_entry(
    entries: &[ItemEntry],
    definition: std::num::NonZeroU32,
    handle: ItemHandle,
) -> Option<ItemEntry> {
    entries
        .iter()
        .copied()
        .find(|entry| entry.item_definition_ref == definition && entry.handle == handle)
}

#[test]
fn browse_entry_selects_exact_dropped_instance() {
    let first = ItemEntry {
        appearance_id: 0,
        handle: ItemHandle::MIN,
        item_definition_ref: std::num::NonZeroU32::MIN,
        count: std::num::NonZeroU32::MIN,
        sub_type: 0,
    };
    let Some(handle) = ItemHandle::new(2) else {
        return;
    };
    let dropped = ItemEntry { handle, ..first };
    assert_eq!(
        browse_entry(&[first, dropped], first.item_definition_ref, handle),
        Some(dropped)
    );
    assert_eq!(
        browse_entry(&[first], first.item_definition_ref, handle),
        None
    );
}

fn observed_views(events: Vec<SessionEvent>, after: u64) -> Option<tree::ContainerViews> {
    events.into_iter().rev().find_map(|event| match event {
        SessionEvent::ContainerViews(delta) if delta.server_sequence > after => Some(delta.value),
        _ => None,
    })
}

#[test]
fn round2_browse_observes_views_among_other_domain_events() {
    let events = vec![
        SessionEvent::Inventory(oteryn_dev_client::AppliedDelta {
            server_sequence: 11, base_revision: 1, new_revision: 2,
            value: oteryn_dev_client::CharacterInventory::default(),
        }),
        SessionEvent::ContainerViews(oteryn_dev_client::AppliedDelta {
            server_sequence: 12, base_revision: 2, new_revision: 3,
            value: tree::ContainerViews { views: vec![] },
        }),
    ];
    assert!(observed_views(events, 10).is_some());
}

fn keeps_drop_projected(origin: TilePos, drop: TilePos) -> bool {
    let at = |tile: TilePos| ActorPosition { x:i32::from(tile.x), y:i32::from(tile.y), floor:i16::from(tile.floor) };
    oteryn_protocol_oteryn::world_map::in_window(at(origin), at(drop))
}

#[test]
fn round2_browse_walk_excludes_tiles_outside_drop_projection() {
    let drop = TilePos { x: 100, y: 100, floor: -7 };
    assert!(keeps_drop_projected(TilePos {x: 104, ..drop}, drop));
    assert!(!keeps_drop_projected(TilePos {x: 120, ..drop}, drop));
}

pub(super) async fn wait_container_views(client: &mut DevClientSession, after: u64) -> TestResult<tree::ContainerViews> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(views) = observed_views(client.take_events(), after) { return Ok(views); }
        if tokio::time::Instant::now() >= deadline { return Err("missing container-view update".into()); }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}
