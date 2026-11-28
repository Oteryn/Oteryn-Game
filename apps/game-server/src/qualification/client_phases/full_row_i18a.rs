//! I18a: browse field, open a tile's browse container with walk-to.
//! Drops a backpack item, verifies the debit after relog, and walks away.
//! A different-floor request must be refused without changing game state.
//! One request for the actual tile must walk the actor back into reach and open the
//! browse view. The item is picked up and the original backpack quantity is verified
//! again after relog.
use super::*;
use oteryn_dev_client::{CharacterInventory, DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::item_view::ItemEntry;

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

fn inventory(client: &DevClientSession) -> TestResult<&CharacterInventory> {
    client
        .inventory()
        .ok_or_else(|| "inventory snapshot absent".into())
}

fn item_properties(item: &ItemEntry) -> ItemEntry {
    ItemEntry {
        handle: std::num::NonZeroU64::MIN,
        ..*item
    }
}

fn persisted_inventory(inventory: &CharacterInventory) -> CharacterInventory {
    let mut inventory = inventory.clone();
    inventory.main_backpack = inventory.main_backpack.as_ref().map(item_properties);
    inventory
        .entries
        .iter_mut()
        .for_each(|item| *item = item_properties(item));
    inventory
        .equipment
        .iter_mut()
        .for_each(|equipped| equipped.item = item_properties(&equipped.item));
    inventory
}

fn require_after_result(sequence: u64, result: u64) -> TestResult {
    if sequence <= result {
        return Err(
            format!("step=item-effect sequence={sequence} must-follow-result={result}").into(),
        );
    }
    Ok(())
}

fn verify_inventory_effect(
    delta: &oteryn_dev_client::AppliedDelta<oteryn_dev_client::CharacterInventory>,
    result: u64,
    expected: &CharacterInventory,
) -> TestResult {
    require_after_result(delta.server_sequence, result)?;
    if &delta.value != expected {
        return Err(format!(
            "inventory-delta sequence={} expected={expected:?} observed={:?}",
            delta.server_sequence, delta.value
        )
        .into());
    }
    Ok(())
}

async fn wait_inventory(
    client: &mut DevClientSession,
    result: u64,
    expected: &CharacterInventory,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let mut checked = false;
        for event in client.take_events() {
            match event {
                SessionEvent::Inventory(delta) => {
                    verify_inventory_effect(&delta, result, expected)?;
                    checked = true;
                }
                SessionEvent::GatedSnapshotReplaced(_) => {
                    return Err("step=drop observed=replacement-snapshot".into());
                }
                _ => {}
            }
        }
        if checked {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("inventory-missing-delta expected={expected:?}").into());
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
    require_accepted(reply.status)?;
    let outcome = tree::decode_container_view_result(&reply.payload)
        .map_err(|error| format!("step=browse-reply observed={error:?}"))?;
    Ok((outcome, reply.server_sequence))
}

fn require_accepted(status: oteryn_protocol_oteryn::CommandStatus) -> TestResult {
    if status != oteryn_protocol_oteryn::CommandStatus::Accepted {
        return Err(format!("step=browse observed=command-status-{status:?}").into());
    }
    Ok(())
}

fn verify_walk(
    mut current: TilePos,
    target: TilePos,
    landings: &[(u64, TilePos)],
    opened_sequence: u64,
    enterable: impl Fn(TilePos) -> bool,
) -> TestResult<TilePos> {
    let mut previous_sequence = 0;
    for &(sequence, next) in landings {
        if sequence <= previous_sequence
            || sequence >= opened_sequence
            || current == next
            || current.x.abs_diff(next.x).max(current.y.abs_diff(next.y)) != 1
            || next.floor != target.floor
            || !enterable(next)
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

fn ordinary_landing(floor_change: bool, teleport: bool) -> bool {
    !floor_change && !teleport
}

fn invalid_open_event(event: &SessionEvent) -> bool {
    matches!(
        event,
        SessionEvent::GatedSnapshotReplaced(_)
            | SessionEvent::ContainerViews(_)
            | SessionEvent::PartyIcons(_)
            | SessionEvent::GuildEmblems(_)
            | SessionEvent::InventoryCapacity(_)
    )
}

async fn browse_open(
    client: &mut DevClientSession,
    world: &BundleWorld,
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
            event if invalid_open_event(&event) => {
                return Err("step=browse-open observed=replacement-or-premature-view".into());
            }
            _ => {}
        }
    }
    let expected = if expect_walk {
        verify_walk(start, target, &landings, reply_sequence, |pos| {
            world.enterable(pos)
                && ordinary_landing(
                    world.floors().contains(pos),
                    world.floors().teleport(pos).is_some()
                        || world.floors().doomforge_portal(pos).is_some()
                        || world.facts().quest_portal_destination(pos, 0).is_some(),
                )
                && world
                    .base()
                    .tile(pos.x, pos.y, pos.floor)
                    .is_some_and(|tile| tile.walkable())
        })?
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
            invalid_open_event(event)
                || matches!(
                    event,
                    SessionEvent::WorldSpatial(_) | SessionEvent::WorldSpatialEntities(_)
                )
        })
    {
        return Err("step=browse-open observed=invalid-view-delta-order".into());
    }
    let views = selected_container_views(client.selected_capabilities(), &delta.payload)
        .map_err(|error| format!("step=browse-open observed=invalid-view-{error:?}"))?;
    exact_field(views, at)
}

fn exact_field(
    views: tree::ContainerViews,
    at: WorldTilePosition,
) -> TestResult<tree::ContainerView> {
    let mut browse_views = views.views.into_iter().filter(|view| view.browse.is_some());
    let field = browse_views
        .next()
        .filter(|view| {
            view.browse
                .is_some_and(|page| page.position == at && page.index == 0)
        })
        .ok_or("step=browse-open observed=missing-exact-field")?;
    if browse_views.next().is_some() {
        return Err("step=browse-open observed=unexpected-extra-browse-view".into());
    }
    Ok(field)
}

fn assert_empty_field(view: &tree::ContainerView) -> TestResult {
    if !view.entries.is_empty() || view.browse.is_none_or(|page| page.total != 0) {
        return Err("step=browse-custody observed=field-not-empty".into());
    }
    Ok(())
}

fn verify_setup_step(
    disposition: StepDisposition,
    landing: Option<TilePos>,
    observed: TilePos,
    expected: TilePos,
) -> bool {
    disposition == StepDisposition::Moved && landing == Some(expected) && observed == expected
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
        if !verify_setup_step(
            receipt.disposition,
            receipt
                .world_spatial_delta
                .as_ref()
                .map(|delta| tile(delta.value.actor_position))
                .transpose()?,
            tile(client.world_spatial().actor_position)?,
            expected,
        ) {
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

/// Drops the first backpack entry and verifies the entire resulting inventory.
async fn drop_first(
    client: &mut DevClientSession,
    item: ItemEntry,
    expected: &CharacterInventory,
) -> TestResult {
    let at = ground(client);
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
    let result_sequence = client.last_server_sequence();
    wait_inventory(client, result_sequence, expected)
        .await
        .map_err(|error| format!("step=drop {error}").into())
}

/// Moves a browse entry into the main backpack and waits for the inventory credit.
async fn take_entry(
    client: &mut DevClientSession,
    entry: &oteryn_protocol_oteryn::item_view::ItemEntry,
    field: &tree::ContainerView,
    expected: &CharacterInventory,
) -> TestResult {
    let at = field
        .browse
        .ok_or("step=take missing-browse-page")?
        .position;
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
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut credited = false;
    let mut emptied = false;
    loop {
        for event in client.take_events() {
            match event {
                SessionEvent::GatedSnapshotReplaced(_) => {
                    return Err("step=take observed=replacement-snapshot".into());
                }
                SessionEvent::Inventory(delta) => {
                    let mut delta = delta;
                    // A moved item can be assigned a new backpack handle. Preserve every
                    // unaffected handle and check all properties of the newly placed stack.
                    let picked_up = delta
                        .value
                        .entries
                        .first_mut()
                        .ok_or("step=take missing-backpack-stack")?;
                    picked_up.handle = std::num::NonZeroU64::MIN;
                    verify_inventory_effect(&delta, moved.result_server_sequence, expected)?;
                    credited = true;
                }
                SessionEvent::ContainerViews(update) => {
                    require_after_result(update.server_sequence, moved.result_server_sequence)?;
                    verify_pickup_refresh(update.value, at, field.view_id)?;
                    emptied = true;
                }
                _ => {}
            }
        }
        if credited && emptied {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step=take observed=missing-pickup-effects credit={credited} empty-field={emptied}"
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

fn verify_pickup_refresh(
    views: tree::ContainerViews,
    at: WorldTilePosition,
    view_id: u8,
) -> TestResult {
    let refreshed = exact_field(views, at)?;
    if refreshed.view_id != view_id {
        return Err("step=take observed=wrong-field-view-id".into());
    }
    assert_empty_field(&refreshed)
}

fn refusal_changed_state(event: &SessionEvent, distant: TilePos) -> bool {
    match event {
        SessionEvent::WorldSpatialEntities(update) => {
            tile(update.value.actor_position).ok() != Some(distant)
        }
        SessionEvent::WorldSpatial(_)
        | SessionEvent::Inventory(_)
        | SessionEvent::PartyIcons(_)
        | SessionEvent::GuildEmblems(_)
        | SessionEvent::InventoryCapacity(_)
        | SessionEvent::ContainerViews(_)
        | SessionEvent::WorldMap(_)
        | SessionEvent::WorldMapSnapshot(_)
        | SessionEvent::WorldObjectOverlay(_)
        | SessionEvent::WorldObjectOverlaySnapshot(_)
        | SessionEvent::GatedSnapshotReplaced(_) => true,
        _ => false,
    }
}

fn authored_wrong_floor(
    actor: TilePos,
    exists: impl Fn(TilePos) -> bool,
) -> TestResult<WorldTilePosition> {
    use oteryn_protocol_oteryn::world_map::{BOTTOM_FLOOR, TOP_FLOOR};
    // Keep horizontal distance within browse reach so floor is the only range difference.
    for floor in TOP_FLOOR as i8..=BOTTOM_FLOOR as i8 {
        if -floor == actor.floor {
            continue;
        }
        for x in actor.x.saturating_sub(1)..=actor.x.saturating_add(1) {
            for y in actor.y.saturating_sub(1)..=actor.y.saturating_add(1) {
                if exists(TilePos {
                    x,
                    y,
                    floor: -floor,
                }) {
                    return Ok(WorldTilePosition {
                        x: i32::from(x),
                        y: i32::from(y),
                        floor: i16::from(floor),
                    });
                }
            }
        }
    }
    Err("step=browse-other-floor observed=no-authored-tile-within-horizontal-reach".into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let world = open_world(clients).await?;
    let mut client = connect_world(clients, 0xf1).await?;
    let t0 = tile(client.world_spatial().actor_position)?;
    let at = ground(&client);
    assert_empty_field(&browse_open(&mut client, world, at, t0, false).await?)?;
    let initial = inventory(&client)?.clone();
    let mut after_drop = initial.clone();
    if after_drop.entries.is_empty() {
        return Err("step=drop observed=no-backpack-item".into());
    }
    let dropped = after_drop.entries.remove(0);
    // The fixture uses a unique definition, so the single-stack pickup cannot merge.
    if after_drop
        .entries
        .iter()
        .any(|entry| entry.item_definition_ref == dropped.item_definition_ref)
    {
        return Err("step=drop observed=ambiguous-pickup-stack".into());
    }
    drop_first(&mut client, dropped, &after_drop).await?;
    logout(&mut client).await?;
    let mut client = connect_world(clients, 0xf1).await?;
    if persisted_inventory(inventory(&client)?) != persisted_inventory(&after_drop) {
        return Err("step=drop-relog observed=wrong-backpack-or-equipment".into());
    }
    let after_drop_relog = inventory(&client)?.clone();
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
    let other_floor = authored_wrong_floor(distant, |pos| {
        world.base().tile(pos.x, pos.y, pos.floor).is_some()
    })?;
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
        || client
            .take_events()
            .iter()
            .any(|event| refusal_changed_state(event, distant))
        || tile(client.world_spatial().actor_position)? != distant
        || client.inventory() != Some(&inventory_before_refusal)
        || client.session_mut().container_views() != views_before_refusal.as_ref()
    {
        return Err(format!("step=browse-other-floor observed={refused:?} barrier={barrier:?};needed=TooFar-with-unchanged-position-inventory-and-views").into());
    }
    let view = browse_open(&mut client, world, at, t0, true).await?;
    let arrived = tile(client.world_spatial().actor_position)?;
    let [entry] = view.entries.as_slice() else {
        return Err("step=browse-custody observed=field-does-not-contain-exactly-one-stack".into());
    };
    if item_properties(entry) != item_properties(&dropped)
        || view.browse.is_none_or(|page| page.total != 1)
    {
        return Err("step=browse-custody observed=dropped-stack-mismatch".into());
    }
    if inventory(&client)? != &after_drop_relog {
        return Err("step=take observed=inventory-changed-before-pickup".into());
    }
    let mut expected_pickup = after_drop_relog;
    expected_pickup.entries.insert(0, item_properties(&dropped));
    take_entry(&mut client, entry, &view, &expected_pickup).await?;
    if persisted_inventory(inventory(&client)?) != persisted_inventory(&initial) {
        return Err("step=restore observed=wrong-backpack-or-equipment".into());
    }
    logout(&mut client).await?;
    let mut relogged = connect_world(clients, 0xf1).await?;
    if persisted_inventory(inventory(&relogged)?) != persisted_inventory(&initial) {
        return Err("step=pickup-relog observed=wrong-backpack-or-equipment".into());
    }
    if tile(relogged.world_spatial().actor_position)? != arrived {
        return Err("step=relog observed=walk-to-position-not-restored".into());
    }
    assert_empty_field(&browse_open(&mut relogged, world, at, t0, false).await?)?;
    logout(&mut relogged).await?;
    Ok(format!(
        "drop-relog=exact-inventory;wrong-floor-unchanged;walk-frames-before-open;field-empty-after-pickup-and-relog;pickup-relog=exact-inventory;position-relog={},{},{};item-entries={}",
        arrived.x,
        arrived.y,
        arrived.floor,
        view.entries.len()
    ))
}

#[test]
fn browse_audit_rejects_jump_and_late_arrival() -> TestResult {
    let start = TilePos {
        x: 10,
        y: 10,
        floor: -7,
    };
    let target = TilePos { x: 14, ..start };
    let adjacent = TilePos { x: 13, ..start };
    assert!(verify_walk(start, target, &[(4, adjacent)], 5, |_| true).is_err());
    let steps = [
        (2, TilePos { x: 11, ..start }),
        (3, TilePos { x: 12, ..start }),
        (4, adjacent),
    ];
    assert!(verify_walk(start, target, &steps, 4, |_| true).is_err());
    assert!(verify_walk(start, target, &steps, 5, |pos| pos != steps[1].1).is_err());
    for (floor_change, teleport) in [(true, false), (false, true)] {
        assert!(
            verify_walk(start, target, &steps, 5, |pos| {
                pos != steps[1].1 || ordinary_landing(floor_change, teleport)
            })
            .is_err()
        );
    }
    assert_eq!(verify_walk(start, target, &steps, 5, |_| true)?, adjacent);
    Ok(())
}

#[test]
fn browse_audit_rejects_rejected_status_and_retained_stack() -> TestResult {
    use oteryn_protocol_oteryn::{CommandStatus, item_view::ItemEntry};
    use std::num::{NonZeroU32, NonZeroU64};

    assert!(require_accepted(CommandStatus::Rejected).is_err());
    require_accepted(CommandStatus::Accepted)?;
    let mut view = tree::ContainerView {
        view_id: 0,
        container_handle: NonZeroU64::MIN,
        parent_handle: None,
        capacity: 20,
        entries: vec![ItemEntry {
            appearance_id: 0,
            handle: NonZeroU64::MIN,
            item_definition_ref: NonZeroU32::MIN,
            count: NonZeroU32::MIN,
            sub_type: 0,
        }],
        browse: Some(browse::BrowseFieldPage {
            position: WorldTilePosition {
                x: 10,
                y: 10,
                floor: 7,
            },
            index: 0,
            total: 1,
        }),
    };
    let at = view.browse.ok_or("browse page")?.position;
    let mut extra = view.clone();
    extra.view_id = 1;
    extra.browse.as_mut().ok_or("extra browse page")?.position.x += 1;
    assert!(
        exact_field(
            tree::ContainerViews {
                views: vec![view.clone(), extra]
            },
            at
        )
        .is_err()
    );
    assert!(assert_empty_field(&view).is_err());
    view.entries.clear();
    assert!(assert_empty_field(&view).is_err());
    view.browse.as_mut().ok_or("browse page")?.total = 0;
    assert_empty_field(&view)?;
    let mut wrong_page = view.clone();
    wrong_page.browse.as_mut().ok_or("browse page")?.index = 1;
    assert!(
        verify_pickup_refresh(
            tree::ContainerViews {
                views: vec![wrong_page]
            },
            at,
            0
        )
        .is_err()
    );
    let mut extra = view.clone();
    extra.view_id = 1;
    assert!(
        verify_pickup_refresh(
            tree::ContainerViews {
                views: vec![view.clone(), extra]
            },
            at,
            0
        )
        .is_err()
    );
    verify_pickup_refresh(tree::ContainerViews { views: vec![view] }, at, 0)
}

#[test]
fn browse_audit_rejects_replacement_delayed_effect_and_first_refusal() {
    use oteryn_dev_client::{AppliedDelta, native_session::GatedSnapshot};
    use oteryn_protocol_oteryn::world_spatial::WorldSpatialObservation;

    let replacement = SessionEvent::GatedSnapshotReplaced(AppliedDelta {
        server_sequence: 12,
        base_revision: 1,
        new_revision: 2,
        value: GatedSnapshot {
            domain_id: tree::STATE_DOMAIN_CONTAINER_VIEWS,
            snapshot_type: 1,
            revision: 2,
            payload: Vec::new(),
        },
    });
    assert!(invalid_open_event(&replacement));

    let distant = TilePos {
        x: 10,
        y: 10,
        floor: -7,
    };
    let delayed = SessionEvent::WorldSpatial(AppliedDelta {
        server_sequence: 13,
        base_revision: 1,
        new_revision: 2,
        value: WorldSpatialObservation {
            content_generation: [0; 32],
            actor_position: ActorPosition {
                x: 11,
                y: 10,
                floor: 7,
            },
        },
    });
    assert!(refusal_changed_state(&delayed, distant));
    for refusal in [
        StepDisposition::Rejected,
        StepDisposition::TooEarly,
        StepDisposition::Blocked,
    ] {
        assert!(!verify_setup_step(refusal, Some(distant), distant, distant));
    }
    assert!(verify_setup_step(
        StepDisposition::Moved,
        Some(distant),
        distant,
        distant
    ));
}

#[test]
fn browse_audit_rejects_absent_wrong_floor_and_unbound_inventory_effects() -> TestResult {
    use oteryn_dev_client::{AppliedDelta, CharacterInventory};

    let actor = TilePos {
        x: 10,
        y: 10,
        floor: -7,
    };
    assert!(authored_wrong_floor(actor, |pos| pos == actor).is_err());
    let authored = TilePos { floor: -8, ..actor };
    assert_eq!(authored_wrong_floor(actor, |pos| pos == authored)?.floor, 8);

    let reference = std::num::NonZeroU32::MIN;
    let bad = AppliedDelta {
        server_sequence: 12,
        base_revision: 1,
        new_revision: 2,
        value: CharacterInventory::default(),
    };
    let mut good = bad.clone();
    good.server_sequence = 13;
    good.value
        .entries
        .push(oteryn_protocol_oteryn::item_view::ItemEntry {
            appearance_id: 0,
            handle: std::num::NonZeroU64::MIN,
            item_definition_ref: reference,
            count: reference,
            sub_type: 0,
        });
    assert!(verify_inventory_effect(&bad, 11, &good.value).is_err());
    verify_inventory_effect(&good, 11, &good.value)?;
    assert!(verify_inventory_effect(&good, 13, &good.value).is_err());
    let mut expected = good.value.clone();
    let item = expected.entries[0];
    expected
        .equipment
        .push(oteryn_protocol_oteryn::item_view::EquippedItem {
            slot: oteryn_protocol_oteryn::item_view::EquipmentSlot::Head,
            item,
        });
    let mut changed_equipment = good.clone();
    changed_equipment.value = expected.clone();
    changed_equipment.value.equipment[0].item.count =
        std::num::NonZeroU32::new(2).ok_or("count")?;
    assert!(verify_inventory_effect(&changed_equipment, 11, &expected).is_err());
    assert_ne!(
        persisted_inventory(&changed_equipment.value),
        persisted_inventory(&expected)
    );
    let mut other_stack = good.clone();
    other_stack.value = expected.clone();
    other_stack.value.entries.push(ItemEntry {
        item_definition_ref: std::num::NonZeroU32::new(2).ok_or("definition")?,
        ..item
    });
    assert!(verify_inventory_effect(&other_stack, 11, &expected).is_err());
    assert_ne!(
        persisted_inventory(&other_stack.value),
        persisted_inventory(&expected)
    );
    assert_ne!(
        item_properties(&item),
        item_properties(&ItemEntry {
            sub_type: 1,
            ..item
        })
    );
    assert!(require_after_result(10, 11).is_err());
    Ok(())
}

#[test]
fn browse_audit_rejects_position_opaque_refusal_frames() {
    use oteryn_dev_client::AppliedDelta;
    let actor = TilePos {
        x: 10,
        y: 10,
        floor: -7,
    };
    let events = [
        SessionEvent::PartyIcons(AppliedDelta {
            server_sequence: 12,
            base_revision: 1,
            new_revision: 2,
            value: Vec::new(),
        }),
        SessionEvent::GuildEmblems(AppliedDelta {
            server_sequence: 13,
            base_revision: 2,
            new_revision: 3,
            value: Vec::new(),
        }),
        SessionEvent::InventoryCapacity(AppliedDelta {
            server_sequence: 14,
            base_revision: 3,
            new_revision: 4,
            value: oteryn_protocol_oteryn::inventory_capacity::InventoryCapacity {
                capacity: 100,
                free: 100,
            },
        }),
    ];
    for event in events {
        assert!(invalid_open_event(&event));
        assert!(refusal_changed_state(&event, actor));
    }
    use oteryn_protocol_oteryn::{
        world_map::{MapViewHeader, WorldMapViewDelta},
        world_object::WorldObjectOverlayEntry,
    };
    let map = SessionEvent::WorldMap(AppliedDelta {
        server_sequence: 15,
        base_revision: 1,
        new_revision: 2,
        value: WorldMapViewDelta {
            header: MapViewHeader {
                content_generation: [0; 32],
                bundle_digest: [0; 32],
                reset_epoch: 0,
                origin: ActorPosition {
                    x: 10,
                    y: 10,
                    floor: 7,
                },
                first_visible_floor: 0,
            },
            tiles: Vec::new(),
            cleared: vec![ActorPosition {
                x: 11,
                y: 10,
                floor: 7,
            }],
        },
    });
    let overlay = SessionEvent::WorldObjectOverlay(AppliedDelta {
        server_sequence: 16,
        base_revision: 1,
        new_revision: 2,
        value: WorldObjectOverlayEntry {
            content_generation: [0; 32],
            placement: vec![1],
            state: vec![2],
            revision: 2,
        },
    });
    assert!(refusal_changed_state(&map, actor));
    assert!(refusal_changed_state(&overlay, actor));
}
