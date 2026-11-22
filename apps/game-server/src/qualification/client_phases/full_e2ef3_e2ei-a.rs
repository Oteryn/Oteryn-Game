//! Ordinary-player item custody and instance inspection journeys.
use super::*;
use oteryn_dev_client::{DevClientSession, native_session::item_movement::MoveRequest};
use oteryn_protocol_oteryn::{
    browse_field as browse,
    item_view::{
        ItemEntry, ItemHandle, ItemMoveDestination as To, ItemMoveOutcome, WorldTilePosition,
    },
    logout::LogoutResult,
};
use std::num::NonZeroU32;

const I01: &str = "I01";
const I16: &str = "I16";
pub(super) const COVERED_ROWS: &[&str] = &[I01, I16];

fn world(clients: &SeamClients<'_>) -> TestResult<&'static BundleWorld> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )
}

async fn return_to_town(client: &mut DevClientSession, world: &BundleWorld) -> TestResult<()> {
    let from = tile(client.world_spatial().actor_position)?;
    let path = route_avoiding(world, from, |at| at == TOWN, route_available)
        .ok_or_else(|| format!("no return route from {from:?} to {TOWN:?}"))?;
    walk(client, world, &path).await
}

pub(super) async fn relogin_starter(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<()> {
    let world = world(clients)?;
    let from = tile(client.world_spatial().actor_position)?;
    let basis = spatial_basis::current(client)?;
    // Quentin wanders up to two tiles from (32368, 32240, 7), which includes the
    // entry cell. Fresh admission requires the recorded cell to remain free after
    // logout, so use the north side of the temple beyond his wander radius.
    let path = route(world, from, |at| {
        at.y < TOWN.y
            && at.floor == TOWN.floor
            && at.x.abs_diff(TOWN.x).max(at.y.abs_diff(TOWN.y)) <= 3
            && world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| tile.protection_zone())
            && client.world_entities().is_some_and(|entities| {
                entities
                    .iter()
                    .all(|entity| entity.position != basis.position(at))
            })
    })
    .ok_or("E2EF3 no unoccupied non-entry relog cell in the temple PZ")?;
    walk(client, world, &path).await?;
    relogin(client, clients).await
}

async fn relogin(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult<()> {
    let session = client.join_snapshot().game_session_id;
    let position = client.world_spatial().actor_position;
    let (result, logout, attempts, busy_timed_out) = logout_retrying_busy(client).await?;
    if result != Some(LogoutResult::Accepted) {
        return Err(format!(
            "logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}"
        )
        .into());
    }
    *client = connect_world(clients, 0x79).await?;
    let observed_session = client.join_snapshot().game_session_id;
    let observed_position = client.world_spatial().actor_position;
    let session_reused = observed_session == session;
    if session_reused || observed_position != position {
        return Err(format!(
            "E2EF3 fresh admission session/position mismatch: session_reused={session_reused}; position={position:?}->{observed_position:?}"
        ).into());
    }
    Ok(())
}

fn report(id: &str, step: &str, started: std::time::Instant, result: &TestResult<()>) {
    let seconds = started.elapsed().as_secs_f64();
    match result {
        Ok(()) => evidence(&format!(
            "e2e_all scenario={id}.full status=PASS seconds={seconds:.3}"
        )),
        Err(error) => {
            let suffix = verdict::classify(error.as_ref()).suffix(&format!("{id}/{step}"));
            evidence(&format!(
                "e2e_all scenario={id}.full status=FAIL seconds={seconds:.3}{suffix}"
            ));
            let observed = error
                .to_string()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join("-");
            evidence(&format!(
                "E2EF3_GAP row={id} unexecuted={step} observed={observed}"
            ));
        }
    }
}

pub(super) async fn run(
    mut client: DevClientSession,
    clients: &SeamClients<'_>,
) -> DevClientSession {
    let world = world(clients);
    let mut step = "ground-inventory";
    let started = std::time::Instant::now();
    let result = match &world {
        Ok(world) => inventory_journey(&mut client, clients, world, &mut step).await,
        Err(error) => Err(error.to_string().into()),
    };
    report(I01, step, started, &result);

    let mut step = "reach-imbuement-shrine";
    let started = std::time::Instant::now();
    let result = match &world {
        Ok(world) => imbuement_journey(&mut client, clients, world, &mut step).await,
        Err(error) => Err(error.to_string().into()),
    };
    report(I16, step, started, &result);
    if let Ok(world) = world
        && let Err(error) = return_to_town(&mut client, world).await
    {
        evidence(&format!(
            "E2EF3_OBSERVATION return-to-town=failed observed={error}"
        ));
    }
    client
}

fn entries(client: &DevClientSession) -> TestResult<&[ItemEntry]> {
    Ok(&client
        .inventory()
        .ok_or("inventory projection absent")?
        .entries)
}

fn matching(entries: &[ItemEntry], definition: NonZeroU32) -> TestResult<ItemEntry> {
    entries
        .iter()
        .find(|item| item.item_definition_ref == definition)
        .copied()
        .ok_or_else(|| format!("definition={definition}; observed entries={entries:?}").into())
}

fn counts(entries: &[ItemEntry], definition: NonZeroU32) -> Vec<u32> {
    let mut counts: Vec<_> = entries
        .iter()
        .filter(|entry| entry.item_definition_ref == definition)
        .map(|entry| entry.count.get())
        .collect();
    counts.sort_unstable();
    counts
}

fn assert_counts(
    entries: &[ItemEntry],
    definition: NonZeroU32,
    expected: &[u32],
) -> TestResult<()> {
    let observed = counts(entries, definition);
    if observed != expected {
        return Err(format!(
            "definition={definition}; expected counts={expected:?}; observed counts={observed:?}"
        )
        .into());
    }
    Ok(())
}

async fn move_item(
    client: &mut DevClientSession,
    source: ItemEntry,
    destination: To,
    count: Option<u32>,
    expected: ItemMoveOutcome,
) -> TestResult<()> {
    let result = client
        .session_mut()
        .move_item_counted(MoveRequest {
            source: source.handle,
            destination,
            count,
        })
        .await?;
    if result.outcome != expected {
        return Err(format!("move definition={} count={count:?} destination={destination:?}; expected={expected:?}; observed={result:?}", source.item_definition_ref).into());
    }
    Ok(())
}

async fn await_projection<T>(
    client: &mut DevClientSession,
    mut lookup: impl FnMut(&mut DevClientSession) -> TestResult<Option<T>>,
) -> TestResult<T> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(value) = lookup(client)? {
            return Ok(value);
        }
        if tokio::time::Instant::now() >= deadline {
            let inventory = format!("{:?}", client.inventory());
            let views = client.session_mut().container_views();
            return Err(
                format!("projection deadline; inventory={inventory}; views={views:?}").into(),
            );
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
}

async fn open_container(
    client: &mut DevClientSession,
    handle: ItemHandle,
) -> TestResult<tree::ContainerView> {
    let result = client.use_item(handle).await?;
    if result.status != CommandStatus::Accepted
        || result.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        return Err(format!("container USE observed={result:?}").into());
    }
    await_projection(client, |client| {
        Ok(client
            .session_mut()
            .container_views()
            .and_then(|views| {
                views
                    .views
                    .iter()
                    .find(|view| view.container_handle == handle)
            })
            .cloned())
    })
    .await
}

async fn wait_counts(
    client: &mut DevClientSession,
    nested: Option<ItemHandle>,
    definition: NonZeroU32,
    expected: &[u32],
) -> TestResult<Vec<ItemEntry>> {
    await_projection(client, |client| {
        let view = match nested {
            None => entries(client)?,
            Some(handle) => client
                .session_mut()
                .container_views()
                .and_then(|views| {
                    views
                        .views
                        .iter()
                        .find(|view| view.container_handle == handle)
                })
                .ok_or("nested view disappeared")?
                .entries
                .as_slice(),
        };
        Ok((counts(view, definition) == expected).then(|| view.to_vec()))
    })
    .await
}

async fn ground_round_trip(client: &mut DevClientSession, rope: NonZeroU32) -> TestResult<()> {
    let source = matching(entries(client)?, rope)?;
    assert_counts(entries(client)?, rope, &[1])?;
    let ActorPosition { x, y, floor } = client.world_spatial().actor_position;
    let position = WorldTilePosition { x, y, floor };
    move_item(
        client,
        source,
        To::Ground(position),
        None,
        ItemMoveOutcome::Moved,
    )
    .await?;
    wait_counts(client, None, rope, &[]).await?;

    let payload = browse::encode_browse_field_intent(&browse::BrowseFieldIntent::Open { position })
        .map_err(|error| format!("browse request: {error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(
            browse::CAPABILITY_BROWSE_FIELD_V1,
            browse::COMMAND_TYPE_BROWSE_FIELD_INTENT,
            &payload,
        )
        .await?;
    if response.status != CommandStatus::Accepted
        || tree::decode_container_view_result(&response.payload).ok()
            != Some(tree::ContainerViewOutcome::Opened)
    {
        return Err(format!("own-tile browse observed={response:?}").into());
    }
    let view = await_projection(client, |client| {
        Ok(client
            .session_mut()
            .container_views()
            .and_then(|views| {
                views
                    .views
                    .iter()
                    .find(|view| view.browse.is_some_and(|page| page.position == position))
            })
            .cloned())
    })
    .await?;
    assert_counts(&view.entries, rope, &[1])?;
    let ground = matching(&view.entries, rope)?;
    move_item(
        client,
        ground,
        To::MainBackpack,
        None,
        ItemMoveOutcome::Moved,
    )
    .await?;
    wait_counts(client, None, rope, &[1]).await?;
    await_projection(client, |client| {
        Ok(client
            .session_mut()
            .container_views()
            .and_then(|views| {
                views
                    .views
                    .iter()
                    .find(|current| current.view_id == view.view_id)
            })
            .filter(|current| counts(&current.entries, rope).is_empty())
            .map(|_| ()))
    })
    .await
}

async fn inventory_journey(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    step: &mut &'static str,
) -> TestResult<()> {
    let index = provisioning::item_index()?;
    ground_round_trip(client, provisioning::item_reference(&index, 3003)?).await?;

    *step = "nested-move";
    // Lubo sells a five-slot present for six gold. Six empty presents let us
    // fill one without needing five distinct definitions.
    let bags = provisioning::item_reference(&index, 2855)?;
    let coins = provisioning::item_reference(&index, 3031)?;
    if full_e2ef3_e2ei_d::carried_coin_count(&quantities(client)?, coins) < 38 {
        let (metadata, hunt_world) = full_e2ef3_e2ei_d::load_hunt_world(clients)?;
        full_e2ef3_e2ei_d::hunt_for_carried_gold(client, hunt_world, &metadata.spawns, coins, 38)
            .await?;
    }
    full_e2ef3_e2ei_d::buy_from_lubo(client, world, bags, coins, 6, 6).await?;
    assert_counts(entries(client)?, bags, &[1; 6])?;
    let bag = matching(entries(client)?, bags)?;
    let view = open_container(client, bag.handle).await?;
    let backpack = client
        .inventory()
        .and_then(|inv| inv.main_backpack)
        .ok_or("missing root backpack")?;
    if view.parent_handle != Some(backpack.handle) || view.capacity != 5 || !view.entries.is_empty()
    {
        return Err(
            format!("expected empty five-slot present under root; observed={view:?}").into(),
        );
    }
    let coin = matching(entries(client)?, coins)?;
    let total = coin.count.get();
    if total < 2 {
        return Err(format!(
            "starter or earned coin stack must contain at least two; observed={total}"
        )
        .into());
    }
    assert_counts(entries(client)?, coins, &[total])?;
    let into_bag = To::Container(bag.handle, None);
    move_item(client, coin, into_bag, None, ItemMoveOutcome::Moved).await?;
    wait_counts(client, None, coins, &[]).await?;
    let nested = wait_counts(client, Some(bag.handle), coins, &[total]).await?;

    *step = "split-stack";
    move_item(
        client,
        matching(&nested, coins)?,
        into_bag,
        Some(1),
        ItemMoveOutcome::Moved,
    )
    .await?;
    let nested = wait_counts(client, Some(bag.handle), coins, &[1, total - 1]).await?;
    assert_counts(entries(client)?, coins, &[])?;

    *step = "merge-stack";
    move_item(
        client,
        matching(&nested, coins)?,
        into_bag,
        None,
        ItemMoveOutcome::Moved,
    )
    .await?;
    wait_counts(client, Some(bag.handle), coins, &[total]).await?;
    assert_counts(entries(client)?, coins, &[])?;

    *step = "capacity-refusal";
    let spare_present = |client: &DevClientSession| -> TestResult<ItemEntry> {
        entries(client)?
            .iter()
            .find(|item| item.item_definition_ref == bags && item.handle != bag.handle)
            .copied()
            .ok_or_else(|| "bought present missing from root during fill".into())
    };
    for filled in 1..=4 {
        let empty = spare_present(client)?;
        move_item(client, empty, into_bag, None, ItemMoveOutcome::Moved).await?;
        wait_counts(client, Some(bag.handle), bags, &[1; 4][..filled]).await?;
    }
    let before = quantities(client)?;
    let empty = spare_present(client)?;
    move_item(client, empty, into_bag, None, ItemMoveOutcome::NoRoom).await?;
    // Reopen for an authoritative read after refusal.
    let result = client
        .session_mut()
        .container_view(tree::ContainerViewIntent::Close {
            view_id: view.view_id,
        })
        .await?;
    if result != Some(tree::ContainerViewOutcome::Closed) {
        return Err(format!("close filled bag observed={result:?}").into());
    }
    await_projection(client, |client| {
        Ok(client
            .session_mut()
            .container_views()
            .filter(|views| {
                views
                    .views
                    .iter()
                    .all(|current| current.view_id != view.view_id)
            })
            .map(|_| ()))
    })
    .await?;
    let filled = open_container(client, bag.handle).await?;
    assert_counts(&filled.entries, bags, &[1; 4])?;
    assert_counts(&filled.entries, coins, &[total])?;
    if filled.entries.len() != 5 || quantities(client)? != before {
        return Err(format!(
            "capacity refusal changed custody; inventory={:?}; nested={filled:?}",
            quantities(client)?
        )
        .into());
    }

    *step = "relogin-quantities";
    relogin_starter(client, clients).await?;
    if quantities(client)? != before {
        return Err(format!(
            "root quantities expected={before:?}; observed={:?}",
            quantities(client)?
        )
        .into());
    }
    // Two identical presents are now in the root. Reopen both to locate the
    // five-entry subtree by its contents using the new session's handles.
    let presents: Vec<_> = entries(client)?
        .iter()
        .filter(|entry| entry.item_definition_ref == bags)
        .copied()
        .collect();
    let mut full = None;
    for present in presents {
        let view = open_container(client, present.handle).await?;
        if view.entries.is_empty() {
            continue;
        }
        assert_counts(&view.entries, bags, &[1; 4])?;
        assert_counts(&view.entries, coins, &[total])?;
        if view.entries.len() != 5 || full.replace(view).is_some() {
            return Err(
                "fresh admission did not preserve one full present and one empty present".into(),
            );
        }
    }
    let full = full.ok_or("fresh admission omitted the present containing the merged stack")?;
    move_item(
        client,
        matching(&full.entries, coins)?,
        To::MainBackpack,
        None,
        ItemMoveOutcome::Moved,
    )
    .await?;
    wait_counts(client, None, coins, &[total]).await?;
    Ok(())
}

async fn inspect_instance(
    client: &mut DevClientSession,
    item: ItemEntry,
) -> TestResult<inspect::InspectedItem> {
    let payload = inspect::encode_request(inspect::Request::Item(item.handle))
        .map_err(|error| format!("inspection request: {error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(
            inspect::CAPABILITY_INSPECTION_V1,
            inspect::COMMAND_TYPE_INSPECTION,
            &payload,
        )
        .await?;
    let reply = inspect::decode_reply(&response.payload)
        .map_err(|error| format!("inspection reply: {error:?}"))?;
    if response.status != CommandStatus::Accepted || reply.status != 1 || reply.items.len() != 1 {
        return Err(format!("instance inspection observed={reply:?}").into());
    }
    let observed = reply
        .items
        .into_iter()
        .next()
        .ok_or("missing inspection item")?;
    if observed.definition_ref != item.item_definition_ref.get()
        || observed.count != item.count.get()
    {
        return Err(format!("expected inspected identity={item:?}; observed={observed:?}").into());
    }
    Ok(observed)
}

async fn imbuement_journey(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    step: &mut &'static str,
) -> TestResult<()> {
    let index = provisioning::item_index()?;
    // Check the physical shrine route before asking the starter to gather loot and gold.
    full_e2ef7::shrine(client, world).await?;
    *step = "obtain-basic-strike-materials";
    let recipe = oteryn_gameplay_items::imbuement::imbuement_recipe(1, 1)
        .ok_or("Basic Strike recipe absent")?;
    for &(item, count) in recipe.materials {
        full_e2ef7::acquire(client, world, &index, item, count)
            .await
            .map_err(|error| format!(
                "Basic Strike requires {count} Protective Charms ({item}) and {} gold; ordinary acquisition observed {error}; the town crop has no Imbuement Assistant placement and one Valkyrie spawn (32286,32121,7) with a 90000ms respawn and a 30974ppm one-charm drop",
                recipe.apply_gold,
            ))?;
    }
    *step = "obtain-sword-and-fee";
    let sword = full_e2ef7::acquire(client, world, &index, "oteryn:item.tibia.i3264", 1).await?;
    full_e2ef7::earn_gold(client, world, recipe.apply_gold).await?;
    *step = "inspect-applied-imbuement";
    full_e2ef7::shrine(client, world).await?;
    assert_counts(entries(client)?, sword, &[1])?;
    let target = matching(entries(client)?, sword)?;
    let before = inspect_instance(client, target).await?;
    full_e2ef7::apply_imbue(client, sword, 1, 1).await?;
    let applied = inspect_instance(client, target).await?;

    *step = "assert-family-tier-time-and-weight";
    let weight = ("Weight".to_owned(), "35.00 oz".to_owned());
    if !before.attributes.contains(&weight) || !applied.attributes.contains(&weight)
        || !before.attributes.iter().any(|(name, value)| name == "Imbuement Slot 1" && value == "empty")
        || !applied.attributes.iter().any(|(name, value)| name == "Imbuement Slot 1"
            && value == "Basic Strike (Raises crit hit damage by 15% and crit hit chance by 10%.), lasts 20:00h while fighting.")
    {
        return Err(format!("expected Basic Strike and original item weight; before={before:?}; observed={applied:?}").into());
    }

    *step = "relogin-imbuement";
    relogin_starter(client, clients).await?;
    assert_counts(entries(client)?, sword, &[1])?;
    let target = matching(entries(client)?, sword)?;
    let reloaded = inspect_instance(client, target).await?;
    if reloaded != applied {
        return Err(
            format!("imbuement inspection expected={applied:?}; observed={reloaded:?}").into(),
        );
    }
    Ok(())
}
